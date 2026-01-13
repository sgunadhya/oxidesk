use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::session::AuthMethod;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthEventType {
    LoginSuccess,
    LoginFailure,
    Logout,
    SessionExpired,
    RateLimitExceeded,
    AuthorizationDenied,  // RBAC System: Permission check failures
}

impl std::fmt::Display for AuthEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthEventType::LoginSuccess => write!(f, "login_success"),
            AuthEventType::LoginFailure => write!(f, "login_failure"),
            AuthEventType::Logout => write!(f, "logout"),
            AuthEventType::SessionExpired => write!(f, "session_expired"),
            AuthEventType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            AuthEventType::AuthorizationDenied => write!(f, "authorization_denied"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEvent {
    pub id: String,
    pub event_type: AuthEventType,
    pub user_id: Option<String>,
    pub email: String,
    pub auth_method: AuthMethod,
    pub provider_name: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub error_reason: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct AuthEventResponse {
    pub id: String,
    pub event_type: AuthEventType,
    pub user_id: Option<String>,
    pub email: String,
    pub auth_method: AuthMethod,
    pub provider_name: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub error_reason: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct AuthEventListResponse {
    pub events: Vec<AuthEventResponse>,
    pub total: i64,
}

impl AuthEvent {
    pub fn new(
        event_type: AuthEventType,
        user_id: Option<String>,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: Option<String>,
        error_reason: Option<String>,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            user_id,
            email,
            auth_method,
            provider_name,
            ip_address,
            user_agent,
            error_reason,
            timestamp: now,
        }
    }
}

impl From<AuthEvent> for AuthEventResponse {
    fn from(event: AuthEvent) -> Self {
        Self {
            id: event.id,
            event_type: event.event_type,
            user_id: event.user_id,
            email: event.email,
            auth_method: event.auth_method,
            provider_name: event.provider_name,
            ip_address: event.ip_address,
            user_agent: event.user_agent,
            error_reason: event.error_reason,
            timestamp: event.timestamp,
        }
    }
}
