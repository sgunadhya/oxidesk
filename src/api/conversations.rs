use crate::api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser};
use crate::models::{
    ConversationListResponse, ConversationStatus, CreateConversation, PaginationMetadata,
    UpdatePriorityRequest, UpdateStatusRequest,
};
use crate::services::conversation_service;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};

use serde::Deserialize;

/// Create a new conversation
pub async fn create_conversation(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateConversation>,
) -> ApiResult<impl IntoResponse> {
    // Check if user has conversations:create permission
    let has_create = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:create",
    );

    if !has_create {
        return Err(ApiError::Forbidden(
            "Missing permission: conversations:create".to_string(),
        ));
    }

    let conversation = conversation_service::create_conversation(
        &state.db,
        &auth_user,
        request,
        Some(&state.sla_service),
    )
    .await?;
    Ok(Json(conversation))
}

/// Update conversation status
pub async fn update_conversation_status(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateStatusRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check if user has conversations:update_all (admin access)
    let has_update_all = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:update_all",
    );

    // Check if user has conversations:update_assigned (restricted access)
    let has_update_assigned = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:update_assigned",
    );

    // User must have at least one of these permissions
    if !has_update_all && !has_update_assigned {
        return Err(ApiError::Forbidden(
            "Missing permission: conversations:update_all or conversations:update_assigned"
                .to_string(),
        ));
    }

    // If user has update_assigned (not update_all), verify assignment
    if !has_update_all && has_update_assigned {
        // Get the conversation first to check assignment
        let conversation = conversation_service::get_conversation(&state.db, &id).await?;

        let is_assigned = conversation.assigned_user_id.as_ref() == Some(&auth_user.user.id) || {
            if let Some(team_id) = &conversation.assigned_team_id {
                // Check if user is member of assigned team
                let user_teams = state.db.get_user_teams(&auth_user.user.id).await?;
                user_teams.iter().any(|team| &team.id == team_id)
            } else {
                false
            }
        };

        if !is_assigned {
            return Err(ApiError::Forbidden(format!(
                "Conversation {} not assigned to you",
                id
            )));
        }
    }

    let conversation = conversation_service::update_conversation_status(
        &state.db,
        &id,
        request,
        Some(auth_user.user.id.clone()),
        Some(state.event_bus.as_ref()),
    )
    .await?;
    Ok(Json(conversation))
}

/// Get conversation by ID
pub async fn get_conversation(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check if user has conversations:read_all (admin access)
    let has_read_all = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:read_all",
    );

    // Check if user has conversations:read_assigned (restricted access)
    let has_read_assigned = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:read_assigned",
    );

    // User must have at least one of these permissions
    if !has_read_all && !has_read_assigned {
        return Err(ApiError::Forbidden(
            "Missing permission: conversations:read_all or conversations:read_assigned".to_string(),
        ));
    }

    // Get conversation
    let conversation = conversation_service::get_conversation(&state.db, &id).await?;

    // If user has read_assigned (not read_all), verify assignment
    if !has_read_all && has_read_assigned {
        let is_assigned = conversation.assigned_user_id.as_ref() == Some(&auth_user.user.id) || {
            if let Some(team_id) = &conversation.assigned_team_id {
                // Check if user is member of assigned team
                let user_teams = state.db.get_user_teams(&auth_user.user.id).await?;
                user_teams.iter().any(|team| &team.id == team_id)
            } else {
                false
            }
        };

        if !is_assigned {
            return Err(ApiError::Forbidden(format!(
                "Conversation {} not assigned to you",
                id
            )));
        }
    }

    Ok(Json(conversation))
}

/// Get conversation by Reference Number
pub async fn get_conversation_by_reference(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(reference_number): Path<i64>,
) -> ApiResult<impl IntoResponse> {
    let conversation =
        conversation_service::get_conversation_by_reference(&state.db, reference_number).await?;
    Ok(Json(conversation))
}

#[derive(Deserialize)]
pub struct ListConversationsParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub status: Option<ConversationStatus>,
    pub inbox_id: Option<String>,
    pub contact_id: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// List conversations with pagination and filters
pub async fn list_conversations(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<ListConversationsParams>,
) -> ApiResult<impl IntoResponse> {
    // Check if user has conversations:read_all (admin access)
    let has_read_all = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:read_all",
    );

    // Check if user has conversations:read_assigned (restricted access)
    let has_read_assigned = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:read_assigned",
    );

    // User must have at least one of these permissions
    if !has_read_all && !has_read_assigned {
        return Err(ApiError::Forbidden(
            "Missing permission: conversations:read_all or conversations:read_assigned".to_string(),
        ));
    }

    // If user has read_all, show all conversations
    if has_read_all {
        let response = conversation_service::list_conversations(
            &state.db,
            params.page,
            params.per_page,
            params.status,
            params.inbox_id,
            params.contact_id,
        )
        .await?;
        return Ok(Json(response));
    }

    // If user has read_assigned, filter by assignment
    // For now, we'll use the existing list but filter the results
    // A more efficient approach would be to add a database query filter
    let all_response = conversation_service::list_conversations(
        &state.db,
        params.page,
        params.per_page,
        params.status,
        params.inbox_id,
        params.contact_id,
    )
    .await?;

    // Get user's teams
    let user_teams = state.db.get_user_teams(&auth_user.user.id).await?;
    let user_team_ids: Vec<String> = user_teams.iter().map(|t| t.id.clone()).collect();

    // Filter conversations to only show assigned ones
    let filtered_conversations: Vec<_> = all_response
        .conversations
        .into_iter()
        .filter(|conv| {
            // Check if assigned to user
            conv.assigned_user_id.as_ref() == Some(&auth_user.user.id)
                // Or assigned to user's team
                || conv.assigned_team_id.as_ref().map_or(false, |team_id| {
                    user_team_ids.contains(team_id)
                })
        })
        .collect();

    // Update total count
    let filtered_total = filtered_conversations.len() as i64;
    let total_pages = (filtered_total + params.per_page - 1) / params.per_page;

    let response = ConversationListResponse {
        conversations: filtered_conversations,
        pagination: PaginationMetadata {
            page: params.page,
            per_page: params.per_page,
            total_count: filtered_total,
            total_pages,
        },
    };

    Ok(Json(response))
}

/// Update conversation priority (Feature 020)
pub async fn update_conversation_priority(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePriorityRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check if user has conversations:update_priority permission
    let has_permission = crate::services::PermissionService::has_permission(
        &auth_user.roles,
        "conversations:update_priority",
    );

    if !has_permission {
        return Err(ApiError::Forbidden(
            "Missing permission: conversations:update_priority".to_string(),
        ));
    }

    // Use the priority service to update the conversation
    let priority_service =
        crate::services::conversation_priority_service::ConversationPriorityService::new(
            state.db.clone(),
            Some(state.event_bus.clone()),
        );

    let updated = priority_service
        .update_conversation_priority(&id, request.priority, &auth_user.user.id)
        .await?;

    Ok(Json(updated))
}
