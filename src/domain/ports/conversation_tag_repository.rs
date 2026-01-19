use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{Conversation, Tag};

/// Repository for conversation tag operations
#[async_trait::async_trait]
pub trait ConversationTagRepository: Send + Sync {
    /// Get all tags for a conversation
    async fn get_conversation_tags(&self, conversation_id: &str) -> ApiResult<Vec<Tag>>;

    /// Add a tag to a conversation (idempotent)
    async fn add_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
        user_id: &str,
    ) -> ApiResult<()>;

    /// Remove a tag from a conversation (idempotent)
    async fn remove_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
    ) -> ApiResult<()>;

    /// Replace all tags for a conversation atomically
    async fn replace_conversation_tags(
        &self,
        conversation_id: &str,
        tag_ids: &[String],
        user_id: &str,
    ) -> ApiResult<()>;

    /// Get conversations with a specific tag
    async fn get_conversations_by_tag(
        &self,
        tag_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)>;
}
