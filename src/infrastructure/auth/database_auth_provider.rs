use crate::application::services::auth::{
    generate_session_token, hash_password, validate_password_complexity, verify_password,
    AuthResult,
};
use crate::application::services::password_reset_email_service::{send_password_reset_email, SmtpConfig};
use crate::domain::entities::{Agent, AuthMethod, PasswordResetToken, Role, Session, User, UserType};
use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::auth_provider::{
    AuthContext, AuthMetadata, AuthProvider, AuthToken, Credentials, PasswordResetCompletion,
    PasswordResetRequest, SignUpRequest,
};
use crate::domain::ports::password_reset_repository::PasswordResetRepository;
use crate::domain::ports::role_repository::RoleRepository;
use crate::domain::ports::session_repository::SessionRepository;
use crate::domain::ports::user_repository::UserRepository;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::shared::utils::email_validator::validate_and_normalize_email;
use crate::shared::utils::generate_reset_token;
use async_trait::async_trait;
use std::sync::Arc;

/// Database-backed authentication provider
///
/// This provider implements the AuthProvider trait using database repositories
/// for sessions, users, agents, roles, and password resets. It wraps the existing
/// authentication logic from AuthService and related services.
pub struct DatabaseAuthProvider {
    user_repo: Arc<dyn UserRepository>,
    agent_repo: Arc<dyn AgentRepository>,
    role_repo: Arc<dyn RoleRepository>,
    session_repo: Arc<dyn SessionRepository>,
    password_reset_repo: PasswordResetRepository, // Note: This is a concrete struct, not a trait
}

impl DatabaseAuthProvider {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        agent_repo: Arc<dyn AgentRepository>,
        role_repo: Arc<dyn RoleRepository>,
        session_repo: Arc<dyn SessionRepository>,
        password_reset_repo: PasswordResetRepository,
    ) -> Self {
        Self {
            user_repo,
            agent_repo,
            role_repo,
            session_repo,
            password_reset_repo,
        }
    }

    /// Helper to compute permissions from roles
    fn compute_permissions(roles: &[Role]) -> Vec<String> {
        let mut permissions = std::collections::HashSet::new();
        for role in roles {
            for permission in &role.permissions {
                permissions.insert(permission.clone());
            }
        }
        permissions.into_iter().collect()
    }

    /// Authenticate with email and password
    async fn authenticate_password(
        &self,
        email: String,
        password: String,
        duration_hours: i64,
    ) -> ApiResult<AuthToken> {
        // 1. Validate and normalize email
        let email = validate_and_normalize_email(&email)?;

        // 2. Get user by email (must be Agent type)
        let user = self
            .user_repo
            .get_user_by_email_and_type(&email, &UserType::Agent)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // 3. Get agent
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // 4. Verify password
        let password_valid = verify_password(&password, &agent.password_hash)?;
        if !password_valid {
            return Err(ApiError::Unauthorized);
        }

        // 5. Check user has roles
        let roles = self.role_repo.get_user_roles(&user.id).await?;
        if roles.is_empty() {
            return Err(ApiError::Internal("User has no roles assigned".to_string()));
        }

        // 6. Generate session token and create session
        let token = generate_session_token();
        let session = Session::new(user.id.clone(), token.clone(), duration_hours);
        self.session_repo.create_session(&session).await?;

        Ok(AuthToken {
            token: session.token,
            csrf_token: Some(session.csrf_token),
            expires_at: session.expires_at,
            refresh_token: None,
            auth_method: AuthMethod::Password,
            provider_name: None,
        })
    }

    /// Authenticate with API key and secret
    /// Note: API key authentication is handled separately in the middleware
    /// This method returns an error for now as API key auth goes through a different path
    async fn authenticate_api_key(
        &self,
        _key: String,
        _secret: String,
        _duration_hours: i64,
    ) -> ApiResult<AuthToken> {
        // API key authentication is handled by api_key_auth_middleware
        // which sets Agent in request extensions before auth_provider is called
        Err(ApiError::BadRequest(
            "API key authentication handled by separate middleware".to_string(),
        ))
    }
}

#[async_trait]
impl AuthProvider for DatabaseAuthProvider {
    async fn authenticate(
        &self,
        credentials: Credentials,
        duration_hours: Option<i64>,
        _metadata: Option<AuthMetadata>,
    ) -> ApiResult<AuthToken> {
        let duration = duration_hours.unwrap_or(24);

        match credentials {
            Credentials::Password { email, password } => {
                self.authenticate_password(email, password, duration).await
            }
            Credentials::ApiKey { key, secret } => {
                self.authenticate_api_key(key, secret, duration).await
            }
            Credentials::BearerToken { token } => {
                // For bearer tokens, just verify they exist and return them
                // This is used when a token is already valid
                let context = self.verify_token(&token).await?;
                Ok(AuthToken {
                    token,
                    csrf_token: Some(context.session.csrf_token),
                    expires_at: context.session.expires_at,
                    refresh_token: None,
                    auth_method: context.session.auth_method,
                    provider_name: context.session.provider_name,
                })
            }
            _ => Err(ApiError::BadRequest(
                "Unsupported authentication method for database provider".to_string(),
            )),
        }
    }

    async fn verify_token(&self, token: &str) -> ApiResult<AuthContext> {
        // Regular session token verification
        let session = self
            .session_repo
            .get_session_by_token(token)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // Check expiration
        if session.is_expired() {
            // Delete expired session
            let _ = self.session_repo.delete_session(token).await;
            return Err(ApiError::Unauthorized);
        }

        // Update last accessed timestamp (sliding window)
        let _ = self
            .session_repo
            .update_session_last_accessed(token)
            .await;

        // Get user
        let user = self
            .user_repo
            .get_user_by_id(&session.user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // Only agents can authenticate
        if !matches!(user.user_type, UserType::Agent) {
            return Err(ApiError::Unauthorized);
        }

        // Get agent
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // Get roles
        let roles = self.role_repo.get_user_roles(&user.id).await?;

        // Compute permissions
        let permissions = Self::compute_permissions(&roles);

        Ok(AuthContext {
            user,
            agent,
            roles,
            permissions,
            session,
            token: token.to_string(),
        })
    }

    async fn revoke_token(&self, token: &str) -> ApiResult<()> {
        // Delete session
        self.session_repo.delete_session(token).await
    }

    async fn revoke_all_user_tokens(&self, user_id: &str) -> ApiResult<u64> {
        // Delete all sessions for user
        self.session_repo.delete_user_sessions(user_id).await
    }

    async fn sign_up(&self, request: SignUpRequest) -> ApiResult<User> {
        // 1. Validate email
        let email = validate_and_normalize_email(&request.email)?;

        // 2. Check email doesn't exist
        if let Some(_existing) = self
            .user_repo
            .get_user_by_email_and_type(&email, &UserType::Agent)
            .await?
        {
            return Err(ApiError::Conflict(
                "User with this email already exists".to_string(),
            ));
        }

        // 3. Validate password complexity
        validate_password_complexity(&request.password)?;

        // 4. Hash password
        let password_hash = hash_password(&request.password)?;

        // 5. Create user
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            email: email.clone(),
            user_type: UserType::Agent,
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            updated_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            deleted_at: None,
            deleted_by: None,
        };

        self.user_repo.create_user(&user).await?;

        // 6. Create agent profile
        let agent = Agent {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            first_name: request.first_name.unwrap_or_else(|| "Agent".to_string()),
            last_name: None,
            password_hash,
            availability_status: crate::domain::entities::user::AgentAvailability::Offline,
            last_login_at: None,
            last_activity_at: None,
            away_since: None,
            api_key: None,
            api_secret_hash: None,
            api_key_description: None,
            api_key_created_at: None,
            api_key_last_used_at: None,
            api_key_revoked_at: None,
        };

        self.agent_repo.create_agent(&agent).await?;

        // 7. Assign default role (if exists)
        // Note: This would require a default role to exist in the system
        // For now, skip automatic role assignment

        Ok(user)
    }

    async fn verify_email(&self, _token: &str) -> ApiResult<()> {
        // Email verification not implemented in current system
        // Return Ok for now to satisfy trait
        Ok(())
    }

    async fn request_password_reset(&self, request: PasswordResetRequest) -> ApiResult<()> {
        // Normalize email
        let email = request.email.trim().to_lowercase();

        // Try to find agent by email
        let user_option = self
            .user_repo
            .get_user_by_email_and_type(&email, &UserType::Agent)
            .await?;

        // If user exists, proceed with reset flow
        if let Some(user) = user_option {
            // Check rate limit (5 requests per hour)
            let rate_limit_window = std::env::var("PASSWORD_RESET_RATE_LIMIT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5);

            let recent_requests = self
                .password_reset_repo
                .count_recent_requests(&user.id, 3600)
                .await?;

            if recent_requests >= rate_limit_window {
                return Err(ApiError::TooManyRequests(
                    "Too many password reset requests. Please try again later.".to_string(),
                ));
            }

            // Generate reset token
            let token_value = generate_reset_token();
            let reset_token = PasswordResetToken::new(user.id.clone(), token_value.clone());

            // Invalidate previous tokens for this user
            self.password_reset_repo
                .invalidate_user_tokens(&user.id)
                .await?;

            // Store new token
            self.password_reset_repo.create_token(&reset_token).await?;

            // Send email (async, best-effort)
            let smtp_config = SmtpConfig::from_env().map_err(|e| {
                ApiError::Internal(format!("SMTP configuration error: {}", e))
            })?;

            let reset_url = format!("{}?token={}", request.reset_url, token_value);

            tokio::spawn(async move {
                if let Err(e) = send_password_reset_email(&email, &reset_url, &smtp_config).await {
                    tracing::error!("Failed to send password reset email: {}", e);
                }
            });
        }

        // Always return success to prevent email enumeration
        Ok(())
    }

    async fn reset_password(&self, completion: PasswordResetCompletion) -> ApiResult<()> {
        // 1. Get and validate token
        let token_record = self
            .password_reset_repo
            .get_token(&completion.token)
            .await?
            .ok_or(ApiError::BadRequest("Invalid or expired reset token".to_string()))?;

        // 2. Check if already used
        if token_record.used {
            return Err(ApiError::BadRequest("Reset token already used".to_string()));
        }

        // 3. Check expiration (1 hour)
        let expires_at = time::OffsetDateTime::parse(
            &token_record.expires_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| ApiError::Internal("Invalid expiration timestamp".to_string()))?;

        if expires_at < time::OffsetDateTime::now_utc() {
            return Err(ApiError::BadRequest("Reset token expired".to_string()));
        }

        // 4. Validate new password
        validate_password_complexity(&completion.new_password)?;

        // 5. Hash new password
        let new_hash = hash_password(&completion.new_password)?;

        // 6. Atomically update password, mark token as used, and destroy sessions
        let _sessions_destroyed = self
            .password_reset_repo
            .reset_password_atomic(&token_record.user_id, &token_record.id, &new_hash)
            .await?;

        Ok(())
    }

    async fn change_password(
        &self,
        user_id: &str,
        current_password: &str,
        new_password: &str,
        revoke_other_sessions: bool,
    ) -> ApiResult<()> {
        // 1. Get agent
        let agent = self
            .agent_repo
            .get_agent_by_user_id(user_id)
            .await?
            .ok_or(ApiError::NotFound("Agent not found".to_string()))?;

        // 2. Verify current password
        let valid = verify_password(current_password, &agent.password_hash)?;
        if !valid {
            return Err(ApiError::Unauthorized);
        }

        // 3. Validate new password
        validate_password_complexity(new_password)?;

        // 4. Hash new password
        let new_hash = hash_password(new_password)?;

        // 5. Update password
        self.agent_repo
            .update_agent_password(&agent.id, &new_hash)
            .await?;

        // 6. Optionally revoke other sessions
        if revoke_other_sessions {
            self.session_repo.delete_user_sessions(user_id).await?;
        }

        Ok(())
    }

    async fn list_user_sessions(&self, user_id: &str) -> ApiResult<Vec<Session>> {
        self.session_repo.get_user_sessions(user_id).await
    }

    async fn cleanup_expired(&self) -> ApiResult<u64> {
        self.session_repo.cleanup_expired_sessions().await
    }

    fn supports_method(&self, method: &AuthMethod) -> bool {
        matches!(method, AuthMethod::Password | AuthMethod::ApiKey)
    }

    fn provider_name(&self) -> &str {
        "database"
    }
}
