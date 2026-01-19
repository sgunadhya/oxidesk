use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{AssignmentHistory, UserNotification};

/// Repository for assignment operations
#[async_trait::async_trait]
pub trait AssignmentRepository: Send + Sync {
    /// Record assignment history
    async fn record_assignment(&self, history: &AssignmentHistory) -> ApiResult<()>;

    /// Create a user notification
    async fn create_notification(&self, notification: &UserNotification) -> ApiResult<()>;
}
