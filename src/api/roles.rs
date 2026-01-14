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
    let responses = crate::services::role_service::list_roles(&state.db).await?;
    Ok(Json(responses))
}

pub async fn get_role(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<RoleResponse>> {
    let response = crate::services::role_service::get_role(&state.db, &id).await?;
    Ok(Json(response))
}

pub async fn create_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResult<(StatusCode, Json<RoleResponse>)> {
    let response =
        crate::services::role_service::create_role(&state.db, &auth_user, request).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn update_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResult<Json<RoleResponse>> {
    let response =
        crate::services::role_service::update_role(&state.db, &auth_user, &id, request).await?;
    Ok(Json(response))
}

pub async fn delete_role(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    crate::services::role_service::delete(&state.db, &auth_user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_permissions(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<PermissionResponse>>> {
    let responses = crate::services::role_service::list_permissions(&state.db).await?;
    Ok(Json(responses))
}
