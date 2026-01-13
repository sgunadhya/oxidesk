use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::models::{Message, MessageStatus};
use crate::database::Database;
use crate::api::middleware::error::ApiResult;

/// Trait for message delivery providers
/// Allows pluggable delivery mechanisms (email, SMS, webhook, etc.)
#[async_trait]
pub trait MessageDeliveryProvider: Send + Sync {
    /// Deliver a message to its destination
    /// Returns Ok(()) if delivery succeeded, Err if failed
    async fn deliver(&self, message: &Message) -> Result<(), String>;

    /// Get the provider name for logging/debugging
    fn provider_name(&self) -> &'static str;
}

/// Mock delivery provider for testing
/// Simulates successful delivery without external dependencies
pub struct MockDeliveryProvider {
    /// If true, simulate delivery failures
    pub should_fail: bool,
}

impl MockDeliveryProvider {
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    pub fn new_failing() -> Self {
        Self { should_fail: true }
    }
}

impl Default for MockDeliveryProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageDeliveryProvider for MockDeliveryProvider {
    async fn deliver(&self, message: &Message) -> Result<(), String> {
        if self.should_fail {
            Err(format!("Mock delivery failure for message {}", message.id))
        } else {
            tracing::debug!(
                "Mock delivery successful for message {} to conversation {}",
                message.id,
                message.conversation_id
            );
            Ok(())
        }
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }
}

/// Message to send through the delivery queue
#[derive(Debug, Clone)]
pub struct DeliveryQueueMessage {
    pub message_id: String,
}

/// Delivery service managing async message delivery
/// Uses tokio mpsc channel for queue and background processing
pub struct DeliveryService {
    db: Database,
    provider: Arc<dyn MessageDeliveryProvider>,
    sender: mpsc::Sender<DeliveryQueueMessage>,
}

impl DeliveryService {
    /// Create a new delivery service with a provider
    /// Spawns background worker to process deliveries
    pub fn new(
        db: Database,
        provider: Arc<dyn MessageDeliveryProvider>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<DeliveryQueueMessage>(100);

        let service = Self {
            db: db.clone(),
            provider: provider.clone(),
            sender,
        };

        // Spawn background worker
        tokio::spawn(Self::delivery_worker(
            db,
            provider,
            receiver,
        ));

        service
    }

    /// Enqueue a message for delivery
    pub async fn enqueue_message(&self, message_id: String) -> ApiResult<()> {
        self.sender
            .send(DeliveryQueueMessage { message_id })
            .await
            .map_err(|e| {
                tracing::error!("Failed to enqueue message for delivery: {}", e);
                crate::api::middleware::error::ApiError::Internal("Failed to enqueue message".to_string())
            })?;

        Ok(())
    }

    /// Background worker processing delivery queue
    async fn delivery_worker(
        db: Database,
        provider: Arc<dyn MessageDeliveryProvider>,
        mut receiver: mpsc::Receiver<DeliveryQueueMessage>,
    ) {
        tracing::info!("Delivery worker started with provider: {}", provider.provider_name());

        while let Some(queue_msg) = receiver.recv().await {
            if let Err(e) = Self::process_delivery(&db, &provider, &queue_msg.message_id).await {
                tracing::error!(
                    "Failed to process delivery for message {}: {:?}",
                    queue_msg.message_id,
                    e
                );
            }
        }

        tracing::warn!("Delivery worker stopped");
    }

    /// Process a single message delivery
    async fn process_delivery(
        db: &Database,
        provider: &Arc<dyn MessageDeliveryProvider>,
        message_id: &str,
    ) -> ApiResult<()> {
        // Fetch message from database
        let message = db.get_message_by_id(message_id).await?
            .ok_or_else(|| {
                tracing::error!("Message not found for delivery: {}", message_id);
                crate::api::middleware::error::ApiError::NotFound("Message not found".to_string())
            })?;

        // Check if already delivered or immutable
        if message.is_immutable {
            tracing::warn!("Attempted to deliver immutable message: {}", message_id);
            return Ok(());
        }

        if message.status == MessageStatus::Sent {
            tracing::debug!("Message already sent: {}", message_id);
            return Ok(());
        }

        // Attempt delivery
        match provider.deliver(&message).await {
            Ok(()) => {
                // Update to sent status
                let now = time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap();

                db.update_message_status(message_id, MessageStatus::Sent, Some(&now)).await?;

                tracing::info!(
                    "Message {} delivered successfully via {}",
                    message_id,
                    provider.provider_name()
                );

                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Delivery failed for message {}: {}",
                    message_id,
                    e
                );

                // Check retry count
                if message.retry_count >= 3 {
                    // Max retries exceeded, mark as failed
                    db.update_message_status(message_id, MessageStatus::Failed, None).await?;
                    tracing::warn!("Message {} marked as failed after max retries", message_id);
                } else {
                    // Schedule retry with exponential backoff
                    let delay = Self::calculate_retry_delay(message.retry_count);
                    tracing::info!(
                        "Scheduling retry for message {} in {} seconds (retry {})",
                        message_id,
                        delay,
                        message.retry_count + 1
                    );

                    // TODO: Implement actual retry scheduling
                    // For now, just mark as failed if retry logic isn't implemented
                    // In production, this would use tokio::time::sleep and re-enqueue
                }

                Ok(())
            }
        }
    }

    /// Calculate retry delay using exponential backoff
    /// Base delay: 60 seconds
    /// Formula: base_delay * 2^retry_count
    pub fn calculate_retry_delay(retry_count: i32) -> u64 {
        let base_delay = 60; // 60 seconds
        let multiplier = 2_u64.pow(retry_count as u32);
        base_delay * multiplier
    }
}

impl Clone for DeliveryService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            provider: self.provider.clone(),
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_retry_delay() {
        assert_eq!(DeliveryService::calculate_retry_delay(0), 60); // 60 * 2^0 = 60
        assert_eq!(DeliveryService::calculate_retry_delay(1), 120); // 60 * 2^1 = 120
        assert_eq!(DeliveryService::calculate_retry_delay(2), 240); // 60 * 2^2 = 240
        assert_eq!(DeliveryService::calculate_retry_delay(3), 480); // 60 * 2^3 = 480
    }

    #[tokio::test]
    async fn test_mock_provider_success() {
        let provider = MockDeliveryProvider::new();
        let message = Message::new_outgoing(
            "conv_123".to_string(),
            "Test message".to_string(),
            "agent_456".to_string(),
        );

        let result = provider.deliver(&message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_provider_failure() {
        let provider = MockDeliveryProvider::new_failing();
        let message = Message::new_outgoing(
            "conv_123".to_string(),
            "Test message".to_string(),
            "agent_456".to_string(),
        );

        let result = provider.deliver(&message).await;
        assert!(result.is_err());
    }
}
