use crate::models::conversation::ConversationStatus;
use tokio::sync::broadcast;
use uuid::Uuid;

/// System events that can trigger automation rules
#[derive(Debug, Clone)]
pub enum SystemEvent {
    ConversationStatusChanged {
        conversation_id: String,
        old_status: ConversationStatus,
        new_status: ConversationStatus,
        agent_id: Option<String>,
        timestamp: String, // ISO 8601
    },
    MessageReceived {
        message_id: String,
        conversation_id: String,
        contact_id: String,
        timestamp: String, // ISO 8601
    },
    MessageSent {
        message_id: String,
        conversation_id: String,
        agent_id: String,
        timestamp: String, // ISO 8601
    },
    MessageFailed {
        message_id: String,
        conversation_id: String,
        retry_count: i32,
        timestamp: String, // ISO 8601
    },
    ConversationAssigned {
        conversation_id: String,
        assigned_user_id: Option<String>,
        assigned_team_id: Option<String>,
        assigned_by: String,
        timestamp: String, // ISO 8601
    },
    ConversationUnassigned {
        conversation_id: String,
        previous_assigned_user_id: Option<String>,
        previous_assigned_team_id: Option<String>,
        unassigned_by: String,
        timestamp: String, // ISO 8601
    },
    ConversationTagsChanged {
        conversation_id: String,
        previous_tags: Vec<String>,
        new_tags: Vec<String>,
        changed_by: String,
        timestamp: String, // ISO 8601
    },
    AgentAvailabilityChanged {
        agent_id: String,
        old_status: String,
        new_status: String,
        timestamp: String, // ISO 8601
        reason: String,    // "manual", "inactivity_timeout", "max_idle_threshold", "login", "logout"
    },
    AgentLoggedIn {
        agent_id: String,
        user_id: String,
        timestamp: String, // ISO 8601
    },
    AgentLoggedOut {
        agent_id: String,
        user_id: String,
        timestamp: String, // ISO 8601
    },
    SlaBreached {
        event_id: String,
        applied_sla_id: String,
        conversation_id: String,
        event_type: String, // "first_response", "resolution", "next_response"
        deadline_at: String, // ISO 8601
        breached_at: String, // ISO 8601
        timestamp: String, // ISO 8601
    },
}

/// Event bus for publishing and subscribing to system events
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    /// Create a new event bus with specified capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event to all subscribers (non-blocking, fire-and-forget)
    pub fn publish(&self, event: SystemEvent) {
        // Fire-and-forget - if no subscribers or channel full, just log and continue
        if let Err(e) = self.tx.send(event) {
            tracing::warn!("Failed to publish event (no subscribers or channel full): {}", e);
        }
    }

    /// Subscribe to events (returns a receiver)
    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000) // Default capacity of 1000 events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new(100);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_publish_subscribe() {
        let bus = EventBus::new(100);
        let mut rx = bus.subscribe();

        let event = SystemEvent::ConversationStatusChanged {
            conversation_id: "test-id".to_string(),
            old_status: ConversationStatus::Open,
            new_status: ConversationStatus::Resolved,
            agent_id: Some("agent-id".to_string()),
            timestamp: "2026-01-12T10:00:00Z".to_string(),
        };

        bus.publish(event);

        // Receive the event
        let received = rx.recv().await.unwrap();
        match received {
            SystemEvent::ConversationStatusChanged { conversation_id, .. } => {
                assert_eq!(conversation_id, "test-id");
            }
            _ => panic!("Unexpected event type"),
        }
    }
}
