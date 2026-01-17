use crate::domain::ports::agent_repository::AgentRepository;
use crate::{
    api::middleware::error::ApiError, database::Database, models::*,
    services::connection_manager::ConnectionManager,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub session_duration_hours: i64,
    pub event_bus: Arc<dyn crate::events::EventBus>,
    pub delivery_service: crate::services::DeliveryService,
    pub notification_service: crate::services::NotificationService,
    pub availability_service: crate::services::AvailabilityService,
    pub sla_service: crate::services::SlaService,
    pub automation_service: Arc<crate::services::AutomationService>,
    pub conversation_tag_service: crate::services::ConversationTagService,
    pub connection_manager: Arc<dyn ConnectionManager>,
    pub rate_limiter: crate::services::AuthRateLimiter,
    pub webhook_service: crate::services::WebhookService,
    pub tag_service: crate::services::TagService,
    pub agent_service: crate::services::AgentService,
    pub user_service: crate::services::UserService,
    pub contact_service: crate::services::ContactService,
    pub session_service: crate::services::SessionService,
    pub oidc_service: crate::services::OidcService,
    pub email_service: crate::services::EmailService,
    pub attachment_service: crate::services::AttachmentService,
    pub conversation_service: crate::services::ConversationService,
    pub message_service: crate::services::MessageService,
    pub macro_service: crate::services::MacroService,
    pub role_service: crate::services::RoleService,
    pub inbox_service: crate::services::InboxService,
    pub auth_service: crate::services::AuthService,
    pub password_reset_service: crate::services::PasswordResetService,
    pub team_service: crate::services::TeamService,
    pub conversation_priority_service: crate::services::ConversationPriorityService,
    pub assignment_service: crate::services::AssignmentService,
    pub auth_logger_service: crate::services::AuthLoggerService,
}

/// Extract and validate session token from Authorization header
/// Also checks if agent was already authenticated via API key
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Check if agent was already authenticated via API key
    if let Some(agent) = request.extensions().get::<Agent>().cloned() {
        // Agent authenticated via API key
        // Get user and roles to build AuthenticatedUser
        let user = state
            .user_service
            .get_user_by_id(&agent.user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        let roles = state.role_service.get_user_roles(&user.id).await?;

        // Compute permissions from all roles
        let permissions = compute_permissions(&roles);

        // Create a dummy session for API key auth (no actual session exists)
        // Use a long duration since API keys don't expire like sessions
        let session = Session::new_with_method(
            user.id.clone(),
            "api-key-auth".to_string(),
            24 * 365, // 1 year (API keys don't expire)
            AuthMethod::ApiKey,
            None,
        );

        request.extensions_mut().insert(AuthenticatedUser {
            user,
            agent,
            roles,
            permissions,
            session,
            token: "api-key-auth".to_string(),
        });

        return Ok(next.run(request).await);
    }

    // Fall back to session-based auth
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
        .session_service
        .get_session_by_token(token)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    if session.is_expired() {
        // Delete expired session
        state.session_service.delete_session(token).await.ok();
        return Err(ApiError::Unauthorized);
    }

    // Update last accessed timestamp for sliding window expiration
    let _ = state
        .session_service
        .update_session_last_accessed(token)
        .await;

    // Get user
    let user = state
        .user_service
        .get_user_by_id(&session.user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // Only agents can authenticate
    if !matches!(user.user_type, UserType::Agent) {
        return Err(ApiError::Unauthorized);
    }

    // Get agent
    let agent = state
        .agent_service
        .get_agent_by_user_id(&user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // Get roles
    let roles = state.role_service.get_user_roles(&user.id).await?;

    // Compute permissions from all roles
    let permissions = compute_permissions(&roles);

    // Clone token before using it (to avoid borrow checker issues)
    let token_owned = token.to_string();

    // Store authenticated user in request extensions
    request.extensions_mut().insert(AuthenticatedUser {
        user,
        agent,
        roles,
        permissions,
        session: session.clone(),
        token: token_owned,
    });

    Ok(next.run(request).await)
}

/// Compute unique permissions from all roles
fn compute_permissions(roles: &[Role]) -> Vec<String> {
    let mut permissions = std::collections::HashSet::new();

    for role in roles {
        for permission in &role.permissions {
            permissions.insert(permission.clone());
        }
    }

    permissions.into_iter().collect()
}

/// Check if user has required permission
pub async fn require_permission(
    permission: &'static str,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
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
    pub permissions: Vec<String>,
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
