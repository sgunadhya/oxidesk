use crate::domain::entities::conversation::ConversationStatus;
use tokio::sync::broadcast;

/// System events that can trigger automation rules
#[derive(Debug, Clone)]
pub enum SystemEvent {
    ConversationCreated {
        conversation_id: String,
        inbox_id: String,
        contact_id: String,
        status: ConversationStatus,
        timestamp: String, // ISO 8601
    },
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
    ConversationPriorityChanged {
        conversation_id: String,
        previous_priority: Option<String>,
        new_priority: Option<String>,
        updated_by: String,
        timestamp: String, // ISO 8601
    },
    AgentAvailabilityChanged {
        agent_id: String,
        old_status: String,
        new_status: String,
        timestamp: String, // ISO 8601
        reason: String, // "manual", "inactivity_timeout", "max_idle_threshold", "login", "logout"
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
        event_type: String,  // "first_response", "resolution", "next_response"
        deadline_at: String, // ISO 8601
        breached_at: String, // ISO 8601
        timestamp: String,   // ISO 8601
    },
}

use crate::ApiResult;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

/// Event bus trait for publishing and subscribing to system events
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event to all subscribers
    fn publish(&self, event: SystemEvent) -> ApiResult<()>;

    /// Subscribe to events
    fn subscribe(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<SystemEvent, BroadcastStreamRecvError>> + Send>>;
}

/// Local in-memory implementation of EventBus
#[derive(Clone)]
pub struct LocalEventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl LocalEventBus {
    /// Create a new event bus with specified capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }
}

#[async_trait]
impl EventBus for LocalEventBus {
    fn publish(&self, event: SystemEvent) -> ApiResult<()> {
        // Fire-and-forget - if no subscribers or channel full, just log and continue
        // We consider this a "success" from the API perspective for now,
        // as we don't want to block operation if just nobody is listening.
        if let Err(e) = self.tx.send(event) {
            tracing::debug!("No active subscribers for event (or channel full): {}", e);
        }
        Ok(())
    }

    fn subscribe(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<SystemEvent, BroadcastStreamRecvError>> + Send>> {
        let rx = self.tx.subscribe();
        Box::pin(BroadcastStream::new(rx))
    }

    // Helper for legacy code that expects a direct receiver (temporary)
    // or maybe we force migration. Let's force migration to Stream to satisfy the plan.
    // However, keeping subscriber_count is useful for tests.
}

impl LocalEventBus {
    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for LocalEventBus {
    fn default() -> Self {
        Self::new(1000) // Default capacity of 1000 events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_creation() {
        let bus = LocalEventBus::new(100);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_publish_subscribe() {
        use tokio_stream::StreamExt;
        let bus = LocalEventBus::new(100);
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
        let received = rx.next().await.unwrap().unwrap();
        match received {
            SystemEvent::ConversationStatusChanged {
                conversation_id, ..
            } => {
                assert_eq!(conversation_id, "test-id");
            }
            _ => panic!("Unexpected event type"),
        }
    }
}
