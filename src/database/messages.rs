use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use crate::models::{Message, MessageStatus, MessageType};
use sqlx::Row;
use time;

impl Database {
    // Message operations
    pub async fn create_message(&self, message: &Message) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, sent_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(message.message_type.as_str())
        .bind(message.status.as_str())
        .bind(&message.content)
        .bind(&message.author_id)
        .bind(message.is_immutable)
        .bind(message.retry_count)
        .bind(&message.created_at)
        .bind(&message.sent_at)
        .bind(&message.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_message_by_id(&self, id: &str) -> ApiResult<Option<Message>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, sent_at, updated_at
             FROM messages
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let message_type_str: String = row.try_get("type")?;
            let status_str: String = row.try_get("status")?;

            Ok(Some(Message {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                message_type: MessageType::from(message_type_str),
                status: MessageStatus::from(status_str),
                content: row.try_get("content")?,
                author_id: row.try_get("author_id")?,
                is_immutable: row.try_get("is_immutable")?,
                retry_count: row.try_get("retry_count")?,
                created_at: row.try_get("created_at")?,
                sent_at: row.try_get("sent_at").ok(),
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_messages(
        &self,
        conversation_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Message>, i64)> {
        // Get total count
        let count_row =
            sqlx::query("SELECT COUNT(*) as count FROM messages WHERE conversation_id = ?")
                .bind(conversation_id)
                .fetch_one(&self.pool)
                .await?;
        let total_count: i64 = count_row.try_get("count")?;

        // Get messages
        let rows = sqlx::query(
            "SELECT id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, sent_at, updated_at
             FROM messages
             WHERE conversation_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(conversation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            let message_type_str: String = row.try_get("type")?;
            let status_str: String = row.try_get("status")?;

            messages.push(Message {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                message_type: MessageType::from(message_type_str),
                status: MessageStatus::from(status_str),
                content: row.try_get("content")?,
                author_id: row.try_get("author_id")?,
                is_immutable: row.try_get::<i32, _>("is_immutable")? != 0,
                retry_count: row.try_get("retry_count")?,
                created_at: row.try_get("created_at")?,
                sent_at: row.try_get("sent_at").ok(),
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok((messages, total_count))
    }

    pub async fn update_message_status(
        &self,
        message_id: &str,
        status: MessageStatus,
        sent_at: Option<&str>,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        if let Some(sent_at_value) = sent_at {
            sqlx::query(
                "UPDATE messages
                 SET status = ?, sent_at = ?, updated_at = ?, is_immutable = ?
                 WHERE id = ?",
            )
            .bind(status.as_str())
            .bind(sent_at_value)
            .bind(&now)
            .bind(status.is_immutable())
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE messages
                 SET status = ?, updated_at = ?, is_immutable = ?
                 WHERE id = ?",
            )
            .bind(status.as_str())
            .bind(&now)
            .bind(status.is_immutable())
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn update_conversation_message_timestamps(
        &self,
        conversation_id: &str,
        message_id: &str,
        last_message_at: &str,
        last_reply_at: Option<&str>,
    ) -> ApiResult<()> {
        if let Some(reply_at) = last_reply_at {
            sqlx::query(
                "UPDATE conversations
                 SET last_message_id = ?, last_message_at = ?, last_reply_at = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(message_id)
            .bind(last_message_at)
            .bind(reply_at)
            .bind(last_message_at)
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE conversations
                 SET last_message_id = ?, last_message_at = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(message_id)
            .bind(last_message_at)
            .bind(last_message_at)
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn count_messages(&self, conversation_id: &str) -> ApiResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM messages WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }
}
