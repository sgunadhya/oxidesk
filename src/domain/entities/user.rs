use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum UserType {
    Agent,
    Contact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentAvailability {
    Offline,
    Online,
    Away,
    AwayManual,
    AwayAndReassigning,
}

impl Default for AgentAvailability {
    fn default() -> Self {
        AgentAvailability::Offline
    }
}

impl std::fmt::Display for AgentAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentAvailability::Offline => write!(f, "offline"),
            AgentAvailability::Online => write!(f, "online"),
            AgentAvailability::Away => write!(f, "away"),
            AgentAvailability::AwayManual => write!(f, "away_manual"),
            AgentAvailability::AwayAndReassigning => write!(f, "away_and_reassigning"),
        }
    }
}

impl std::str::FromStr for AgentAvailability {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "offline" => Ok(AgentAvailability::Offline),
            "online" => Ok(AgentAvailability::Online),
            "away" => Ok(AgentAvailability::Away),
            "away_manual" => Ok(AgentAvailability::AwayManual),
            "away_and_reassigning" => Ok(AgentAvailability::AwayAndReassigning),
            _ => Err(format!("Invalid agent availability: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub user_type: UserType,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub user_id: String,
    pub first_name: String,
    pub last_name: Option<String>, // Feature 016: Added for agent creation
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub availability_status: AgentAvailability,
    pub last_login_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub away_since: Option<String>,
    // API Key Authentication fields (Feature 015)
    pub api_key: Option<String>,
    #[serde(skip_serializing)]
    pub api_secret_hash: Option<String>,
    pub api_key_description: Option<String>,
    pub api_key_created_at: Option<String>,
    pub api_key_last_used_at: Option<String>,
    pub api_key_revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub user_id: String,
    pub first_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactChannel {
    pub id: String,
    pub contact_id: String,
    pub inbox_id: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
}

// DTOs for API requests/responses

/// Create Agent Request (Feature 016: User Creation)
/// Administrator creates agent with automatically generated password
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentRequest {
    pub email: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub role_id: Option<String>, // Optional, defaults to "Agent" role if not provided
}

/// Create Agent Response (Feature 016: User Creation)
/// Returns agent details with plaintext password (shown only once)
#[derive(Debug, Serialize)]
pub struct CreateAgentResponse {
    pub agent_id: String,
    pub user_id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub password: String, // Plaintext password - shown only once, never stored
    pub password_note: String, // Warning message about one-time display
    pub availability_status: String, // Always "offline" initially
    pub enabled: bool,    // Always true initially
    pub role_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateContactRequest {
    pub email: String,
    pub first_name: Option<String>,
    pub inbox_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub first_name: String,
    pub role_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub first_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub email: String,
    pub user_type: UserType,
    pub first_name: String,
    pub roles: Vec<crate::domain::entities::role::RoleResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentResponse>,
    pub pagination: PaginationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    pub page: i64,
    pub per_page: i64,
    pub total_count: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct ContactResponse {
    pub id: String,
    pub email: String,
    pub user_type: UserType,
    pub first_name: Option<String>,
    pub channels: Vec<ContactChannel>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ContactListResponse {
    pub contacts: Vec<ContactResponse>,
    pub pagination: PaginationMetadata,
}

// User list response (polymorphic - can be agent or contact)
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum UserSummary {
    #[serde(rename = "agent")]
    Agent {
        id: String,
        email: String,
        first_name: String,
        roles: Vec<crate::domain::entities::role::RoleResponse>,
        created_at: String,
    },
    #[serde(rename = "contact")]
    Contact {
        id: String,
        email: String,
        first_name: Option<String>,
        channel_count: usize,
        created_at: String,
    },
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserSummary>,
    pub pagination: PaginationMetadata,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UserDetail {
    Agent(AgentResponse),
    Contact(ContactResponse),
}

impl User {
    pub fn new(email: String, user_type: UserType) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            email: email.to_lowercase(),
            user_type,
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        }
    }

    /// Validate user type immutability (Feature 025: Mutual Exclusion Invariants)
    /// FR-004: User type is immutable after creation
    pub fn validate_type_immutable(&self, new_type: &UserType) -> Result<(), String> {
        if std::mem::discriminant(&self.user_type) != std::mem::discriminant(new_type) {
            return Err("User type is immutable after creation".to_string());
        }
        Ok(())
    }
}

impl Agent {
    pub fn new(
        user_id: String,
        first_name: String,
        last_name: Option<String>,
        password_hash: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            first_name,
            last_name,
            password_hash,
            availability_status: AgentAvailability::default(),
            last_login_at: None,
            last_activity_at: None,
            away_since: None,
            api_key: None,
            api_secret_hash: None,
            api_key_description: None,
            api_key_created_at: None,
            api_key_last_used_at: None,
            api_key_revoked_at: None,
        }
    }
}

impl Contact {
    pub fn new(user_id: String, first_name: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            first_name,
        }
    }
}

impl ContactChannel {
    pub fn new(contact_id: String, inbox_id: String, email: String) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            contact_id,
            inbox_id,
            email,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
