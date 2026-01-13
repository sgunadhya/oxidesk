use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Extension, Json,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::UserNotification,
    services::connection_manager::NotificationEvent,
};

// Request DTOs
#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

// Response DTOs
#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String, // Maps to notification_type
    pub created_at: String,
    pub is_read: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
}

impl From<UserNotification> for NotificationResponse {
    fn from(notification: UserNotification) -> Self {
        Self {
            id: notification.id,
            type_: notification.notification_type.as_str().to_string(),
            created_at: notification.created_at,
            is_read: notification.is_read,
            conversation_id: notification.conversation_id,
            message_id: notification.message_id,
            actor_id: notification.actor_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<NotificationResponse>,
    pub total: i32,
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: i32,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct MarkAllReadResponse {
    pub message: String,
    pub count: i32,
}

// API Handlers

/// List notifications for the authenticated user
pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(query): Query<ListNotificationsQuery>,
) -> ApiResult<impl IntoResponse> {
    // Validate pagination parameters
    if query.limit < 1 || query.limit > 100 {
        return Err(ApiError::BadRequest(
            "Limit must be between 1 and 100".to_string(),
        ));
    }

    if query.offset < 0 {
        return Err(ApiError::BadRequest("Offset must be non-negative".to_string()));
    }

    // Fetch notifications from database
    let notifications = state
        .db
        .list_notifications(&user.user.id, query.limit, query.offset)
        .await?;

    let total = notifications.len() as i32;

    // Convert to response DTOs
    let notification_responses: Vec<NotificationResponse> = notifications
        .into_iter()
        .map(NotificationResponse::from)
        .collect();

    Ok(Json(NotificationListResponse {
        notifications: notification_responses,
        total,
    }))
}

/// Get unread notification count for the authenticated user
pub async fn get_unread_count(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> ApiResult<impl IntoResponse> {
    // Fetch unread count from database
    let count = state.db.get_unread_count(&user.user.id).await?;

    Ok(Json(UnreadCountResponse { count }))
}

/// SSE endpoint for real-time notification streaming
pub async fn notification_stream(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a channel for this connection
    let (tx, rx) = mpsc::channel::<NotificationEvent>(100);

    // Register the connection with the connection manager
    let user_id = user.user.id.clone();
    state.connection_manager.add_connection(&user_id, tx).await;

    // Log the new SSE connection
    tracing::info!("SSE connection established for user {}", user_id);

    // Create a stream from the receiver
    let stream = ReceiverStream::new(rx).map(|event| {
        // Serialize the notification event to JSON
        let json_data = serde_json::to_string(&event).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize notification event: {}", e);
            "{}".to_string()
        });

        // Create an SSE event with the JSON data
        Ok(Event::default().event("notification").data(json_data))
    });

    // Create the SSE response with keep-alive
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Mark a notification as read
pub async fn mark_notification_as_read(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Get the notification to verify ownership
    let notification = state
        .db
        .get_notification_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Notification not found".to_string()))?;

    // Check authorization - user can only mark their own notifications as read
    if notification.user_id != user.user.id {
        return Err(ApiError::Forbidden(
            "Cannot mark another agent's notification as read".to_string(),
        ));
    }

    // Mark the notification as read
    state.db.mark_notification_as_read(&id).await?;

    Ok(Json(SuccessResponse {
        message: "Notification marked as read".to_string(),
    }))
}

/// Mark all notifications as read for the authenticated user
pub async fn mark_all_notifications_as_read(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> ApiResult<impl IntoResponse> {
    // Call database method to mark all notifications as read
    let count = state.db.mark_all_notifications_as_read(&user.user.id).await?;

    Ok(Json(MarkAllReadResponse {
        message: "All notifications marked as read".to_string(),
        count,
    }))
}
