use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    events::{EventBus, SystemEvent},
    models::*,
};

/// Service for conversation tagging operations (agents)
#[derive(Clone)]
pub struct ConversationTagService {
    db: Database,
    event_bus: EventBus,
}

impl ConversationTagService {
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    /// Helper: Check if user has permission
    fn has_permission(&self, permissions: &[Permission], required: &str) -> bool {
        permissions.iter().any(|p| p.name == required)
    }

    /// Add tags to a conversation (requires conversations:update_tags permission)
    pub async fn add_tags(
        &self,
        conversation_id: &str,
        request: AddTagsRequest,
        user_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Vec<Tag>> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_tags") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_tags".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let _ = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Validate request
        if request.tag_ids.is_empty() {
            return Err(ApiError::BadRequest(
                "Tag IDs list cannot be empty".to_string(),
            ));
        }

        // 4. Get previous tags for event
        let previous_tags = self.db.get_conversation_tags(conversation_id).await?;
        let previous_tag_ids: Vec<String> = previous_tags.iter().map(|t| t.id.clone()).collect();

        // 5. Verify all tags exist and add them
        for tag_id in &request.tag_ids {
            // Verify tag exists
            let _ = self
                .db
                .get_tag_by_id(tag_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))?;

            // Add tag (idempotent)
            self.db
                .add_conversation_tag(conversation_id, tag_id, user_id)
                .await?;
        }

        // 6. Get new tags
        let new_tags = self.db.get_conversation_tags(conversation_id).await?;
        let new_tag_ids: Vec<String> = new_tags.iter().map(|t| t.id.clone()).collect();

        // 7. Emit ConversationTagsChanged event
        self.event_bus
            .publish(SystemEvent::ConversationTagsChanged {
                conversation_id: conversation_id.to_string(),
                previous_tags: previous_tag_ids,
                new_tags: new_tag_ids,
                changed_by: user_id.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });

        // 8. Return updated tag list
        Ok(new_tags)
    }

    /// Remove a tag from a conversation (requires conversations:update_tags permission)
    pub async fn remove_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
        user_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Vec<Tag>> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_tags") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_tags".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let _ = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Get previous tags for event
        let previous_tags = self.db.get_conversation_tags(conversation_id).await?;
        let previous_tag_ids: Vec<String> = previous_tags.iter().map(|t| t.id.clone()).collect();

        // 4. Remove tag (idempotent)
        self.db
            .remove_conversation_tag(conversation_id, tag_id)
            .await?;

        // 5. Get new tags
        let new_tags = self.db.get_conversation_tags(conversation_id).await?;
        let new_tag_ids: Vec<String> = new_tags.iter().map(|t| t.id.clone()).collect();

        // 6. Emit ConversationTagsChanged event
        self.event_bus
            .publish(SystemEvent::ConversationTagsChanged {
                conversation_id: conversation_id.to_string(),
                previous_tags: previous_tag_ids,
                new_tags: new_tag_ids,
                changed_by: user_id.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });

        // 7. Return updated tag list
        Ok(new_tags)
    }

    /// Replace all conversation tags atomically (requires conversations:update_tags permission)
    pub async fn replace_tags(
        &self,
        conversation_id: &str,
        request: ReplaceTagsRequest,
        user_id: &str,
        permissions: &[Permission],
    ) -> ApiResult<Vec<Tag>> {
        // 1. Check permission
        if !self.has_permission(permissions, "conversations:update_tags") {
            return Err(ApiError::Forbidden(
                "Missing permission: conversations:update_tags".to_string(),
            ));
        }

        // 2. Verify conversation exists
        let _ = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 3. Get previous tags for event
        let previous_tags = self.db.get_conversation_tags(conversation_id).await?;
        let previous_tag_ids: Vec<String> = previous_tags.iter().map(|t| t.id.clone()).collect();

        // 4. Verify all new tags exist
        for tag_id in &request.tag_ids {
            let _ = self
                .db
                .get_tag_by_id(tag_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))?;
        }

        // 5. Replace tags atomically
        self.db
            .replace_conversation_tags(conversation_id, &request.tag_ids, user_id)
            .await?;

        // 6. Get new tags
        let new_tags = self.db.get_conversation_tags(conversation_id).await?;
        let new_tag_ids: Vec<String> = new_tags.iter().map(|t| t.id.clone()).collect();

        // 7. Emit ConversationTagsChanged event
        self.event_bus
            .publish(SystemEvent::ConversationTagsChanged {
                conversation_id: conversation_id.to_string(),
                previous_tags: previous_tag_ids,
                new_tags: new_tag_ids,
                changed_by: user_id.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });

        // 8. Return updated tag list
        Ok(new_tags)
    }

    /// Get all tags for a conversation
    pub async fn get_conversation_tags(&self, conversation_id: &str) -> ApiResult<Vec<Tag>> {
        // 1. Verify conversation exists
        let _ = self
            .db
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Conversation {} not found", conversation_id))
            })?;

        // 2. Get tags
        self.db.get_conversation_tags(conversation_id).await
    }

    /// Get conversations with a specific tag
    pub async fn get_conversations_by_tag(
        &self,
        tag_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // 1. Verify tag exists
        let _ = self
            .db
            .get_tag_by_id(tag_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))?;

        // 2. Get conversations
        self.db
            .get_conversations_by_tag(tag_id, limit, offset)
            .await
    }

    /// Get user permissions (helper for service layer)
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        self.db.get_user_permissions(user_id).await
    }
}
