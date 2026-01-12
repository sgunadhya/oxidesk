use crate::{
    api::middleware::{ApiError, AuthenticatedUser, AppState},
    database::Database,
    models::*,
};
use std::fmt;

#[derive(Debug)]
pub enum AgentError {
    NotFound,
    Forbidden,
    CannotDeleteSelf,
    CannotDeleteLastAdmin,
    Database(ApiError),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Agent not found"),
            Self::Forbidden => write!(f, "Permission denied"),
            Self::CannotDeleteSelf => write!(f, "Cannot delete your own account"),
            Self::CannotDeleteLastAdmin => write!(f, "Cannot delete the last admin user"),
            Self::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<ApiError> for AgentError {
    fn from(e: ApiError) -> Self {
        Self::Database(e)
    }
}

/// Delete an agent with business logic validation
pub async fn delete(
    db: &Database,
    auth_user: &AuthenticatedUser,
    agent_id: &str,
) -> Result<(), AgentError> {
    tracing::info!(
        "Agent deletion requested by user {} for agent {}",
        auth_user.user.email,
        agent_id
    );

    // Check permission (admin only)
    if !auth_user.is_admin() {
        tracing::warn!(
            "Agent deletion denied: user {} lacks admin permission",
            auth_user.user.email
        );
        return Err(AgentError::Forbidden);
    }

    // Prevent deleting self
    if auth_user.user.id == agent_id {
        tracing::warn!(
            "Agent deletion denied: user {} attempted self-deletion",
            auth_user.user.email
        );
        return Err(AgentError::CannotDeleteSelf);
    }

    // Check if user exists and is an agent
    let user = db.get_user_by_id(agent_id).await?
        .ok_or(AgentError::NotFound)?;

    if !matches!(user.user_type, UserType::Agent) {
        tracing::warn!("Agent deletion failed: user {} is not an agent", agent_id);
        return Err(AgentError::NotFound);
    }

    // Check if this agent has Admin role
    let roles = db.get_user_roles(&user.id).await?;
    let is_admin = roles.iter().any(|r| r.name == "Admin");

    if is_admin {
        // Check if this is the last admin (FR-017)
        let admin_count = db.count_admin_users().await?;

        if admin_count <= 1 {
            tracing::warn!(
                "Agent deletion denied: cannot delete last admin user {}",
                user.email
            );
            return Err(AgentError::CannotDeleteLastAdmin);
        }
    }

    // Delete user (cascade will delete agent and user_roles)
    db.delete_user(agent_id).await?;

    tracing::info!(
        "Agent {} ({}) successfully deleted by {}",
        user.email,
        agent_id,
        auth_user.user.email
    );

    Ok(())
}
