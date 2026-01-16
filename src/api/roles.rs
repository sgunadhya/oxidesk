use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    models::*,
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
    let responses = state.role_service.list_roles().await?;
    Ok(Json(responses))
}

pub async fn get_role(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<RoleResponse>> {
    let response = state.role_service.get_role(&id).await?;
    Ok(Json(response))
}

pub async fn create_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResult<(StatusCode, Json<RoleResponse>)> {
    let response = state.role_service.create_role(&auth_user, request).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn update_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResult<Json<RoleResponse>> {
    let response = state
        .role_service
        .update_role(&auth_user, &id, request)
        .await?;
    Ok(Json(response))
}

pub async fn delete_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.role_service.delete(&auth_user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_permissions(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<PermissionResponse>>> {
    let responses = state.role_service.list_permissions().await?;
    Ok(Json(responses))
}
