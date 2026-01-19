use crate::{
    infrastructure::http::middleware::error::ApiResult,
    infrastructure::persistence::Database,
    domain::entities::Webhook,
};

#[derive(Clone)]
pub struct WebhookRepository {
    db: Database,
}

impl WebhookRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create a new webhook
    pub async fn create_webhook(&self, webhook: &Webhook) -> ApiResult<()> {
        self.db.create_webhook(webhook).await
    }

    /// Get webhook by ID
    pub async fn get_webhook_by_id(&self, id: &str) -> ApiResult<Option<Webhook>> {
        self.db.get_webhook_by_id(id).await
    }

    /// Update webhook
    pub async fn update_webhook(&self, webhook: &Webhook) -> ApiResult<()> {
        self.db.update_webhook(webhook).await
    }

    /// Delete webhook
    pub async fn delete_webhook(&self, id: &str) -> ApiResult<()> {
        self.db.delete_webhook(id).await
    }

    /// List webhooks with pagination
    pub async fn list_webhooks(&self, limit: i64, offset: i64) -> ApiResult<Vec<Webhook>> {
        self.db.list_webhooks(limit, offset).await
    }

    /// Count total webhooks
    pub async fn count_webhooks(&self) -> ApiResult<i64> {
        self.db.count_webhooks().await
    }

    /// Get deliveries for a specific webhook with pagination
    pub async fn get_deliveries_for_webhook(
        &self,
        webhook_id: &str,
        limit: i64,
        offset: i64,
        status_filter: Option<&str>,
    ) -> ApiResult<Vec<crate::domain::entities::WebhookDelivery>> {
        self.db.get_deliveries_for_webhook(webhook_id, limit, offset, status_filter).await
    }

    /// Count deliveries for a specific webhook
    pub async fn count_deliveries_for_webhook(
        &self,
        webhook_id: &str,
        status_filter: Option<&str>,
    ) -> ApiResult<i64> {
        self.db.count_deliveries_for_webhook(webhook_id, status_filter).await
    }

    /// Get active webhooks subscribed to a specific event type
    pub async fn get_active_webhooks_for_event(&self, event_type: &str) -> ApiResult<Vec<Webhook>> {
        self.db.get_active_webhooks_for_event(event_type).await
    }

    /// Create a webhook delivery record
    pub async fn create_webhook_delivery(&self, delivery: &crate::domain::entities::WebhookDelivery) -> ApiResult<()> {
        self.db.create_webhook_delivery(delivery).await
    }
}
