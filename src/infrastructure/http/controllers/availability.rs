use crate::{
    infrastructure::http::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    domain::entities::*,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// POST /api/agents/:id/availability - Set agent availability
pub async fn set_availability(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(agent_id): Path<String>,
    Json(request): Json<SetAvailabilityRequest>,
) -> ApiResult<Json<AvailabilityResponse>> {
    // Verify the agent is changing their own status (or has admin permission)
    let has_admin = auth_user.roles.iter().any(|r| r.name == "Admin");

    if auth_user.agent.id != agent_id && !has_admin {
        return Err(ApiError::Forbidden(
            "You can only change your own availability status".to_string(),
        ));
    }

    // Set availability
    state
        .availability_service
        .set_availability(&agent_id, request.status)
        .await?;

    // Return updated availability
    let response = state
        .availability_service
        .get_availability(&agent_id)
        .await?;

    Ok(Json(response))
}

/// GET /api/agents/:id/availability - Get current availability
pub async fn get_availability(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(agent_id): Path<String>,
) -> ApiResult<Json<AvailabilityResponse>> {
    // Check permissions - agents can view their own, admins can view any
    let has_admin = auth_user.roles.iter().any(|r| r.name == "Admin");

    if auth_user.agent.id != agent_id && !has_admin {
        return Err(ApiError::Forbidden(
            "You can only view your own availability".to_string(),
        ));
    }

    let response = state
        .availability_service
        .get_availability(&agent_id)
        .await?;

    Ok(Json(response))
}

/// GET /api/agents/:id/activity - Get activity log
pub async fn get_activity_log(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(agent_id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<ActivityLogResponse>> {
    // Agents can view their own activity, admins can view any
    let has_admin = auth_user.roles.iter().any(|r| r.name == "Admin");

    if auth_user.agent.id != agent_id && !has_admin {
        return Err(ApiError::Forbidden(
            "You don't have permission to view agent activity logs".to_string(),
        ));
    }

    let limit = params.per_page;
    let offset = (params.page - 1) * params.per_page;

    let response = state
        .availability_service
        .get_activity_logs(&agent_id, limit, offset)
        .await?;

    Ok(Json(response))
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
