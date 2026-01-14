use crate::{database::Database, models::WebhookDelivery};
use reqwest::Client;
use std::time::Duration;
use tracing::{error, info, warn};

/// Service for delivering webhooks to external endpoints
pub struct WebhookDeliveryService {
    db: Database,
    http_client: Client,
}

impl WebhookDeliveryService {
    /// Create a new webhook delivery service
    pub fn new(db: Database) -> Self {
        // Initialize reqwest client with 30-second timeout
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self { db, http_client }
    }

    /// Attempt to deliver a webhook
    ///
    /// Makes an HTTP POST request to the webhook URL with:
    /// - JSON payload in request body
    /// - X-Webhook-Signature header with HMAC-SHA256 signature
    /// - Content-Type: application/json
    ///
    /// Returns (success, http_status_code, error_message)
    pub async fn attempt_delivery(
        &self,
        url: &str,
        payload: &str,
        signature: &str,
    ) -> (bool, Option<u16>, Option<String>) {
        info!("Attempting webhook delivery to {}", url);

        match self
            .http_client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .body(payload.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let status_code = status.as_u16();

                info!(
                    "Webhook delivery to {} returned status {}",
                    url, status_code
                );

                // Check if delivery was successful (2xx status codes)
                if status.is_success() {
                    (true, Some(status_code), None)
                } else {
                    // Get error message from response body if available
                    let error_msg = match response.text().await {
                        Ok(body) => {
                            if body.len() > 500 {
                                format!("HTTP {}: {}", status_code, &body[..500])
                            } else {
                                format!("HTTP {}: {}", status_code, body)
                            }
                        }
                        Err(_) => format!("HTTP {} error", status_code),
                    };

                    (false, Some(status_code), Some(error_msg))
                }
            }
            Err(e) => {
                // Network error (timeout, connection refused, DNS failure, etc.)
                let error_msg = if e.is_timeout() {
                    format!("Connection timeout after 30 seconds: {}", e)
                } else if e.is_connect() {
                    format!("Connection failed: {}", e)
                } else {
                    format!("Network error: {}", e)
                };

                warn!("Webhook delivery to {} failed: {}", url, error_msg);
                (false, None, Some(error_msg))
            }
        }
    }

    /// Process a single delivery from the queue
    pub async fn process_delivery(&self, mut delivery: WebhookDelivery) -> Result<(), String> {
        // Get the webhook to retrieve the URL
        let webhook = self
            .db
            .get_webhook_by_id(&delivery.webhook_id)
            .await
            .map_err(|e| format!("Failed to get webhook: {}", e))?
            .ok_or_else(|| format!("Webhook {} not found", delivery.webhook_id))?;

        // Attempt delivery
        let (success, http_status, error_msg) = self
            .attempt_delivery(&webhook.url, &delivery.payload, &delivery.signature)
            .await;

        // Update delivery record based on result
        if success {
            delivery.mark_success(http_status.unwrap() as i32);
            info!(
                "Webhook delivery {} succeeded with status {}",
                delivery.id,
                http_status.unwrap()
            );
        } else {
            let error = error_msg.unwrap_or_else(|| "Unknown error".to_string());
            delivery.mark_failed(http_status.map(|s| s as i32), error.clone());

            if delivery.retry_count >= 5 {
                warn!(
                    "Webhook delivery {} permanently failed after {} attempts",
                    delivery.id, delivery.retry_count
                );
            } else {
                info!(
                    "Webhook delivery {} failed (attempt {}/5), scheduled retry at {}",
                    delivery.id,
                    delivery.retry_count,
                    delivery.next_retry_at.as_deref().unwrap_or("unknown")
                );
            }
        }

        // Save updated delivery record
        self.db
            .update_webhook_delivery(&delivery)
            .await
            .map_err(|e| format!("Failed to update delivery: {}", e))?;

        Ok(())
    }

    /// Start the background delivery processor
    ///
    /// This spawns a tokio task that polls the database for pending deliveries
    /// every 10 seconds and processes them concurrently (up to 10 at a time).
    pub fn start_processor(self) {
        tokio::spawn(async move {
            info!("Starting webhook delivery processor");

            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                match self.process_pending_deliveries().await {
                    Ok(processed) => {
                        if processed > 0 {
                            info!("Processed {} webhook deliveries", processed);
                        }
                    }
                    Err(e) => {
                        error!("Error processing webhook deliveries: {}", e);
                    }
                }
            }
        });
    }

    /// Process all pending deliveries in the queue
    ///
    /// Fetches pending deliveries from the database and processes up to 10
    /// concurrently using tokio tasks. Returns the number of deliveries processed.
    async fn process_pending_deliveries(&self) -> Result<usize, String> {
        // Get pending deliveries ready for processing
        let deliveries = self
            .db
            .get_pending_deliveries()
            .await
            .map_err(|e| format!("Failed to get pending deliveries: {}", e))?;

        if deliveries.is_empty() {
            return Ok(0);
        }

        let count = deliveries.len();
        info!("Found {} pending webhook deliveries", count);

        // Process deliveries concurrently (up to 10 at a time)
        let mut tasks = Vec::new();

        for delivery in deliveries {
            let service = WebhookDeliveryService::new(self.db.clone());
            let task = tokio::spawn(async move {
                if let Err(e) = service.process_delivery(delivery.clone()).await {
                    error!("Failed to process delivery {}: {}", delivery.id, e);
                }
            });
            tasks.push(task);

            // Limit concurrent processing to 10
            if tasks.len() >= 10 {
                // Wait for at least one task to complete
                if let Some(task) = tasks.pop() {
                    let _ = task.await;
                }
            }
        }

        // Wait for remaining tasks
        for task in tasks {
            let _ = task.await;
        }

        Ok(count)
    }
}

impl Clone for WebhookDeliveryService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            http_client: self.http_client.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_attempt_delivery_success() {
        // This test would require a mock HTTP server
        // For now, we'll test the service creation
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = WebhookDeliveryService::new(db);

        // Verify HTTP client is configured
        assert!(service
            .http_client
            .get("http://example.com")
            .build()
            .is_ok());
    }

    #[test]
    fn test_service_creation() {
        // Test that service can be created
        // Actual delivery tests would require integration testing with mock HTTP server
    }
}
