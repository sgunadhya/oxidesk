use crate::api::middleware::error::{ApiError, ApiResult};
use crate::database::Database;
use crate::models::Inbox;
use chrono;
use sqlx::Row;

impl Database {
    /// Soft delete an inbox
    /// Sets deleted_at timestamp and records who performed the deletion
    pub async fn soft_delete_inbox(&self, inbox_id: &str, deleted_by: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE inboxes
             SET deleted_at = ?, deleted_by = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&now)
        .bind(deleted_by)
        .bind(inbox_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Inbox not found or already deleted".to_string(),
            ));
        }

        Ok(())
    }

    /// Restore a soft deleted inbox
    /// Clears deleted_at and deleted_by fields
    pub async fn restore_inbox(&self, inbox_id: &str) -> ApiResult<()> {
        let result = sqlx::query(
            "UPDATE inboxes
             SET deleted_at = NULL, deleted_by = NULL
             WHERE id = ? AND deleted_at IS NOT NULL",
        )
        .bind(inbox_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Inbox not found or not deleted".to_string(),
            ));
        }

        Ok(())
    }

    // Inbox operations
    pub async fn create_inbox(&self, inbox: &Inbox) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO inboxes (id, name, channel_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&inbox.id)
        .bind(&inbox.name)
        .bind(&inbox.channel_type)
        .bind(&inbox.created_at)
        .bind(&inbox.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_inboxes(&self) -> ApiResult<Vec<Inbox>> {
        let rows = sqlx::query(
            "SELECT id, name, channel_type, created_at, updated_at, deleted_at, deleted_by
             FROM inboxes
             WHERE deleted_at IS NULL
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut inboxes = Vec::new();
        for row in rows {
            inboxes.push(Inbox {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                channel_type: row.try_get("channel_type")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: row.try_get("deleted_at").ok(),
                deleted_by: row.try_get("deleted_by").ok(),
            });
        }
        Ok(inboxes)
    }
}
