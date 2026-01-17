use crate::{
    api::middleware::error::{ApiError, ApiResult},
    domain::ports::{
        conversation_repository::ConversationRepository, sla_repository::SlaRepository,
        team_repository::TeamRepository,
    },
    events::{EventBus, SystemEvent},
    models::*,
};
use chrono::Timelike;
use std::sync::Arc;
use tracing::info;

/// Service for managing SLA policies, applied SLAs, and SLA events
#[derive(Clone)]
pub struct SlaService {
    sla_repo: Arc<dyn SlaRepository>,
    conversation_repo: Arc<dyn ConversationRepository>,
    team_repo: Arc<dyn TeamRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl SlaService {
    /// Create a new SLA service
    pub fn new(
        sla_repo: Arc<dyn SlaRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        team_repo: Arc<dyn TeamRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            sla_repo,
            conversation_repo,
            team_repo,
            event_bus,
        }
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
        parse_duration(&first_response_time)
            .map_err(|e| ApiError::BadRequest(format!("Invalid first_response_time: {}", e)))?;
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

        self.sla_repo.create_sla_policy(&policy).await?;

        info!("Created SLA policy: {} ({})", policy.name, policy.id);
        Ok(policy)
    }

    /// Get SLA policy by ID
    pub async fn get_policy(&self, policy_id: &str) -> ApiResult<Option<SlaPolicy>> {
        self.sla_repo.get_sla_policy(policy_id).await
    }

    /// Get SLA policy by name
    pub async fn get_policy_by_name(&self, name: &str) -> ApiResult<Option<SlaPolicy>> {
        self.sla_repo.get_sla_policy_by_name(name).await
    }

    /// List all SLA policies
    pub async fn list_policies(&self, limit: i64, offset: i64) -> ApiResult<(Vec<SlaPolicy>, i64)> {
        self.sla_repo.list_sla_policies(limit, offset).await
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
            parse_duration(time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid first_response_time: {}", e)))?;
        }
        if let Some(ref time) = resolution_time {
            parse_duration(time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid resolution_time: {}", e)))?;
        }
        if let Some(ref time) = next_response_time {
            parse_duration(time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid next_response_time: {}", e)))?;
        }

        self.sla_repo
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
        self.sla_repo.delete_sla_policy(policy_id).await?;
        info!("Deleted SLA policy: {}", policy_id);
        Ok(())
    }

    // ========================================
    // Applied SLA Management
    // ========================================

    /// Get applied SLA by ID
    pub async fn get_applied_sla(&self, applied_sla_id: &str) -> ApiResult<Option<AppliedSla>> {
        self.sla_repo.get_applied_sla(applied_sla_id).await
    }

    /// Get applied SLA by conversation ID
    pub async fn get_applied_sla_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Option<AppliedSla>> {
        self.sla_repo
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
        self.sla_repo
            .list_applied_slas(status_filter, limit, offset)
            .await
    }

    // ========================================
    // SLA Event Management
    // ========================================

    /// Get SLA event by ID
    pub async fn get_event(&self, event_id: &str) -> ApiResult<Option<SlaEvent>> {
        self.sla_repo.get_sla_event(event_id).await
    }

    /// Get all SLA events for an applied SLA
    pub async fn get_events_by_applied_sla(
        &self,
        applied_sla_id: &str,
    ) -> ApiResult<Vec<SlaEvent>> {
        self.sla_repo.get_sla_events_by_applied_sla(applied_sla_id).await
    }

    /// Get pending SLA event by type for an applied SLA
    pub async fn get_pending_event(
        &self,
        applied_sla_id: &str,
        event_type: SlaEventType,
    ) -> ApiResult<Option<SlaEvent>> {
        self.sla_repo
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
        // Validate conversation exists and get it
        let conversation = self
            .conversation_repo
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation not found: {}", conversation_id))
            })?;

        // Get the policy
        let policy = self
            .get_policy(policy_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("SLA policy not found: {}", policy_id)))?;

        // Check for duplicate: conversation must not already have an applied SLA
        if let Some(_existing) = self
            .sla_repo
            .get_applied_sla_by_conversation(conversation_id)
            .await?
        {
            return Err(ApiError::BadRequest(
                "Conversation already has an applied SLA".to_string(),
            ));
        }

        // Check if conversation is assigned to a team with business hours
        let business_hours = if let Some(team_id) = &conversation.assigned_team_id {
            if let Some(team) = self.team_repo.get_team_by_id(team_id).await? {
                if let Some(bh_json) = &team.business_hours {
                    // Parse business hours JSON
                    match crate::models::team::BusinessHours::parse(bh_json) {
                        Ok(bh) => {
                            info!(
                                "Using business hours for team {} (timezone: {})",
                                team_id, bh.timezone
                            );
                            Some(bh)
                        }
                        Err(e) => {
                            info!("Invalid business hours format for team {}: {}. Using 24/7 calculation.", team_id, e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Calculate deadlines (with or without business hours)
        let first_response_deadline = if let Some(ref bh) = business_hours {
            let duration_seconds = crate::parse_duration(&policy.first_response_time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid first_response_time: {}", e)))?;
            self.calculate_deadline_with_business_hours(base_timestamp, duration_seconds, bh)
                .await?
        } else {
            self.calculate_deadline(base_timestamp, &policy.first_response_time)?
        };

        let resolution_deadline = if let Some(ref bh) = business_hours {
            let duration_seconds = crate::parse_duration(&policy.resolution_time)
                .map_err(|e| ApiError::BadRequest(format!("Invalid resolution_time: {}", e)))?;
            self.calculate_deadline_with_business_hours(base_timestamp, duration_seconds, bh)
                .await?
        } else {
            self.calculate_deadline(base_timestamp, &policy.resolution_time)?
        };

        // Create applied SLA
        let applied_sla = AppliedSla::new(
            conversation_id.to_string(),
            policy_id.to_string(),
            first_response_deadline.clone(),
            resolution_deadline.clone(),
        );

        self.sla_repo.create_applied_sla(&applied_sla).await?;

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

        self.sla_repo.create_sla_event(&first_response_event).await?;
        self.sla_repo.create_sla_event(&resolution_event).await?;

        if business_hours.is_some() {
            info!(
                "Applied SLA policy {} to conversation {} with business hours (first_response: {}, resolution: {})",
                policy_id, conversation_id, first_response_deadline, resolution_deadline
            );
        } else {
            info!(
                "Applied SLA policy {} to conversation {} (24/7) (first_response: {}, resolution: {})",
                policy_id, conversation_id, first_response_deadline, resolution_deadline
            );
        }

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
            .sla_repo
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
            .sla_repo
            .get_pending_sla_event(&applied_sla.id, SlaEventType::FirstResponse)
            .await?
        {
            self.sla_repo
                .mark_sla_event_met(&first_response_event.id, message_timestamp)
                .await?;

            info!(
                "First response SLA met for conversation {} at {}",
                conversation_id, message_timestamp
            );
        }

        // Try to mark next response event as met (only if pending)
        if let Some(next_response_event) = self
            .sla_repo
            .get_pending_sla_event(&applied_sla.id, SlaEventType::NextResponse)
            .await?
        {
            self.sla_repo
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
            .sla_repo
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
            .sla_repo
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
        self.sla_repo
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
        // Get applied SLA for this conversation (must be applied before creating events)
        let applied_sla = self
            .sla_repo
            .get_applied_sla_by_conversation(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::BadRequest("SLA must be applied before event creation".to_string())
            })?;

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

        self.sla_repo.create_sla_event(&next_response_event).await?;

        info!(
            "Next response SLA event created for conversation {} (deadline: {})",
            conversation_id, next_response_deadline
        );

        Ok(())
    }

    /// Check for breached SLA events and update their status
    pub async fn check_breaches(&self) -> ApiResult<()> {
        // Get all pending events past their deadline
        let breached_events = self.sla_repo.get_pending_events_past_deadline().await?;

        if breached_events.is_empty() {
            return Ok(());
        }

        info!(
            "Found {} breached SLA events to process",
            breached_events.len()
        );

        for event in breached_events {
            // Get the applied SLA to get conversation_id
            let applied_sla = self
                .sla_repo
                .get_applied_sla_by_id(&event.applied_sla_id)
                .await?
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Applied SLA not found: {}", event.applied_sla_id))
                })?;

            // Mark the event as breached (using deadline as breached_at)
            self.mark_event_breached(&event.id, &event.deadline_at)
                .await?;

            // Update the applied SLA status if needed
            self.update_applied_sla_status(&event.applied_sla_id)
                .await?;

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
            })
            .await;

            info!(
                "SLA event {} breached for conversation {} (type: {:?}, deadline: {})",
                event.id, applied_sla.conversation_id, event.event_type, event.deadline_at
            );
        }

        Ok(())
    }

    /// Mark an SLA event as breached
    async fn mark_event_breached(&self, event_id: &str, deadline_at: &str) -> ApiResult<()> {
        self.sla_repo
            .mark_sla_event_breached(event_id, deadline_at)
            .await?;
        Ok(())
    }

    /// Update applied SLA status based on event outcomes
    /// Uses "worst outcome" logic: breached > pending > met
    async fn update_applied_sla_status(&self, applied_sla_id: &str) -> ApiResult<()> {
        // Get all events for this applied SLA
        let events = self
            .sla_repo
            .get_sla_events_by_applied_sla(applied_sla_id)
            .await?;

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
        self.sla_repo
            .update_applied_sla_status(applied_sla_id, new_status)
            .await?;

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

    /// Calculate deadline with business hours (skipping non-working hours)
    pub async fn calculate_deadline_with_business_hours(
        &self,
        base_time: &str,
        duration_seconds: i64,
        business_hours: &crate::models::team::BusinessHours,
    ) -> ApiResult<String> {
        use chrono_tz::Tz;

        // Parse base time
        let mut current = chrono::DateTime::parse_from_rfc3339(base_time)
            .map_err(|e| ApiError::BadRequest(format!("Invalid base time: {}", e)))?
            .with_timezone(&chrono::Utc);

        // Parse timezone
        let tz: Tz = business_hours.timezone.parse().map_err(|_| {
            ApiError::BadRequest(format!("Invalid timezone: {}", business_hours.timezone))
        })?;

        // Convert to team timezone
        current = current.with_timezone(&tz).with_timezone(&chrono::Utc);

        // Add duration while skipping non-working hours
        let mut remaining_seconds = duration_seconds;

        while remaining_seconds > 0 {
            // Convert current time to team timezone for business hours check
            let current_in_tz = current.with_timezone(&tz);

            if self
                .is_working_hour(&current_in_tz, business_hours, &tz)
                .await?
            {
                // We're in working hours, advance by 1 minute
                current = current + chrono::Duration::minutes(1);
                remaining_seconds -= 60;
            } else {
                // We're outside working hours, jump to next working hour
                current = self
                    .next_working_hour(&current_in_tz, business_hours, &tz)
                    .await?
                    .with_timezone(&chrono::Utc);
            }
        }

        Ok(current.to_rfc3339())
    }

    /// Check if a datetime falls within business hours and is not a holiday
    async fn is_working_hour(
        &self,
        datetime: &chrono::DateTime<impl chrono::TimeZone>,
        business_hours: &crate::models::team::BusinessHours,
        _tz: &chrono_tz::Tz,
    ) -> ApiResult<bool> {
        use chrono::Datelike;

        // Check if this date is a holiday (Feature 029)
        let date_str = format!(
            "{:04}-{:02}-{:02}",
            datetime.year(),
            datetime.month(),
            datetime.day()
        );
        if self.sla_repo.is_holiday(&date_str).await? {
            return Ok(false); // Holidays are non-working days
        }

        let day_name = match datetime.weekday() {
            chrono::Weekday::Mon => "Monday",
            chrono::Weekday::Tue => "Tuesday",
            chrono::Weekday::Wed => "Wednesday",
            chrono::Weekday::Thu => "Thursday",
            chrono::Weekday::Fri => "Friday",
            chrono::Weekday::Sat => "Saturday",
            chrono::Weekday::Sun => "Sunday",
        };

        // Find schedule for this day
        let day_schedule = business_hours.schedule.iter().find(|s| s.day == day_name);

        if let Some(schedule) = day_schedule {
            // Parse start and end times (format: "HH:MM")
            let time_str = format!("{:02}:{:02}", datetime.hour(), datetime.minute());

            // Simple string comparison works for HH:MM format
            Ok(time_str >= schedule.start && time_str < schedule.end)
        } else {
            // No schedule for this day (weekend/non-working day)
            Ok(false)
        }
    }

    /// Find the next working hour after the given datetime
    async fn next_working_hour(
        &self,
        datetime: &chrono::DateTime<impl chrono::TimeZone>,
        business_hours: &crate::models::team::BusinessHours,
        tz: &chrono_tz::Tz,
    ) -> ApiResult<chrono::DateTime<chrono_tz::Tz>> {
        let mut current = datetime.with_timezone(tz);

        // Try up to 14 days to find next working hour (2 weeks should cover any schedule)
        for _ in 0..14 * 24 * 60 {
            current = current + chrono::Duration::minutes(1);

            if self.is_working_hour(&current, business_hours, tz).await? {
                return Ok(current);
            }
        }

        Err(ApiError::Internal(
            "Could not find next working hour within 14 days".to_string(),
        ))
    }

    /// Publish event to the event bus
    async fn publish_event(&self, event: SystemEvent) {
        if let Err(e) = self.event_bus.publish(event) {
            tracing::error!("Failed to publish SLA event: {}", e);
        }
    }
}

