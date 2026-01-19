use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Entity: Password reset token stored in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PasswordResetToken {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: String,
    pub used: bool,
    pub created_at: String,
}

/// DTO: Request to initiate password reset
#[derive(Debug, Clone, Deserialize)]
pub struct RequestPasswordResetRequest {
    pub email: String,
}

/// DTO: Response for password reset request
#[derive(Debug, Clone, Serialize)]
pub struct RequestPasswordResetResponse {
    pub message: String,
}

/// DTO: Request to complete password reset
#[derive(Debug, Clone, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

/// DTO: Response for password reset completion
#[derive(Debug, Clone, Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
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
