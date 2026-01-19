use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::Agent;

/// Repository for API key operations
#[async_trait::async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Revoke API key for an agent
    async fn revoke_api_key(&self, agent_id: &str) -> ApiResult<bool>;

    /// Count all active API keys
    async fn count_api_keys(&self) -> ApiResult<i64>;

    /// Get agent by API key (for authentication)
    async fn get_agent_by_api_key(&self, api_key: &str) -> ApiResult<Option<Agent>>;

    /// Update API key last used timestamp
    async fn update_api_key_last_used(&self, api_key: &str) -> ApiResult<()>;

    /// Create a new API key
    async fn create_api_key(
        &self,
        agent_id: &str,
        api_key: &str,
        api_secret_hash: &str,
        description: Option<String>,
    ) -> ApiResult<()>;

    /// List API keys with pagination and sorting
    async fn list_api_keys(
        &self,
        limit: i64,
        offset: i64,
        sort_by: &str,
        sort_order: &str,
    ) -> ApiResult<Vec<(String, String, Option<String>, String, Option<String>)>>;
}
