use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    models::*,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

pub async fn create_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateAgentRequest>,
) -> ApiResult<(StatusCode, Json<CreateAgentResponse>)> {
    let response =
        crate::services::agent_service::create_agent(&state.db, &auth_user, request).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_agent(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentResponse>> {
    let response = crate::services::agent_service::get_agent(&state.db, &id).await?;
    Ok(Json(response))
}

pub async fn delete_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    crate::services::agent_service::delete(&state.db, &auth_user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

pub async fn list_agents(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<AgentListResponse>> {
    let response =
        crate::services::agent_service::list_agents(&state.db, params.page, params.per_page)
            .await?;
    Ok(Json(response))
}

pub async fn update_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> ApiResult<Json<AgentResponse>> {
    let response =
        crate::services::agent_service::update_agent(&state.db, &auth_user, &id, request).await?;
    Ok(Json(response))
}

pub async fn change_agent_password(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    crate::services::agent_service::change_agent_password(&state.db, &auth_user, &id, request)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
