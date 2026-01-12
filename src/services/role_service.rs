use crate::{
    api::middleware::{ApiError, AuthenticatedUser},
    database::Database,
};
use std::fmt;

#[derive(Debug)]
pub enum RoleError {
    NotFound,
    Forbidden,
    CannotDeleteSystemRole,
    RoleInUse(i64),
    Database(ApiError),
}

impl fmt::Display for RoleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Role not found"),
            Self::Forbidden => write!(f, "Permission denied"),
            Self::CannotDeleteSystemRole => write!(f, "Cannot delete system roles"),
            Self::RoleInUse(count) => write!(f, "Cannot delete role assigned to {} user(s)", count),
            Self::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<ApiError> for RoleError {
    fn from(e: ApiError) -> Self {
        Self::Database(e)
    }
}

/// Delete a role with business logic validation
pub async fn delete(
    db: &Database,
    auth_user: &AuthenticatedUser,
    role_id: &str,
) -> Result<(), RoleError> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(RoleError::Forbidden);
    }

    // Check if role exists
    let role = db.get_role_by_id(role_id).await?
        .ok_or(RoleError::NotFound)?;

    // Prevent deleting system roles
    if role.name == "Admin" || role.name == "Agent" {
        return Err(RoleError::CannotDeleteSystemRole);
    }

    // Check if role is assigned to users
    let user_count = db.count_users_with_role(role_id).await?;

    if user_count > 0 {
        return Err(RoleError::RoleInUse(user_count));
    }

    // Delete role (cascade will delete role_permissions)
    db.delete_role(role_id).await?;

    Ok(())
}
