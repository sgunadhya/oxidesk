use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    models::{Message, IncomingMessageRequest, SendMessageRequest},
    services::DeliveryService,
    events::{EventBus, SystemEvent},
};

pub struct MessageService {
    db: Database,
    delivery_service: Option<DeliveryService>,
    event_bus: Option<EventBus>,
}

impl MessageService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            delivery_service: None,
            event_bus: None,
        }
    }

    pub fn with_delivery(db: Database, delivery_service: DeliveryService) -> Self {
        Self {
            db,
            delivery_service: Some(delivery_service),
            event_bus: None,
        }
    }

    pub fn with_delivery_and_events(
        db: Database,
        delivery_service: DeliveryService,
        event_bus: EventBus,
    ) -> Self {
        Self {
            db,
            delivery_service: Some(delivery_service),
            event_bus: Some(event_bus),
        }
    }

    /// Create an incoming message from external source (webhook)
    /// Validates content, creates message, and updates conversation timestamps
    pub async fn create_incoming_message(
        &self,
        request: IncomingMessageRequest,
    ) -> ApiResult<Message> {
        // Validate content
        Message::validate_content(&request.content)
            .map_err(|e| ApiError::BadRequest(e))?;

        // Verify conversation exists
        let _conversation = self.db
            .get_conversation_by_id(&request.conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", request.conversation_id))
            })?;

        // Create incoming message
        let message = Message::new_incoming(
            request.conversation_id.clone(),
            request.content,
            request.contact_id,
        );

        // Save to database
        self.db.create_message(&message).await?;

        // Update conversation timestamps
        self.db
            .update_conversation_message_timestamps(
                &request.conversation_id,
                &message.id,
                &message.created_at,
                None, // Incoming messages don't update last_reply_at
            )
            .await?;

        tracing::info!(
            "Incoming message created: id={}, conversation_id={}",
            message.id,
            message.conversation_id
        );

        // Publish MessageReceived event
        if let Some(ref event_bus) = self.event_bus {
            event_bus.publish(SystemEvent::MessageReceived {
                message_id: message.id.clone(),
                conversation_id: message.conversation_id.clone(),
                contact_id: message.author_id.clone(),
                timestamp: message.created_at.clone(),
            });
        }

        Ok(message)
    }

    /// Send an outgoing message from agent to customer
    /// Validates content, creates message, queues for delivery
    pub async fn send_message(
        &self,
        conversation_id: String,
        agent_id: String,
        request: SendMessageRequest,
    ) -> ApiResult<Message> {
        // Validate content
        Message::validate_content(&request.content)
            .map_err(|e| ApiError::BadRequest(e))?;

        // Verify conversation exists
        let _conversation = self.db
            .get_conversation_by_id(&conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // TODO: Add permission check (messages:write)
        // TODO: Add conversation access validation (agent assigned to conversation)

        // Create outgoing message
        let message = Message::new_outgoing(
            conversation_id.clone(),
            request.content,
            agent_id,
        );

        // Save to database
        self.db.create_message(&message).await?;

        // Update conversation timestamps (last_reply_at for agent replies)
        self.db
            .update_conversation_message_timestamps(
                &conversation_id,
                &message.id,
                &message.created_at,
                Some(&message.created_at), // Outgoing messages update last_reply_at
            )
            .await?;

        // Queue message for delivery
        if let Some(ref delivery_service) = self.delivery_service {
            delivery_service.enqueue_message(message.id.clone()).await?;
            tracing::info!(
                "Outgoing message queued for delivery: id={}, conversation_id={}",
                message.id,
                message.conversation_id
            );
        } else {
            tracing::warn!(
                "Delivery service not configured, message not queued: id={}",
                message.id
            );
        }

        Ok(message)
    }

    /// Get message by ID
    pub async fn get_message(&self, message_id: &str) -> ApiResult<Message> {
        self.db
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Message {} not found", message_id)))
    }

    /// List messages for a conversation with pagination
    pub async fn list_messages(
        &self,
        conversation_id: &str,
        page: i64,
        per_page: i64,
    ) -> ApiResult<(Vec<Message>, i64)> {
        let offset = (page - 1) * per_page;
        self.db.list_messages(conversation_id, per_page, offset).await
    }

    /// Check if a message is immutable (prevents updates to sent/received messages)
    async fn check_message_immutable(&self, message_id: &str) -> ApiResult<()> {
        let message = self.get_message(message_id).await?;

        if message.is_immutable {
            return Err(ApiError::BadRequest(format!(
                "Cannot modify immutable message (status: {})",
                message.status
            )));
        }

        Ok(())
    }

    /// Update message status (with immutability check)
    pub async fn update_message_status(
        &self,
        message_id: &str,
        new_status: crate::models::MessageStatus,
    ) -> ApiResult<()> {
        // Check immutability before allowing status change
        self.check_message_immutable(message_id).await?;

        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let sent_at = if new_status == crate::models::MessageStatus::Sent {
            Some(now.as_str())
        } else {
            None
        };

        self.db.update_message_status(message_id, new_status, sent_at).await?;

        Ok(())
    }
}

impl Clone for MessageService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            delivery_service: self.delivery_service.clone(),
            event_bus: self.event_bus.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_service_creation() {
        // Just a placeholder test to verify compilation
        // Real tests will be integration tests
    }
}
