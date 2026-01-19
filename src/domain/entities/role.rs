use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>, // Permission strings like "conversations:read_assigned"
    pub is_protected: bool,       // Prevents modification of Admin role
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct UserRole {
    pub user_id: String,
    pub role_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RolePermission {
    pub role_id: String,
    pub permission_id: String,
    pub created_at: String,
}

// DTOs for API
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>, // Array of permission strings
    pub is_protected: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct PermissionResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>, // Permission strings like "conversations:read_assigned"
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>, // Permission strings
}

impl Role {
    pub fn new(name: String, description: Option<String>, permissions: Vec<String>) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            permissions,
            is_protected: false, // Only Admin role should have this set to true
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl Permission {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl UserRole {
    pub fn new(user_id: String, role_id: String) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            user_id,
            role_id,
            created_at: now,
        }
    }
}

impl RolePermission {
    pub fn new(role_id: String, permission_id: String) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            role_id,
            permission_id,
            created_at: now,
        }
    }
}

// Conversions for API responses
impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        Self {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: role.permissions,
            is_protected: role.is_protected,
            created_at: role.created_at,
            updated_at: role.updated_at,
        }
    }
}

impl From<Permission> for PermissionResponse {
    fn from(permission: Permission) -> Self {
        Self {
            id: permission.id,
            name: permission.name,
            description: permission.description,
            created_at: permission.created_at,
            updated_at: permission.updated_at,
        }
    }
}
