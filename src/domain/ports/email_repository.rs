use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{EmailProcessingLog, InboxEmailConfig, UpdateInboxEmailConfigRequest};

#[async_trait::async_trait]
pub trait EmailRepository: Send + Sync {
    async fn get_inbox_email_config(&self, inbox_id: &str) -> ApiResult<Option<InboxEmailConfig>>;
    async fn get_enabled_email_configs(&self) -> ApiResult<Vec<InboxEmailConfig>>;
    async fn create_inbox_email_config(
        &self,
        config: &InboxEmailConfig,
    ) -> ApiResult<InboxEmailConfig>;
    async fn update_inbox_email_config(
        &self,
        id: &str,
        updates: &UpdateInboxEmailConfigRequest,
    ) -> ApiResult<InboxEmailConfig>;
    async fn delete_inbox_email_config(&self, id: &str) -> ApiResult<()>;
    async fn update_last_poll_time(&self, inbox_id: &str) -> ApiResult<()>;
    async fn log_email_processing(&self, log: &EmailProcessingLog)
        -> ApiResult<EmailProcessingLog>;
    async fn check_email_processed(
        &self,
        inbox_id: &str,
        email_message_id: &str,
    ) -> ApiResult<bool>;
}
