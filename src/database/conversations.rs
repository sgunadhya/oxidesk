use crate::api::middleware::error::{ApiError, ApiResult};
use crate::database::Database;
use crate::models::{
    AssignmentHistory, Conversation, ConversationStatus, CreateConversation, Priority,
};

use sqlx::Row;
use time;
use tracing;
use uuid;

impl Database {
    // Conversation operations
    pub async fn create_conversation(
        &self,
        create: &CreateConversation,
    ) -> ApiResult<Conversation> {
        // Handle Option<String> for subject
        let subject_value: Option<&str> = create.subject.as_deref();

        tracing::debug!(
            "Creating conversation for inbox_id={}, contact_id={}",
            create.inbox_id,
            create.contact_id
        );

        // Generate conversation ID
        let conversation_id = uuid::Uuid::new_v4().to_string();

        // Insert the conversation
        sqlx::query(
            "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, subject, created_at, updated_at)
             VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations), 'open', ?, ?, ?, datetime('now'), datetime('now'))",
        )
        .bind(&conversation_id)
        .bind(&create.inbox_id)
        .bind(&create.contact_id)
        .bind(subject_value)
        .execute(&self.pool)
        .await?;

        // Fetch the created conversation using the generated ID
        let row = sqlx::query(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, created_at, updated_at, version
             FROM conversations
             WHERE id = ?",
        )
        .bind(&conversation_id)
        .fetch_one(&self.pool)
        .await?;

        let status_str: String = row.try_get("status")?;
        let conversation = Conversation {
            id: row.try_get("id")?,
            reference_number: row.try_get("reference_number")?,
            status: ConversationStatus::from(status_str),
            inbox_id: row.try_get("inbox_id")?,
            contact_id: row.try_get("contact_id")?,
            subject: row.try_get("subject").ok(),
            resolved_at: row.try_get("resolved_at").ok(),
            closed_at: None,
            snoozed_until: row.try_get("snoozed_until").ok(),
            assigned_user_id: None,
            assigned_team_id: None,
            assigned_at: None,
            assigned_by: None,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            version: row.try_get("version")?,
            tags: None,
            priority: None,
        };

        tracing::info!(
            "Conversation created: id={}, reference_number={}, status={:?}",
            conversation.id,
            conversation.reference_number,
            conversation.status
        );

        Ok(conversation)
    }

    pub async fn get_conversation_by_id(&self, id: &str) -> ApiResult<Option<Conversation>> {
        let row = sqlx::query(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, closed_at, snoozed_until, assigned_user_id, assigned_team_id,
                    assigned_at, assigned_by, created_at, updated_at, version, priority
             FROM conversations
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let conversation = Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: row.try_get("status")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            Ok(Some(conversation))
        } else {
            Ok(None)
        }
    }

    pub async fn get_conversation_by_reference_number(
        &self,
        reference_number: i64,
    ) -> ApiResult<Option<Conversation>> {
        let row = sqlx::query(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, created_at, updated_at, version
             FROM conversations
             WHERE reference_number = ?",
        )
        .bind(reference_number)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let conversation = Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: row.try_get("status")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            Ok(Some(conversation))
        } else {
            Ok(None)
        }
    }

    pub async fn update_conversation_fields(
        &self,
        id: &str,
        status: ConversationStatus,
        resolved_at: Option<String>,
        closed_at: Option<String>, // Feature 019
        snoozed_until: Option<String>,
    ) -> ApiResult<Conversation> {
        // Optimistic locking not strictly enforced here as previous version isn't passed,
        // but can be added if we pass expected_version.
        // For now, simple update.

        sqlx::query(
            "UPDATE conversations
             SET status = ?, resolved_at = ?, closed_at = ?, snoozed_until = ?, version = version + 1
             WHERE id = ?"
        )
        .bind(status.to_string())
        .bind(resolved_at)
        .bind(closed_at)
        .bind(snoozed_until)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_conversation_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found after update".to_string()))
    }

    pub async fn get_conversation_by_reference(
        &self,
        ref_num: i64,
    ) -> ApiResult<Option<Conversation>> {
        let row = sqlx::query("SELECT * FROM conversations WHERE reference_number = ?")
            .bind(ref_num)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let conversation = Conversation {
                id: row.try_get("id")?,
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                status: row.try_get("status")?,
                reference_number: row.try_get("reference_number")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            Ok(Some(conversation))
        } else {
            Ok(None)
        }
    }

    /// List conversations with pagination and optional filters
    pub async fn list_conversations(
        &self,
        limit: i64,
        offset: i64,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<Vec<Conversation>> {
        let mut query = String::from(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, created_at, updated_at, version, priority
             FROM conversations
             WHERE 1=1",
        );

        // Add filters
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if inbox_id.is_some() {
            query.push_str(" AND inbox_id = ?");
        }
        if contact_id.is_some() {
            query.push_str(" AND contact_id = ?");
        }

        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut sql_query = sqlx::query(&query);

        // Bind filter parameters
        if let Some(s) = status {
            sql_query = sql_query.bind(s.to_string());
        }
        if let Some(inbox) = inbox_id {
            sql_query = sql_query.bind(inbox);
        }
        if let Some(contact) = contact_id {
            sql_query = sql_query.bind(contact);
        }

        // Bind pagination parameters
        sql_query = sql_query.bind(limit).bind(offset);

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut conversations = Vec::new();
        for row in rows {
            use sqlx::Row;

            let conversation = Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: row.try_get("status")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            conversations.push(conversation);
        }

        Ok(conversations)
    }

    /// Count total conversations with optional filters
    pub async fn count_conversations(
        &self,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<i64> {
        let mut query = String::from("SELECT COUNT(*) as count FROM conversations WHERE 1=1");

        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if inbox_id.is_some() {
            query.push_str(" AND inbox_id = ?");
        }
        if contact_id.is_some() {
            query.push_str(" AND contact_id = ?");
        }

        let mut sql_query = sqlx::query(&query);

        if let Some(s) = status {
            sql_query = sql_query.bind(s.to_string());
        }
        if let Some(inbox) = inbox_id {
            sql_query = sql_query.bind(inbox);
        }
        if let Some(contact) = contact_id {
            sql_query = sql_query.bind(contact);
        }

        let row = sql_query.fetch_one(&self.pool).await?;
        use sqlx::Row;
        let count: i64 = row.try_get("count")?;

        Ok(count)
    }

    /// Set conversation priority (for automation rules)
    pub async fn set_conversation_priority(
        &self,
        conversation_id: &str,
        priority: &Priority,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE conversations
             SET priority = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(priority.to_string())
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error setting conversation priority: {:?}", e);
            ApiError::Internal(format!("Database error: {}", e))
        })?;

        tracing::info!(
            "Set priority to '{}' for conversation {}",
            priority,
            conversation_id
        );

        Ok(())
    }

    /// Clear conversation priority (set to null) - Feature 020
    pub async fn clear_conversation_priority(&self, conversation_id: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE conversations
             SET priority = NULL, updated_at = ?
             WHERE id = ?",
        )
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error clearing conversation priority: {:?}", e);
            ApiError::Internal(format!("Database error: {}", e))
        })?;

        tracing::info!("Cleared priority for conversation {}", conversation_id);

        Ok(())
    }

    /// Update conversation status (for automation rules - bypasses state machine)
    pub async fn update_conversation_status(
        &self,
        conversation_id: &str,
        status: ConversationStatus,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        // Set resolved_at if transitioning to Resolved
        let resolved_at = if status == ConversationStatus::Resolved {
            Some(now.clone())
        } else {
            None
        };

        // Clear resolved_at if not resolved
        if status != ConversationStatus::Resolved {
            sqlx::query(
                "UPDATE conversations
                 SET status = ?, resolved_at = NULL, updated_at = ?
                 WHERE id = ?",
            )
            .bind(status.to_string())
            .bind(&now)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error updating conversation status: {:?}", e);
                ApiError::Internal(format!("Database error: {}", e))
            })?;
        } else {
            sqlx::query(
                "UPDATE conversations
                 SET status = ?, resolved_at = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(status.to_string())
            .bind(&resolved_at)
            .bind(&now)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error updating conversation status: {:?}", e);
                ApiError::Internal(format!("Database error: {}", e))
            })?;
        }

        tracing::info!(
            "Updated status to {:?} for conversation {}",
            status,
            conversation_id
        );

        Ok(())
    }

    // ========== Assignment Operations ==========

    pub async fn assign_conversation_to_user(
        &self,
        conversation_id: &str,
        user_id: Option<String>,
        assigned_by: Option<String>,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE conversations
             SET assigned_user_id = ?, assigned_by = ?, assigned_at = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(user_id)
        .bind(assigned_by)
        .bind(&now)
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn assign_conversation_to_team(
        &self,
        conversation_id: &str,
        team_id: Option<String>,
        assigned_by: Option<String>,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE conversations
             SET assigned_team_id = ?, assigned_by = ?, assigned_at = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(team_id)
        .bind(assigned_by)
        .bind(&now)
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_conversation_participant(
        &self,
        conversation_id: &str,
        user_id: &str,
        _role: &str,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO conversation_participants (id, conversation_id, user_id, added_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(conversation_id)
        .bind(user_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_unassigned_conversations(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM conversations
             WHERE assigned_user_id IS NULL AND status = 'open'",
        )
        .fetch_one(&self.pool)
        .await?;
        let total_count: i64 = count_row.try_get("count")?;

        let rows = sqlx::query(
            "SELECT * FROM conversations
             WHERE assigned_user_id IS NULL AND status = 'open'
             ORDER BY created_at ASC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            use sqlx::Row;
            let conversation = Conversation {
                id: row.try_get("id")?,
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                status: row.try_get("status")?,
                reference_number: row.try_get("reference_number")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            conversations.push(conversation);
        }
        Ok((conversations, total_count))
    }

    pub async fn get_user_assigned_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM conversations
             WHERE assigned_user_id = ? AND status != 'closed'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let total_count: i64 = count_row.try_get("count")?;

        let rows = sqlx::query(
            "SELECT * FROM conversations
             WHERE assigned_user_id = ? AND status != 'closed'
             ORDER BY updated_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            use sqlx::Row;
            let conversation = Conversation {
                id: row.try_get("id")?,
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                status: row.try_get("status")?,
                reference_number: row.try_get("reference_number")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            conversations.push(conversation);
        }
        Ok((conversations, total_count))
    }

    pub async fn get_team_conversations(
        &self,
        team_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM conversations
             WHERE assigned_team_id = ? AND status != 'closed'",
        )
        .bind(team_id)
        .fetch_one(&self.pool)
        .await?;
        let total_count: i64 = count_row.try_get("count")?;

        let rows = sqlx::query(
            "SELECT * FROM conversations
             WHERE assigned_team_id = ? AND status != 'closed'
             ORDER BY updated_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(team_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            use sqlx::Row;
            let conversation = Conversation {
                id: row.try_get("id")?,
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                status: row.try_get("status")?,
                reference_number: row.try_get("reference_number")?,
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
                priority: row
                    .try_get::<Option<String>, _>("priority")
                    .ok()
                    .flatten()
                    .map(Priority::from),
            };
            conversations.push(conversation);
        }
        Ok((conversations, total_count))
    }

    pub async fn unassign_agent_open_conversations(&self, user_id: &str) -> ApiResult<u64> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query(
            "UPDATE conversations
             SET assigned_user_id = NULL, assigned_at = NULL, updated_at = ?
             WHERE assigned_user_id = ? AND status = 'open'",
        )
        .bind(&now)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn unassign_conversation_user(&self, conversation_id: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE conversations
             SET assigned_user_id = NULL, assigned_at = NULL, updated_at = ?
             WHERE id = ?",
        )
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_assignment(&self, history: &AssignmentHistory) -> ApiResult<()> {
        let _now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "INSERT INTO assignment_history (id, conversation_id, assigned_user_id, assigned_team_id, assigned_by, assigned_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&history.id)
        .bind(&history.conversation_id)
        .bind(&history.assigned_user_id)
        .bind(&history.assigned_team_id)
        .bind(&history.assigned_by)
        .bind(&history.assigned_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_assignment_history(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Vec<AssignmentHistory>> {
        let rows = sqlx::query(
            "SELECT * FROM assignment_history
             WHERE conversation_id = ?
             ORDER BY assigned_at DESC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut history = Vec::new();
        for row in rows {
            use sqlx::Row;
            history.push(AssignmentHistory {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_by: row.try_get("assigned_by")?,
                assigned_at: row.try_get("assigned_at")?,
                unassigned_at: row.try_get("unassigned_at").ok(),
            });
        }
        Ok(history)
    }

    pub async fn get_conversation_participants(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Vec<String>> {
        let rows = sqlx::query(
            "SELECT user_id FROM conversation_participants
             WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut user_ids = Vec::new();
        for row in rows {
            user_ids.push(row.try_get("user_id")?);
        }
        Ok(user_ids)
    }
}
