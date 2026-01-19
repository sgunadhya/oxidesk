use crate::{
    application::services::{DeliveryService, NotificationService},
    domain::entities::{IncomingMessageRequest, Message, SendMessageRequest, UserNotification},
    domain::events::SystemEvent,
    domain::ports::conversation_repository::ConversationRepository,
    domain::ports::event_bus::EventBus,
    domain::ports::message_repository::MessageRepository,
    infrastructure::http::middleware::error::{ApiError, ApiResult},
    infrastructure::providers::connection_manager::ConnectionManager,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct MessageService {
    message_repo: Arc<dyn MessageRepository>,
    conversation_repo: Arc<dyn ConversationRepository>,
    delivery_service: Option<DeliveryService>,
    event_bus: Option<Arc<dyn EventBus>>,
    connection_manager: Option<Arc<dyn ConnectionManager>>,
}

impl MessageService {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
    ) -> Self {
        Self {
            message_repo,
            conversation_repo,
            delivery_service: None,
            event_bus: None,
            connection_manager: None,
        }
    }

    pub fn with_delivery(
        message_repo: Arc<dyn MessageRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        delivery_service: DeliveryService,
    ) -> Self {
        Self {
            message_repo,
            conversation_repo,
            delivery_service: Some(delivery_service),
            event_bus: None,
            connection_manager: None,
        }
    }

    pub fn with_all_services(
        message_repo: Arc<dyn MessageRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        delivery_service: DeliveryService,
        event_bus: Arc<dyn EventBus>,
        connection_manager: Arc<dyn ConnectionManager>,
    ) -> Self {
        Self {
            message_repo,
            conversation_repo,
            delivery_service: Some(delivery_service),
            event_bus: Some(event_bus),
            connection_manager: Some(connection_manager),
        }
    }

    /// Create an incoming message from external source (webhook)
    pub async fn create_incoming_message(
        &self,
        request: IncomingMessageRequest,
    ) -> ApiResult<Message> {
        // Validate content
        Message::validate_content(&request.content).map_err(|e| ApiError::BadRequest(e))?;

        // Verify conversation exists
        let _conversation = self
            .conversation_repo
            .get_conversation_by_id(&request.conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "Conversation {} not found",
                    request.conversation_id
                ))
            })?;

        // Feature 023: Contact ID must be resolved by this point
        let contact_id = request.contact_id.ok_or_else(|| {
            ApiError::BadRequest("Message must have exactly one sender".to_string())
        })?;

        // Create incoming message
        let message =
            Message::new_incoming(request.conversation_id.clone(), request.content, contact_id);

        // Save to database
        self.message_repo.create_message(&message).await?;

        // Update conversation timestamps
        self.message_repo
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
            let _ = event_bus.publish(SystemEvent::MessageReceived {
                message_id: message.id.clone(),
                conversation_id: message.conversation_id.clone(),
                contact_id: message.author_id.clone(),
                timestamp: message.created_at.clone(),
            });
        }

        Ok(message)
    }

    /// Send an outgoing message from agent to customer
    pub async fn send_message(
        &self,
        conversation_id: String,
        agent_id: String,
        request: SendMessageRequest,
    ) -> ApiResult<Message> {
        // Validate content
        Message::validate_content(&request.content).map_err(|e| ApiError::BadRequest(e))?;

        // Verify conversation exists
        let _conversation = self
            .conversation_repo
            .get_conversation_by_id(&conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // Create outgoing message
        let message =
            Message::new_outgoing(conversation_id.clone(), request.content, agent_id.clone());

        // Save to database
        self.message_repo.create_message(&message).await?;

        // Update conversation timestamps (last_reply_at for agent replies)
        self.message_repo
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

        // Parse @mentions and create notifications
        let mention_usernames = NotificationService::extract_mentions(&message.content);
        if !mention_usernames.is_empty() {
            // Batch verify usernames
            let mentioned_users = self
                .message_repo
                .get_users_by_usernames(&mention_usernames)
                .await?;

            // Create notifications (filter self-mentions)
            let mut notifications = Vec::new();
            for user in mentioned_users {
                if user.id == message.author_id {
                    continue; // Skip self-mention
                }

                let notification = UserNotification::new_mention(
                    user.id.clone(),
                    conversation_id.clone(),
                    message.id.clone(),
                    message.author_id.clone(),
                );

                self.message_repo.create_notification(&notification).await?;
                notifications.push(notification);
            }

            // Send real-time notifications (best-effort)
            if let Some(ref connection_manager) = self.connection_manager {
                for notification in notifications {
                    let connection_manager = connection_manager.clone();
                    let notification_clone = notification.clone();
                    tokio::spawn(async move {
                        if let Err(e) = NotificationService::send_realtime_notification(
                            &notification_clone,
                            &connection_manager,
                        )
                        .await
                        {
                            tracing::debug!("Failed to send real-time notification: {}", e);
                        }
                    });
                }
            }
        }

        Ok(message)
    }

    /// Get message by ID
    pub async fn get_message(&self, message_id: &str) -> ApiResult<Message> {
        self.message_repo
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
        self.message_repo
            .list_messages(conversation_id, per_page, offset)
            .await
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
        new_status: crate::domain::entities::MessageStatus,
    ) -> ApiResult<()> {
        // Check immutability before allowing status change
        self.check_message_immutable(message_id).await?;

        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let sent_at = if new_status == crate::domain::entities::MessageStatus::Sent {
            Some(now.as_str())
        } else {
            None
        };

        self.message_repo
            .update_message_status(message_id, new_status, sent_at)
            .await?;

        Ok(())
    }
}
