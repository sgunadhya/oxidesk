use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    Password,
    Oidc,
    ApiKey,
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::Password
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub csrf_token: String,
    pub expires_at: String,
    pub created_at: String,
    pub last_accessed_at: String,
    pub auth_method: AuthMethod,
    pub provider_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub csrf_token: String,
    pub expires_at: String,
    pub user: crate::models::user::AgentResponse,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub user_id: String,
    pub expires_at: String,
    pub created_at: String,
}

impl Session {
    pub fn new(user_id: String, token: String, duration_hours: i64) -> Self {
        Self::new_with_method(user_id, token, duration_hours, AuthMethod::Password, None)
    }

    pub fn new_with_method(
        user_id: String,
        token: String,
        duration_hours: i64,
        auth_method: AuthMethod,
        provider_name: Option<String>,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let expires_at = now + time::Duration::hours(duration_hours);

        // Generate CSRF token using csrf service
        let csrf_token = crate::services::csrf::generate_csrf_token();

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            token,
            csrf_token,
            expires_at: expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            created_at: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            last_accessed_at: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            auth_method,
            provider_name,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Ok(expires_at) = time::OffsetDateTime::parse(
            &self.expires_at,
            &time::format_description::well_known::Rfc3339,
        ) {
            expires_at < time::OffsetDateTime::now_utc()
        } else {
            true
        }
    }
}
