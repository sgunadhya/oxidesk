pub use crate::domain::events::SystemEvent;

use crate::domain::ports::event_bus::EventBus;
use crate::ApiResult;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

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
        if let Err(e) = self.tx.send(event) {
            tracing::debug!("No active subscribers for event (or channel full): {}", e);
        }
        Ok(())
    }

    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = Result<SystemEvent, String>> + Send>> {
        let rx = self.tx.subscribe();
        // Map BroadcastStream errors to String errors
        let stream = BroadcastStream::new(rx)
            .map(|res| res.map_err(|e| format!("Broadcast channel error: {}", e)));
        Box::pin(stream)
    }
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
    use crate::domain::entities::ConversationStatus;

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

        let _ = bus.publish(event);

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
