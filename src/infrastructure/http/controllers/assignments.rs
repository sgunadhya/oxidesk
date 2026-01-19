use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    infrastructure::http::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    domain::entities::*,
    application::services::AssignmentService,
};

// POST /api/conversations/:id/assign - Assign conversation
pub async fn assign_conversation(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
    Json(req): Json<AssignConversationRequest>,
) -> ApiResult<Json<ConversationResponse>> {
    // Get user's permissions via service
    let permissions = state.assignment_service
        .get_user_permissions(&user.user.id)
        .await?;

    let conversation = if let Some(user_id) = req.assigned_user_id {
        // Check if self-assignment or agent-to-agent
        if user_id == user.user.id {
            // Self-assignment
            state.assignment_service
                .self_assign_conversation(&conversation_id, &user.user.id, &permissions)
                .await?
        } else {
            // Agent-to-agent assignment
            state.assignment_service
                .assign_conversation_to_agent(
                    &conversation_id,
                    &user_id,
                    &user.user.id,
                    &permissions,
                )
                .await?
        }
    } else if let Some(team_id) = req.assigned_team_id {
        // Team assignment
        state.assignment_service
            .assign_conversation_to_team(&conversation_id, &team_id, &user.user.id, &permissions)
            .await?
    } else {
        return Err(ApiError::BadRequest(
            "Must specify either assigned_user_id or assigned_team_id".to_string(),
        ));
    };

    Ok(Json(ConversationResponse::from(conversation)))
}

// POST /api/conversations/:id/unassign - Unassign conversation
pub async fn unassign_conversation(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
) -> ApiResult<Json<ConversationResponse>> {
    let conversation = state.assignment_service
        .unassign_conversation(&conversation_id, &user.user.id)
        .await?;

    Ok(Json(ConversationResponse::from(conversation)))
}

// PUT /api/agents/:id/availability - Update agent availability
pub async fn update_agent_availability(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(agent_id): Path<String>,
    Json(req): Json<UpdateAvailabilityRequest>,
) -> ApiResult<StatusCode> {
    // Verify the user is updating their own availability
    if agent_id != user.user.id {
        return Err(ApiError::Forbidden(
            "You can only update your own availability".to_string(),
        ));
    }

    // Use shared assignment service from state
    // Update availability via service
    state.assignment_service
        .update_agent_availability(&agent_id, req.availability_status)
        .await?;

    // If setting to away_and_reassigning, auto-unassign conversations
    if req.availability_status == AgentAvailability::AwayAndReassigning {
        let unassigned_conversations = state.assignment_service.auto_unassign_on_away(&agent_id).await?;

        tracing::info!(
            "Auto-unassigned {} conversations for agent {}",
            unassigned_conversations.len(),
            agent_id
        );
    }

    Ok(StatusCode::NO_CONTENT)
}

// GET /api/conversations/unassigned - Get unassigned conversations
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
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

pub async fn get_unassigned_conversations(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<PaginationQuery>,
) -> ApiResult<Json<ConversationListResponse>> {
    // Use shared assignment service from state

    // Check permission via service
    let permissions = state.assignment_service
        .get_user_permissions(&user.user.id)
        .await?;
    if !permissions
        .iter()
        .any(|p| p.name == "conversations:read_unassigned")
    {
        return Err(ApiError::Forbidden(
            "Missing permission: conversations:read_unassigned".to_string(),
        ));
    }

    let offset = (params.page - 1) * params.per_page;
    let (conversations, total) = state.assignment_service
        .get_unassigned_conversations(params.per_page, offset)
        .await?;

    let total_pages = (total + params.per_page - 1) / params.per_page;

    Ok(Json(ConversationListResponse {
        conversations,
        pagination: PaginationMetadata {
            page: params.page,
            per_page: params.per_page,
            total_count: total,
            total_pages,
        },
    }))
}

// GET /api/conversations/assigned - Get conversations assigned to current user
pub async fn get_assigned_conversations(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<PaginationQuery>,
) -> ApiResult<Json<ConversationListResponse>> {
    // Use shared assignment service from state

    let offset = (params.page - 1) * params.per_page;
    let (conversations, total) = state.assignment_service
        .get_user_assigned_conversations(&user.user.id, params.per_page, offset)
        .await?;

    let total_pages = (total + params.per_page - 1) / params.per_page;

    Ok(Json(ConversationListResponse {
        conversations,
        pagination: PaginationMetadata {
            page: params.page,
            per_page: params.per_page,
            total_count: total,
            total_pages,
        },
    }))
}

// GET /api/teams/:id/conversations - Get team conversations
pub async fn get_team_conversations(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(team_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> ApiResult<Json<ConversationListResponse>> {
    // Use shared assignment service from state

    // Verify user is a member of the team via service
    let is_member = state.assignment_service
        .is_team_member(&team_id, &user.user.id)
        .await?;
    if !is_member {
        return Err(ApiError::Forbidden(
            "You are not a member of this team".to_string(),
        ));
    }

    let offset = (params.page - 1) * params.per_page;
    let (conversations, total) = state.assignment_service
        .get_team_conversations(&team_id, params.per_page, offset)
        .await?;

    let total_pages = (total + params.per_page - 1) / params.per_page;

    Ok(Json(ConversationListResponse {
        conversations,
        pagination: PaginationMetadata {
            page: params.page,
            per_page: params.per_page,
            total_count: total,
            total_pages,
        },
    }))
}
