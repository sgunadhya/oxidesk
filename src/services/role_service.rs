use crate::api::middleware::{ApiError, ApiResult, AuthenticatedUser};
use crate::database::Database;
use crate::models::*;

/// List all roles
pub async fn list_roles(db: &Database) -> ApiResult<Vec<RoleResponse>> {
    let roles = db.list_roles().await?;

    let responses: Vec<RoleResponse> = roles
        .iter()
        .map(|r| RoleResponse {
            id: r.id.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            permissions: r.permissions.clone(),
            is_protected: r.is_protected,
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
        })
        .collect();

    Ok(responses)
}

/// Get a role by ID
pub async fn get_role(db: &Database, id: &str) -> ApiResult<RoleResponse> {
    let role = db
        .get_role_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    Ok(RoleResponse {
        id: role.id.clone(),
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: role.permissions.clone(),
        is_protected: role.is_protected,
        created_at: role.created_at.clone(),
        updated_at: role.updated_at.clone(),
    })
}

/// Create a new role
pub async fn create_role(
    db: &Database,
    auth_user: &AuthenticatedUser,
    request: CreateRoleRequest,
) -> ApiResult<RoleResponse> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'roles:manage' permission".to_string(),
        ));
    }

    // Validate name
    if request.name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Role name cannot be empty".to_string(),
        ));
    }

    // Check if role name already exists
    if let Some(_) = db.get_role_by_name(&request.name).await? {
        return Err(ApiError::Conflict("Role name already exists".to_string()));
    }

    // Feature 023: Validate cardinality invariants
    // FR-012, FR-014: Role must have at least one permission
    if request.permissions.is_empty() {
        return Err(ApiError::BadRequest(
            "Role must have at least one permission".to_string(),
        ));
    }

    // Validate permissions format (must match "resource:action" pattern)
    for permission in &request.permissions {
        if !permission.contains(':') {
            return Err(ApiError::BadRequest(format!(
                "Invalid permission format: '{}'. Must match pattern 'resource:action'",
                permission
            )));
        }
    }

    // Create role with permissions
    let role = Role::new(
        request.name.clone(),
        request.description.clone(),
        request.permissions.clone(),
    );
    db.create_role(&role).await?;

    Ok(RoleResponse {
        id: role.id.clone(),
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: role.permissions.clone(),
        is_protected: role.is_protected,
        created_at: role.created_at.clone(),
        updated_at: role.updated_at.clone(),
    })
}

/// Update a role (T060-T061: Add is_protected check)
pub async fn update_role(
    db: &Database,
    auth_user: &AuthenticatedUser,
    id: &str,
    request: UpdateRoleRequest,
) -> ApiResult<RoleResponse> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'roles:manage' permission".to_string(),
        ));
    }

    // Get existing role
    let role = db
        .get_role_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    // T060-T061: Prevent updating protected roles (Admin role)
    if role.is_protected {
        return Err(ApiError::Forbidden("Cannot modify Admin role".to_string()));
    }

    // Feature 023: Validate cardinality invariants
    // FR-013, FR-014: Role must have at least one permission
    if let Some(ref perms) = request.permissions {
        if perms.is_empty() {
            return Err(ApiError::BadRequest(
                "Role must have at least one permission".to_string(),
            ));
        }

        // Validate permissions format
        for permission in perms {
            if !permission.contains(':') {
                return Err(ApiError::BadRequest(format!(
                    "Invalid permission format: '{}'. Must match pattern 'resource:action'",
                    permission
                )));
            }
        }
    }

    // Update role
    db.update_role(
        id,
        request.name.as_deref(),
        request.description.as_deref(),
        request.permissions.as_ref(),
    )
    .await?;

    // Get updated role
    let updated_role = db.get_role_by_id(id).await?.unwrap();

    Ok(RoleResponse {
        id: updated_role.id.clone(),
        name: updated_role.name.clone(),
        description: updated_role.description.clone(),
        permissions: updated_role.permissions.clone(),
        is_protected: updated_role.is_protected,
        created_at: updated_role.created_at.clone(),
        updated_at: updated_role.updated_at.clone(),
    })
}

/// Delete a role (T062-T063: Add is_protected check)
pub async fn delete(db: &Database, auth_user: &AuthenticatedUser, id: &str) -> ApiResult<()> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'roles:manage' permission".to_string(),
        ));
    }

    // Get role
    let role = db
        .get_role_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    // T062-T063: Prevent deleting protected roles (Admin role)
    if role.is_protected {
        return Err(ApiError::Forbidden("Cannot modify Admin role".to_string()));
    }

    // Check if role is assigned to any users
    let user_count = db.count_users_with_role(id).await?;
    if user_count > 0 {
        return Err(ApiError::Conflict(format!(
            "Cannot delete role: {} agents currently assigned",
            user_count
        )));
    }

    // Delete role
    db.delete_role(id).await?;

    Ok(())
}

/// List all permissions
pub async fn list_permissions(db: &Database) -> ApiResult<Vec<PermissionResponse>> {
    let permissions = db.list_permissions().await?;

    let responses: Vec<PermissionResponse> = permissions
        .iter()
        .map(|p| PermissionResponse {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            created_at: p.created_at.clone(),
            updated_at: p.updated_at.clone(),
        })
        .collect();

    Ok(responses)
}
