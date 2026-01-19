use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    infrastructure::http::middleware::{ApiResult, AppState, AuthenticatedUser},
    domain::entities::*,
};

/// GET /api/conversations/:id/tags - Get conversation tags
pub async fn get_conversation_tags(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
) -> ApiResult<Json<ConversationTagsResponse>> {
    // Get tags
    let tags = state
        .conversation_tag_service
        .get_conversation_tags(&conversation_id)
        .await?;

    let tag_responses: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();

    Ok(Json(ConversationTagsResponse {
        conversation_id,
        tags: tag_responses,
    }))
}

/// POST /api/conversations/:id/tags - Add tags to conversation
pub async fn add_tags_to_conversation(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
    Json(req): Json<AddTagsRequest>,
) -> ApiResult<Json<ConversationTagsResponse>> {
    // Get user permissions
    let permissions = state
        .conversation_tag_service
        .get_user_permissions(&user.user.id)
        .await?;

    // Add tags
    let tags = state
        .conversation_tag_service
        .add_tags(&conversation_id, req, &user.user.id, &permissions)
        .await?;

    let tag_responses: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();

    Ok(Json(ConversationTagsResponse {
        conversation_id,
        tags: tag_responses,
    }))
}

/// DELETE /api/conversations/:id/tags/:tag_id - Remove tag from conversation
pub async fn remove_tag_from_conversation(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path((conversation_id, tag_id)): Path<(String, String)>,
) -> ApiResult<Json<ConversationTagsResponse>> {
    // Get user permissions
    let permissions = state
        .conversation_tag_service
        .get_user_permissions(&user.user.id)
        .await?;

    // Remove tag
    let tags = state
        .conversation_tag_service
        .remove_tag(&conversation_id, &tag_id, &user.user.id, &permissions)
        .await?;

    let tag_responses: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();

    Ok(Json(ConversationTagsResponse {
        conversation_id,
        tags: tag_responses,
    }))
}

/// PUT /api/conversations/:id/tags - Replace all conversation tags
pub async fn replace_conversation_tags(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
    Json(req): Json<ReplaceTagsRequest>,
) -> ApiResult<Json<ConversationTagsResponse>> {
    // Get user permissions
    let permissions = state
        .conversation_tag_service
        .get_user_permissions(&user.user.id)
        .await?;

    // Replace tags
    let tags = state
        .conversation_tag_service
        .replace_tags(&conversation_id, req, &user.user.id, &permissions)
        .await?;

    let tag_responses: Vec<TagResponse> = tags.into_iter().map(TagResponse::from).collect();

    Ok(Json(ConversationTagsResponse {
        conversation_id,
        tags: tag_responses,
    }))
}
