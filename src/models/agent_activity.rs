use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventType {
    AgentLogin,
    AgentLogout,
    AvailabilityChanged,
}

impl std::fmt::Display for ActivityEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityEventType::AgentLogin => write!(f, "agent_login"),
            ActivityEventType::AgentLogout => write!(f, "agent_logout"),
            ActivityEventType::AvailabilityChanged => write!(f, "availability_changed"),
        }
    }
}

impl std::str::FromStr for ActivityEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "agent_login" => Ok(ActivityEventType::AgentLogin),
            "agent_logout" => Ok(ActivityEventType::AgentLogout),
            "availability_changed" => Ok(ActivityEventType::AvailabilityChanged),
            _ => Err(format!("Invalid activity event type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivityLog {
    pub id: String,
    pub agent_id: String,
    pub event_type: ActivityEventType,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub metadata: Option<String>, // JSON for extensibility
    pub created_at: String,
}

impl AgentActivityLog {
    pub fn new(
        agent_id: String,
        event_type: ActivityEventType,
        old_status: Option<String>,
        new_status: Option<String>,
        metadata: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id,
            event_type,
            old_status,
            new_status,
            metadata,
            created_at: now,
        }
    }
}

// DTOs for API requests/responses
#[derive(Debug, Deserialize)]
pub struct SetAvailabilityRequest {
    pub status: crate::models::user::AgentAvailability,
}

#[derive(Debug, Serialize)]
pub struct AvailabilityResponse {
    pub agent_id: String,
    pub availability_status: crate::models::user::AgentAvailability,
    pub last_activity_at: Option<String>,
    pub away_since: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActivityLogResponse {
    pub logs: Vec<AgentActivityLog>,
    pub total: i64,
    pub pagination: crate::models::user::PaginationMetadata,
}
