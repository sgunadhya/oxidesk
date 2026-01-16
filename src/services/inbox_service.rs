use crate::api::middleware::ApiResult;
use crate::domain::ports::inbox_repository::InboxRepository;
use crate::models::Inbox;
use std::sync::Arc;
use time;

#[derive(Clone)]
pub struct InboxService {
    repo: Arc<dyn InboxRepository>,
}

impl InboxService {
    pub fn new(repo: Arc<dyn InboxRepository>) -> Self {
        Self { repo }
    }

    /// List all available inboxes
    pub async fn list_inboxes(&self) -> ApiResult<Vec<Inbox>> {
        self.repo.list_inboxes().await
    }

    /// Get a default inbox ID (usually the first one available). Creates a default one if none exist.
    pub async fn get_default_inbox_id(&self) -> ApiResult<String> {
        let inboxes = self.repo.list_inboxes().await?;

        if let Some(inbox) = inboxes.first() {
            Ok(inbox.id.clone())
        } else {
            // If no inboxes exist, create a default one
            tracing::warn!("No inboxes found in database. Creating default 'inbox-001'.");

            let now = time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap();

            let default_inbox = Inbox {
                id: "inbox-001".to_string(),
                name: "Default Inbox".to_string(),
                channel_type: "email".to_string(),
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
                deleted_by: None,
            };

            self.repo.create_inbox(&default_inbox).await?;
            Ok(default_inbox.id)
        }
    }
}
