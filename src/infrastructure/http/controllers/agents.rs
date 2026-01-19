use crate::{
    infrastructure::http::middleware::{ApiResult, AppState, AuthenticatedUser},
    domain::entities::*,
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
    let response = state
        .agent_service
        .create_agent(&auth_user, request)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_agent(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentResponse>> {
    let response = state.agent_service.get_agent(&id).await?;
    Ok(Json(response))
}

pub async fn delete_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.agent_service.delete(&auth_user, &id).await?;
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
    let response = state
        .agent_service
        .list_agents(params.page, params.per_page)
        .await?;
    Ok(Json(response))
}

pub async fn update_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> ApiResult<Json<AgentResponse>> {
    let response = state
        .agent_service
        .update_agent(&auth_user, &id, request)
        .await?;
    Ok(Json(response))
}

pub async fn change_agent_password(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    state
        .agent_service
        .change_agent_password(&auth_user, &id, request)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
