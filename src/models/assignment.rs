use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationParticipant {
    pub id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub added_at: String,
    pub added_by: Option<String>,
}

impl ConversationParticipant {
    pub fn new(conversation_id: String, user_id: String, added_by: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            user_id,
            added_at: chrono::Utc::now().to_rfc3339(),
            added_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentHistory {
    pub id: String,
    pub conversation_id: String,
    pub assigned_user_id: Option<String>,
    pub assigned_team_id: Option<String>,
    pub assigned_by: String,
    pub assigned_at: String,
    pub unassigned_at: Option<String>,
}

impl AssignmentHistory {
    pub fn new(
        conversation_id: String,
        assigned_user_id: Option<String>,
        assigned_team_id: Option<String>,
        assigned_by: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            assigned_user_id,
            assigned_team_id,
            assigned_by,
            assigned_at: chrono::Utc::now().to_rfc3339(),
            unassigned_at: None,
        }
    }
}

// API Request models
#[derive(Debug, Deserialize)]
pub struct AssignConversationRequest {
    pub assigned_user_id: Option<String>,
    pub assigned_team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAvailabilityRequest {
    pub availability_status: crate::models::AgentAvailability,
}
