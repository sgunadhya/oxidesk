use crate::domain::ports::{
    agent_repository::AgentRepository, assignment_repository::AssignmentRepository,
    availability_repository::AvailabilityRepository,
    conversation_repository::ConversationRepository, role_repository::RoleRepository,
    team_repository::TeamRepository, user_repository::UserRepository,
};
use crate::{
    infrastructure::http::middleware::error::{ApiError, ApiResult},
    shared::events::{EventBus, SystemEvent},
    domain::entities::{
        AgentAvailability, AssignmentHistory, Conversation, ConversationStatus, Permission,
        UserNotification,
    },
    infrastructure::providers::connection_manager::ConnectionManager,
    application::services::{NotificationService, SlaService},
};
use std::sync::Arc;

/// Service for handling conversation assignment logic
#[derive(Clone)]
pub struct AssignmentService {
    assignment_repo: Arc<dyn AssignmentRepository>,
    conversation_repo: Arc<dyn ConversationRepository>,
    agent_repo: Arc<dyn AgentRepository>,
    user_repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
    team_repo: Arc<dyn TeamRepository>,
    availability_repo: Arc<dyn AvailabilityRepository>,
    event_bus: Arc<dyn EventBus>,
    notification_service: NotificationService,
    connection_manager: Arc<dyn ConnectionManager>,
    sla_service: Option<Arc<SlaService>>,
}

impl AssignmentService {
    pub fn new(
        assignment_repo: Arc<dyn AssignmentRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        agent_repo: Arc<dyn AgentRepository>,
        user_repo: Arc<dyn UserRepository>,
        role_repo: Arc<dyn RoleRepository>,
        team_repo: Arc<dyn TeamRepository>,
        availability_repo: Arc<dyn AvailabilityRepository>,
        event_bus: Arc<dyn EventBus>,
        notification_service: NotificationService,
        connection_manager: Arc<dyn ConnectionManager>,
    ) -> Self {
        Self {
            assignment_repo,
            conversation_repo,
            agent_repo,
            user_repo,
            role_repo,
            team_repo,
            availability_repo,
            event_bus,
            notification_service,
            connection_manager,
            sla_service: None,
        }
    }

    /// Set the SLA service (called after initialization to avoid circular dependencies)
    pub fn set_sla_service(&mut self, sla_service: Arc<SlaService>) {
        self.sla_service = Some(sla_service);
    }

    // Helper: Check if user has permission
    fn has_permission(&self, permissions: &[Permission], required: &str) -> bool {
        permissions.iter().any(|p| p.name == required)
    }

    // User Story 1: Self-assignment
    pub async fn self_assign_conversation(
        &self,
        conversation_id: &str,
        agent_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Conversation> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_user_assignee") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_user_assignee".to_string(),
            ));
        }

        // 2. Verify conversation exists and check for idempotency
        let conversation = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // Idempotency check: If already assigned to this agent, return success
        if conversation.assigned_user_id.as_ref() == Some(&agent_id.to_string()) {
            tracing::info!(
                "Conversation {} already assigned to agent {} (idempotent request)",
                conversation_id,
                agent_id
            );
            return Ok(conversation);
        }

        // 3. Assign to database with retry logic for optimistic concurrency conflicts
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAYS_MS: [u64; 3] = [50, 100, 200]; // Exponential backoff

        for attempt in 0..=MAX_RETRIES {
            match self
                .conversation_repo
                .assign_conversation_to_user(
                    conversation_id,
                    Some(agent_id.to_string()),
                    Some(agent_id.to_string()),
                )
                .await
            {
                Ok(_) => break, // Success - exit retry loop
                Err(ApiError::Conflict(_)) if attempt < MAX_RETRIES => {
                    // Conflict detected - retry with exponential backoff
                    let delay_ms = RETRY_DELAYS_MS[attempt as usize];
                    tracing::info!(
                        "Assignment conflict on attempt {} for conversation {}, retrying in {}ms",
                        attempt + 1,
                        conversation_id,
                        delay_ms
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(e), // Other error or max retries exceeded
            }
        }

        // 4. Add as participant (ignore if already exists)
        let _ = self
            .conversation_repo
            .add_conversation_participant(conversation_id, agent_id, "assignee")
            .await;

        // 5. Record in history
        let history = AssignmentHistory::new(
            conversation_id.to_string(),
            Some(agent_id.to_string()),
            None,
            agent_id.to_string(),
        );
        self.assignment_repo.record_assignment(&history).await?;

        // 6. Publish event
        self.event_bus.publish(SystemEvent::ConversationAssigned {
            conversation_id: conversation_id.to_string(),
            assigned_user_id: Some(agent_id.to_string()),
            assigned_team_id: None,
            assigned_by: agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 7. Create notification in database
        let notification = UserNotification::new_assignment(
            agent_id.to_string(),
            conversation_id.to_string(),
            agent_id.to_string(), // Self-assignment: assigner = assignee
        );

        if let Err(e) = self.assignment_repo.create_notification(&notification).await {
            tracing::error!("Failed to create assignment notification: {}", e);
            // Continue execution - notification failure shouldn't fail assignment
        }

        // 8. Send real-time notification (best-effort, fire-and-forget)
        let connection_manager = self.connection_manager.clone();
        let notification_clone = notification.clone();
        tokio::spawn(async move {
            if let Err(e) = NotificationService::send_realtime_notification(
                &notification_clone,
                &connection_manager,
            )
            .await
            {
                tracing::debug!("Failed to send real-time notification: {}", e);
                // This is expected if user is not connected - notification is still in DB
            }
        });

        // 9. Return updated conversation
        self.conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after assignment".to_string())
            })
    }

    // User Story 2: Agent-to-agent assignment
    pub async fn assign_conversation_to_agent(
        &self,
        conversation_id: &str,
        target_agent_id: &str,
        assigning_agent_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Conversation> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_user_assignee") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_user_assignee".to_string(),
            ));
        }

        // 2. Verify conversation exists and check for idempotency
        let conversation = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Verify target agent exists
        let _target_agent = self
            .agent_repo
            .get_agent_by_user_id(target_agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", target_agent_id)))?;

        // Idempotency check: If already assigned to target agent, return success
        if conversation.assigned_user_id.as_ref() == Some(&target_agent_id.to_string()) {
            tracing::info!(
                "Conversation {} already assigned to agent {} (idempotent request)",
                conversation_id,
                target_agent_id
            );
            return Ok(conversation);
        }

        // 4. Assign to database with retry logic for optimistic concurrency conflicts
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAYS_MS: [u64; 3] = [50, 100, 200]; // Exponential backoff

        for attempt in 0..=MAX_RETRIES {
            match self
                .conversation_repo
                .assign_conversation_to_user(
                    conversation_id,
                    Some(target_agent_id.to_string()),
                    Some(assigning_agent_id.to_string()),
                )
                .await
            {
                Ok(_) => break, // Success - exit retry loop
                Err(ApiError::Conflict(_)) if attempt < MAX_RETRIES => {
                    // Conflict detected - retry with exponential backoff
                    let delay_ms = RETRY_DELAYS_MS[attempt as usize];
                    tracing::info!(
                        "Assignment conflict on attempt {} for conversation {}, retrying in {}ms",
                        attempt + 1,
                        conversation_id,
                        delay_ms
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(e), // Other error or max retries exceeded
            }
        }

        // 5. Add target agent as participant (ignore if already exists)
        let _ = self
            .conversation_repo
            .add_conversation_participant(conversation_id, target_agent_id, "assignee")
            .await;

        // 6. Record in history
        let history = AssignmentHistory::new(
            conversation_id.to_string(),
            Some(target_agent_id.to_string()),
            None,
            assigning_agent_id.to_string(),
        );
        self.assignment_repo.record_assignment(&history).await?;

        // 7. Publish event
        self.event_bus.publish(SystemEvent::ConversationAssigned {
            conversation_id: conversation_id.to_string(),
            assigned_user_id: Some(target_agent_id.to_string()),
            assigned_team_id: None,
            assigned_by: assigning_agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 8. Create notification in database
        let notification = UserNotification::new_assignment(
            target_agent_id.to_string(),
            conversation_id.to_string(),
            assigning_agent_id.to_string(),
        );

        if let Err(e) = self.assignment_repo.create_notification(&notification).await {
            tracing::error!("Failed to create assignment notification: {}", e);
            // Continue execution - notification failure shouldn't fail assignment
        }

        // 9. Send real-time notification (best-effort, fire-and-forget)
        let connection_manager = self.connection_manager.clone();
        let notification_clone = notification.clone();
        tokio::spawn(async move {
            if let Err(e) = NotificationService::send_realtime_notification(
                &notification_clone,
                &connection_manager,
            )
            .await
            {
                tracing::debug!("Failed to send real-time notification: {}", e);
                // This is expected if user is not connected - notification is still in DB
            }
        });

        // 10. Return updated conversation
        self.conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after assignment".to_string())
            })
    }

    // User Story 3: Team assignment
    pub async fn assign_conversation_to_team(
        &self,
        conversation_id: &str,
        team_id: &str,
        assigning_agent_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Conversation> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_team_assignee") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_team_assignee".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let _conversation = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Verify team exists
        let _team = self
            .team_repo
            .get_team_by_id(team_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Team {} not found", team_id)))?;

        // 4. Assign to database
        self.conversation_repo
            .assign_conversation_to_team(
                conversation_id,
                Some(team_id.to_string()),
                Some(assigning_agent_id.to_string()),
            )
            .await?;

        // 5. Apply team SLA (stub)
        self.apply_team_sla(conversation_id, team_id).await?;

        // 6. Record in history
        let history = AssignmentHistory::new(
            conversation_id.to_string(),
            None,
            Some(team_id.to_string()),
            assigning_agent_id.to_string(),
        );
        self.assignment_repo.record_assignment(&history).await?;

        // 7. Publish event
        self.event_bus.publish(SystemEvent::ConversationAssigned {
            conversation_id: conversation_id.to_string(),
            assigned_user_id: None,
            assigned_team_id: Some(team_id.to_string()),
            assigned_by: assigning_agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 8. Return updated conversation
        self.conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after assignment".to_string())
            })
    }

    /// Apply team's SLA policy to conversation
    async fn apply_team_sla(&self, conversation_id: &str, team_id: &str) -> ApiResult<()> {
        // Get the team
        let team = self
            .team_repo
            .get_team_by_id(team_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Team not found: {}", team_id)))?;

        // Check if team has an SLA policy
        if let Some(sla_policy_id) = team.sla_policy_id {
            // Get SLA service
            if let Some(sla_service) = &self.sla_service {
                // Get conversation to get its created_at timestamp
                let conversation = self
                    .conversation_repo
                    .get_conversation_by_id(conversation_id)
                    .await?
                    .ok_or_else(|| {
                        ApiError::NotFound(format!("Conversation not found: {}", conversation_id))
                    })?;

                // Apply SLA using conversation's created_at as base timestamp
                sla_service
                    .apply_sla(conversation_id, &sla_policy_id, &conversation.created_at)
                    .await?;

                tracing::info!(
                    "Applied SLA policy {} from team {} to conversation {}",
                    sla_policy_id,
                    team_id,
                    conversation_id
                );
            } else {
                tracing::warn!("SLA service not initialized, skipping SLA application");
            }
        } else {
            tracing::debug!(
                "Team {} has no SLA policy, skipping SLA application",
                team_id
            );
        }

        Ok(())
    }

    // User Story 4: Manual unassignment
    pub async fn unassign_conversation(
        &self,
        conversation_id: &str,
        agent_id: &str,
    ) -> ApiResult<Conversation> {
        // 1. Get conversation
        let conversation = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 2. Verify agent is currently assigned
        if conversation.assigned_user_id.as_ref() != Some(&agent_id.to_string()) {
            return Err(ApiError::BadRequest(
                "You are not assigned to this conversation".to_string(),
            ));
        }

        // 3. Unassign from database
        self.conversation_repo.unassign_conversation_user(conversation_id).await?;

        // 4. Publish event
        self.event_bus.publish(SystemEvent::ConversationUnassigned {
            conversation_id: conversation_id.to_string(),
            previous_assigned_user_id: Some(agent_id.to_string()),
            previous_assigned_team_id: conversation.assigned_team_id.clone(),
            unassigned_by: agent_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // 5. Return updated conversation
        self.conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::Internal("Conversation disappeared after unassignment".to_string())
            })
    }

    // User Story 5: Automatic unassignment on away
    pub async fn auto_unassign_on_away(&self, agent_id: &str) -> ApiResult<Vec<Conversation>> {
        // 1. Get all open/snoozed conversations assigned to agent before unassignment
        let (conversations, _) = self
            .conversation_repo
            .get_user_assigned_conversations(agent_id, 1000, 0)
            .await?;

        // Filter to only open/snoozed
        let open_conversations: Vec<_> = conversations
            .into_iter()
            .filter(|c| {
                matches!(
                    c.status,
                    ConversationStatus::Open | ConversationStatus::Snoozed
                )
            })
            .collect();

        // 2. Batch unassign
        let count = self.conversation_repo.unassign_agent_open_conversations(agent_id).await?;

        // 3. Publish ConversationUnassigned event for each
        for conversation in &open_conversations {
            self.event_bus.publish(SystemEvent::ConversationUnassigned {
                conversation_id: conversation.id.clone(),
                previous_assigned_user_id: Some(agent_id.to_string()),
                previous_assigned_team_id: conversation.assigned_team_id.clone(),
                unassigned_by: agent_id.to_string(), // System-triggered but by agent's action
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        tracing::info!(
            "Auto-unassigned {} conversations for agent {}",
            count,
            agent_id
        );

        // 4. Return list of unassigned conversations
        Ok(open_conversations)
    }

    // Get user permissions from database
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        self.role_repo.get_user_permissions(user_id).await
    }

    // Check if user is a team member
    pub async fn is_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool> {
        self.team_repo.is_team_member(team_id, user_id).await
    }

    // Get unassigned conversations
    pub async fn get_unassigned_conversations(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.conversation_repo.get_unassigned_conversations(limit, offset).await
    }

    // Get conversations assigned to a user
    pub async fn get_user_assigned_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.conversation_repo
            .get_user_assigned_conversations(user_id, limit, offset)
            .await
    }

    // Get team conversations
    pub async fn get_team_conversations(
        &self,
        team_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.conversation_repo.get_team_conversations(team_id, limit, offset).await
    }

    // Update agent availability
    pub async fn update_agent_availability(
        &self,
        user_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        self.availability_repo.update_agent_availability_with_timestamp(user_id, status).await
    }

    // RBAC System: Check conversation access based on assignment
    /// Check if user has access to a conversation based on assignment
    /// Used for assignment-based filtering with "conversations:read_assigned" permission
    ///
    /// Returns true if:
    /// - Conversation is assigned directly to the user (assigned_user_id == user_id), OR
    /// - Conversation is assigned to a team the user is a member of (assigned_team_id IN user_teams)
    pub async fn has_conversation_access(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> ApiResult<bool> {
        // Get conversation assignment
        let conversation = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // Check direct user assignment
        if let Some(assigned_user_id) = &conversation.assigned_user_id {
            if assigned_user_id == user_id {
                return Ok(true);
            }
        }

        // Check team assignment
        if let Some(assigned_team_id) = &conversation.assigned_team_id {
            // Get user's teams
            let user_teams = self.get_user_teams(user_id).await?;
            if user_teams.iter().any(|team_id| team_id == assigned_team_id) {
                return Ok(true);
            }
        }

        // No assignment match
        Ok(false)
    }

    /// Get all team IDs that a user is a member of
    /// Used for team-based assignment filtering
    pub async fn get_user_teams(&self, user_id: &str) -> ApiResult<Vec<String>> {
        let teams = self.team_repo.get_user_teams(user_id).await?;
        Ok(teams.into_iter().map(|team| team.id).collect())
    }
}
