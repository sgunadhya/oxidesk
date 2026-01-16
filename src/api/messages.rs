use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    models::{IncomingMessageRequest, MessageListResponse, PaginationMetadata, SendMessageRequest},
};

/// Webhook endpoint for receiving incoming messages from external sources
///
/// Feature 016: Automatically creates contacts from from_header if not exists.
/// Either contact_id or from_header must be provided.
pub async fn receive_incoming_message(
    State(state): State<AppState>,
    Json(mut request): Json<IncomingMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    // Feature 016: Automatic contact creation from from_header
    if request.contact_id.is_none() {
        if let Some(ref from_header) = request.from_header {
            // Parse email and display name
            let (first_name, last_name, email) =
                crate::services::user_service::parse_email_display_name(from_header);

            // Combine first_name and last_name
            let full_name = match (first_name, last_name) {
                (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
                (Some(first), None) => Some(first),
                (None, Some(last)) => Some(last),
                (None, None) => None,
            };

            // Create or get existing contact
            let contact_id = state
                .contact_service
                .create_contact_from_message(&email, full_name.as_deref(), &request.inbox_id)
                .await?;

            request.contact_id = Some(contact_id);
        } else {
            return Err(crate::api::middleware::ApiError::BadRequest(
                "Either contact_id or from_header must be provided".to_string(),
            ));
        }
    }

    let message = state
        .message_service
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
    let message = state
        .message_service
        .send_message(conversation_id, auth_user.user.id, request)
        .await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Get a specific message by ID
pub async fn get_message(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(message_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let message = state.message_service.get_message(&message_id).await?;

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
    let (messages, total) = state
        .message_service
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
        .route(
            "/api/webhooks/messages/incoming",
            post(receive_incoming_message),
        )
        // Protected endpoints (require authentication)
        .route("/api/messages/:id", get(get_message))
        .route(
            "/api/conversations/:conversation_id/messages",
            get(list_messages),
        )
        .route(
            "/api/conversations/:conversation_id/messages",
            post(send_message),
        )
}
