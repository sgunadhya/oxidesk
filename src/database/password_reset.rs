use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use crate::models::PasswordResetToken;
use sqlx::Row;
use time;

impl Database {
    // ==================== Password Reset Operations (Feature 017) ====================

    /// Create a password reset token
    pub async fn create_password_reset_token(&self, token: &PasswordResetToken) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token, expires_at, used, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&token.id)
        .bind(&token.user_id)
        .bind(&token.token)
        .bind(&token.expires_at)
        .bind(if token.used { 1 } else { 0 })
        .bind(&token.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get password reset token by token value
    pub async fn get_password_reset_token(
        &self,
        token: &str,
    ) -> ApiResult<Option<PasswordResetToken>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, expires_at, used, created_at
             FROM password_reset_tokens
             WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(PasswordResetToken {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get::<i32, _>("used")? == 1,
                created_at: row.try_get("created_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Count recent password reset requests for a user (for rate limiting)
    /// Returns count of ALL tokens created in the last hour (used or unused)
    /// Rate limiting counts all requests to prevent abuse, regardless of token status
    pub async fn count_recent_reset_requests(
        &self,
        user_id: &str,
        window_seconds: i64,
    ) -> ApiResult<i64> {
        let now = time::OffsetDateTime::now_utc();
        let window_start = now - time::Duration::seconds(window_seconds);
        let window_start_str = window_start
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM password_reset_tokens
             WHERE user_id = ? AND created_at > ?",
        )
        .bind(user_id)
        .bind(&window_start_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }

    /// Mark password reset token as used
    pub async fn mark_token_as_used(&self, token_id: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE password_reset_tokens
             SET used = 1
             WHERE id = ?",
        )
        .bind(token_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete password reset token (for lazy cleanup)
    pub async fn delete_password_reset_token(&self, token_id: &str) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM password_reset_tokens
             WHERE id = ?",
        )
        .bind(token_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Invalidate all unused password reset tokens for a user
    /// Used when generating a new token to invalidate previous tokens
    pub async fn invalidate_user_reset_tokens(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE password_reset_tokens
             SET used = 1
             WHERE user_id = ? AND used = 0",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reset password with transaction (Feature 017: Password Reset)
    /// Performs all password reset operations atomically:
    /// 1. Update agent password
    /// 2. Mark token as used
    /// 3. Delete all user sessions
    ///
    /// If any step fails, the entire transaction is rolled back
    pub async fn reset_password_atomic(
        &self,
        user_id: &str,
        token_id: &str,
        password_hash: &str,
    ) -> ApiResult<u64> {
        let mut tx = self.pool.begin().await?;

        // 1. Update agent password
        sqlx::query(
            "UPDATE agents
             SET password_hash = ?
             WHERE user_id = ?",
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // 2. Mark token as used
        sqlx::query(
            "UPDATE password_reset_tokens
             SET used = 1
             WHERE id = ?",
        )
        .bind(token_id)
        .execute(&mut *tx)
        .await?;

        // 3. Delete all user sessions
        let result = sqlx::query(
            "DELETE FROM sessions
             WHERE user_id = ?",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Commit transaction - if this fails, all changes are rolled back
        tx.commit().await?;

        Ok(result.rows_affected())
    }

    /// Get all password reset tokens for a user (for testing)
    pub async fn get_all_password_reset_tokens_for_user(
        &self,
        user_id: &str,
    ) -> ApiResult<Vec<PasswordResetToken>> {
        let rows = sqlx::query(
            "SELECT id, user_id, token, expires_at, used, created_at
             FROM password_reset_tokens WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tokens = Vec::new();
        for row in rows {
            let used_int: i64 = row.try_get("used")?;
            tokens.push(PasswordResetToken {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                expires_at: row.try_get("expires_at")?,
                used: used_int != 0,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(tokens)
    }
}
