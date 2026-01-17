use crate::{api::middleware::ApiResult, database::Database, models::AuthMethod, services::AuthLogger};
use std::sync::Arc;

/// Service wrapper for AuthLogger to avoid direct Database access in AppState
#[derive(Clone)]
pub struct AuthLoggerService {
    db: Arc<Database>,
}

impl AuthLoggerService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn log_rate_limit_exceeded(
        &self,
        email: String,
        auth_method: AuthMethod,
        ip_address: String,
        user_agent: String,
        wait_seconds: u64,
    ) -> ApiResult<()> {
        AuthLogger::log_rate_limit_exceeded(&self.db, email, auth_method, ip_address, Some(user_agent), wait_seconds).await
    }

    pub async fn get_user_events(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<crate::models::AuthEvent>> {
        AuthLogger::get_user_events(&self.db, user_id, limit, offset).await
    }

    pub async fn get_recent_events(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<crate::models::AuthEvent>> {
        AuthLogger::get_recent_events(&self.db, limit, offset).await
    }

    pub async fn log_login_success(
        &self,
        user_id: String,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: String,
    ) -> ApiResult<()> {
        AuthLogger::log_login_success(&self.db, user_id, email, auth_method, provider_name, ip_address, Some(user_agent)).await
    }

    pub async fn log_login_failure(
        &self,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: String,
        error_reason: String,
    ) -> ApiResult<()> {
        AuthLogger::log_login_failure(&self.db, email, auth_method, provider_name, ip_address, Some(user_agent), error_reason).await
    }

    pub async fn log_logout(
        &self,
        user_id: String,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: String,
    ) -> ApiResult<()> {
        AuthLogger::log_logout(&self.db, user_id, email, auth_method, provider_name, ip_address, Some(user_agent)).await
    }
}
