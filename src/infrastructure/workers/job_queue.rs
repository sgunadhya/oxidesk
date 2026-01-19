use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::{infrastructure::persistence::Database, infrastructure::http::middleware::error::ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ToString for JobStatus {
    fn to_string(&self) -> String {
        match self {
            JobStatus::Pending => "pending".to_string(),
            JobStatus::Processing => "processing".to_string(),
            JobStatus::Completed => "completed".to_string(),
            JobStatus::Failed => "failed".to_string(),
        }
    }
}

impl From<String> for JobStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => JobStatus::Pending,
            "processing" => JobStatus::Processing,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            _ => JobStatus::Pending, // Default fallback
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub payload: Value,
    pub status: JobStatus,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_error: Option<String>,
}

#[async_trait]
pub trait TaskQueue: Send + Sync {
    /// Enqueue a job for immediate execution
    async fn enqueue(&self, job_type: &str, payload: Value, max_attempts: i32)
        -> ApiResult<String>;

    /// Enqueue a job to run at a specific future time
    async fn enqueue_at(
        &self,
        job_type: &str,
        payload: Value,
        run_at: DateTime<Utc>,
        max_attempts: i32,
    ) -> ApiResult<String>;

    /// Poll for the next available job and lock it
    async fn fetch_next_job(&self) -> ApiResult<Option<Job>>;

    /// Mark a job as completed
    async fn complete_job(&self, job_id: &str) -> ApiResult<()>;

    /// Mark a job as failed (scheduling a retry if appropriate)
    async fn fail_job(&self, job_id: &str, error: &str) -> ApiResult<()>;
}

/// SQLite implementation of the TaskQueue
#[derive(Clone)]
pub struct SqliteTaskQueue {
    db: Database,
}

impl SqliteTaskQueue {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TaskQueue for SqliteTaskQueue {
    async fn enqueue(
        &self,
        job_type: &str,
        payload: Value,
        max_attempts: i32,
    ) -> ApiResult<String> {
        self.enqueue_at(job_type, payload, Utc::now(), max_attempts)
            .await
    }

    async fn enqueue_at(
        &self,
        job_type: &str,
        payload: Value,
        run_at: DateTime<Utc>,
        max_attempts: i32,
    ) -> ApiResult<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let payload_str = serde_json::to_string(&payload).unwrap_or_default();

        sqlx::query(
            "INSERT INTO jobs (id, job_type, payload, status, run_at, created_at, updated_at, max_attempts)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(job_type)
        .bind(&payload_str)
        .bind(JobStatus::Pending.to_string())
        .bind(run_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(max_attempts)
        .execute(self.db.pool())
        .await?;

        Ok(id)
    }

    async fn fetch_next_job(&self) -> ApiResult<Option<Job>> {
        let now = Utc::now();
        // 5 minutes lock timeout
        let lock_timeout = now + chrono::Duration::minutes(5);

        // Transaction to ensure atomic fetch-and-lock
        let mut tx = self.db.pool().begin().await?;

        // 1. Find a candidate job (pending and ready to run)
        let candidate_row = sqlx::query(
            "SELECT id FROM jobs
             WHERE status = 'pending' AND run_at <= ?
             ORDER BY run_at ASC
             LIMIT 1",
        )
        .bind(now.to_rfc3339())
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = candidate_row {
            let id: String = row.try_get("id")?;

            // 2. Lock the job (set to processing)
            sqlx::query(
                "UPDATE jobs
                 SET status = 'processing', updated_at = ?, locked_until = ?
                 WHERE id = ?",
            )
            .bind(now.to_rfc3339())
            .bind(lock_timeout.to_rfc3339())
            .bind(&id)
            .execute(&mut *tx)
            .await?;

            // 3. Fetch full details
            let job_row = sqlx::query(
                "SELECT id, job_type, payload, status, run_at, created_at, updated_at, attempts, max_attempts, last_error
                 FROM jobs WHERE id = ?",
            )
            .bind(&id)
            .fetch_one(&mut *tx)
            .await?;

            tx.commit().await?;

            let status_str: String = job_row.try_get("status")?;
            let payload_str: String = job_row.try_get("payload")?;
            let payload: Value = serde_json::from_str(&payload_str).unwrap_or(Value::Null);

            // Helper to parse string timestamp back to DateTime<Utc>
            fn parse_date_col(row: &sqlx::any::AnyRow, col: &str) -> ApiResult<DateTime<Utc>> {
                let s: String = row.try_get(col)?;
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)).into())
            }

            // Handle optional date
            let last_error: Option<String> = job_row.try_get("last_error").ok();

            Ok(Some(Job {
                id: job_row.try_get("id")?,
                job_type: job_row.try_get("job_type")?,
                payload,
                status: JobStatus::from(status_str),
                run_at: parse_date_col(&job_row, "run_at")?,
                created_at: parse_date_col(&job_row, "created_at")?,
                updated_at: parse_date_col(&job_row, "updated_at")?,
                attempts: job_row.try_get("attempts")?,
                max_attempts: job_row.try_get("max_attempts")?,
                last_error,
            }))
        } else {
            Ok(None)
        }
    }

    async fn complete_job(&self, job_id: &str) -> ApiResult<()> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE jobs
             SET status = 'completed', updated_at = ?
             WHERE id = ?",
        )
        .bind(now.to_rfc3339())
        .bind(job_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    async fn fail_job(&self, job_id: &str, error: &str) -> ApiResult<()> {
        let now = Utc::now();

        // Fetch current attempts to decide on retry
        let row = sqlx::query("SELECT attempts, max_attempts FROM jobs WHERE id = ?")
            .bind(job_id)
            .fetch_one(self.db.pool())
            .await?;

        let attempts: i32 = row.try_get("attempts")?;
        let max_attempts: i32 = row.try_get("max_attempts")?;
        let new_attempts = attempts + 1;

        if new_attempts < max_attempts {
            // Exponential backoff: 2^attempts * 30 seconds
            let backoff_seconds = 30 * (1 << attempts);
            let next_run = now + chrono::Duration::seconds(backoff_seconds as i64);

            sqlx::query(
                "UPDATE jobs
                 SET status = 'pending', attempts = ?, last_error = ?, run_at = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(new_attempts)
            .bind(error)
            .bind(next_run.to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(job_id)
            .execute(self.db.pool())
            .await?;
        } else {
            // Permanent failure
            sqlx::query(
                "UPDATE jobs
                 SET status = 'failed', attempts = ?, last_error = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(new_attempts)
            .bind(error)
            .bind(now.to_rfc3339())
            .bind(job_id)
            .execute(self.db.pool())
            .await?;
        }

        Ok(())
    }
}
