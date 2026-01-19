use crate::infrastructure::http::middleware::error::ApiResult;
use async_trait::async_trait;

#[async_trait]
pub trait FileStorage: Send + Sync {
    /// Save a file to the storage
    async fn save(&self, path: &str, content: &[u8]) -> ApiResult<()>;

    /// Read a file from the storage
    async fn read(&self, path: &str) -> ApiResult<Vec<u8>>;

    /// Delete a file from the storage
    async fn delete(&self, path: &str) -> ApiResult<()>;

    /// Check if a file exists
    async fn exists(&self, path: &str) -> ApiResult<bool>;

    /// Create directory structure (mostly for local storage)
    async fn create_dir_all(&self, path: &str) -> ApiResult<()>;
}
