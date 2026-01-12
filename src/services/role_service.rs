use crate::api::middleware::{ApiResult, ApiError, AuthenticatedUser};
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
            permissions: None,
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
        })
        .collect();

    Ok(responses)
}

/// Get a role by ID
pub async fn get_role(
    db: &Database,
    id: &str,
) -> ApiResult<RoleResponse> {
    let role = db
        .get_role_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    // Get permissions for this role
    let permissions = db.get_role_permissions(&role.id).await?;

    let permission_responses: Vec<PermissionResponse> = permissions
        .iter()
        .map(|p| PermissionResponse {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            created_at: p.created_at.clone(),
            updated_at: p.updated_at.clone(),
        })
        .collect();

    Ok(RoleResponse {
        id: role.id.clone(),
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: Some(permission_responses),
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
        return Err(ApiError::BadRequest("Role name cannot be empty".to_string()));
    }

    // Check if role name already exists
    if let Some(_) = db.get_role_by_name(&request.name).await? {
        return Err(ApiError::Conflict("Role name already exists".to_string()));
    }

    // Create role
    let role = Role::new(request.name.clone(), request.description.clone());
    db.create_role(&role).await?;

    // Assign permissions if provided
    for permission_id in &request.permission_ids {
        let role_permission = RolePermission::new(role.id.clone(), permission_id.clone());
        db.assign_permission_to_role(&role_permission).await?;
    }

    // Get assigned permissions for response
    let permissions = db.get_role_permissions(&role.id).await?;

    let permission_responses: Vec<PermissionResponse> = permissions
        .iter()
        .map(|p| PermissionResponse {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            created_at: p.created_at.clone(),
            updated_at: p.updated_at.clone(),
        })
        .collect();

    Ok(RoleResponse {
        id: role.id.clone(),
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: Some(permission_responses),
        created_at: role.created_at.clone(),
        updated_at: role.updated_at.clone(),
    })
}

/// Update a role
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

    // Prevent updating system roles (Admin, Agent)
    if role.name == "Admin" || role.name == "Agent" {
        return Err(ApiError::BadRequest(
            "Cannot modify system roles".to_string(),
        ));
    }

    // Update role
    db.update_role(id, &request.name, &request.description).await?;

    // Get updated role
    let updated_role = db.get_role_by_id(id).await?.unwrap();

    // Get permissions
    let permissions = db.get_role_permissions(&updated_role.id).await?;

    let permission_responses: Vec<PermissionResponse> = permissions
        .iter()
        .map(|p| PermissionResponse {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            created_at: p.created_at.clone(),
            updated_at: p.updated_at.clone(),
        })
        .collect();

    Ok(RoleResponse {
        id: updated_role.id.clone(),
        name: updated_role.name.clone(),
        description: updated_role.description.clone(),
        permissions: Some(permission_responses),
        created_at: updated_role.created_at.clone(),
        updated_at: updated_role.updated_at.clone(),
    })
}

/// Delete a role
pub async fn delete(
    db: &Database,
    auth_user: &AuthenticatedUser,
    id: &str,
) -> ApiResult<()> {
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

    // Prevent deleting system roles
    if role.name == "Admin" || role.name == "Agent" {
        return Err(ApiError::BadRequest(
            "Cannot delete system roles".to_string(),
        ));
    }

    // Check if role is assigned to any users
    let user_count = db.count_users_with_role(id).await?;
    if user_count > 0 {
        return Err(ApiError::BadRequest(
            format!("Cannot delete role assigned to {} user(s)", user_count),
        ));
    }

    // Delete role (cascade will delete role_permissions)
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
