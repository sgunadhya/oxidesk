use async_trait::async_trait;
use sqlx::Row;

use crate::{domain::entities::AuthEvent, infrastructure::http::middleware::error::ApiResult, infrastructure::persistence::Database};

#[async_trait]
impl AuthEventRepository for Database {
    /// Create a new authentication event
    async fn create_auth_event(&self, event: &AuthEvent) -> ApiResult<()> {
        let event_type_str = event.event_type.to_string();
        let auth_method_str = match event.auth_method {
            crate::domain::entities::AuthMethod::Password => "password",
            crate::domain::entities::AuthMethod::Oidc => "oidc",
            crate::domain::entities::AuthMethod::ApiKey => "apikey",
        };

        sqlx::query(
            "INSERT INTO auth_events (id, event_type, user_id, email, auth_method, provider_name, ip_address, user_agent, error_reason, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
            .bind(&event.id)
            .bind(&event_type_str)
            .bind(&event.user_id)
            .bind(&event.email)
            .bind(auth_method_str)
            .bind(&event.provider_name)
            .bind(&event.ip_address)
            .bind(&event.user_agent)
            .bind(&event.error_reason)
            .bind(&event.timestamp)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Get auth events for a specific user with pagination
    async fn get_auth_events_by_user(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<AuthEvent>> {
        let rows = sqlx::query(
            "SELECT id, event_type, user_id, email, auth_method, provider_name, ip_address, user_agent, error_reason, timestamp
             FROM auth_events
             WHERE user_id = ?
             ORDER BY timestamp DESC
             LIMIT ? OFFSET ?",
        )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut events = Vec::new();
        for row in rows {
            let event_type_str: String = row.try_get("event_type")?;
            let event_type = match event_type_str.as_str() {
                "login_success" => crate::domain::entities::AuthEventType::LoginSuccess,
                "login_failure" => crate::domain::entities::AuthEventType::LoginFailure,
                "logout" => crate::domain::entities::AuthEventType::Logout,
                "session_expired" => crate::domain::entities::AuthEventType::SessionExpired,
                "rate_limit_exceeded" => crate::domain::entities::AuthEventType::RateLimitExceeded,
                _ => crate::domain::entities::AuthEventType::LoginFailure,
            };

            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => crate::domain::entities::AuthMethod::Password,
                "oidc" => crate::domain::entities::AuthMethod::Oidc,
                _ => crate::domain::entities::AuthMethod::Password,
            };

            events.push(AuthEvent {
                id: row.try_get("id")?,
                event_type,
                user_id: row.try_get("user_id")?,
                email: row.try_get("email")?,
                auth_method,
                provider_name: row.try_get("provider_name")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                error_reason: row.try_get("error_reason")?,
                timestamp: row.try_get("timestamp")?,
            });
        }

        Ok(events)
    }
    /// Get recent auth events (admin view) with pagination
    async fn get_recent_auth_events(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<AuthEvent>> {
        let rows = sqlx::query(
            "SELECT id, event_type, user_id, email, auth_method, provider_name, ip_address, user_agent, error_reason, timestamp
             FROM auth_events
             ORDER BY timestamp DESC
             LIMIT ? OFFSET ?",
        )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut events = Vec::new();
        for row in rows {
            let event_type_str: String = row.try_get("event_type")?;
            let event_type = match event_type_str.as_str() {
                "login_success" => crate::domain::entities::AuthEventType::LoginSuccess,
                "login_failure" => crate::domain::entities::AuthEventType::LoginFailure,
                "logout" => crate::domain::entities::AuthEventType::Logout,
                "session_expired" => crate::domain::entities::AuthEventType::SessionExpired,
                "rate_limit_exceeded" => crate::domain::entities::AuthEventType::RateLimitExceeded,
                _ => crate::domain::entities::AuthEventType::LoginFailure,
            };

            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => crate::domain::entities::AuthMethod::Password,
                "oidc" => crate::domain::entities::AuthMethod::Oidc,
                _ => crate::domain::entities::AuthMethod::Password,
            };

            events.push(AuthEvent {
                id: row.try_get("id")?,
                event_type,
                user_id: row.try_get("user_id")?,
                email: row.try_get("email")?,
                auth_method,
                provider_name: row.try_get("provider_name")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                error_reason: row.try_get("error_reason")?,
                timestamp: row.try_get("timestamp")?,
            });
        }

        Ok(events)
    }
}

#[async_trait]
pub trait AuthEventRepository: Send + Sync {
    /// Create a new authentication event
    async fn create_auth_event(&self, event: &AuthEvent) -> ApiResult<()>;
    /// Get auth events for a specific user with pagination
    async fn get_auth_events_by_user(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<AuthEvent>>;
    /// Get recent auth events (admin view) with pagination
    async fn get_recent_auth_events(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<AuthEvent>>;
}
