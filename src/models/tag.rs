use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tag entity for conversation classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Tag {
    /// Create a new tag
    pub fn new(name: String, description: Option<String>, color: Option<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            color,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// ConversationTag join entity linking conversations to tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTag {
    pub conversation_id: String,
    pub tag_id: String,
    pub added_by: String,
    pub added_at: String,
}

impl ConversationTag {
    /// Create a new conversation-tag association
    pub fn new(conversation_id: String, tag_id: String, added_by: String) -> Self {
        Self {
            conversation_id,
            tag_id,
            added_by,
            added_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// ========== DTOs (Data Transfer Objects) ==========

/// Request to create a new tag
#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

/// Request to update tag properties (name is immutable)
#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub description: Option<String>,
    pub color: Option<String>,
}

/// Response containing full tag data
#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Tag> for TagResponse {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id,
            name: tag.name,
            description: tag.description,
            color: tag.color,
            created_at: tag.created_at,
            updated_at: tag.updated_at,
        }
    }
}

/// Response containing paginated list of tags
#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub tags: Vec<TagResponse>,
    pub pagination: crate::models::PaginationMetadata,
}

/// Request to add tags to a conversation
#[derive(Debug, Deserialize)]
pub struct AddTagsRequest {
    pub tag_ids: Vec<String>,
}

/// Request to replace all conversation tags
#[derive(Debug, Deserialize)]
pub struct ReplaceTagsRequest {
    pub tag_ids: Vec<String>,
}

/// Response containing conversation's tags
#[derive(Debug, Serialize)]
pub struct ConversationTagsResponse {
    pub conversation_id: String,
    pub tags: Vec<TagResponse>,
}
