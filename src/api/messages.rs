use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    models::{IncomingMessageRequest, Message, MessageListResponse, PaginationMetadata, SendMessageRequest},
    services::MessageService,
};

/// Webhook endpoint for receiving incoming messages from external sources
pub async fn receive_incoming_message(
    State(state): State<AppState>,
    Json(request): Json<IncomingMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let message_service = MessageService::new(state.db.clone());

    let message = message_service
        .create_incoming_message(request)
        .await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Agent sends a message to a conversation
pub async fn send_message(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    let message_service = MessageService::with_all_services(
        state.db.clone(),
        state.delivery_service.clone(),
        state.event_bus.clone(),
        state.connection_manager.clone(),
    );

    let message = message_service
        .send_message(
            conversation_id,
            auth_user.user.id,
            request,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Get a specific message by ID
pub async fn get_message(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(message_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let message_service = MessageService::new(state.db.clone());

    let message = message_service.get_message(&message_id).await?;

    Ok(Json(message))
}

#[derive(Debug, Deserialize)]
pub struct MessageListQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    50
}

/// List messages for a conversation
pub async fn list_messages(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(conversation_id): Path<String>,
    Query(query): Query<MessageListQuery>,
) -> ApiResult<impl IntoResponse> {
    let message_service = MessageService::new(state.db.clone());

    let (messages, total) = message_service
        .list_messages(&conversation_id, query.page, query.per_page)
        .await?;

    let response = MessageListResponse {
        messages,
        pagination: PaginationMetadata {
            page: query.page,
            per_page: query.per_page,
            total_count: total,
            total_pages: (total + query.per_page - 1) / query.per_page,
        },
    };

    Ok(Json(response))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Webhook endpoint (no auth required - external systems)
        .route("/api/webhooks/messages/incoming", post(receive_incoming_message))
        // Protected endpoints (require authentication)
        .route("/api/messages/:id", get(get_message))
        .route("/api/conversations/:conversation_id/messages", get(list_messages))
        .route("/api/conversations/:conversation_id/messages", post(send_message))
}
