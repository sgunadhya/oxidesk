use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum UserType {
    Agent,
    Contact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub user_type: UserType,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub user_id: String,
    pub first_name: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
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
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub email: String,
    pub first_name: String,
    pub password: String,
    pub role_ids: Vec<String>,
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
    pub roles: Vec<crate::models::role::RoleResponse>,
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
        roles: Vec<crate::models::role::RoleResponse>,
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
        }
    }
}

impl Agent {
    pub fn new(user_id: String, first_name: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            first_name,
            password_hash,
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
