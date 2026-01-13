use crate::api::middleware::error::ApiResult;

/// Notification service stub - will be implemented in Feature 011
#[derive(Clone)]
pub struct NotificationService {}

impl NotificationService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn notify_assignment(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> ApiResult<()> {
        tracing::info!(
            "STUB: Would notify user {} about conversation {} assignment",
            user_id,
            conversation_id
        );
        // TODO: Feature 011 will implement actual notification delivery
        // This would send in-app notifications and/or emails
        Ok(())
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}
