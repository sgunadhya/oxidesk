use crate::api::middleware::ApiResult;
use crate::database::Database;
use crate::models::Inbox;
use time;

/// List all available inboxes
pub async fn list_inboxes(db: &Database) -> ApiResult<Vec<Inbox>> {
    db.list_inboxes().await
}

/// Get a default inbox ID (usually the first one available). Creates a default one if none exist.
pub async fn get_default_inbox_id(db: &Database) -> ApiResult<String> {
    let inboxes = db.list_inboxes().await?;

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

        db.create_inbox(&default_inbox).await?;
        Ok(default_inbox.id)
    }
}
