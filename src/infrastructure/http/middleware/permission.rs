use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    infrastructure::http::middleware::auth::AuthenticatedUser,
    infrastructure::persistence::Database,
    application::services::{AssignmentService, PermissionService},
};

// NOTE: require_permission is already implemented in auth.rs and now uses PermissionService

/// Helper function to format consistent permission error responses (T022)
pub fn format_permission_error(required_permission: &str) -> serde_json::Value {
    json!({
        "error": "Forbidden: Missing required permission",
        "required_permission": required_permission
    })
}

/// Helper function to format consistent multi-permission error responses
pub fn format_multi_permission_error(required_permissions: &[&str]) -> serde_json::Value {
    json!({
        "error": "Forbidden: Missing required permissions",
        "required_permissions": required_permissions
    })
}

/// Middleware to require any of multiple permissions
/// Returns 403 Forbidden if user lacks all of the specified permissions
pub async fn require_any_permission(
    permissions: &'static [&'static str],
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |req: Request, next: Next| {
        Box::pin(async move {
            // Extract authenticated user from request extensions
            let user = match req.extensions().get::<AuthenticatedUser>() {
                Some(user) => user.clone(),
                None => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({
                            "error": "Unauthorized: Authentication required"
                        })),
                    )
                        .into_response();
                }
            };

            // Check if user has any of the required permissions
            if !PermissionService::has_any_permission(&user.roles, permissions) {
                tracing::warn!(
                    "Permission denied: User {} lacks any of permissions {:?}",
                    user.user.email,
                    permissions
                );

                // TODO: Log authorization denial to auth_events table

                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "error": "Forbidden: Missing required permissions",
                        "required_permissions": permissions
                    })),
                )
                    .into_response();
            }

            // Permission granted, proceed to handler
            next.run(req).await
        })
    }
}

/// State for conversation access middleware
#[derive(Clone)]
pub struct ConversationAccessState {
    pub db: Database,
    pub assignment_service: Arc<AssignmentService>,
}

/// Middleware to require conversation access based on assignment
/// Checks if user has conversations:read_all or conversations:read_assigned with actual assignment
pub async fn require_conversation_access(
    State(state): State<ConversationAccessState>,
    req: Request,
    next: Next,
) -> Response {
    // Extract authenticated user from request extensions
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(user) => user.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Unauthorized: Authentication required"
                })),
            )
                .into_response();
        }
    };

    // Extract conversation_id from path parameters
    let conversation_id = match req.uri().path().split('/').nth(3) {
        Some(id) if !id.is_empty() => id,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Bad Request: Missing conversation ID in path"
                })),
            )
                .into_response();
        }
    };

    // Check if user has read_all permission (admin-level access)
    if PermissionService::has_permission(&user.roles, "conversations:read_all") {
        return next.run(req).await;
    }

    // Check if user has read_assigned permission
    if !PermissionService::has_permission(&user.roles, "conversations:read_assigned") {
        tracing::warn!(
            "Permission denied: User {} lacks conversations:read_all or conversations:read_assigned",
            user.user.email
        );

        // TODO: Log authorization denial to auth_events table

        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Forbidden: Missing required permission",
                "required_permissions": ["conversations:read_all", "conversations:read_assigned"]
            })),
        )
            .into_response();
    }

    // User has read_assigned permission - check if conversation is assigned to them
    match state
        .assignment_service
        .has_conversation_access(&user.user.id, conversation_id)
        .await
    {
        Ok(true) => {
            // User has access via assignment
            next.run(req).await
        }
        Ok(false) => {
            tracing::warn!(
                "Access denied: Conversation {} not assigned to user {}",
                conversation_id,
                user.user.email
            );

            // TODO: Log authorization denial to auth_events table

            (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "Forbidden: Conversation not assigned to you",
                    "conversation_id": conversation_id,
                    "required_permission": "conversations:read_assigned"
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(
                "Error checking conversation access for user {} and conversation {}: {}",
                user.user.id,
                conversation_id,
                e
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal Server Error: Failed to check conversation access"
                })),
            )
                .into_response()
        }
    }
}
