use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetToken {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: String,
    pub used: bool,
    pub created_at: String,
}

impl PasswordResetToken {
    pub fn new(user_id: String, token: String) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let expires_at = now + time::Duration::hours(1); // 1 hour expiration

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            token,
            expires_at: expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            used: false,
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
