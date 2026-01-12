use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
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
        let now = time::OffsetDateTime::now_utc();
        let expires_at = now + time::Duration::hours(duration_hours);

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            token,
            expires_at: expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            created_at: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
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
