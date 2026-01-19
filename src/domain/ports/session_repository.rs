use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::Session;

#[async_trait::async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create_session(&self, session: &Session) -> ApiResult<()>;
    async fn get_session_by_token(&self, token: &str) -> ApiResult<Option<Session>>;
    async fn delete_session(&self, token: &str) -> ApiResult<()>;
    async fn cleanup_expired_sessions(&self) -> ApiResult<u64>;
    async fn get_user_sessions(&self, user_id: &str) -> ApiResult<Vec<Session>>;
    async fn delete_user_sessions(&self, user_id: &str) -> ApiResult<u64>;
    async fn update_session_last_accessed(&self, token: &str) -> ApiResult<()>;
}
