use crate::infrastructure::http::middleware::error::ApiResult;
// Database import removed
use crate::domain::ports::session_repository::SessionRepository;
use crate::domain::entities::Session;
use std::sync::Arc;

#[derive(Clone)]
pub struct SessionService {
    session_repo: Arc<dyn SessionRepository>,
}

impl SessionService {
    pub fn new(session_repo: Arc<dyn SessionRepository>) -> Self {
        Self { session_repo }
    }

    pub async fn create_session(&self, session: &Session) -> ApiResult<()> {
        self.session_repo.create_session(session).await
    }

    pub async fn get_session_by_token(&self, token: &str) -> ApiResult<Option<Session>> {
        self.session_repo.get_session_by_token(token).await
    }

    pub async fn delete_session(&self, token: &str) -> ApiResult<()> {
        self.session_repo.delete_session(token).await
    }

    pub async fn cleanup_expired_sessions(&self) -> ApiResult<u64> {
        self.session_repo.cleanup_expired_sessions().await
    }

    pub async fn get_user_sessions(&self, user_id: &str) -> ApiResult<Vec<Session>> {
        self.session_repo.get_user_sessions(user_id).await
    }

    pub async fn delete_user_sessions(&self, user_id: &str) -> ApiResult<u64> {
        self.session_repo.delete_user_sessions(user_id).await
    }

    pub async fn update_session_last_accessed(&self, token: &str) -> ApiResult<()> {
        self.session_repo.update_session_last_accessed(token).await
    }
}
