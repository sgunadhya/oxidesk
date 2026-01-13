use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub sla_policy_id: Option<String>,
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
            created_at: now.clone(),
            updated_at: now,
        }
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
