use crate::{
    infrastructure::http::middleware::error::ApiResult,
    infrastructure::persistence::Database,
    domain::entities::PasswordResetToken,
};

#[derive(Clone)]
pub struct PasswordResetRepository {
    db: Database,
}

impl PasswordResetRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Count recent password reset requests for a user within a time window
    pub async fn count_recent_requests(&self, user_id: &str, seconds: i64) -> ApiResult<i64> {
        self.db.count_recent_reset_requests(user_id, seconds).await
    }

    /// Invalidate all existing reset tokens for a user (marks them as used)
    pub async fn invalidate_user_tokens(&self, user_id: &str) -> ApiResult<()> {
        self.db.invalidate_user_reset_tokens(user_id).await
    }

    /// Create a new password reset token
    pub async fn create_token(&self, token: &PasswordResetToken) -> ApiResult<()> {
        self.db.create_password_reset_token(token).await
    }

    /// Get a password reset token by its value
    pub async fn get_token(&self, token: &str) -> ApiResult<Option<PasswordResetToken>> {
        self.db.get_password_reset_token(token).await
    }

    /// Delete a password reset token by ID
    pub async fn delete_token(&self, token_id: &str) -> ApiResult<()> {
        self.db.delete_password_reset_token(token_id).await
    }

    /// Atomically reset password and destroy sessions
    /// Returns the number of sessions destroyed
    pub async fn reset_password_atomic(
        &self,
        user_id: &str,
        token_id: &str,
        password_hash: &str,
    ) -> ApiResult<i64> {
        let count = self.db
            .reset_password_atomic(user_id, token_id, password_hash)
            .await?;
        Ok(count as i64)
    }
}
