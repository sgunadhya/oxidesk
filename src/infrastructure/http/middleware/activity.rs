use crate::infrastructure::http::middleware::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

/// Middleware to track agent activity on every authenticated request
/// Updates last_activity_at timestamp for agents
pub async fn track_activity_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // Check if this is an authenticated agent request
    if let Some(auth_user) = request
        .extensions()
        .get::<crate::infrastructure::http::middleware::AuthenticatedUser>()
    {
        // Only track activity for agent users (not contacts)
        if matches!(auth_user.user.user_type, crate::domain::entities::UserType::Agent) {
            // Update activity timestamp (fire and forget, don't block request)
            let agent_id = auth_user.agent.id.clone();
            let availability_service = state.availability_service.clone();

            tokio::spawn(async move {
                if let Err(e) = availability_service.record_activity(&agent_id).await {
                    tracing::warn!("Failed to record agent activity: {}", e);
                }
            });
        }
    }

    next.run(request).await
}
