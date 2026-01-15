use sqlx::Row;

use crate::{ApiResult, Database, NotificationType, UserNotification};

impl Database {
    pub async fn create_notification(&self, notification: &UserNotification) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO user_notifications (id, user_id, type, created_at, is_read, conversation_id, message_id, actor_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&notification.id)
        .bind(&notification.user_id)
        .bind(notification.notification_type.as_str())
        .bind(&notification.created_at)
        .bind(if notification.is_read { 1 } else { 0 })
        .bind(&notification.conversation_id)
        .bind(&notification.message_id)
        .bind(&notification.actor_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_notification_by_id(&self, id: &str) -> ApiResult<Option<UserNotification>> {
        let row = sqlx::query(
            "SELECT id, user_id, type, created_at, is_read, conversation_id, message_id, actor_id
             FROM user_notifications
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let notification_type_str: String = row.try_get("type")?;
            let is_read_int: i32 = row.try_get("is_read")?;

            Ok(Some(UserNotification {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                notification_type: NotificationType::from(notification_type_str),
                created_at: row.try_get("created_at")?,
                is_read: is_read_int != 0,
                conversation_id: row.try_get("conversation_id").ok(),
                message_id: row.try_get("message_id").ok(),
                actor_id: row.try_get("actor_id").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_notifications(
        &self,
        user_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<UserNotification>> {
        let rows = sqlx::query(
            "SELECT id, user_id, type, created_at, is_read, conversation_id, message_id, actor_id
             FROM user_notifications
             WHERE user_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut notifications = Vec::new();
        for row in rows {
            let notification_type_str: String = row.try_get("type")?;
            let is_read_int: i32 = row.try_get("is_read")?;

            notifications.push(UserNotification {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                notification_type: NotificationType::from(notification_type_str),
                created_at: row.try_get("created_at")?,
                is_read: is_read_int != 0,
                conversation_id: row.try_get("conversation_id").ok(),
                message_id: row.try_get("message_id").ok(),
                actor_id: row.try_get("actor_id").ok(),
            });
        }

        Ok(notifications)
    }

    pub async fn mark_notification_as_read(&self, id: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE user_notifications
             SET is_read = 1
             WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn mark_all_notifications_as_read(&self, user_id: &str) -> ApiResult<i32> {
        let result = sqlx::query(
            "UPDATE user_notifications
             SET is_read = 1
             WHERE user_id = ? AND is_read = 0",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i32)
    }

    pub async fn get_unread_count(&self, user_id: &str) -> ApiResult<i32> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM user_notifications
             WHERE user_id = ? AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i32 = row.try_get("count")?;
        Ok(count)
    }

    pub async fn delete_old_notifications(&self, older_than_days: i32) -> ApiResult<i32> {
        // Calculate the cutoff timestamp
        let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(older_than_days as i64);
        let cutoff_str = cutoff
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query(
            "DELETE FROM user_notifications
             WHERE created_at < ?",
        )
        .bind(&cutoff_str)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i32)
    }
}
