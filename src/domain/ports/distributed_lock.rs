use crate::infrastructure::http::middleware::error::ApiResult;
use async_trait::async_trait;

#[async_trait]
pub trait DistributedLock: Send + Sync {
    /// Attempt to acquire a lock with the given key and expiration time (in seconds).
    /// Returns true if lock was acquired, false if it is already held by someone else.
    async fn acquire(&self, key: &str, owner: &str, ttl_seconds: u64) -> ApiResult<bool>;

    /// Release the lock if it is held by the given owner.
    async fn release(&self, key: &str, owner: &str) -> ApiResult<()>;
}
