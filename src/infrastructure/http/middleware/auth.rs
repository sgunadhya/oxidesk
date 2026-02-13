use crate::{
    application::services,
    domain::entities::*,
    infrastructure::{
        http::middleware::error::ApiError, providers::connection_manager::ConnectionManager,
    },
    shared::rate_limiter::AuthRateLimiter,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{Redirect, Response},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub session_duration_hours: i64,
    pub auth_provider: Arc<dyn crate::domain::ports::auth_provider::AuthProvider>,
    pub event_bus: Arc<dyn crate::domain::ports::event_bus::EventBus>,
    pub delivery_service: services::DeliveryService,
    pub notification_service: services::NotificationService,
    pub availability_service: services::AvailabilityService,
    pub sla_service: services::SlaService,
    pub automation_service: Arc<services::AutomationService>,
    pub conversation_tag_service: services::ConversationTagService,
    pub connection_manager: Arc<dyn ConnectionManager>,
    pub rate_limiter: AuthRateLimiter,
    pub webhook_service: services::WebhookService,
    pub tag_service: services::TagService,
    pub agent_service: services::AgentService,
    pub user_service: services::UserService,
    pub contact_service: services::ContactService,
    pub session_service: services::SessionService,
    pub oidc_service: services::OidcService,
    pub email_service: services::EmailService,
    pub attachment_service: services::AttachmentService,
    pub conversation_service: services::ConversationService,
    pub message_service: services::MessageService,
    pub macro_service: services::MacroService,
    pub role_service: services::RoleService,
    pub inbox_service: services::InboxService,
    pub auth_service: services::AuthService,
    pub password_reset_service: services::PasswordResetService,
    pub team_service: services::TeamService,
    pub conversation_priority_service: services::ConversationPriorityService,
    pub assignment_service: services::AssignmentService,
    pub auth_logger_service: services::AuthLoggerService,
}

/// Extract and validate session token from Authorization header
/// Also checks if agent was already authenticated via API key
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract token from Authorization header
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

    // Use AuthProvider to verify token and get full context
    // This replaces ~80 lines of manual user/agent/role fetching
    let auth_context = state.auth_provider.verify_token(token).await?;

    // Store authenticated user in request extensions
    request.extensions_mut().insert(AuthenticatedUser {
        user: auth_context.user,
        agent: auth_context.agent,
        roles: auth_context.roles,
        permissions: auth_context.permissions,
        session: auth_context.session,
        token: auth_context.token,
    });

    Ok(next.run(request).await)
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
        services::PermissionService::has_permission(&self.roles, permission)
    }

    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r.name == "Admin")
    }
}

/// Web authentication middleware that checks session cookie
pub async fn web_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, Redirect> {
    // Get session token from cookie
    let cookies = request
        .headers()
        .get("Cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let token = cookies.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        if cookie.starts_with("session_token=") {
            Some(cookie.trim_start_matches("session_token="))
        } else {
            None
        }
    });

    let token = match token {
        Some(t) => t.to_string(),
        None => return Err(Redirect::to("/login")),
    };

    // Use AuthProvider to verify token and get full context
    let auth_context = match state.auth_provider.verify_token(&token).await {
        Ok(ctx) => ctx,
        Err(_) => return Err(Redirect::to("/login")),
    };

    // Store authenticated user in request extensions
    request.extensions_mut().insert(AuthenticatedUser {
        user: auth_context.user,
        agent: auth_context.agent,
        roles: auth_context.roles,
        permissions: auth_context.permissions,
        session: auth_context.session,
        token: auth_context.token,
    });

    Ok(next.run(request).await)
}
