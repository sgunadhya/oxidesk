use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    models::{
        CreateWebhookRequest, UpdateWebhookRequest, Webhook, WebhookListResponse,
        WebhookResponse,
    },
};
use tracing::info;

/// Service for managing webhooks
pub struct WebhookService {
    db: Database,
}

impl WebhookService {
    /// Create a new webhook service
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create a new webhook
    pub async fn create_webhook(
        &self,
        request: CreateWebhookRequest,
        created_by: &str,
    ) -> ApiResult<WebhookResponse> {
        // Create webhook model
        let mut webhook = Webhook::new(
            request.name,
            request.url,
            request.subscribed_events,
            request.secret,
            created_by.to_string(),
        );

        // Set is_active if provided, otherwise defaults to true
        if let Some(is_active) = request.is_active {
            webhook.is_active = is_active;
        }

        // Validate webhook
        webhook
            .validate()
            .map_err(|e| ApiError::BadRequest(e))?;

        // Save to database
        self.db.create_webhook(&webhook).await?;

        info!("Created webhook {} by user {}", webhook.id, created_by);

        Ok(WebhookResponse::from(webhook))
    }

    /// Update an existing webhook
    pub async fn update_webhook(
        &self,
        id: &str,
        request: UpdateWebhookRequest,
    ) -> ApiResult<WebhookResponse> {
        // Get existing webhook
        let mut webhook = self
            .db
            .get_webhook_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;

        // Update fields if provided
        if let Some(name) = request.name {
            webhook.name = name;
        }
        if let Some(url) = request.url {
            webhook.url = url;
        }
        if let Some(subscribed_events) = request.subscribed_events {
            webhook.subscribed_events = subscribed_events;
        }
        if let Some(secret) = request.secret {
            webhook.secret = secret;
        }
        if let Some(is_active) = request.is_active {
            webhook.is_active = is_active;
        }

        // Update timestamp
        webhook.touch();

        // Validate updated webhook
        webhook
            .validate()
            .map_err(|e| ApiError::BadRequest(e))?;

        // Save to database
        self.db.update_webhook(&webhook).await?;

        info!("Updated webhook {}", webhook.id);

        Ok(WebhookResponse::from(webhook))
    }

    /// Delete a webhook
    pub async fn delete_webhook(&self, id: &str) -> ApiResult<()> {
        // Verify webhook exists
        self.db
            .get_webhook_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;

        // Delete webhook (cascades to deliveries)
        self.db.delete_webhook(id).await?;

        info!("Deleted webhook {}", id);

        Ok(())
    }

    /// Toggle webhook active status
    pub async fn toggle_webhook_status(&self, id: &str) -> ApiResult<WebhookResponse> {
        // Get existing webhook
        let mut webhook = self
            .db
            .get_webhook_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;

        // Toggle is_active status
        webhook.is_active = !webhook.is_active;
        webhook.touch();

        // Save to database
        self.db.update_webhook(&webhook).await?;

        info!(
            "Toggled webhook {} status to {}",
            webhook.id,
            if webhook.is_active { "active" } else { "inactive" }
        );

        Ok(WebhookResponse::from(webhook))
    }

    /// Get a webhook by ID
    pub async fn get_webhook(&self, id: &str) -> ApiResult<WebhookResponse> {
        let webhook = self
            .db
            .get_webhook_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;

        Ok(WebhookResponse::from(webhook))
    }

    /// List webhooks with pagination
    pub async fn list_webhooks(&self, limit: i64, offset: i64) -> ApiResult<WebhookListResponse> {
        // Validate pagination parameters
        if limit < 1 || limit > 100 {
            return Err(ApiError::BadRequest(
                "Limit must be between 1 and 100".to_string(),
            ));
        }
        if offset < 0 {
            return Err(ApiError::BadRequest(
                "Offset must be non-negative".to_string(),
            ));
        }

        // Get webhooks from database
        let webhooks = self.db.list_webhooks(limit, offset).await?;

        // Get total count
        let total = self.db.count_webhooks().await?;

        // Convert to response models (without secrets)
        let webhook_responses: Vec<WebhookResponse> = webhooks
            .into_iter()
            .map(WebhookResponse::from)
            .collect();

        Ok(WebhookListResponse {
            webhooks: webhook_responses,
            total,
        })
    }
}

impl Clone for WebhookService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let _service = WebhookService::new(db);

        // Verify service is created
        assert!(true);
    }

    #[tokio::test]
    async fn test_create_webhook_validation() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let service = WebhookService::new(db.clone());

        // Test with invalid name (empty)
        let request = CreateWebhookRequest {
            name: "".to_string(),
            url: "https://example.com/webhook".to_string(),
            subscribed_events: vec!["conversation.created".to_string()],
            secret: "secret123456789012".to_string(),
            is_active: Some(true),
        };

        let result = service.create_webhook(request, "admin-123").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[tokio::test]
    async fn test_list_webhooks_pagination_validation() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = WebhookService::new(db);

        // Test with invalid limit (too high)
        let result = service.list_webhooks(101, 0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Limit"));

        // Test with invalid offset (negative)
        let result = service.list_webhooks(50, -1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Offset"));
    }
}
