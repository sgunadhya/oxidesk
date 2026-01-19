use crate::{
    infrastructure::http::middleware::{ApiResult, AppState, AuthenticatedUser},
    domain::entities::*,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub async fn list_roles(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<RoleResponse>>> {
    let roles = state.role_service.list_roles().await?;
    let responses = roles.into_iter().map(RoleResponse::from).collect();
    Ok(Json(responses))
}

pub async fn get_role(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<RoleResponse>> {
    let role = state.role_service.get_role(&id).await?;
    Ok(Json(RoleResponse::from(role)))
}

pub async fn create_role(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResult<(StatusCode, Json<RoleResponse>)> {
    let role = state.role_service.create_role(
        request.name,
        request.description,
        request.permissions,
    ).await?;
    Ok((StatusCode::CREATED, Json(RoleResponse::from(role))))
}

pub async fn update_role(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResult<Json<RoleResponse>> {
    let role = state
        .role_service
        .update_role(&id, request.name, request.description, request.permissions)
        .await?;
    Ok(Json(RoleResponse::from(role)))
}

pub async fn delete_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.role_service.delete(&auth_user.user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_permissions(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<PermissionResponse>>> {
    let permissions = state.role_service.list_permissions().await?;
    let responses = permissions.into_iter().map(PermissionResponse::from).collect();
    Ok(Json(responses))
}
