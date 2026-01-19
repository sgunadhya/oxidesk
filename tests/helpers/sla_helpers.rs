#![allow(dead_code)]
use chrono::{DateTime, Duration, Utc};
use oxidesk::{
    infrastructure::persistence::Database,
    domain::entities::{AppliedSla, SlaEvent, SlaEventStatus, SlaEventType, SlaPolicy},
};

/// Create a test SLA policy with custom times
pub async fn create_test_sla_policy(
    db: &Database,
    name: &str,
    first_response: &str,
    resolution: &str,
    next_response: &str,
) -> SlaPolicy {
    let policy = SlaPolicy::new(
        name.to_string(),
        Some(format!("Test policy: {}", name)),
        first_response.to_string(),
        resolution.to_string(),
        next_response.to_string(),
    );

    db.create_sla_policy(&policy)
        .await
        .expect("Failed to create SLA policy");
    policy
}

/// Create a test applied SLA
pub async fn create_test_applied_sla(
    db: &Database,
    conversation_id: &str,
    policy_id: &str,
    first_response_deadline: DateTime<Utc>,
    resolution_deadline: DateTime<Utc>,
) -> AppliedSla {
    let applied_sla = AppliedSla::new(
        conversation_id.to_string(),
        policy_id.to_string(),
        first_response_deadline.to_rfc3339(),
        resolution_deadline.to_rfc3339(),
    );

    db.create_applied_sla(&applied_sla)
        .await
        .expect("Failed to create applied SLA");
    applied_sla
}

/// Create a test SLA event
pub async fn create_test_sla_event(
    db: &Database,
    applied_sla_id: &str,
    event_type: SlaEventType,
    deadline: DateTime<Utc>,
) -> SlaEvent {
    let event = SlaEvent::new(
        applied_sla_id.to_string(),
        event_type,
        deadline.to_rfc3339(),
    );

    db.create_sla_event(&event)
        .await
        .expect("Failed to create SLA event");
    event
}

/// Get applied SLA by conversation ID
pub async fn get_applied_sla(db: &Database, conversation_id: &str) -> Option<AppliedSla> {
    db.get_applied_sla_by_conversation(conversation_id)
        .await
        .expect("Failed to get applied SLA")
}

/// Get SLA events for an applied SLA
pub async fn get_sla_events(db: &Database, applied_sla_id: &str) -> Vec<SlaEvent> {
    db.get_sla_events_by_applied_sla(applied_sla_id)
        .await
        .expect("Failed to get SLA events")
}

/// Count pending events past deadline
pub async fn count_breached_events(db: &Database) -> i64 {
    let events = db
        .get_pending_events_past_deadline()
        .await
        .expect("Failed to get pending events");
    events.len() as i64
}

/// Set event status for testing
pub async fn set_event_status(
    db: &Database,
    event_id: &str,
    status: SlaEventStatus,
    timestamp: Option<DateTime<Utc>>,
) {
    let now = timestamp.unwrap_or_else(Utc::now);
    match status {
        SlaEventStatus::Met => {
            db.mark_sla_event_met(event_id, &now.to_rfc3339())
                .await
                .expect("Failed to mark event as met");
        }
        SlaEventStatus::Breached => {
            db.mark_sla_event_breached(event_id, &now.to_rfc3339())
                .await
                .expect("Failed to mark event as breached");
        }
        SlaEventStatus::Pending => {
            // No-op, events are pending by default
        }
    }
}

/// Calculate deadline from duration string
pub fn calculate_deadline(base: DateTime<Utc>, duration_str: &str) -> DateTime<Utc> {
    let seconds = oxidesk::parse_duration(duration_str).expect("Invalid duration");
    base + Duration::seconds(seconds)
}

/// Helper to wait for time to pass (for breach detection tests)
pub async fn wait_until(target: DateTime<Utc>) {
    let now = Utc::now();
    if target > now {
        let wait_duration = (target - now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(0));
        tokio::time::sleep(wait_duration).await;
    }
}
