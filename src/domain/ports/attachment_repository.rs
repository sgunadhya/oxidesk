use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::MessageAttachment;

#[async_trait::async_trait]
pub trait AttachmentRepository: Send + Sync {
    async fn create_message_attachment(
        &self,
        attachment: &MessageAttachment,
    ) -> ApiResult<MessageAttachment>;
    async fn get_message_attachments(&self, message_id: &str) -> ApiResult<Vec<MessageAttachment>>;
    // Defined in implementation but maybe should be part of trait if we want full abstraction for AttachmentService?
    // AttachmentService uses: create_message_attachment, get_message_attachments
}
