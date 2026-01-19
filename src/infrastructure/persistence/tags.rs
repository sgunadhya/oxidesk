use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::persistence::Database;
use crate::domain::entities::{Conversation, ConversationStatus, Tag};
use chrono;
use sqlx::Row;

impl Database {
    // ========== Tag Operations (Feature 005) ==========

    /// Create a new tag
    pub async fn create_tag(&self, tag: &Tag) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO tags (id, name, description, color, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&tag.id)
        .bind(&tag.name)
        .bind(&tag.description)
        .bind(&tag.color)
        .bind(&tag.created_at)
        .bind(&tag.updated_at)
        .execute(&self.pool)
        .await?;

        tracing::info!("Tag created: id={}, name={}", tag.id, tag.name);
        Ok(())
    }

    /// Get tag by ID
    pub async fn get_tag_by_id(&self, id: &str) -> ApiResult<Option<Tag>> {
        let row = sqlx::query(
            "SELECT id, name, description, color, created_at, updated_at
             FROM tags
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get tag by name
    pub async fn get_tag_by_name(&self, name: &str) -> ApiResult<Option<Tag>> {
        let row = sqlx::query(
            "SELECT id, name, description, color, created_at, updated_at
             FROM tags
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all tags
    pub async fn list_tags(&self, limit: i64, offset: i64) -> ApiResult<(Vec<Tag>, i64)> {
        // Get total count
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM tags")
            .fetch_one(&self.pool)
            .await?;
        let total_count: i64 = count_row.try_get("count")?;

        let rows = sqlx::query(
            "SELECT id, name, description, color, created_at, updated_at
             FROM tags
             ORDER BY name
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok((tags, total_count))
    }

    /// Update a tag
    pub async fn update_tag(
        &self,
        id: &str,
        description: Option<String>,
        color: Option<String>,
    ) -> ApiResult<()> {
        let mut builder = sqlx::QueryBuilder::new("UPDATE tags SET ");
        let mut separated = builder.separated(", ");

        if let Some(d) = description {
            separated.push("description = ");
            separated.push_bind_unseparated(d);
        }

        if let Some(c) = color {
            separated.push("color = ");
            separated.push_bind_unseparated(c);
        }

        let now = chrono::Utc::now().to_rfc3339();
        separated.push("updated_at = ");
        separated.push_bind_unseparated(now);

        builder.push(" WHERE id = ");
        builder.push_bind(id);

        let query = builder.build();
        query.execute(&self.pool).await?;

        tracing::info!("Tag updated: id={}", id);
        Ok(())
    }

    /// Delete a tag
    pub async fn delete_tag(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        tracing::info!("Tag deleted: id={}", id);
        Ok(())
    }

    /// Get tags for a conversation
    pub async fn get_conversation_tags(&self, conversation_id: &str) -> ApiResult<Vec<Tag>> {
        let rows = sqlx::query(
            "SELECT t.id, t.name, t.description, t.color, t.created_at, t.updated_at
             FROM tags t
             INNER JOIN conversation_tags ct ON t.id = ct.tag_id
             WHERE ct.conversation_id = ?
             ORDER BY t.name",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(tags)
    }

    /// Add a tag to a conversation (idempotent)
    pub async fn add_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
        added_by: &str,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Use INSERT OR IGNORE for SQLite idempotency
        // This will silently ignore if the tag is already associated
        let result = sqlx::query(
            "INSERT OR IGNORE INTO conversation_tags (conversation_id, tag_id, added_by, added_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(tag_id)
        .bind(added_by)
        .bind(&now)
        .execute(&self.pool)
        .await;

        // For databases that don't support INSERT OR IGNORE, we can check if it exists first
        // But for now, we'll handle the error gracefully
        match result {
            Ok(_) => {
                tracing::debug!("Tag {} added to conversation {}", tag_id, conversation_id);
                Ok(())
            }
            Err(e) => {
                // If it's a unique constraint violation, treat as success (idempotent)
                if e.to_string().contains("UNIQUE") || e.to_string().contains("duplicate") {
                    tracing::debug!(
                        "Tag {} already associated with conversation {} (idempotent)",
                        tag_id,
                        conversation_id
                    );
                    Ok(())
                } else {
                    Err(ApiError::Internal(format!("Failed to add tag: {}", e)))
                }
            }
        }
    }

    /// Remove a tag from a conversation (idempotent)
    pub async fn remove_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM conversation_tags
             WHERE conversation_id = ? AND tag_id = ?",
        )
        .bind(conversation_id)
        .bind(tag_id)
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            "Tag {} removed from conversation {}",
            tag_id,
            conversation_id
        );
        Ok(())
    }

    /// Replace all conversation tags atomically
    pub async fn replace_conversation_tags(
        &self,
        conversation_id: &str,
        tag_ids: &[String],
        added_by: &str,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Delete all existing tags
        sqlx::query("DELETE FROM conversation_tags WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        // Insert new tags
        for tag_id in tag_ids {
            sqlx::query(
                "INSERT INTO conversation_tags (conversation_id, tag_id, added_by, added_at)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(conversation_id)
            .bind(tag_id)
            .bind(added_by)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        // Commit transaction
        tx.commit().await?;

        tracing::info!(
            "Replaced tags for conversation {}: {} tags",
            conversation_id,
            tag_ids.len()
        );
        Ok(())
    }

    /// Get conversations with a specific tag
    pub async fn get_conversations_by_tag(
        &self,
        tag_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(DISTINCT c.id) as count
             FROM conversations c
             INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
             WHERE ct.tag_id = ?",
        )
        .bind(tag_id)
        .fetch_one(&self.pool)
        .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get conversations
        let rows = sqlx::query(
            "SELECT c.*
             FROM conversations c
             INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
             WHERE ct.tag_id = ?
             ORDER BY ct.added_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(tag_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            let status_str: String = row.try_get("status")?;
            conversations.push(Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                closed_at: row.try_get("closed_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: None,
            });
        }

        Ok((conversations, total))
    }

    /// Get conversations with multiple tags (AND or OR logic)
    pub async fn get_conversations_by_tags(
        &self,
        tag_ids: &[String],
        match_all: bool,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        if tag_ids.is_empty() {
            return Ok((Vec::new(), 0));
        }

        if match_all {
            // AND logic: conversation must have ALL specified tags
            let placeholders = tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let tag_count = tag_ids.len() as i64;

            let count_query = format!(
                "SELECT COUNT(DISTINCT c.id) as count
                 FROM conversations c
                 WHERE (
                     SELECT COUNT(DISTINCT ct.tag_id)
                     FROM conversation_tags ct
                     WHERE ct.conversation_id = c.id
                     AND ct.tag_id IN ({})
                 ) = ?",
                placeholders
            );

            let mut count_query_builder = sqlx::query(&count_query);
            for tag_id in tag_ids {
                count_query_builder = count_query_builder.bind(tag_id);
            }
            count_query_builder = count_query_builder.bind(tag_count);

            let count_row = count_query_builder.fetch_one(&self.pool).await?;
            let total: i64 = count_row.try_get("count")?;

            let conversations_query = format!(
                "SELECT c.*
                 FROM conversations c
                 WHERE (
                     SELECT COUNT(DISTINCT ct.tag_id)
                     FROM conversation_tags ct
                     WHERE ct.conversation_id = c.id
                     AND ct.tag_id IN ({})
                 ) = ?
                 ORDER BY c.created_at DESC
                 LIMIT ? OFFSET ?",
                placeholders
            );

            let mut query_builder = sqlx::query(&conversations_query);
            for tag_id in tag_ids {
                query_builder = query_builder.bind(tag_id);
            }
            query_builder = query_builder.bind(tag_count).bind(limit).bind(offset);

            let rows = query_builder.fetch_all(&self.pool).await?;

            let mut conversations = Vec::new();
            for row in rows {
                let status_str: String = row.try_get("status")?;
                conversations.push(Conversation {
                    id: row.try_get("id")?,
                    reference_number: row.try_get("reference_number")?,
                    status: ConversationStatus::from(status_str),
                    inbox_id: row.try_get("inbox_id")?,
                    contact_id: row.try_get("contact_id")?,
                    subject: row.try_get("subject").ok(),
                    resolved_at: row.try_get("resolved_at").ok(),
                    closed_at: row.try_get("closed_at").ok(),
                    snoozed_until: row.try_get("snoozed_until").ok(),
                    assigned_user_id: row.try_get("assigned_user_id").ok(),
                    assigned_team_id: row.try_get("assigned_team_id").ok(),
                    assigned_at: row.try_get("assigned_at").ok(),
                    assigned_by: row.try_get("assigned_by").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    version: row.try_get("version")?,
                    tags: None,
                    priority: None,
                });
            }

            Ok((conversations, total))
        } else {
            // OR logic: conversation has ANY of the specified tags
            let placeholders = tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

            let count_query = format!(
                "SELECT COUNT(DISTINCT c.id) as count
                 FROM conversations c
                 INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
                 WHERE ct.tag_id IN ({})",
                placeholders
            );

            let mut count_query_builder = sqlx::query(&count_query);
            for tag_id in tag_ids {
                count_query_builder = count_query_builder.bind(tag_id);
            }

            let count_row = count_query_builder.fetch_one(&self.pool).await?;
            let total: i64 = count_row.try_get("count")?;

            let conversations_query = format!(
                "SELECT DISTINCT c.*
                 FROM conversations c
                 INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
                 WHERE ct.tag_id IN ({})
                 ORDER BY c.created_at DESC
                 LIMIT ? OFFSET ?",
                placeholders
            );

            let mut query_builder = sqlx::query(&conversations_query);
            for tag_id in tag_ids {
                query_builder = query_builder.bind(tag_id);
            }
            query_builder = query_builder.bind(limit).bind(offset);

            let rows = query_builder.fetch_all(&self.pool).await?;

            let mut conversations = Vec::new();
            for row in rows {
                let status_str: String = row.try_get("status")?;
                conversations.push(Conversation {
                    id: row.try_get("id")?,
                    reference_number: row.try_get("reference_number")?,
                    status: ConversationStatus::from(status_str),
                    inbox_id: row.try_get("inbox_id")?,
                    contact_id: row.try_get("contact_id")?,
                    subject: row.try_get("subject").ok(),
                    resolved_at: row.try_get("resolved_at").ok(),
                    closed_at: row.try_get("closed_at").ok(),
                    snoozed_until: row.try_get("snoozed_until").ok(),
                    assigned_user_id: row.try_get("assigned_user_id").ok(),
                    assigned_team_id: row.try_get("assigned_team_id").ok(),
                    assigned_at: row.try_get("assigned_at").ok(),
                    assigned_by: row.try_get("assigned_by").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    version: row.try_get("version")?,
                    tags: None,
                    priority: None,
                });
            }

            Ok((conversations, total))
        }
    }
}

// Repository trait implementation
#[async_trait::async_trait]
impl crate::domain::ports::conversation_tag_repository::ConversationTagRepository for Database {
    async fn get_conversation_tags(&self, conversation_id: &str) -> ApiResult<Vec<Tag>> {
        self.get_conversation_tags(conversation_id).await
    }

    async fn add_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
        user_id: &str,
    ) -> ApiResult<()> {
        self.add_conversation_tag(conversation_id, tag_id, user_id).await
    }

    async fn remove_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
    ) -> ApiResult<()> {
        self.remove_conversation_tag(conversation_id, tag_id).await
    }

    async fn replace_conversation_tags(
        &self,
        conversation_id: &str,
        tag_ids: &[String],
        user_id: &str,
    ) -> ApiResult<()> {
        self.replace_conversation_tags(conversation_id, tag_ids, user_id).await
    }

    async fn get_conversations_by_tag(
        &self,
        tag_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        self.get_conversations_by_tag(tag_id, limit, offset).await
    }
}
