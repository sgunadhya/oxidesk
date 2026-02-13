use crate::domain::entities::{Agent, AuthMethod, Role, Session, User};
use crate::infrastructure::http::middleware::error::ApiResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Authentication credentials for different auth methods
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Email and password authentication
    Password { email: String, password: String },

    /// API key and secret authentication
    ApiKey { key: String, secret: String },

    /// OAuth/OIDC token
    OAuth {
        provider: String,
        code: String,
        redirect_uri: Option<String>,
    },

    /// Magic link token
    MagicLink { token: String },

    /// WebAuthn/Passkey challenge response
    Passkey {
        credential_id: String,
        assertion: Vec<u8>,
    },

    /// JWT token (for stateless auth)
    BearerToken { token: String },
}

/// Result of a successful authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// The authentication token (session token or JWT)
    pub token: String,

    /// CSRF token for web sessions (optional)
    pub csrf_token: Option<String>,

    /// Token expiration timestamp (RFC3339)
    pub expires_at: String,

    /// Refresh token (optional, for JWT flows)
    pub refresh_token: Option<String>,

    /// Authentication method used
    pub auth_method: AuthMethod,

    /// Provider name (for OAuth/OIDC)
    pub provider_name: Option<String>,
}

/// Complete authenticated user context
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User entity
    pub user: User,

    /// Agent profile (only for agent users)
    pub agent: Agent,

    /// User's roles
    pub roles: Vec<Role>,

    /// Computed permissions from all roles
    pub permissions: Vec<String>,

    /// Session (may be synthetic for stateless auth)
    pub session: Session,

    /// Original token used for authentication
    pub token: String,
}

/// Options for password reset request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    /// User's email address
    pub email: String,

    /// Base URL for reset link (e.g., "https://app.com/reset")
    pub reset_url: String,
}

/// Options for password reset completion
#[derive(Debug, Clone)]
pub struct PasswordResetCompletion {
    /// Reset token from email
    pub token: String,

    /// New password
    pub new_password: String,
}

/// User registration/signup data
#[derive(Debug, Clone)]
pub struct SignUpRequest {
    /// Email address
    pub email: String,

    /// Password
    pub password: String,

    /// First name (optional)
    pub first_name: Option<String>,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// AuthProvider trait - Abstraction for all authentication methods
///
/// This trait allows swapping between different auth implementations:
/// - DatabaseAuthProvider: Current session-based auth with database
/// - JwtAuthProvider: Stateless JWT authentication
/// - BetterAuthProvider: Integration with better-auth service
/// - WorkersAuthProvider: Cloudflare Workers KV + JWT
/// - OAuthProvider: External OAuth providers (Clerk, Auth0, etc.)
#[async_trait]
pub trait AuthProvider: Send + Sync {
    // ============================================================================
    // Core Authentication
    // ============================================================================

    /// Authenticate user with credentials and return auth token
    ///
    /// # Arguments
    /// * `credentials` - Authentication credentials (password, API key, OAuth, etc.)
    /// * `duration_hours` - Session/token duration in hours (optional, uses default if None)
    /// * `metadata` - Additional context (IP, user agent, etc.) for audit logging
    ///
    /// # Returns
    /// * `AuthToken` with token, expiration, and auth method
    ///
    /// # Errors
    /// * `ApiError::Unauthorized` - Invalid credentials
    /// * `ApiError::BadRequest` - Malformed credentials
    async fn authenticate(
        &self,
        credentials: Credentials,
        duration_hours: Option<i64>,
        metadata: Option<AuthMetadata>,
    ) -> ApiResult<AuthToken>;

    /// Verify a token and return authenticated user context
    ///
    /// This method validates the token and returns the full user context
    /// including user, agent, roles, and permissions. It should also:
    /// - Check token expiration
    /// - Update last_accessed_at for sliding window sessions (if applicable)
    /// - Verify token signature (for JWT)
    ///
    /// # Arguments
    /// * `token` - Session token, JWT, or other auth token
    ///
    /// # Returns
    /// * `AuthContext` with user, agent, roles, permissions, and session
    ///
    /// # Errors
    /// * `ApiError::Unauthorized` - Invalid or expired token
    async fn verify_token(&self, token: &str) -> ApiResult<AuthContext>;

    /// Revoke/invalidate an authentication token (logout)
    ///
    /// For database sessions: Delete the session
    /// For JWTs: Add to revocation list (if supported)
    ///
    /// # Arguments
    /// * `token` - Token to revoke
    ///
    /// # Errors
    /// * `ApiError::NotFound` - Token not found (may be acceptable)
    async fn revoke_token(&self, token: &str) -> ApiResult<()>;

    /// Revoke all sessions/tokens for a specific user
    ///
    /// Useful for:
    /// - Security: Force logout on all devices
    /// - Password change: Invalidate existing sessions
    /// - Account suspension: Remove access
    ///
    /// # Arguments
    /// * `user_id` - User whose tokens should be revoked
    ///
    /// # Returns
    /// * Number of tokens revoked
    async fn revoke_all_user_tokens(&self, user_id: &str) -> ApiResult<u64>;

    // ============================================================================
    // User Registration & Management
    // ============================================================================

    /// Create a new user account (sign up)
    ///
    /// This should:
    /// 1. Validate email format and uniqueness
    /// 2. Validate password complexity
    /// 3. Hash password securely
    /// 4. Create user and agent records
    /// 5. Assign default roles
    /// 6. Optionally send verification email
    ///
    /// # Arguments
    /// * `request` - Sign up data (email, password, metadata)
    ///
    /// # Returns
    /// * Created user entity
    ///
    /// # Errors
    /// * `ApiError::BadRequest` - Validation failure (weak password, invalid email)
    /// * `ApiError::Conflict` - Email already exists
    async fn sign_up(&self, request: SignUpRequest) -> ApiResult<User>;

    /// Verify user's email address
    ///
    /// # Arguments
    /// * `token` - Email verification token
    ///
    /// # Errors
    /// * `ApiError::BadRequest` - Invalid or expired token
    async fn verify_email(&self, token: &str) -> ApiResult<()>;

    // ============================================================================
    // Password Management
    // ============================================================================

    /// Initiate password reset flow (send reset email)
    ///
    /// Should:
    /// 1. Validate email exists
    /// 2. Generate secure reset token
    /// 3. Store token with expiration
    /// 4. Send reset email
    /// 5. Rate limit reset requests
    ///
    /// Note: Should NOT reveal if email exists (timing-safe)
    ///
    /// # Arguments
    /// * `request` - Email and reset URL
    ///
    /// # Errors
    /// * `ApiError::TooManyRequests` - Rate limit exceeded
    async fn request_password_reset(&self, request: PasswordResetRequest) -> ApiResult<()>;

    /// Complete password reset with token
    ///
    /// Should:
    /// 1. Validate reset token
    /// 2. Check token expiration
    /// 3. Validate new password complexity
    /// 4. Update password hash
    /// 5. Mark token as used
    /// 6. Revoke all existing sessions
    ///
    /// # Arguments
    /// * `completion` - Reset token and new password
    ///
    /// # Errors
    /// * `ApiError::BadRequest` - Invalid/expired token or weak password
    async fn reset_password(&self, completion: PasswordResetCompletion) -> ApiResult<()>;

    /// Change password for authenticated user
    ///
    /// Should:
    /// 1. Verify current password
    /// 2. Validate new password
    /// 3. Update password hash
    /// 4. Optionally revoke other sessions (keep current)
    ///
    /// # Arguments
    /// * `user_id` - User changing password
    /// * `current_password` - Current password for verification
    /// * `new_password` - New password
    /// * `revoke_other_sessions` - Logout other devices
    ///
    /// # Errors
    /// * `ApiError::Unauthorized` - Current password incorrect
    /// * `ApiError::BadRequest` - New password doesn't meet requirements
    async fn change_password(
        &self,
        user_id: &str,
        current_password: &str,
        new_password: &str,
        revoke_other_sessions: bool,
    ) -> ApiResult<()>;

    // ============================================================================
    // Session Management (Optional - for stateful providers)
    // ============================================================================

    /// List active sessions for a user
    ///
    /// For JWT providers: May return empty or not be supported
    /// For database providers: Return all valid sessions
    ///
    /// # Arguments
    /// * `user_id` - User whose sessions to list
    ///
    /// # Returns
    /// * List of active sessions
    async fn list_user_sessions(&self, user_id: &str) -> ApiResult<Vec<Session>> {
        // Default implementation for stateless providers
        let _ = user_id;
        Ok(Vec::new())
    }

    /// Refresh/extend a token's expiration
    ///
    /// For sessions: Update expires_at and last_accessed_at
    /// For JWTs: Issue new token with refreshed expiration
    ///
    /// # Arguments
    /// * `token` - Token to refresh
    ///
    /// # Returns
    /// * New token (may be same as input for session-based)
    ///
    /// # Errors
    /// * `ApiError::Unauthorized` - Token invalid or expired
    async fn refresh_token(&self, token: &str) -> ApiResult<AuthToken> {
        // Default: Verify token and return it (sliding window handled in verify)
        let context = self.verify_token(token).await?;
        Ok(AuthToken {
            token: token.to_string(),
            csrf_token: Some(context.session.csrf_token),
            expires_at: context.session.expires_at,
            refresh_token: None,
            auth_method: context.session.auth_method,
            provider_name: context.session.provider_name,
        })
    }

    // ============================================================================
    // Utility Methods
    // ============================================================================

    /// Cleanup expired tokens/sessions
    ///
    /// Should be called periodically by a background job
    ///
    /// # Returns
    /// * Number of expired tokens removed
    async fn cleanup_expired(&self) -> ApiResult<u64> {
        // Default: No-op for stateless providers
        Ok(0)
    }

    /// Check if provider supports a specific auth method
    ///
    /// # Arguments
    /// * `method` - Auth method to check
    ///
    /// # Returns
    /// * `true` if supported
    fn supports_method(&self, method: &AuthMethod) -> bool {
        match method {
            AuthMethod::Password => true, // All providers should support password
            AuthMethod::ApiKey => false,  // Optional
            AuthMethod::Oidc => false,    // Optional
        }
    }

    /// Get provider name/type for debugging/logging
    fn provider_name(&self) -> &str;
}

/// Metadata for authentication requests (for audit logging)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMetadata {
    /// Client IP address
    pub ip_address: Option<String>,

    /// User agent string
    pub user_agent: Option<String>,

    /// Additional context
    pub extra: Option<serde_json::Value>,
}
