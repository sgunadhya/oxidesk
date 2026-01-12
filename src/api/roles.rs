use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
};

pub async fn list_roles(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<RoleResponse>>> {
    let roles = state.db.list_roles().await?;

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

    Ok(Json(responses))
}

pub async fn get_role(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<RoleResponse>> {
    let role = state
        .db
        .get_role_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    // Get permissions for this role
    let permissions = state.db.get_role_permissions(&role.id).await?;

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

    let response = RoleResponse {
        id: role.id.clone(),
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: Some(permission_responses),
        created_at: role.created_at.clone(),
        updated_at: role.updated_at.clone(),
    };

    Ok(Json(response))
}

pub async fn create_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResult<(StatusCode, Json<RoleResponse>)> {
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
    if let Some(_) = state.db.get_role_by_name(&request.name).await? {
        return Err(ApiError::Conflict("Role name already exists".to_string()));
    }

    // Create role
    let role = Role::new(request.name.clone(), request.description.clone());
    state.db.create_role(&role).await?;

    // Assign permissions if provided
    for permission_id in &request.permission_ids {
        let role_permission = RolePermission::new(role.id.clone(), permission_id.clone());
        state.db.assign_permission_to_role(&role_permission).await?;
    }

    // Get assigned permissions for response
    let permissions = state.db.get_role_permissions(&role.id).await?;

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

    let response = RoleResponse {
        id: role.id.clone(),
        name: role.name.clone(),
        description: role.description.clone(),
        permissions: Some(permission_responses),
        created_at: role.created_at.clone(),
        updated_at: role.updated_at.clone(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn update_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResult<Json<RoleResponse>> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'roles:manage' permission".to_string(),
        ));
    }

    // Get existing role
    let role = state
        .db
        .get_role_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    // Prevent updating system roles (Admin, Agent)
    if role.name == "Admin" || role.name == "Agent" {
        return Err(ApiError::BadRequest(
            "Cannot modify system roles".to_string(),
        ));
    }

    // Update role
    state.db.update_role(&id, &request.name, &request.description).await?;

    // Get updated role
    let updated_role = state.db.get_role_by_id(&id).await?.unwrap();

    // Get permissions
    let permissions = state.db.get_role_permissions(&updated_role.id).await?;

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

    let response = RoleResponse {
        id: updated_role.id.clone(),
        name: updated_role.name.clone(),
        description: updated_role.description.clone(),
        permissions: Some(permission_responses),
        created_at: updated_role.created_at.clone(),
        updated_at: updated_role.updated_at.clone(),
    };

    Ok(Json(response))
}

pub async fn delete_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'roles:manage' permission".to_string(),
        ));
    }

    // Get role
    let role = state
        .db
        .get_role_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Role not found".to_string()))?;

    // Prevent deleting system roles
    if role.name == "Admin" || role.name == "Agent" {
        return Err(ApiError::BadRequest(
            "Cannot delete system roles".to_string(),
        ));
    }

    // Check if role is assigned to any users
    let user_count = state.db.count_users_with_role(&id).await?;
    if user_count > 0 {
        return Err(ApiError::BadRequest(
            format!("Cannot delete role assigned to {} user(s)", user_count),
        ));
    }

    // Delete role (cascade will delete role_permissions)
    state.db.delete_role(&id).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_permissions(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<PermissionResponse>>> {
    let permissions = state.db.list_permissions().await?;

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

    Ok(Json(responses))
}
