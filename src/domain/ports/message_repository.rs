use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::Message;

#[async_trait::async_trait]
pub trait MessageRepository: Send + Sync {
    async fn create_message(&self, message: &Message) -> ApiResult<()>;

    async fn get_message_by_id(&self, message_id: &str) -> ApiResult<Option<Message>>;

    async fn list_messages(
        &self,
        conversation_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Message>, i64)>;

    async fn update_message_status(
        &self,
        message_id: &str,
        status: crate::domain::entities::MessageStatus,
        sent_at: Option<&str>,
    ) -> ApiResult<()>;

    async fn update_conversation_message_timestamps(
        &self,
        conversation_id: &str,
        message_id: &str,
        last_message_at: &str,
        last_reply_at: Option<&str>,
    ) -> ApiResult<()>;

    async fn count_messages(&self, conversation_id: &str) -> ApiResult<i64>;

    // Notifications (related to messaging)
    async fn create_notification(
        &self,
        notification: &crate::domain::entities::UserNotification,
    ) -> ApiResult<()>;
    async fn get_users_by_usernames(
        &self,
        usernames: &[String],
    ) -> ApiResult<Vec<crate::domain::entities::User>>;
}
