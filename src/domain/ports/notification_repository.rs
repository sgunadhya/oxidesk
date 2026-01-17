use crate::api::middleware::error::ApiResult;
use crate::models::UserNotification;

/// Repository for notification operations
#[async_trait::async_trait]
pub trait NotificationRepository: Send + Sync {
    /// Get unread notification count for a user
    async fn get_unread_count(&self, user_id: &str) -> ApiResult<i32>;

    /// Mark a notification as read
    async fn mark_notification_as_read(&self, notification_id: &str) -> ApiResult<()>;

    /// List notifications for a user with pagination
    async fn list_notifications(&self, user_id: &str, limit: i32, offset: i32) -> ApiResult<Vec<UserNotification>>;

    /// Get a notification by ID
    async fn get_notification_by_id(&self, id: &str) -> ApiResult<Option<UserNotification>>;

    /// Mark all notifications as read for a user
    async fn mark_all_notifications_as_read(&self, user_id: &str) -> ApiResult<i32>;
}
