use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
};

// ========================================
// Request/Response Types
// ========================================

#[derive(Debug, Deserialize)]
pub struct CreateSlaPolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub first_response_time: String, // e.g., "2h", "30m"
    pub resolution_time: String,     // e.g., "24h", "2d"
    pub next_response_time: String,  // e.g., "4h", "1h"
}

#[derive(Debug, Deserialize)]
pub struct UpdateSlaPolicyRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub first_response_time: Option<String>,
    pub resolution_time: Option<String>,
    pub next_response_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SlaPolicyResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub first_response_time: String,
    pub resolution_time: String,
    pub next_response_time: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<SlaPolicy> for SlaPolicyResponse {
    fn from(policy: SlaPolicy) -> Self {
        Self {
            id: policy.id,
            name: policy.name,
            description: policy.description,
            first_response_time: policy.first_response_time,
            resolution_time: policy.resolution_time,
            next_response_time: policy.next_response_time,
            created_at: policy.created_at,
            updated_at: policy.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SlaPolicyListResponse {
    pub policies: Vec<SlaPolicyResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListSlaPoliciesQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct AssignSlaPolicyToTeamRequest {
    pub sla_policy_id: Option<String>, // None to remove SLA policy
}

#[derive(Debug, Serialize)]
pub struct AppliedSlaResponse {
    pub id: String,
    pub conversation_id: String,
    pub sla_policy_id: String,
    pub status: String,
    pub first_response_deadline_at: String,
    pub resolution_deadline_at: String,
    pub applied_at: String,
    pub updated_at: String,
}

impl From<AppliedSla> for AppliedSlaResponse {
    fn from(applied_sla: AppliedSla) -> Self {
        Self {
            id: applied_sla.id,
            conversation_id: applied_sla.conversation_id,
            sla_policy_id: applied_sla.sla_policy_id,
            status: applied_sla.status.to_string(),
            first_response_deadline_at: applied_sla.first_response_deadline_at,
            resolution_deadline_at: applied_sla.resolution_deadline_at,
            applied_at: applied_sla.applied_at,
            updated_at: applied_sla.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AppliedSlaListResponse {
    pub applied_slas: Vec<AppliedSlaResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListAppliedSlasQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct ApplySlaRequest {
    pub conversation_id: String,
    pub sla_policy_id: String,
}

#[derive(Debug, Serialize)]
pub struct SlaEventResponse {
    pub id: String,
    pub applied_sla_id: String,
    pub event_type: String,
    pub status: String,
    pub deadline_at: String,
    pub met_at: Option<String>,
    pub breached_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<SlaEvent> for SlaEventResponse {
    fn from(event: SlaEvent) -> Self {
        Self {
            id: event.id,
            applied_sla_id: event.applied_sla_id,
            event_type: event.event_type.to_string(),
            status: event.status.to_string(),
            deadline_at: event.deadline_at,
            met_at: event.met_at,
            breached_at: event.breached_at,
            created_at: event.created_at,
            updated_at: event.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SlaEventListResponse {
    pub events: Vec<SlaEventResponse>,
}

// ========================================
// SLA Policy Endpoints
// ========================================

/// Create a new SLA policy
/// POST /api/sla/policies
pub async fn create_sla_policy(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Json(req): Json<CreateSlaPolicyRequest>,
) -> ApiResult<(StatusCode, Json<SlaPolicyResponse>)> {
    let policy = state
        .sla_service
        .create_policy(
            req.name,
            req.description,
            req.first_response_time,
            req.resolution_time,
            req.next_response_time,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(SlaPolicyResponse::from(policy))))
}

/// Get SLA policy by ID
/// GET /api/sla/policies/:id
pub async fn get_sla_policy(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<SlaPolicyResponse>> {
    let policy = state
        .sla_service
        .get_policy(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("SLA policy not found: {}", id)))?;

    Ok(Json(SlaPolicyResponse::from(policy)))
}

/// List all SLA policies
/// GET /api/sla/policies
pub async fn list_sla_policies(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Query(query): Query<ListSlaPoliciesQuery>,
) -> ApiResult<Json<SlaPolicyListResponse>> {
    let (policies, total) = state
        .sla_service
        .list_policies(query.limit, query.offset)
        .await?;

    let response = SlaPolicyListResponse {
        policies: policies.into_iter().map(SlaPolicyResponse::from).collect(),
        total,
        limit: query.limit,
        offset: query.offset,
    };

    Ok(Json(response))
}

/// Update SLA policy
/// PUT /api/sla/policies/:id
pub async fn update_sla_policy(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSlaPolicyRequest>,
) -> ApiResult<StatusCode> {
    state
        .sla_service
        .update_policy(
            &id,
            req.name,
            req.description,
            req.first_response_time,
            req.resolution_time,
            req.next_response_time,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete SLA policy
/// DELETE /api/sla/policies/:id
pub async fn delete_sla_policy(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    state.sla_service.delete_policy(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ========================================
// Applied SLA Endpoints
// ========================================

/// Get applied SLA by conversation ID
/// GET /api/sla/conversations/:conversation_id
pub async fn get_applied_sla_by_conversation(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
) -> ApiResult<Json<AppliedSlaResponse>> {
    let applied_sla = state
        .sla_service
        .get_applied_sla_by_conversation(&conversation_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "No applied SLA found for conversation: {}",
                conversation_id
            ))
        })?;

    Ok(Json(AppliedSlaResponse::from(applied_sla)))
}

/// List applied SLAs
/// GET /api/sla/applied
pub async fn list_applied_slas(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Query(query): Query<ListAppliedSlasQuery>,
) -> ApiResult<Json<AppliedSlaListResponse>> {
    let status_filter = if let Some(status_str) = query.status {
        Some(
            status_str
                .parse::<AppliedSlaStatus>()
                .map_err(|e| ApiError::BadRequest(format!("Invalid status: {}", e)))?,
        )
    } else {
        None
    };

    let (applied_slas, total) = state
        .sla_service
        .list_applied_slas(status_filter, query.limit, query.offset)
        .await?;

    let response = AppliedSlaListResponse {
        applied_slas: applied_slas
            .into_iter()
            .map(AppliedSlaResponse::from)
            .collect(),
        total,
        limit: query.limit,
        offset: query.offset,
    };

    Ok(Json(response))
}

/// Apply SLA policy to a conversation
/// POST /api/sla/apply
pub async fn apply_sla(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Json(req): Json<ApplySlaRequest>,
) -> ApiResult<(StatusCode, Json<AppliedSlaResponse>)> {
    // Check for "sla:manage" permission
    if !user.has_permission("sla:manage").await {
        return Err(ApiError::Forbidden(
            "User does not have permission to apply SLA policies".to_string(),
        ));
    }

    // Use current time as base timestamp for deadline calculation
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply the SLA policy
    let applied_sla = state
        .sla_service
        .apply_sla(&req.conversation_id, &req.sla_policy_id, &base_timestamp)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AppliedSlaResponse::from(applied_sla)),
    ))
}

// ========================================
// SLA Event Endpoints
// ========================================

/// Get SLA events for an applied SLA
/// GET /api/sla/applied/:applied_sla_id/events
pub async fn get_sla_events(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(applied_sla_id): Path<String>,
) -> ApiResult<Json<SlaEventListResponse>> {
    let events = state
        .sla_service
        .get_events_by_applied_sla(&applied_sla_id)
        .await?;

    let response = SlaEventListResponse {
        events: events.into_iter().map(SlaEventResponse::from).collect(),
    };

    Ok(Json(response))
}

// ========================================
// Team SLA Assignment
// ========================================

/// Assign SLA policy to a team
/// PUT /api/teams/:id/sla-policy
pub async fn assign_sla_policy_to_team(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(team_id): Path<String>,
    Json(req): Json<AssignSlaPolicyToTeamRequest>,
) -> ApiResult<StatusCode> {
    // Verify team exists
    state
        .team_service
        .get_team(&team_id)
        .await?;

    // If SLA policy ID is provided, verify it exists
    if let Some(ref policy_id) = req.sla_policy_id {
        state
            .sla_service
            .get_policy(policy_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("SLA policy not found: {}", policy_id)))?;
    }

    // Update team's SLA policy
    state
        .team_service
        .update_team_sla_policy(&team_id, req.sla_policy_id.as_deref())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
