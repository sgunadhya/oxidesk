use crate::domain::ports::distributed_lock::DistributedLock;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::persistence::Database;
use async_trait::async_trait;
use chrono::Utc;

#[derive(Clone)]
pub struct DatabaseDistributedLock {
    db: Database,
}

impl DatabaseDistributedLock {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DistributedLock for DatabaseDistributedLock {
    async fn acquire(&self, key: &str, owner: &str, ttl_seconds: u64) -> ApiResult<bool> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(ttl_seconds as i64);

        // Try to insert a new lock
        // If it fails (key exists), check if it is expired (expires_at < now).
        // If expired, take it over.
        // We use string comparison for timestamps (ISO 8601 is lexicographically sortable)

        let query = r#"
            INSERT INTO distributed_locks (key, owner, expires_at, created_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                owner = excluded.owner,
                expires_at = excluded.expires_at,
                created_at = excluded.created_at
            WHERE distributed_locks.expires_at < ?
        "#;

        // Note: For 'now' in WHERE clause, we use the current time.
        // If the existing lock expires_at is less than 'now', we overwrite it.

        let result = sqlx::query(query)
            .bind(key)
            .bind(owner)
            .bind(expires_at.to_rfc3339())
            .bind(now.to_rfc3339()) // created_at
            .bind(now.to_rfc3339()) // WHERE expires_at < now
            .execute(&self.db.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to acquire lock: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn release(&self, key: &str, owner: &str) -> ApiResult<()> {
        let query = "DELETE FROM distributed_locks WHERE key = ? AND owner = ?";
        sqlx::query(query)
            .bind(key)
            .bind(owner)
            .execute(&self.db.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to release lock: {}", e)))?;
        Ok(())
    }
}
