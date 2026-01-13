use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    events::{EventBus, SystemEvent},
    models::*,
};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Service for managing SLA policies, applied SLAs, and SLA events
pub struct SlaService {
    db: Database,
    event_bus: Arc<RwLock<EventBus>>,
}

impl SlaService {
    /// Create a new SLA service
    pub fn new(db: Database, event_bus: Arc<RwLock<EventBus>>) -> Self {
        Self { db, event_bus }
    }

    // ========================================
    // SLA Policy Management
    // ========================================

    /// Create a new SLA policy
    pub async fn create_policy(
        &self,
        name: String,
        description: Option<String>,
        first_response_time: String,
        resolution_time: String,
        next_response_time: String,
    ) -> ApiResult<SlaPolicy> {
        // Validate duration formats
        parse_duration(&first_response_time).map_err(|e| {
            ApiError::BadRequest(format!("Invalid first_response_time: {}", e))
        })?;
        parse_duration(&resolution_time)
            .map_err(|e| ApiError::BadRequest(format!("Invalid resolution_time: {}", e)))?;
        parse_duration(&next_response_time)
            .map_err(|e| ApiError::BadRequest(format!("Invalid next_response_time: {}", e)))?;

        let policy = SlaPolicy::new(
            name,
            description,
            first_response_time,
            resolution_time,
            next_response_time,
        );

        self.db.create_sla_policy(&policy).await?;

        info!("Created SLA policy: {} ({})", policy.name, policy.id);
        Ok(policy)
    }

    /// Get SLA policy by ID
    pub async fn get_policy(&self, policy_id: &str) -> ApiResult<Option<SlaPolicy>> {
        self.db.get_sla_policy(policy_id).await
    }

    /// Get SLA policy by name
    pub async fn get_policy_by_name(&self, name: &str) -> ApiResult<Option<SlaPolicy>> {
        self.db.get_sla_policy_by_name(name).await
    }

    /// List all SLA policies
    pub async fn list_policies(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<SlaPolicy>, i64)> {
        self.db.list_sla_policies(limit, offset).await
    }

    /// Update SLA policy
    pub async fn update_policy(
        &self,
        policy_id: &str,
        name: Option<String>,
        description: Option<Option<String>>,
        first_response_time: Option<String>,
        resolution_time: Option<String>,
        next_response_time: Option<String>,
    ) -> ApiResult<()> {
        // Validate duration formats if provided
        if let Some(ref time) = first_response_time {
            parse_duration(time).map_err(|e| {
                ApiError::BadRequest(format!("Invalid first_response_time: {}", e))
            })?;
        }
        if let Some(ref time) = resolution_time {
            parse_duration(time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid resolution_time: {}", e)))?;
        }
        if let Some(ref time) = next_response_time {
            parse_duration(time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid next_response_time: {}", e)))?;
        }

        self.db
            .update_sla_policy(
                policy_id,
                name.as_deref(),
                description.as_ref().map(|o| o.as_deref()),
                first_response_time.as_deref(),
                resolution_time.as_deref(),
                next_response_time.as_deref(),
            )
            .await?;

        info!("Updated SLA policy: {}", policy_id);
        Ok(())
    }

    /// Delete SLA policy
    pub async fn delete_policy(&self, policy_id: &str) -> ApiResult<()> {
        self.db.delete_sla_policy(policy_id).await?;
        info!("Deleted SLA policy: {}", policy_id);
        Ok(())
    }

    // ========================================
    // Applied SLA Management
    // ========================================

    /// Get applied SLA by ID
    pub async fn get_applied_sla(&self, applied_sla_id: &str) -> ApiResult<Option<AppliedSla>> {
        self.db.get_applied_sla(applied_sla_id).await
    }

    /// Get applied SLA by conversation ID
    pub async fn get_applied_sla_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Option<AppliedSla>> {
        self.db
            .get_applied_sla_by_conversation(conversation_id)
            .await
    }

    /// List applied SLAs with optional status filter
    pub async fn list_applied_slas(
        &self,
        status_filter: Option<AppliedSlaStatus>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<AppliedSla>, i64)> {
        self.db
            .list_applied_slas(status_filter, limit, offset)
            .await
    }

    // ========================================
    // SLA Event Management
    // ========================================

    /// Get SLA event by ID
    pub async fn get_event(&self, event_id: &str) -> ApiResult<Option<SlaEvent>> {
        self.db.get_sla_event(event_id).await
    }

    /// Get all SLA events for an applied SLA
    pub async fn get_events_by_applied_sla(
        &self,
        applied_sla_id: &str,
    ) -> ApiResult<Vec<SlaEvent>> {
        self.db.get_sla_events_by_applied_sla(applied_sla_id).await
    }

    /// Get pending SLA event by type for an applied SLA
    pub async fn get_pending_event(
        &self,
        applied_sla_id: &str,
        event_type: SlaEventType,
    ) -> ApiResult<Option<SlaEvent>> {
        self.db
            .get_pending_sla_event(applied_sla_id, event_type)
            .await
    }

    // ========================================
    // SLA Application
    // ========================================

    /// Apply SLA policy to a conversation
    /// Creates an applied SLA with calculated deadlines and creates initial SLA events
    pub async fn apply_sla(
        &self,
        conversation_id: &str,
        policy_id: &str,
        base_timestamp: &str,
    ) -> ApiResult<AppliedSla> {
        // Get the policy
        let policy = self
            .get_policy(policy_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("SLA policy not found: {}", policy_id)))?;

        // Calculate deadlines
        let first_response_deadline = self.calculate_deadline(base_timestamp, &policy.first_response_time)?;
        let resolution_deadline = self.calculate_deadline(base_timestamp, &policy.resolution_time)?;

        // Create applied SLA
        let applied_sla = AppliedSla::new(
            conversation_id.to_string(),
            policy_id.to_string(),
            first_response_deadline.clone(),
            resolution_deadline.clone(),
        );

        self.db.create_applied_sla(&applied_sla).await?;

        // Create SLA events for first response and resolution
        let first_response_event = SlaEvent::new(
            applied_sla.id.clone(),
            SlaEventType::FirstResponse,
            first_response_deadline.clone(),
        );

        let resolution_event = SlaEvent::new(
            applied_sla.id.clone(),
            SlaEventType::Resolution,
            resolution_deadline.clone(),
        );

        self.db.create_sla_event(&first_response_event).await?;
        self.db.create_sla_event(&resolution_event).await?;

        info!(
            "Applied SLA policy {} to conversation {} (first_response: {}, resolution: {})",
            policy_id, conversation_id, first_response_deadline, resolution_deadline
        );

        Ok(applied_sla)
    }

    /// Handle agent message - mark first response and next response as met if pending
    pub async fn handle_agent_message(
        &self,
        conversation_id: &str,
        _agent_id: &str,
        message_timestamp: &str,
    ) -> ApiResult<()> {
        // Get applied SLA for this conversation
        let applied_sla = match self
            .db
            .get_applied_sla_by_conversation(conversation_id)
            .await?
        {
            Some(sla) => sla,
            None => {
                // No SLA applied to this conversation
                return Ok(());
            }
        };

        // Try to mark first response event as met (only if pending)
        if let Some(first_response_event) = self
            .db
            .get_pending_sla_event(&applied_sla.id, SlaEventType::FirstResponse)
            .await?
        {
            self.db
                .mark_sla_event_met(&first_response_event.id, message_timestamp)
                .await?;

            info!(
                "First response SLA met for conversation {} at {}",
                conversation_id, message_timestamp
            );
        }

        // Try to mark next response event as met (only if pending)
        if let Some(next_response_event) = self
            .db
            .get_pending_sla_event(&applied_sla.id, SlaEventType::NextResponse)
            .await?
        {
            self.db
                .mark_sla_event_met(&next_response_event.id, message_timestamp)
                .await?;

            info!(
                "Next response SLA met for conversation {} at {}",
                conversation_id, message_timestamp
            );
        }

        Ok(())
    }

    /// Handle conversation resolved - mark resolution as met if pending
    pub async fn handle_conversation_resolved(
        &self,
        conversation_id: &str,
        resolution_timestamp: &str,
    ) -> ApiResult<()> {
        // Get applied SLA for this conversation
        let applied_sla = match self
            .db
            .get_applied_sla_by_conversation(conversation_id)
            .await?
        {
            Some(sla) => sla,
            None => {
                // No SLA applied to this conversation
                return Ok(());
            }
        };

        // Get the resolution event (only if pending)
        let resolution_event = match self
            .db
            .get_pending_sla_event(&applied_sla.id, SlaEventType::Resolution)
            .await?
        {
            Some(event) => event,
            None => {
                // Resolution event is already met or breached
                // Don't change breached events to met
                return Ok(());
            }
        };

        // Mark the event as met
        self.db
            .mark_sla_event_met(&resolution_event.id, resolution_timestamp)
            .await?;

        // Update the applied SLA status (worst outcome logic)
        self.update_applied_sla_status(&applied_sla.id).await?;

        info!(
            "Resolution SLA met for conversation {} at {}",
            conversation_id, resolution_timestamp
        );

        Ok(())
    }

    /// Handle contact message - create next response event
    pub async fn handle_contact_message(
        &self,
        conversation_id: &str,
        _contact_id: &str,
        message_timestamp: &str,
    ) -> ApiResult<()> {
        // Get applied SLA for this conversation
        let applied_sla = match self
            .db
            .get_applied_sla_by_conversation(conversation_id)
            .await?
        {
            Some(sla) => sla,
            None => {
                // No SLA applied to this conversation
                return Ok(());
            }
        };

        // Get the SLA policy to know the next_response_time
        let policy = self
            .get_policy(&applied_sla.sla_policy_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "SLA policy not found: {}",
                    applied_sla.sla_policy_id
                ))
            })?;

        // Calculate deadline for next response
        let next_response_deadline =
            self.calculate_deadline(message_timestamp, &policy.next_response_time)?;

        // Create next response event
        let next_response_event = SlaEvent::new(
            applied_sla.id.clone(),
            SlaEventType::NextResponse,
            next_response_deadline.clone(),
        );

        self.db.create_sla_event(&next_response_event).await?;

        info!(
            "Next response SLA event created for conversation {} (deadline: {})",
            conversation_id, next_response_deadline
        );

        Ok(())
    }

    /// Check for breached SLA events and update their status
    pub async fn check_breaches(&self) -> ApiResult<()> {
        // Get all pending events past their deadline
        let breached_events = self.db.get_pending_events_past_deadline().await?;

        if breached_events.is_empty() {
            return Ok(());
        }

        info!("Found {} breached SLA events to process", breached_events.len());

        for event in breached_events {
            // Get the applied SLA to get conversation_id
            let applied_sla = self.db.get_applied_sla_by_id(&event.applied_sla_id).await?
                .ok_or_else(|| ApiError::NotFound(format!("Applied SLA not found: {}", event.applied_sla_id)))?;

            // Mark the event as breached (using deadline as breached_at)
            self.mark_event_breached(&event.id, &event.deadline_at).await?;

            // Update the applied SLA status if needed
            self.update_applied_sla_status(&event.applied_sla_id).await?;

            // Emit SLA breached event
            let now = chrono::Utc::now().to_rfc3339();
            self.publish_event(SystemEvent::SlaBreached {
                event_id: event.id.clone(),
                applied_sla_id: event.applied_sla_id.clone(),
                conversation_id: applied_sla.conversation_id.clone(),
                event_type: event.event_type.to_string(),
                deadline_at: event.deadline_at.clone(),
                breached_at: event.deadline_at.clone(), // Use deadline as breached time
                timestamp: now,
            }).await;

            info!(
                "SLA event {} breached for conversation {} (type: {:?}, deadline: {})",
                event.id, applied_sla.conversation_id, event.event_type, event.deadline_at
            );
        }

        Ok(())
    }

    /// Mark an SLA event as breached
    async fn mark_event_breached(&self, event_id: &str, deadline_at: &str) -> ApiResult<()> {
        self.db.mark_sla_event_breached(event_id, deadline_at).await?;
        Ok(())
    }

    /// Update applied SLA status based on event outcomes
    /// Uses "worst outcome" logic: breached > pending > met
    async fn update_applied_sla_status(&self, applied_sla_id: &str) -> ApiResult<()> {
        // Get all events for this applied SLA
        let events = self.db.get_sla_events_by_applied_sla(applied_sla_id).await?;

        // Determine the worst status
        let has_breached = events.iter().any(|e| e.status == SlaEventStatus::Breached);
        let has_pending = events.iter().any(|e| e.status == SlaEventStatus::Pending);

        let new_status = if has_breached {
            AppliedSlaStatus::Breached
        } else if has_pending {
            AppliedSlaStatus::Pending
        } else {
            AppliedSlaStatus::Met
        };

        // Update the applied SLA status
        self.db.update_applied_sla_status(applied_sla_id, new_status).await?;

        Ok(())
    }

    // ========================================
    // Helper Methods
    // ========================================

    /// Calculate deadline from base time and duration string
    pub fn calculate_deadline(&self, base_time: &str, duration: &str) -> ApiResult<String> {
        let base = chrono::DateTime::parse_from_rfc3339(base_time)
            .map_err(|e| ApiError::BadRequest(format!("Invalid base time: {}", e)))?;

        let seconds =
            parse_duration(duration).map_err(|e| ApiError::BadRequest(format!("{}", e)))?;

        let deadline = base + chrono::Duration::seconds(seconds);
        Ok(deadline.to_rfc3339())
    }

    /// Publish event to the event bus
    async fn publish_event(&self, event: SystemEvent) {
        let bus = self.event_bus.read().await;
        bus.publish(event);
    }
}

impl Clone for SlaService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            event_bus: Arc::clone(&self.event_bus),
        }
    }
}
