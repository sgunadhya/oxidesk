use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    models::*,
    services::connection_manager::ConnectionManager,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub session_duration_hours: i64,
    pub event_bus: crate::events::EventBus,
    pub delivery_service: crate::services::DeliveryService,
    pub notification_service: crate::services::NotificationService,
    pub availability_service: crate::services::AvailabilityService,
    pub sla_service: crate::services::SlaService,
    pub connection_manager: Arc<dyn ConnectionManager>,
    pub rate_limiter: crate::services::AuthRateLimiter,
}

/// Extract and validate session token from Authorization header
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = if let Some(auth_value) = auth_header {
        if let Some(token) = auth_value.strip_prefix("Bearer ") {
            token
        } else {
            return Err(ApiError::Unauthorized);
        }
    } else {
        return Err(ApiError::Unauthorized);
    };

    // Validate session
    let session = state
        .db
        .get_session_by_token(token)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    if session.is_expired() {
        // Delete expired session
        state.db.delete_session(token).await.ok();
        return Err(ApiError::Unauthorized);
    }

    // Update last accessed timestamp for sliding window expiration
    let _ = state.db.update_session_last_accessed(token).await;

    // Get user
    let user = state
        .db
        .get_user_by_id(&session.user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // Only agents can authenticate
    if !matches!(user.user_type, UserType::Agent) {
        return Err(ApiError::Unauthorized);
    }

    // Get agent
    let agent = state
        .db
        .get_agent_by_user_id(&user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // Get roles
    let roles = state.db.get_user_roles(&user.id).await?;

    // Clone token before using it (to avoid borrow checker issues)
    let token_owned = token.to_string();

    // Store authenticated user in request extensions
    request.extensions_mut().insert(AuthenticatedUser {
        user,
        agent,
        roles,
        session: session.clone(),
        token: token_owned,
    });

    Ok(next.run(request).await)
}

/// Check if user has required permission
pub async fn require_permission(
    permission: &'static str,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |mut request: Request, next: Next| {
        Box::pin(async move {
            let auth_user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(ApiError::Unauthorized)?
                .clone();

            if !auth_user.has_permission(permission).await {
                return Err(ApiError::Forbidden(format!(
                    "Requires '{}' permission",
                    permission
                )));
            }

            Ok(next.run(request).await)
        })
    }
}

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user: User,
    pub agent: Agent,
    pub roles: Vec<Role>,
    pub session: Session,
    pub token: String,
}

impl AuthenticatedUser {
    pub async fn has_permission(&self, permission: &str) -> bool {
        // Use PermissionService to check permission across all roles
        crate::services::PermissionService::has_permission(&self.roles, permission)
    }

    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r.name == "Admin")
    }
}
