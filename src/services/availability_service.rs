use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::availability_repository::AvailabilityRepository;
use crate::domain::ports::conversation_repository::ConversationRepository;
use crate::{
    api::middleware::error::{ApiError, ApiResult},
    events::{EventBus, SystemEvent},
    models::{ActivityEventType, AgentActivityLog, AgentAvailability},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AvailabilityService {
    agent_repo: Arc<dyn AgentRepository>,
    availability_repo: Arc<dyn AvailabilityRepository>,
    conversation_repo: Arc<dyn ConversationRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl AvailabilityService {
    pub fn new(
        agent_repo: Arc<dyn AgentRepository>,
        availability_repo: Arc<dyn AvailabilityRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            agent_repo,
            availability_repo,
            conversation_repo,
            event_bus,
        }
    }

    /// Manually set agent availability status
    pub async fn set_availability(
        &self,
        agent_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        // Prevent manual setting of away_and_reassigning
        if status == AgentAvailability::AwayAndReassigning {
            return Err(ApiError::BadRequest(
                "Cannot manually set away_and_reassigning status".to_string(),
            ));
        }

        // Get current agent to check old status
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        let old_status = agent.availability_status;

        // Update status
        self.availability_repo
            .update_agent_availability_with_timestamp(&agent.id, status)
            .await?;

        // Update activity timestamp when going online
        if status == AgentAvailability::Online {
            self.availability_repo.update_agent_activity(&agent.id).await?;
        }

        // Log activity
        self.log_activity(
            &agent.id,
            ActivityEventType::AvailabilityChanged,
            Some(old_status),
            Some(status),
        )
        .await?;

        // Emit event
        let now = chrono::Utc::now().to_rfc3339();
        let _ = self
            .event_bus
            .publish(SystemEvent::AgentAvailabilityChanged {
                agent_id: agent.id.clone(),
                old_status: old_status.to_string(),
                new_status: status.to_string(),
                timestamp: now,
                reason: "manual".to_string(),
            });

        tracing::info!(
            "Agent {} availability changed from {} to {} (manual)",
            agent.id,
            old_status,
            status
        );

        Ok(())
    }

    /// Record agent activity (updates last_activity_at)
    pub async fn record_activity(&self, agent_id: &str) -> ApiResult<()> {
        self.availability_repo.update_agent_activity(agent_id).await
    }

    /// Handle agent login - set online, update timestamps
    pub async fn handle_login(&self, user_id: &str) -> ApiResult<()> {
        let agent = self
            .agent_repo
            .get_agent_by_user_id(user_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        let old_status = agent.availability_status;

        // Set online
        self.availability_repo
            .update_agent_availability_with_timestamp(&agent.id, AgentAvailability::Online)
            .await?;

        // Update login and activity timestamps
        self.availability_repo.update_agent_last_login(&agent.id).await?;
        self.availability_repo.update_agent_activity(&agent.id).await?;

        // Log activity
        self.log_activity(&agent.id, ActivityEventType::AgentLogin, None, None)
            .await?;

        // Emit event
        let now = chrono::Utc::now().to_rfc3339();
        let _ = self.event_bus.publish(SystemEvent::AgentLoggedIn {
            agent_id: agent.id.clone(),
            user_id: user_id.to_string(),
            timestamp: now.clone(),
        });

        // Also emit availability changed if status changed
        if old_status != AgentAvailability::Online {
            let _ = self
                .event_bus
                .publish(SystemEvent::AgentAvailabilityChanged {
                    agent_id: agent.id.clone(),
                    old_status: old_status.to_string(),
                    new_status: "online".to_string(),
                    timestamp: now,
                    reason: "login".to_string(),
                });
        }

        tracing::info!("Agent {} logged in and went online", agent.id);

        Ok(())
    }

    /// Handle agent logout - set offline, log event
    pub async fn handle_logout(&self, user_id: &str) -> ApiResult<()> {
        let agent = self
            .agent_repo
            .get_agent_by_user_id(user_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        let old_status = agent.availability_status;

        // Set offline
        self.availability_repo
            .update_agent_availability_with_timestamp(&agent.id, AgentAvailability::Offline)
            .await?;

        // Log activity
        self.log_activity(&agent.id, ActivityEventType::AgentLogout, None, None)
            .await?;

        // Emit event
        let now = chrono::Utc::now().to_rfc3339();
        let _ = self.event_bus.publish(SystemEvent::AgentLoggedOut {
            agent_id: agent.id.clone(),
            user_id: user_id.to_string(),
            timestamp: now.clone(),
        });

        // Also emit availability changed
        let _ = self
            .event_bus
            .publish(SystemEvent::AgentAvailabilityChanged {
                agent_id: agent.id.clone(),
                old_status: old_status.to_string(),
                new_status: "offline".to_string(),
                timestamp: now,
                reason: "logout".to_string(),
            });

        tracing::info!("Agent {} logged out and went offline", agent.id);

        Ok(())
    }

    /// Check for online agents who exceeded inactivity timeout
    pub async fn check_inactivity_timeouts(&self) -> ApiResult<Vec<String>> {
        let threshold = self.load_inactivity_timeout().await?;
        let agents = self.availability_repo.get_inactive_online_agents(threshold).await?;

        let mut affected = Vec::new();

        for agent in agents {
            // Transition to away
            self.availability_repo
                .update_agent_availability_with_timestamp(&agent.id, AgentAvailability::Away)
                .await?;

            // Log activity
            self.log_activity(
                &agent.id,
                ActivityEventType::AvailabilityChanged,
                Some(AgentAvailability::Online),
                Some(AgentAvailability::Away),
            )
            .await?;

            // Emit event
            let now = chrono::Utc::now().to_rfc3339();
            let _ = self
                .event_bus
                .publish(SystemEvent::AgentAvailabilityChanged {
                    agent_id: agent.id.clone(),
                    old_status: "online".to_string(),
                    new_status: "away".to_string(),
                    timestamp: now,
                    reason: "inactivity_timeout".to_string(),
                });

            tracing::info!("Agent {} transitioned to away due to inactivity", agent.id);
            affected.push(agent.id);
        }

        Ok(affected)
    }

    /// Check for away agents who exceeded max idle threshold
    pub async fn check_max_idle_thresholds(&self) -> ApiResult<Vec<String>> {
        let threshold = self.load_max_idle_threshold().await?;
        let agents = self.availability_repo.get_idle_away_agents(threshold).await?;

        let mut affected = Vec::new();

        for agent in agents {
            // Transition to away_and_reassigning
            self.availability_repo
                .update_agent_availability_with_timestamp(
                    &agent.id,
                    AgentAvailability::AwayAndReassigning,
                )
                .await?;

            // Unassign all open conversations (this method already handles open/snoozed filtering)
            let unassigned_count = self
                .conversation_repo
                .unassign_agent_open_conversations(&agent.user_id)
                .await?;

            tracing::info!(
                "Unassigned {} open conversations from agent {}",
                unassigned_count,
                agent.id
            );

            // Note: The existing assignment system already emits ConversationUnassigned events
            // via the assignment_history trigger or service layer

            // Transition to offline
            self.availability_repo
                .update_agent_availability_with_timestamp(&agent.id, AgentAvailability::Offline)
                .await?;

            // Log activity
            self.log_activity(
                &agent.id,
                ActivityEventType::AvailabilityChanged,
                Some(agent.availability_status),
                Some(AgentAvailability::Offline),
            )
            .await?;

            // Emit event
            let now = chrono::Utc::now().to_rfc3339();
            let _ = self
                .event_bus
                .publish(SystemEvent::AgentAvailabilityChanged {
                    agent_id: agent.id.clone(),
                    old_status: agent.availability_status.to_string(),
                    new_status: "offline".to_string(),
                    timestamp: now,
                    reason: "max_idle_threshold".to_string(),
                });

            tracing::info!(
                "Agent {} reached max idle threshold, conversations unassigned, went offline",
                agent.id
            );
            affected.push(agent.id);
        }

        Ok(affected)
    }

    /// Get agent availability info
    pub async fn get_availability(
        &self,
        agent_id: &str,
    ) -> ApiResult<crate::models::AvailabilityResponse> {
        let agent = self
            .agent_repo
            .get_agent_by_user_id(agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        Ok(crate::models::AvailabilityResponse {
            agent_id: agent.id,
            availability_status: agent.availability_status,
            last_activity_at: agent.last_activity_at,
            away_since: agent.away_since,
        })
    }

    /// Get agent activity logs
    pub async fn get_activity_logs(
        &self,
        agent_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<crate::models::ActivityLogResponse> {
        let (logs, total) = self
            .availability_repo
            .get_agent_activity_logs(agent_id, limit, offset)
            .await?;

        let page = (offset / limit) + 1;
        let total_pages = (total as f64 / limit as f64).ceil() as i64;

        Ok(crate::models::ActivityLogResponse {
            logs,
            total,
            pagination: crate::models::PaginationMetadata {
                page,
                per_page: limit,
                total_count: total,
                total_pages,
            },
        })
    }

    // Private helpers

    /// Log activity to database
    async fn log_activity(
        &self,
        agent_id: &str,
        event_type: ActivityEventType,
        old_status: Option<AgentAvailability>,
        new_status: Option<AgentAvailability>,
    ) -> ApiResult<()> {
        let log = AgentActivityLog::new(
            agent_id.to_string(),
            event_type,
            old_status.map(|s| s.to_string()),
            new_status.map(|s| s.to_string()),
            None, // metadata
        );

        self.availability_repo.create_activity_log(&log).await
    }

    /// Load inactivity timeout from config or env
    async fn load_inactivity_timeout(&self) -> ApiResult<i64> {
        // Try environment variable first
        if let Ok(value) = std::env::var("INACTIVITY_TIMEOUT_SECONDS") {
            return value
                .parse()
                .map_err(|_| ApiError::Internal("Invalid INACTIVITY_TIMEOUT_SECONDS".to_string()));
        }

        // Try database config
        if let Some(value) = self
            .availability_repo
            .get_config_value("availability.inactivity_timeout_seconds")
            .await?
        {
            return value.parse().map_err(|_| {
                ApiError::Internal("Invalid inactivity timeout in config".to_string())
            });
        }

        // Default: 5 minutes
        Ok(300)
    }

    /// Load max idle threshold from config or env
    async fn load_max_idle_threshold(&self) -> ApiResult<i64> {
        // Try environment variable first
        if let Ok(value) = std::env::var("MAX_IDLE_THRESHOLD_SECONDS") {
            return value
                .parse()
                .map_err(|_| ApiError::Internal("Invalid MAX_IDLE_THRESHOLD_SECONDS".to_string()));
        }

        // Try database config
        if let Some(value) = self
            .availability_repo
            .get_config_value("availability.max_idle_threshold_seconds")
            .await?
        {
            return value.parse().map_err(|_| {
                ApiError::Internal("Invalid max idle threshold in config".to_string())
            });
        }

        // Default: 30 minutes
        Ok(1800)
    }
}
