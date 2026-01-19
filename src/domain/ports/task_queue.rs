use crate::domain::entities::Job;
use crate::infrastructure::http::middleware::error::ApiResult;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait TaskQueue: Send + Sync {
    async fn enqueue(&self, job_type: &str, payload: Value, max_retries: i32) -> ApiResult<String>;
    async fn enqueue_at(
        &self,
        job_type: &str,
        payload: Value,
        run_at: chrono::DateTime<chrono::Utc>,
        max_retries: i32,
    ) -> ApiResult<String>;
    async fn fetch_next_job(&self) -> ApiResult<Option<Job>>;
    async fn complete_job(&self, job_id: &str) -> ApiResult<()>;
    async fn fail_job(&self, job_id: &str, error: &str) -> ApiResult<()>;
}
