use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use crate::models::{AuthMethod, Session};
use sqlx::Row;
use time;

use crate::domain::ports::session_repository::SessionRepository;

#[async_trait::async_trait]
impl SessionRepository for Database {
    async fn create_session(&self, session: &Session) -> ApiResult<()> {
        let auth_method_str = match session.auth_method {
            AuthMethod::Password => "password",
            AuthMethod::Oidc => "oidc",
            AuthMethod::ApiKey => "apikey",
        };

        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, csrf_token, expires_at, created_at, last_accessed_at, auth_method, provider_name)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.token)
        .bind(&session.csrf_token)
        .bind(&session.expires_at)
        .bind(&session.created_at)
        .bind(&session.last_accessed_at)
        .bind(auth_method_str)
        .bind(&session.provider_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_session_by_token(&self, token: &str) -> ApiResult<Option<Session>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, csrf_token,
                    CAST(expires_at AS TEXT) as expires_at,
                    CAST(created_at AS TEXT) as created_at,
                    CAST(last_accessed_at AS TEXT) as last_accessed_at,
                    auth_method, provider_name
             FROM sessions
             WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => AuthMethod::Password,
                "oidc" => AuthMethod::Oidc,
                "apikey" => AuthMethod::ApiKey,
                _ => AuthMethod::Password,
            };

            // Handle NULL provider_name gracefully (NULL in DB becomes None)
            let provider_name: Option<String> = row.try_get("provider_name").ok();

            Ok(Some(Session {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                csrf_token: row.try_get("csrf_token")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                last_accessed_at: row.try_get("last_accessed_at")?,
                auth_method,
                provider_name,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete_session(&self, token: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> ApiResult<u64> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn get_user_sessions(&self, user_id: &str) -> ApiResult<Vec<Session>> {
        // Cast datetime columns to TEXT for compatibility with sqlx::any driver
        let rows = sqlx::query(
            "SELECT id, user_id, token, csrf_token,
                    CAST(expires_at AS TEXT) as expires_at,
                    CAST(created_at AS TEXT) as created_at,
                    CAST(last_accessed_at AS TEXT) as last_accessed_at,
                    auth_method, provider_name
             FROM sessions WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => AuthMethod::Password,
                "oidc" => AuthMethod::Oidc,
                "apikey" => AuthMethod::ApiKey,
                _ => AuthMethod::Password,
            };

            // Handle NULL provider_name gracefully (NULL in DB becomes None)
            let provider_name: Option<String> = row.try_get("provider_name").ok();

            sessions.push(Session {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                csrf_token: row.try_get("csrf_token")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                last_accessed_at: row.try_get("last_accessed_at")?,
                auth_method,
                provider_name,
            });
        }

        Ok(sessions)
    }

    async fn delete_user_sessions(&self, user_id: &str) -> ApiResult<u64> {
        let result = sqlx::query(
            "DELETE FROM sessions
             WHERE user_id = ?",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn update_session_last_accessed(&self, token: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query("UPDATE sessions SET last_accessed_at = ? WHERE token = ?")
            .bind(&now)
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
