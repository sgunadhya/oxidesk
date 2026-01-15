use sqlx::Row;

use crate::{ApiError, ApiResult, Database, DeliveryStatus, Webhook, WebhookDelivery};

impl Database {
    pub async fn create_webhook(&self, webhook: &Webhook) -> ApiResult<()> {
        let subscribed_events_json = serde_json::to_string(&webhook.subscribed_events)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize events: {}", e)))?;

        sqlx::query(
            "INSERT INTO webhooks (id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&webhook.id)
        .bind(&webhook.name)
        .bind(&webhook.url)
        .bind(&subscribed_events_json)
        .bind(&webhook.secret)
        .bind(webhook.is_active)
        .bind(&webhook.created_at)
        .bind(&webhook.updated_at)
        .bind(&webhook.created_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a webhook by ID
    pub async fn get_webhook_by_id(&self, id: &str) -> ApiResult<Option<Webhook>> {
        let row = sqlx::query(
            "SELECT id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by
             FROM webhooks
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let subscribed_events_str: String = row.try_get("subscribed_events")?;
            let subscribed_events: Vec<String> = serde_json::from_str(&subscribed_events_str)
                .map_err(|e| ApiError::Internal(format!("Failed to parse events: {}", e)))?;

            Ok(Some(Webhook {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                url: row.try_get("url")?,
                subscribed_events,
                secret: row.try_get("secret")?,
                is_active: row.try_get("is_active")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                created_by: row.try_get("created_by")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all webhooks with pagination
    pub async fn list_webhooks(&self, limit: i64, offset: i64) -> ApiResult<Vec<Webhook>> {
        let rows = sqlx::query(
            "SELECT id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by
             FROM webhooks
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut webhooks = Vec::new();
        for row in rows {
            let subscribed_events_str: String = row.try_get("subscribed_events")?;
            let subscribed_events: Vec<String> = serde_json::from_str(&subscribed_events_str)
                .map_err(|e| ApiError::Internal(format!("Failed to parse events: {}", e)))?;

            webhooks.push(Webhook {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                url: row.try_get("url")?,
                subscribed_events,
                secret: row.try_get("secret")?,
                is_active: row.try_get("is_active")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                created_by: row.try_get("created_by")?,
            });
        }

        Ok(webhooks)
    }

    /// Get active webhooks that subscribe to a specific event type
    pub async fn get_active_webhooks_for_event(&self, event_type: &str) -> ApiResult<Vec<Webhook>> {
        let rows = sqlx::query(
            "SELECT id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by
             FROM webhooks
             WHERE is_active = ?",
        )
        .bind(true)
        .fetch_all(&self.pool)
        .await?;

        let mut matching_webhooks = Vec::new();
        for row in rows {
            let subscribed_events_str: String = row.try_get("subscribed_events")?;
            let subscribed_events: Vec<String> = serde_json::from_str(&subscribed_events_str)
                .map_err(|e| ApiError::Internal(format!("Failed to parse events: {}", e)))?;

            // Filter webhooks that subscribe to this event
            if subscribed_events.contains(&event_type.to_string()) {
                matching_webhooks.push(Webhook {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    url: row.try_get("url")?,
                    subscribed_events,
                    secret: row.try_get("secret")?,
                    is_active: row.try_get("is_active")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    created_by: row.try_get("created_by")?,
                });
            }
        }

        Ok(matching_webhooks)
    }

    /// Update a webhook
    pub async fn update_webhook(&self, webhook: &Webhook) -> ApiResult<()> {
        let subscribed_events_json = serde_json::to_string(&webhook.subscribed_events)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize events: {}", e)))?;

        sqlx::query(
            "UPDATE webhooks
             SET name = ?, url = ?, subscribed_events = ?, secret = ?, is_active = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&webhook.name)
        .bind(&webhook.url)
        .bind(&subscribed_events_json)
        .bind(&webhook.secret)
        .bind(webhook.is_active)
        .bind(&webhook.updated_at)
        .bind(&webhook.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a webhook (cascades to deliveries)
    pub async fn delete_webhook(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM webhooks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Count total webhooks
    pub async fn count_webhooks(&self) -> ApiResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM webhooks")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("count")?)
    }

    // ========================================================================
    // Webhook Delivery Operations
    // ========================================================================

    /// Create a new webhook delivery record
    pub async fn create_webhook_delivery(&self, delivery: &WebhookDelivery) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO webhook_deliveries
             (id, webhook_id, event_type, payload, signature, status, http_status_code,
              retry_count, next_retry_at, attempted_at, completed_at, error_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&delivery.id)
        .bind(&delivery.webhook_id)
        .bind(&delivery.event_type)
        .bind(&delivery.payload)
        .bind(&delivery.signature)
        .bind(delivery.status.as_str())
        .bind(delivery.http_status_code)
        .bind(delivery.retry_count)
        .bind(&delivery.next_retry_at)
        .bind(&delivery.attempted_at)
        .bind(&delivery.completed_at)
        .bind(&delivery.error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update an existing webhook delivery record
    pub async fn update_webhook_delivery(&self, delivery: &WebhookDelivery) -> ApiResult<()> {
        sqlx::query(
            "UPDATE webhook_deliveries
             SET status = ?, http_status_code = ?, retry_count = ?,
                 next_retry_at = ?, attempted_at = ?, completed_at = ?, error_message = ?
             WHERE id = ?",
        )
        .bind(delivery.status.as_str())
        .bind(delivery.http_status_code)
        .bind(delivery.retry_count)
        .bind(&delivery.next_retry_at)
        .bind(&delivery.attempted_at)
        .bind(&delivery.completed_at)
        .bind(&delivery.error_message)
        .bind(&delivery.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending deliveries ready for processing
    pub async fn get_pending_deliveries(&self) -> ApiResult<Vec<WebhookDelivery>> {
        let now = chrono::Utc::now().to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, webhook_id, event_type, payload, signature, status,
                    http_status_code, retry_count, next_retry_at, attempted_at, completed_at, error_message
             FROM webhook_deliveries
             WHERE status = 'queued' AND (next_retry_at IS NULL OR next_retry_at <= ?)
             ORDER BY next_retry_at ASC, attempted_at ASC
             LIMIT 100",
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;

        let mut deliveries = Vec::new();
        for row in rows {
            deliveries.push(WebhookDelivery {
                id: row.try_get("id")?,
                webhook_id: row.try_get("webhook_id")?,
                event_type: row.try_get("event_type")?,
                payload: row.try_get("payload")?,
                signature: row.try_get("signature")?,
                status: DeliveryStatus::from(row.try_get::<String, _>("status")?),
                http_status_code: row.try_get("http_status_code")?,
                retry_count: row.try_get("retry_count")?,
                next_retry_at: row.try_get("next_retry_at")?,
                attempted_at: row.try_get("attempted_at")?,
                completed_at: row.try_get("completed_at")?,
                error_message: row.try_get("error_message")?,
            });
        }

        Ok(deliveries)
    }

    /// Get deliveries for a specific webhook with pagination
    pub async fn get_deliveries_for_webhook(
        &self,
        webhook_id: &str,
        limit: i64,
        offset: i64,
        status_filter: Option<&str>,
    ) -> ApiResult<Vec<WebhookDelivery>> {
        let query = if let Some(status) = status_filter {
            sqlx::query(
                "SELECT id, webhook_id, event_type, payload, signature, status,
                        http_status_code, retry_count, next_retry_at, attempted_at, completed_at, error_message
                 FROM webhook_deliveries
                 WHERE webhook_id = ? AND status = ?
                 ORDER BY attempted_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(webhook_id)
            .bind(status)
            .bind(limit)
            .bind(offset)
        } else {
            sqlx::query(
                "SELECT id, webhook_id, event_type, payload, signature, status,
                        http_status_code, retry_count, next_retry_at, attempted_at, completed_at, error_message
                 FROM webhook_deliveries
                 WHERE webhook_id = ?
                 ORDER BY attempted_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(webhook_id)
            .bind(limit)
            .bind(offset)
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut deliveries = Vec::new();
        for row in rows {
            deliveries.push(WebhookDelivery {
                id: row.try_get("id")?,
                webhook_id: row.try_get("webhook_id")?,
                event_type: row.try_get("event_type")?,
                payload: row.try_get("payload")?,
                signature: row.try_get("signature")?,
                status: DeliveryStatus::from(row.try_get::<String, _>("status")?),
                http_status_code: row.try_get("http_status_code")?,
                retry_count: row.try_get("retry_count")?,
                next_retry_at: row.try_get("next_retry_at")?,
                attempted_at: row.try_get("attempted_at")?,
                completed_at: row.try_get("completed_at")?,
                error_message: row.try_get("error_message")?,
            });
        }

        Ok(deliveries)
    }

    /// Count deliveries for a specific webhook
    pub async fn count_deliveries_for_webhook(
        &self,
        webhook_id: &str,
        status_filter: Option<&str>,
    ) -> ApiResult<i64> {
        let row = if let Some(status) = status_filter {
            sqlx::query(
                "SELECT COUNT(*) as count FROM webhook_deliveries WHERE webhook_id = ? AND status = ?",
            )
            .bind(webhook_id)
            .bind(status)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM webhook_deliveries WHERE webhook_id = ?")
                .bind(webhook_id)
                .fetch_one(&self.pool)
                .await?
        };

        Ok(row.try_get("count")?)
    }
}
