use axum::{
    extract::{State, Path, Query},
    Json,
    response::IntoResponse,
};
use serde::Deserialize;
use crate::api::middleware::{AppState, ApiError, ApiResult, AuthenticatedUser};
use crate::models::{CreateConversation, UpdateStatusRequest, ConversationStatus};
use crate::services::conversation_service;

/// Create a new conversation
pub async fn create_conversation(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateConversation>,
) -> ApiResult<impl IntoResponse> {
    let conversation = conversation_service::create_conversation(
        &state.db,
        &auth_user,
        request,
        Some(&state.sla_service),
    ).await?;
    Ok(Json(conversation))
}

/// Update conversation status
pub async fn update_conversation_status(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateStatusRequest>,
) -> ApiResult<impl IntoResponse> {
    let conversation = conversation_service::update_conversation_status(
        &state.db,
        &id,
        request,
        Some(auth_user.user.id.clone()),
        Some(&state.event_bus)
    ).await?;
    Ok(Json(conversation))
}

/// Get conversation by ID
pub async fn get_conversation(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let conversation = conversation_service::get_conversation(&state.db, &id).await?;
    Ok(Json(conversation))
}

/// Get conversation by Reference Number
pub async fn get_conversation_by_reference(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(reference_number): Path<i64>,
) -> ApiResult<impl IntoResponse> {
    let conversation = conversation_service::get_conversation_by_reference(&state.db, reference_number).await?;
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
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<ListConversationsParams>,
) -> ApiResult<impl IntoResponse> {
    let response = conversation_service::list_conversations(
        &state.db,
        params.page,
        params.per_page,
        params.status,
        params.inbox_id,
        params.contact_id,
    )
    .await?;
    Ok(Json(response))
}
