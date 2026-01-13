use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub sla_policy_id: Option<String>,
    pub business_hours: Option<String>, // JSON format: {"timezone": "America/New_York", "schedule": [...]}
    pub created_at: String,
    pub updated_at: String,
}

impl Team {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            sla_policy_id: None,
            business_hours: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Business hours configuration for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessHours {
    pub timezone: String, // IANA timezone (e.g., "America/New_York")
    pub schedule: Vec<DaySchedule>,
}

/// Schedule for a specific day of the week
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySchedule {
    pub day: String, // "Monday", "Tuesday", etc.
    pub start: String, // "09:00"
    pub end: String,   // "17:00"
}

impl BusinessHours {
    /// Validate business hours JSON format
    pub fn validate(json_str: &str) -> Result<BusinessHours, String> {
        serde_json::from_str::<BusinessHours>(json_str)
            .map_err(|e| format!("Invalid business hours format: {}", e))
    }

    /// Parse business hours from JSON string
    pub fn parse(json_str: &str) -> Result<BusinessHours, String> {
        Self::validate(json_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMembership {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: TeamMemberRole,
    pub joined_at: String,
}

impl TeamMembership {
    pub fn new(team_id: String, user_id: String, role: TeamMemberRole) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id,
            user_id,
            role,
            joined_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamMemberRole {
    Member,
    Lead,
}

impl std::fmt::Display for TeamMemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamMemberRole::Member => write!(f, "member"),
            TeamMemberRole::Lead => write!(f, "lead"),
        }
    }
}

impl std::str::FromStr for TeamMemberRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "member" => Ok(TeamMemberRole::Member),
            "lead" => Ok(TeamMemberRole::Lead),
            _ => Err(format!("Invalid team member role: {}", s)),
        }
    }
}
