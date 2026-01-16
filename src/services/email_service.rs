use crate::api::middleware::error::ApiResult;
use crate::domain::ports::email_repository::EmailRepository;
use crate::models::{EmailProcessingLog, InboxEmailConfig, UpdateInboxEmailConfigRequest};
use std::sync::Arc;

/// Service for managing email configurations and logs
#[derive(Clone)]
pub struct EmailService {
    repo: Arc<dyn EmailRepository>,
}

impl EmailService {
    pub fn new(repo: Arc<dyn EmailRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_inbox_email_config(
        &self,
        inbox_id: &str,
    ) -> ApiResult<Option<InboxEmailConfig>> {
        self.repo.get_inbox_email_config(inbox_id).await
    }

    pub async fn get_enabled_email_configs(&self) -> ApiResult<Vec<InboxEmailConfig>> {
        self.repo.get_enabled_email_configs().await
    }

    pub async fn create_inbox_email_config(
        &self,
        config: &InboxEmailConfig,
    ) -> ApiResult<InboxEmailConfig> {
        self.repo.create_inbox_email_config(config).await
    }

    pub async fn update_inbox_email_config(
        &self,
        id: &str,
        updates: &UpdateInboxEmailConfigRequest,
    ) -> ApiResult<InboxEmailConfig> {
        self.repo.update_inbox_email_config(id, updates).await
    }

    pub async fn delete_inbox_email_config(&self, id: &str) -> ApiResult<()> {
        self.repo.delete_inbox_email_config(id).await
    }

    pub async fn update_last_poll_time(&self, inbox_id: &str) -> ApiResult<()> {
        self.repo.update_last_poll_time(inbox_id).await
    }

    pub async fn log_email_processing(
        &self,
        log: &EmailProcessingLog,
    ) -> ApiResult<EmailProcessingLog> {
        self.repo.log_email_processing(log).await
    }

    pub async fn check_email_processed(
        &self,
        inbox_id: &str,
        email_message_id: &str,
    ) -> ApiResult<bool> {
        self.repo
            .check_email_processed(inbox_id, email_message_id)
            .await
    }
}
