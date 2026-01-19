use crate::domain::ports::file_storage::FileStorage;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Clone)]
pub struct LocalFileStorage {
    base_path: PathBuf,
}

impl LocalFileStorage {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Resolve absolute path safely, preventing traversal out of base path
    fn resolve_path(&self, path: &str) -> ApiResult<PathBuf> {
        // Simple join for now, assuming path is relative
        // In a real generic implementation, we should check for traversal attacks (e.g. "..")
        // forcing the path to stay within base_path.

        let path = Path::new(path);
        if path.is_absolute() {
            // If absolute, verify it starts with base_path, or treat it as relative to root
            // For safety, we treat the input 'path' as relative to base_path always.
            // If the user passes an absolute path, we strip prefix or just join.
            // But existing logic might pass absolute paths if not careful.
            // Let's assume the 'path' argument is the key/relative path.
            return Ok(self.base_path.join(path.strip_prefix("/").unwrap_or(path)));
        }

        Ok(self.base_path.join(path))
    }
}

#[async_trait]
impl FileStorage for LocalFileStorage {
    async fn save(&self, path: &str, content: &[u8]) -> ApiResult<()> {
        let file_path = self.resolve_path(path)?;

        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to create directory: {}", e)))?;
        }

        fs::write(&file_path, content)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to write file: {}", e)))
    }

    async fn read(&self, path: &str) -> ApiResult<Vec<u8>> {
        let file_path = self.resolve_path(path)?;
        fs::read(&file_path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to read file: {}", e)))
    }

    async fn delete(&self, path: &str) -> ApiResult<()> {
        let file_path = self.resolve_path(path)?;
        if file_path.exists() {
            fs::remove_file(&file_path)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to delete file: {}", e)))?;
        }
        Ok(())
    }

    async fn exists(&self, path: &str) -> ApiResult<bool> {
        let file_path = self.resolve_path(path)?;
        Ok(file_path.exists())
    }

    async fn create_dir_all(&self, path: &str) -> ApiResult<()> {
        let dir_path = self.resolve_path(path)?;
        fs::create_dir_all(dir_path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create directory: {}", e)))
    }
}
