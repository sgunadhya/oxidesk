use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::persistence::Database;
use crate::domain::entities::{User, UserType};
use sqlx::Row;

use crate::domain::ports::user_repository::UserRepository;
use async_trait::async_trait;

// Internal helpers
impl Database {
    pub(crate) async fn create_user_internal<'e, E>(
        &self,
        executor: E,
        user: &User,
    ) -> ApiResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Any>,
    {
        sqlx::query(
            "INSERT INTO users (id, email, user_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(match user.user_type {
            UserType::Agent => "agent",
            UserType::Contact => "contact",
        })
        .bind(&user.created_at)
        .bind(&user.updated_at)
        .execute(executor)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl UserRepository for Database {
    // User operations
    async fn create_user(&self, user: &User) -> ApiResult<()> {
        self.create_user_internal(&self.pool, user).await
    }

    async fn get_user_by_email_and_type(
        &self,
        email: &str,
        user_type: &UserType,
    ) -> ApiResult<Option<User>> {
        let user_type_str = match user_type {
            UserType::Agent => "agent",
            UserType::Contact => "contact",
        };

        let row = sqlx::query(
            "SELECT id, email, user_type, created_at, updated_at, deleted_at, deleted_by
             FROM users
             WHERE email = ? AND user_type = ? AND deleted_at IS NULL",
        )
        .bind(email)
        .bind(user_type_str)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if row.try_get::<String, _>("user_type")? == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: row.try_get("deleted_at").ok(),
                deleted_by: row.try_get("deleted_by").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_id(&self, id: &str) -> ApiResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, email, user_type, created_at, updated_at, deleted_at, deleted_by
             FROM users
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if row.try_get::<String, _>("user_type")? == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: row.try_get("deleted_at").ok(),
                deleted_by: row.try_get("deleted_by").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_user_email(&self, id: &str, email: &str, updated_at: &str) -> ApiResult<()> {
        sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
            .bind(email)
            .bind(updated_at)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Soft Delete operations
    /// Soft delete a user (agent or contact)
    /// Sets deleted_at timestamp and records who performed the deletion
    async fn soft_delete_user(&self, user_id: &str, deleted_by: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE users
             SET deleted_at = ?, deleted_by = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&now)
        .bind(deleted_by)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "User not found or already deleted".to_string(),
            ));
        }

        // Invalidate all sessions for the soft deleted user
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Restore a soft deleted user
    /// Clears deleted_at and deleted_by fields
    async fn restore_user(&self, user_id: &str) -> ApiResult<()> {
        let result = sqlx::query(
            "UPDATE users
             SET deleted_at = NULL, deleted_by = NULL
             WHERE id = ? AND deleted_at IS NOT NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "User not found or not deleted".to_string(),
            ));
        }

        Ok(())
    }

    // Generic user operations
    async fn list_users(
        &self,
        limit: i64,
        offset: i64,
        user_type_filter: Option<UserType>,
    ) -> ApiResult<(Vec<User>, i64)> {
        let (query, count_query) = if let Some(user_type) = user_type_filter {
            let type_str = match user_type {
                UserType::Agent => "agent",
                UserType::Contact => "contact",
            };

            (
                format!(
                    "SELECT id, email, user_type, created_at, updated_at
                     FROM users
                     WHERE user_type = '{}'
                     ORDER BY created_at DESC
                     LIMIT ? OFFSET ?",
                    type_str
                ),
                format!(
                    "SELECT COUNT(*) as count
                     FROM users
                     WHERE user_type = '{}'",
                    type_str
                ),
            )
        } else {
            (
                "SELECT id, email, user_type, created_at, updated_at
                 FROM users
                 ORDER BY created_at DESC
                 LIMIT ? OFFSET ?"
                    .to_string(),
                "SELECT COUNT(*) as count FROM users".to_string(),
            )
        };

        // Get total count
        let count_row = sqlx::query(&count_query).fetch_one(&self.pool).await?;
        let total_count: i64 = count_row.try_get("count")?;

        // Get users
        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            let user_type = match user_type_str.as_str() {
                "agent" => UserType::Agent,
                "contact" => UserType::Contact,
                _ => continue,
            };

            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: None,
                deleted_by: None,
            });
        }

        Ok((users, total_count))
    }

    async fn get_users_by_usernames(&self, usernames: &[String]) -> ApiResult<Vec<User>> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        // Build the placeholders for the IN clause
        let placeholders = usernames.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_str = format!(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at
             FROM users u
             JOIN agents a ON u.id = a.user_id
             WHERE a.first_name IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for username in usernames {
            query = query.bind(username);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if user_type_str == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: None,
                deleted_by: None,
            });
        }

        Ok(users)
    }

    async fn get_users_by_ids(&self, ids: &[String]) -> ApiResult<Vec<User>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_str = format!(
            "SELECT id, email, user_type, created_at, updated_at, deleted_at, deleted_by
             FROM users
             WHERE id IN ({}) AND deleted_at IS NULL",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for id in ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if user_type_str == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: row.try_get("deleted_at").ok(),
                deleted_by: row.try_get("deleted_by").ok(),
            });
        }

        Ok(users)
    }

    async fn count_admin_users(&self) -> ApiResult<i64> {
        Database::count_admin_users(self).await
    }

    async fn delete_user(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
