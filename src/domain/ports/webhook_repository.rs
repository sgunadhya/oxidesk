use crate::{
    api::middleware::error::ApiResult,
    database::Database,
    models::Webhook,
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
}
