use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::persistence::Database;
use sqlx::Row;
use time;

impl Database {
    // ========================================
    // SLA Policy Operations
    // ========================================

    /// Create a new SLA policy
    pub async fn create_sla_policy(&self, policy: &crate::domain::entities::SlaPolicy) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO sla_policies (id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&policy.id)
        .bind(&policy.name)
        .bind(&policy.description)
        .bind(&policy.first_response_time)
        .bind(&policy.resolution_time)
        .bind(&policy.next_response_time)
        .bind(&policy.created_at)
        .bind(&policy.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get SLA policy by ID
    pub async fn get_sla_policy(&self, id: &str) -> ApiResult<Option<crate::domain::entities::SlaPolicy>> {
        let row = sqlx::query(
            "SELECT id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at
             FROM sla_policies WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::domain::entities::SlaPolicy {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten(),
                first_response_time: row.try_get("first_response_time")?,
                resolution_time: row.try_get("resolution_time")?,
                next_response_time: row.try_get("next_response_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get SLA policy by name
    pub async fn get_sla_policy_by_name(
        &self,
        name: &str,
    ) -> ApiResult<Option<crate::domain::entities::SlaPolicy>> {
        let row = sqlx::query(
            "SELECT id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at
             FROM sla_policies WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::domain::entities::SlaPolicy {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten(),
                first_response_time: row.try_get("first_response_time")?,
                resolution_time: row.try_get("resolution_time")?,
                next_response_time: row.try_get("next_response_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all SLA policies with pagination
    pub async fn list_sla_policies(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<crate::domain::entities::SlaPolicy>, i64)> {
        let rows = sqlx::query(
            "SELECT id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at
             FROM sla_policies
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let policies: Vec<crate::domain::entities::SlaPolicy> = rows
            .iter()
            .map(|row| {
                Ok(crate::domain::entities::SlaPolicy {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    description: row
                        .try_get::<Option<String>, _>("description")
                        .ok()
                        .flatten(),
                    first_response_time: row.try_get("first_response_time")?,
                    resolution_time: row.try_get("resolution_time")?,
                    next_response_time: row.try_get("next_response_time")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<ApiResult<Vec<_>>>()?;

        let count_row = sqlx::query("SELECT COUNT(*) as count FROM sla_policies")
            .fetch_one(&self.pool)
            .await?;
        let total: i64 = count_row.try_get("count")?;

        Ok((policies, total))
    }

    /// Update SLA policy
    pub async fn update_sla_policy(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
        first_response_time: Option<&str>,
        resolution_time: Option<&str>,
        next_response_time: Option<&str>,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let mut query_parts = Vec::new();
        let mut bindings: Vec<String> = Vec::new();

        if let Some(name) = name {
            query_parts.push("name = ?");
            bindings.push(name.to_string());
        }

        if let Some(desc) = description {
            query_parts.push("description = ?");
            bindings.push(desc.map(|s| s.to_string()).unwrap_or_default());
        }

        if let Some(time) = first_response_time {
            query_parts.push("first_response_time = ?");
            bindings.push(time.to_string());
        }

        if let Some(time) = resolution_time {
            query_parts.push("resolution_time = ?");
            bindings.push(time.to_string());
        }

        if let Some(time) = next_response_time {
            query_parts.push("next_response_time = ?");
            bindings.push(time.to_string());
        }

        if query_parts.is_empty() {
            return Ok(());
        }

        query_parts.push("updated_at = ?");
        bindings.push(now.clone());

        let query_str = format!(
            "UPDATE sla_policies SET {} WHERE id = ?",
            query_parts.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for binding in bindings {
            query = query.bind(binding);
        }
        query = query.bind(id);

        query.execute(&self.pool).await?;

        Ok(())
    }

    /// Delete SLA policy
    pub async fn delete_sla_policy(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM sla_policies WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================
    // Applied SLA Operations
    // ========================================

    /// Create a new applied SLA
    pub async fn create_applied_sla(
        &self,
        applied_sla: &crate::domain::entities::AppliedSla,
    ) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO applied_slas (id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&applied_sla.id)
        .bind(&applied_sla.conversation_id)
        .bind(&applied_sla.sla_policy_id)
        .bind(applied_sla.status.to_string())
        .bind(&applied_sla.first_response_deadline_at)
        .bind(&applied_sla.resolution_deadline_at)
        .bind(&applied_sla.applied_at)
        .bind(&applied_sla.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get applied SLA by ID
    pub async fn get_applied_sla(&self, id: &str) -> ApiResult<Option<crate::domain::entities::AppliedSla>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
             FROM applied_slas WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::domain::entities::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get applied SLA by conversation ID
    pub async fn get_applied_sla_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Option<crate::domain::entities::AppliedSla>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
             FROM applied_slas WHERE conversation_id = ?"
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::domain::entities::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get applied SLA by ID
    pub async fn get_applied_sla_by_id(
        &self,
        id: &str,
    ) -> ApiResult<Option<crate::domain::entities::AppliedSla>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
             FROM applied_slas WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::domain::entities::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List applied SLAs with optional filters
    pub async fn list_applied_slas(
        &self,
        status_filter: Option<crate::domain::entities::AppliedSlaStatus>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<crate::domain::entities::AppliedSla>, i64)> {
        let (query_str, count_query_str) = if status_filter.is_some() {
            (
                "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
                 FROM applied_slas WHERE status = ? ORDER BY applied_at DESC LIMIT ? OFFSET ?",
                "SELECT COUNT(*) as count FROM applied_slas WHERE status = ?"
            )
        } else {
            (
                "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
                 FROM applied_slas ORDER BY applied_at DESC LIMIT ? OFFSET ?",
                "SELECT COUNT(*) as count FROM applied_slas"
            )
        };

        let rows = if let Some(status) = status_filter {
            sqlx::query(query_str)
                .bind(status.to_string())
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query_str)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };

        let applied_slas: Vec<crate::domain::entities::AppliedSla> = rows
            .iter()
            .map(|row| {
                let status_str: String = row.try_get("status")?;
                Ok(crate::domain::entities::AppliedSla {
                    id: row.try_get("id")?,
                    conversation_id: row.try_get("conversation_id")?,
                    sla_policy_id: row.try_get("sla_policy_id")?,
                    status: status_str.parse().map_err(|e: String| {
                        sqlx::Error::Decode(Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e,
                        )))
                    })?,
                    first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                    resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                    applied_at: row.try_get("applied_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<ApiResult<Vec<_>>>()?;

        let count_row = if let Some(status) = status_filter {
            sqlx::query(count_query_str)
                .bind(status.to_string())
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query(count_query_str).fetch_one(&self.pool).await?
        };

        let total: i64 = count_row.try_get("count")?;

        Ok((applied_slas, total))
    }

    /// Update applied SLA status
    pub async fn update_applied_sla_status(
        &self,
        id: &str,
        status: crate::domain::entities::AppliedSlaStatus,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query("UPDATE applied_slas SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.to_string())
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete applied SLA
    pub async fn delete_applied_sla(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM applied_slas WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================
    // SLA Event Operations
    // ========================================

    /// Create a new SLA event
    pub async fn create_sla_event(&self, event: &crate::domain::entities::SlaEvent) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO sla_events (id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&event.id)
        .bind(&event.applied_sla_id)
        .bind(event.event_type.to_string())
        .bind(event.status.to_string())
        .bind(&event.deadline_at)
        .bind(&event.met_at)
        .bind(&event.breached_at)
        .bind(&event.created_at)
        .bind(&event.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get SLA event by ID
    pub async fn get_sla_event(&self, id: &str) -> ApiResult<Option<crate::domain::entities::SlaEvent>> {
        let row = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await.map_err(|e| {
            ApiError::Internal(format!("Database error: {}", e))
        })?;

        if let Some(row) = row {
            let event_type_str: String = row.try_get("event_type").map_err(|e| {
                ApiError::Internal(format!("Column parsing error event_type: {}", e))
            })?;

            let status_str: String = row
                .try_get("status")
                .map_err(|e| ApiError::Internal(format!("Column parsing error status: {}", e)))?;

            let event_type = event_type_str.parse().map_err(|e: String| {
                ApiError::Internal(format!("Event type parsing error: {}", e))
            })?;

            let status = status_str
                .parse()
                .map_err(|e: String| ApiError::Internal(format!("Status parsing error: {}", e)))?;

            Ok(Some(crate::domain::entities::SlaEvent {
                id: row
                    .try_get("id")
                    .map_err(|e| ApiError::Internal(format!("Column parsing error id: {}", e)))?,
                applied_sla_id: row.try_get("applied_sla_id").map_err(|e| {
                    ApiError::Internal(format!("Column parsing error applied_sla_id: {}", e))
                })?,
                event_type,
                status,
                deadline_at: row.try_get("deadline_at").map_err(|e| {
                    ApiError::Internal(format!("Column parsing error deadline_at: {}", e))
                })?,
                met_at: row
                    .try_get::<Option<String>, _>("met_at")
                    .or_else(|_| Ok::<_, sqlx::Error>(None))
                    .map_err(|e| {
                        ApiError::Internal(format!("Column parsing error met_at: {}", e))
                    })?,
                breached_at: row
                    .try_get::<Option<String>, _>("breached_at")
                    .or_else(|_| Ok::<_, sqlx::Error>(None))
                    .map_err(|e| {
                        ApiError::Internal(format!("Column parsing error breached_at: {}", e))
                    })?,
                created_at: row.try_get("created_at").map_err(|e| {
                    ApiError::Internal(format!("Column parsing error created_at: {}", e))
                })?,
                updated_at: row.try_get("updated_at").map_err(|e| {
                    ApiError::Internal(format!("Column parsing error updated_at: {}", e))
                })?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all SLA events for an applied SLA
    pub async fn get_sla_events_by_applied_sla(
        &self,
        applied_sla_id: &str,
    ) -> ApiResult<Vec<crate::domain::entities::SlaEvent>> {
        let rows = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE applied_sla_id = ? ORDER BY created_at ASC"
        )
        .bind(applied_sla_id)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows.iter() {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;

            let event_type = event_type_str.parse().map_err(|e: String| {
                crate::infrastructure::http::middleware::ApiError::Internal(format!("Invalid event_type: {}", e))
            })?;

            let status = status_str.parse().map_err(|e: String| {
                crate::infrastructure::http::middleware::ApiError::Internal(format!("Invalid status: {}", e))
            })?;

            let id: String = row.try_get("id")?;
            let applied_sla_id: String = row.try_get("applied_sla_id")?;
            let deadline_at: String = row.try_get("deadline_at")?;
            // For nullable columns, try_get may fail with NULL values in sqlx Any driver
            // Use a workaround to handle this
            let met_at: Option<String> = row
                .try_get::<Option<String>, _>("met_at")
                .or_else(|_| Ok::<_, sqlx::Error>(None))?;
            let breached_at: Option<String> = row
                .try_get::<Option<String>, _>("breached_at")
                .or_else(|_| Ok::<_, sqlx::Error>(None))?;
            let created_at: String = row.try_get("created_at")?;
            let updated_at: String = row.try_get("updated_at")?;

            events.push(crate::domain::entities::SlaEvent {
                id,
                applied_sla_id,
                event_type,
                status,
                deadline_at,
                met_at,
                breached_at,
                created_at,
                updated_at,
            });
        }

        Ok(events)
    }

    /// Get pending SLA event by type for an applied SLA
    pub async fn get_pending_sla_event(
        &self,
        applied_sla_id: &str,
        event_type: crate::domain::entities::SlaEventType,
    ) -> ApiResult<Option<crate::domain::entities::SlaEvent>> {
        let row = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE applied_sla_id = ? AND event_type = ? AND status = 'pending'"
        )
        .bind(applied_sla_id)
        .bind(event_type.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::domain::entities::SlaEvent {
                id: row.try_get("id")?,
                applied_sla_id: row.try_get("applied_sla_id")?,
                event_type: event_type_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    )))
                })?,
                deadline_at: row.try_get("deadline_at")?,
                // For nullable columns, try_get may fail with NULL values in sqlx Any driver
                met_at: row
                    .try_get::<Option<String>, _>("met_at")
                    .or_else(|_| Ok::<_, sqlx::Error>(None))?,
                breached_at: row
                    .try_get::<Option<String>, _>("breached_at")
                    .or_else(|_| Ok::<_, sqlx::Error>(None))?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all pending SLA events past their deadline
    pub async fn get_pending_events_past_deadline(
        &self,
    ) -> ApiResult<Vec<crate::domain::entities::SlaEvent>> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let rows = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE status = 'pending' AND deadline_at < ? ORDER BY deadline_at ASC"
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows.iter() {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;

            let event_type = event_type_str.parse().map_err(|e: String| {
                crate::infrastructure::http::middleware::ApiError::Internal(format!("Invalid event_type: {}", e))
            })?;

            let status = status_str.parse().map_err(|e: String| {
                crate::infrastructure::http::middleware::ApiError::Internal(format!("Invalid status: {}", e))
            })?;

            events.push(crate::domain::entities::SlaEvent {
                id: row.try_get("id")?,
                applied_sla_id: row.try_get("applied_sla_id")?,
                event_type,
                status,
                deadline_at: row.try_get("deadline_at")?,
                // For nullable columns, try_get may fail with NULL values in sqlx Any driver
                met_at: row
                    .try_get::<Option<String>, _>("met_at")
                    .or_else(|_| Ok::<_, sqlx::Error>(None))?,
                breached_at: row
                    .try_get::<Option<String>, _>("breached_at")
                    .or_else(|_| Ok::<_, sqlx::Error>(None))?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(events)
    }

    /// Mark SLA event as met
    pub async fn mark_sla_event_met(&self, event_id: &str, met_at: &str) -> ApiResult<()> {
        // Feature 025: Validate status exclusivity before update
        // Get existing event to check if it's already breached
        let existing_event = self
            .get_sla_event(event_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("SLA event not found".to_string()))?;

        if existing_event.breached_at.is_some() {
            return Err(ApiError::BadRequest(
                "SLA event status is exclusive".to_string(),
            ));
        }

        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE sla_events SET status = 'met', met_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(met_at)
        .bind(now)
        .bind(event_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark SLA event as breached
    pub async fn mark_sla_event_breached(
        &self,
        event_id: &str,
        breached_at: &str,
    ) -> ApiResult<()> {
        // Feature 025: Validate status exclusivity before update
        // Get existing event to check if it's already met
        let existing_event = self
            .get_sla_event(event_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("SLA event not found".to_string()))?;

        if existing_event.met_at.is_some() {
            return Err(ApiError::BadRequest(
                "SLA event status is exclusive".to_string(),
            ));
        }

        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE sla_events SET status = 'breached', breached_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind(breached_at)
        .bind(now)
        .bind(event_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete SLA event
    pub async fn delete_sla_event(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM sla_events WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

// Implement SlaRepository trait for Database
#[async_trait::async_trait]
impl crate::domain::ports::sla_repository::SlaRepository for Database {
    async fn create_sla_policy(&self, policy: &crate::domain::entities::SlaPolicy) -> ApiResult<()> {
        self.create_sla_policy(policy).await
    }

    async fn get_sla_policy(&self, policy_id: &str) -> ApiResult<Option<crate::domain::entities::SlaPolicy>> {
        self.get_sla_policy(policy_id).await
    }

    async fn get_sla_policy_by_name(&self, name: &str) -> ApiResult<Option<crate::domain::entities::SlaPolicy>> {
        self.get_sla_policy_by_name(name).await
    }

    async fn list_sla_policies(&self, limit: i64, offset: i64) -> ApiResult<(Vec<crate::domain::entities::SlaPolicy>, i64)> {
        self.list_sla_policies(limit, offset).await
    }

    async fn update_sla_policy(
        &self,
        policy_id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
        first_response_time: Option<&str>,
        resolution_time: Option<&str>,
        next_response_time: Option<&str>,
    ) -> ApiResult<()> {
        self.update_sla_policy(policy_id, name, description, first_response_time, resolution_time, next_response_time).await
    }

    async fn delete_sla_policy(&self, policy_id: &str) -> ApiResult<()> {
        self.delete_sla_policy(policy_id).await
    }

    async fn create_applied_sla(&self, applied_sla: &crate::domain::entities::AppliedSla) -> ApiResult<()> {
        self.create_applied_sla(applied_sla).await
    }

    async fn get_applied_sla(&self, applied_sla_id: &str) -> ApiResult<Option<crate::domain::entities::AppliedSla>> {
        self.get_applied_sla(applied_sla_id).await
    }

    async fn get_applied_sla_by_id(&self, applied_sla_id: &str) -> ApiResult<Option<crate::domain::entities::AppliedSla>> {
        self.get_applied_sla_by_id(applied_sla_id).await
    }

    async fn get_applied_sla_by_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Option<crate::domain::entities::AppliedSla>> {
        self.get_applied_sla_by_conversation(conversation_id).await
    }

    async fn list_applied_slas(
        &self,
        status_filter: Option<crate::domain::entities::AppliedSlaStatus>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<crate::domain::entities::AppliedSla>, i64)> {
        self.list_applied_slas(status_filter, limit, offset).await
    }

    async fn update_applied_sla_status(
        &self,
        applied_sla_id: &str,
        status: crate::domain::entities::AppliedSlaStatus,
    ) -> ApiResult<()> {
        self.update_applied_sla_status(applied_sla_id, status).await
    }

    async fn create_sla_event(&self, event: &crate::domain::entities::SlaEvent) -> ApiResult<()> {
        self.create_sla_event(event).await
    }

    async fn get_sla_event(&self, event_id: &str) -> ApiResult<Option<crate::domain::entities::SlaEvent>> {
        self.get_sla_event(event_id).await
    }

    async fn get_sla_events_by_applied_sla(&self, applied_sla_id: &str) -> ApiResult<Vec<crate::domain::entities::SlaEvent>> {
        self.get_sla_events_by_applied_sla(applied_sla_id).await
    }

    async fn get_pending_sla_event(
        &self,
        applied_sla_id: &str,
        event_type: crate::domain::entities::SlaEventType,
    ) -> ApiResult<Option<crate::domain::entities::SlaEvent>> {
        self.get_pending_sla_event(applied_sla_id, event_type).await
    }

    async fn get_pending_events_past_deadline(&self) -> ApiResult<Vec<crate::domain::entities::SlaEvent>> {
        self.get_pending_events_past_deadline().await
    }

    async fn mark_sla_event_met(&self, event_id: &str, met_at: &str) -> ApiResult<()> {
        self.mark_sla_event_met(event_id, met_at).await
    }

    async fn mark_sla_event_breached(&self, event_id: &str, breached_at: &str) -> ApiResult<()> {
        self.mark_sla_event_breached(event_id, breached_at).await
    }

    async fn is_holiday(&self, date: &str) -> ApiResult<bool> {
        self.is_holiday(date).await
    }
}
