use crate::{
    database::Database,
    events::{EventBus, SystemEvent},
    models::{Webhook, WebhookDelivery},
    services::webhook_signature::sign_payload,
};
use serde_json::json;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Worker that subscribes to EventBus and queues webhook deliveries
pub struct WebhookWorker {
    db: Database,
    event_bus: EventBus,
}

impl WebhookWorker {
    /// Create a new webhook worker
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    /// Start the webhook worker in the background
    ///
    /// This method spawns a long-lived tokio task that subscribes to the EventBus
    /// and processes events as they arrive. For each event, it:
    /// 1. Determines the event type string (e.g., "conversation.created")
    /// 2. Finds all active webhooks subscribed to this event type
    /// 3. Constructs a JSON payload with event data
    /// 4. Signs the payload with HMAC-SHA256 using the webhook's secret
    /// 5. Creates a delivery record in the database
    ///
    /// The worker runs until the server shuts down.
    pub fn start(self) {
        tokio::spawn(async move {
            info!("Starting webhook worker");

            let mut rx = self.event_bus.subscribe();

            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = self.handle_event(event).await {
                            error!("Failed to handle webhook event: {}", e);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Webhook worker lagged, skipped {} events", skipped);
                        // Continue processing - this just means we fell behind
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        error!("EventBus closed, stopping webhook worker");
                        break;
                    }
                }
            }
        });
    }

    /// Handle a single system event
    async fn handle_event(&self, event: SystemEvent) -> Result<(), String> {
        // Determine event type and construct payload
        let (event_type, payload) = self.construct_payload(&event)?;

        // Get all active webhooks subscribed to this event type
        let webhooks = self
            .db
            .get_active_webhooks_for_event(&event_type)
            .await
            .map_err(|e| format!("Failed to fetch webhooks: {}", e))?;

        if webhooks.is_empty() {
            // No webhooks subscribed to this event, nothing to do
            return Ok(());
        }

        info!(
            "Found {} webhooks subscribed to event type: {}",
            webhooks.len(),
            event_type
        );

        // Queue delivery for each matching webhook
        for webhook in webhooks {
            if let Err(e) = self.queue_delivery(&webhook, &event_type, &payload).await {
                error!("Failed to queue delivery for webhook {}: {}", webhook.id, e);
                // Continue with other webhooks even if one fails
            }
        }

        Ok(())
    }

    /// Construct JSON payload from system event
    fn construct_payload(
        &self,
        event: &SystemEvent,
    ) -> Result<(String, serde_json::Value), String> {
        let (event_type, data) = match event {
            SystemEvent::ConversationCreated {
                conversation_id,
                inbox_id,
                contact_id,
                status,
                timestamp,
            } => (
                "conversation.created",
                json!({
                    "conversation_id": conversation_id,
                    "inbox_id": inbox_id,
                    "contact_id": contact_id,
                    "status": status.to_string(),
                    "created_at": timestamp,
                }),
            ),
            SystemEvent::ConversationStatusChanged {
                conversation_id,
                old_status,
                new_status,
                agent_id,
                timestamp,
            } => (
                "conversation.status_changed",
                json!({
                    "conversation_id": conversation_id,
                    "old_status": old_status.to_string(),
                    "new_status": new_status.to_string(),
                    "agent_id": agent_id,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::ConversationAssigned {
                conversation_id,
                assigned_user_id,
                assigned_team_id,
                assigned_by,
                timestamp,
            } => (
                "conversation.assigned",
                json!({
                    "conversation_id": conversation_id,
                    "assigned_user_id": assigned_user_id,
                    "assigned_team_id": assigned_team_id,
                    "assigned_by": assigned_by,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::ConversationUnassigned {
                conversation_id,
                previous_assigned_user_id,
                previous_assigned_team_id,
                unassigned_by,
                timestamp,
            } => (
                "conversation.unassigned",
                json!({
                    "conversation_id": conversation_id,
                    "previous_assigned_user_id": previous_assigned_user_id,
                    "previous_assigned_team_id": previous_assigned_team_id,
                    "unassigned_by": unassigned_by,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::ConversationTagsChanged {
                conversation_id,
                previous_tags,
                new_tags,
                changed_by,
                timestamp,
            } => (
                "conversation.tags_changed",
                json!({
                    "conversation_id": conversation_id,
                    "previous_tags": previous_tags,
                    "new_tags": new_tags,
                    "changed_by": changed_by,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::ConversationPriorityChanged {
                conversation_id,
                previous_priority,
                new_priority,
                updated_by,
                timestamp,
            } => (
                "conversation.priority_changed",
                json!({
                    "conversation_id": conversation_id,
                    "previous_priority": previous_priority,
                    "new_priority": new_priority,
                    "updated_by": updated_by,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::MessageReceived {
                message_id,
                conversation_id,
                contact_id,
                timestamp,
            } => (
                "message.received",
                json!({
                    "message_id": message_id,
                    "conversation_id": conversation_id,
                    "contact_id": contact_id,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::MessageSent {
                message_id,
                conversation_id,
                agent_id,
                timestamp,
            } => (
                "message.sent",
                json!({
                    "message_id": message_id,
                    "conversation_id": conversation_id,
                    "agent_id": agent_id,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::MessageFailed {
                message_id,
                conversation_id,
                retry_count,
                timestamp,
            } => (
                "message.failed",
                json!({
                    "message_id": message_id,
                    "conversation_id": conversation_id,
                    "retry_count": retry_count,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::SlaBreached {
                event_id,
                applied_sla_id,
                conversation_id,
                event_type,
                deadline_at,
                breached_at,
                timestamp,
            } => (
                "sla.breached",
                json!({
                    "event_id": event_id,
                    "applied_sla_id": applied_sla_id,
                    "conversation_id": conversation_id,
                    "event_type": event_type,
                    "deadline_at": deadline_at,
                    "breached_at": breached_at,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::AgentAvailabilityChanged {
                agent_id,
                old_status,
                new_status,
                timestamp,
                reason,
            } => (
                "agent.availability_changed",
                json!({
                    "agent_id": agent_id,
                    "old_status": old_status,
                    "new_status": new_status,
                    "reason": reason,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::AgentLoggedIn {
                agent_id,
                user_id,
                timestamp,
            } => (
                "agent.logged_in",
                json!({
                    "agent_id": agent_id,
                    "user_id": user_id,
                    "timestamp": timestamp,
                }),
            ),
            SystemEvent::AgentLoggedOut {
                agent_id,
                user_id,
                timestamp,
            } => (
                "agent.logged_out",
                json!({
                    "agent_id": agent_id,
                    "user_id": user_id,
                    "timestamp": timestamp,
                }),
            ),
        };

        // Wrap in envelope with event_type and timestamp
        let envelope = json!({
            "event_type": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": data,
        });

        Ok((event_type.to_string(), envelope))
    }

    /// Queue a webhook delivery for processing
    async fn queue_delivery(
        &self,
        webhook: &Webhook,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        // Serialize payload to JSON string
        let payload_str = serde_json::to_string(payload)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?;

        // Sign the payload with webhook secret
        let signature = sign_payload(&payload_str, &webhook.secret);

        // Create delivery record
        let delivery = WebhookDelivery::new(
            webhook.id.clone(),
            event_type.to_string(),
            payload_str,
            signature,
        );

        // Save to database
        self.db
            .create_webhook_delivery(&delivery)
            .await
            .map_err(|e| format!("Failed to create delivery: {}", e))?;

        info!(
            "Queued webhook delivery {} for webhook {} (event: {})",
            delivery.id, webhook.id, event_type
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::conversation::ConversationStatus;

    #[tokio::test]
    async fn test_construct_payload_conversation_created() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let event_bus = EventBus::default();
        let worker = WebhookWorker::new(db, event_bus);

        let event = SystemEvent::ConversationCreated {
            conversation_id: "conv-123".to_string(),
            inbox_id: "inbox-456".to_string(),
            contact_id: "contact-789".to_string(),
            status: ConversationStatus::Open,
            timestamp: "2026-01-13T10:00:00Z".to_string(),
        };

        let (event_type, payload) = worker.construct_payload(&event).unwrap();

        assert_eq!(event_type, "conversation.created");
        assert_eq!(payload["event_type"], "conversation.created");
        assert_eq!(payload["data"]["conversation_id"], "conv-123");
        assert_eq!(payload["data"]["inbox_id"], "inbox-456");
        assert_eq!(payload["data"]["contact_id"], "contact-789");
        assert_eq!(payload["data"]["status"], "open");
    }

    #[tokio::test]
    async fn test_construct_payload_message_sent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let event_bus = EventBus::default();
        let worker = WebhookWorker::new(db, event_bus);

        let event = SystemEvent::MessageSent {
            message_id: "msg-123".to_string(),
            conversation_id: "conv-456".to_string(),
            agent_id: "agent-789".to_string(),
            timestamp: "2026-01-13T10:00:00Z".to_string(),
        };

        let (event_type, payload) = worker.construct_payload(&event).unwrap();

        assert_eq!(event_type, "message.sent");
        assert_eq!(payload["event_type"], "message.sent");
        assert_eq!(payload["data"]["message_id"], "msg-123");
        assert_eq!(payload["data"]["agent_id"], "agent-789");
    }
}
