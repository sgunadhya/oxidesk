use crate::infrastructure::http::middleware::ApiResult;
use crate::domain::entities::Inbox;
use async_trait::async_trait;

#[async_trait]
pub trait InboxRepository: Send + Sync {
    async fn list_inboxes(&self) -> ApiResult<Vec<Inbox>>;
    async fn create_inbox(&self, inbox: &Inbox) -> ApiResult<()>;
    async fn soft_delete_inbox(&self, inbox_id: &str, deleted_by: &str) -> ApiResult<()>;
    async fn restore_inbox(&self, inbox_id: &str) -> ApiResult<()>;
}
