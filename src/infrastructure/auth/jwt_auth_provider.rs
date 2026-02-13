/// JWT-based Authentication Provider
///
/// Implements stateless authentication using JSON Web Tokens (JWT).
/// - No database sessions (tokens are self-contained)
/// - Fast verification (cryptographic signature check)
/// - Compatible with Cloudflare Workers and edge platforms
/// - Horizontal scalability (no shared state)

use crate::application::services::auth::{
    generate_session_token, hash_password, validate_password_complexity, verify_password,
};
use crate::domain::entities::{Agent, AuthMethod, Role, Session, User, UserType};
use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::auth_provider::{
    AuthContext, AuthMetadata, AuthProvider, AuthToken, Credentials, PasswordResetCompletion,
    PasswordResetRequest, SignUpRequest,
};
use crate::domain::ports::role_repository::RoleRepository;
use crate::domain::ports::user_repository::UserRepository;
use crate::infrastructure::auth::jwt_claims::JwtClaims;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::shared::utils::email_validator::validate_and_normalize_email;
use async_trait::async_trait;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::sync::Arc;

/// JWT-based authentication provider
///
/// Uses HS256 (HMAC-SHA256) algorithm for token signing and verification.
/// All user identity and authorization data is embedded in the JWT token.
pub struct JwtAuthProvider {
    user_repo: Arc<dyn UserRepository>,
    agent_repo: Arc<dyn AgentRepository>,
    role_repo: Arc<dyn RoleRepository>,
    jwt_secret: String,
}

impl JwtAuthProvider {
    /// Create a new JWT auth provider
    ///
    /// # Arguments
    /// * `jwt_secret` - Secret key for signing JWTs (must be at least 32 characters)
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        agent_repo: Arc<dyn AgentRepository>,
        role_repo: Arc<dyn RoleRepository>,
        jwt_secret: String,
    ) -> Self {
        // Validate secret key length
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters for security");
        }

        Self {
            user_repo,
            agent_repo,
            role_repo,
            jwt_secret,
        }
    }

    /// Sign JWT claims into a token string
    fn sign_token(&self, claims: &JwtClaims) -> ApiResult<String> {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| ApiError::Internal(format!("JWT signing failed: {}", e)))
    }

    /// Verify JWT signature and decode claims
    fn verify_token_signature(&self, token: &str) -> ApiResult<JwtClaims> {
        let validation = Validation::new(Algorithm::HS256);

        decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| {
            use jsonwebtoken::errors::ErrorKind;
            match e.kind() {
                ErrorKind::ExpiredSignature => {
                    tracing::debug!("JWT token expired");
                    ApiError::Unauthorized
                }
                ErrorKind::InvalidSignature => {
                    tracing::warn!("JWT signature invalid");
                    ApiError::Unauthorized
                }
                _ => {
                    tracing::error!("JWT verification failed: {}", e);
                    ApiError::Unauthorized
                }
            }
        })
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

        // 5. Get user roles
        let roles = self.role_repo.get_user_roles(&user.id).await?;
        if roles.is_empty() {
            return Err(ApiError::Internal("User has no roles assigned".to_string()));
        }

        // 6. Compute permissions
        let permissions = Self::compute_permissions(&roles);

        // 7. Create JWT claims with user data
        let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();

        let claims = JwtClaims::new(
            user.id.clone(),
            user.email.clone(),
            match user.user_type {
                UserType::Agent => "agent".to_string(),
                UserType::Contact => "contact".to_string(),
            },
            role_ids,
            permissions,
            Some(agent.id.clone()),
            Some(agent.first_name.clone()),
            duration_hours,
        );

        // 8. Sign JWT token
        let token = self.sign_token(&claims)?;

        Ok(AuthToken {
            token,
            csrf_token: None, // JWTs don't need CSRF tokens
            expires_at: time::OffsetDateTime::from_unix_timestamp(claims.exp)
                .unwrap()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            refresh_token: None,
            auth_method: AuthMethod::Password,
            provider_name: None,
        })
    }
}

#[async_trait]
impl AuthProvider for JwtAuthProvider {
    async fn authenticate(
        &self,
        credentials: Credentials,
        duration_hours: Option<i64>,
        _metadata: Option<AuthMetadata>,
    ) -> ApiResult<AuthToken> {
        let duration = duration_hours.unwrap_or(1); // Default: 1 hour (short-lived)

        match credentials {
            Credentials::Password { email, password } => {
                self.authenticate_password(email, password, duration).await
            }
            Credentials::BearerToken { token } => {
                // For bearer tokens, just verify they exist and return them
                let context = self.verify_token(&token).await?;
                Ok(AuthToken {
                    token,
                    csrf_token: None,
                    expires_at: context.session.expires_at,
                    refresh_token: None,
                    auth_method: context.session.auth_method,
                    provider_name: context.session.provider_name,
                })
            }
            _ => Err(ApiError::BadRequest(
                "Unsupported authentication method for JWT provider".to_string(),
            )),
        }
    }

    async fn verify_token(&self, token: &str) -> ApiResult<AuthContext> {
        // 1. Verify JWT signature and decode claims
        let claims = self.verify_token_signature(token)?;

        // 2. Check if expired (redundant with jwt library but explicit)
        if claims.is_expired() {
            return Err(ApiError::Unauthorized);
        }

        // 3. Reconstruct user from claims (NO DATABASE LOOKUP!)
        let user = User {
            id: claims.sub.clone(),
            email: claims.email.clone(),
            user_type: if claims.user_type == "agent" {
                UserType::Agent
            } else {
                UserType::Contact
            },
            created_at: String::new(), // Not in JWT
            updated_at: String::new(), // Not in JWT
            deleted_at: None,
            deleted_by: None,
        };

        // 4. Reconstruct agent from claims
        let agent = Agent {
            id: claims.agent_id.clone().unwrap_or_default(),
            user_id: claims.sub.clone(),
            first_name: claims.first_name.clone().unwrap_or_default(),
            last_name: None,
            password_hash: String::new(), // Not in JWT (never expose!)
            availability_status: crate::domain::entities::user::AgentAvailability::Online,
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

        // 5. Roles and permissions are already in JWT claims
        // For full role objects, we'd need to fetch from DB
        // For now, create minimal role objects from IDs
        let roles: Vec<Role> = claims
            .roles
            .iter()
            .map(|role_id| Role {
                id: role_id.clone(),
                name: String::new(), // Not in JWT
                description: None,
                permissions: claims.permissions.clone(),
                is_protected: false,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .collect();

        // 6. Create synthetic session (no actual session exists)
        let session = Session {
            id: claims.jti.clone(),
            user_id: claims.sub.clone(),
            token: token.to_string(),
            csrf_token: String::new(), // Not used with JWT
            expires_at: time::OffsetDateTime::from_unix_timestamp(claims.exp)
                .unwrap()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            created_at: time::OffsetDateTime::from_unix_timestamp(claims.iat)
                .unwrap()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            last_accessed_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            auth_method: AuthMethod::Password,
            provider_name: None,
        };

        Ok(AuthContext {
            user,
            agent,
            roles,
            permissions: claims.permissions.clone(),
            session,
            token: token.to_string(),
        })
    }

    async fn revoke_token(&self, _token: &str) -> ApiResult<()> {
        // JWT tokens are stateless - cannot be revoked before expiration
        // Options:
        // 1. Keep tokens short-lived (recommended)
        // 2. Maintain a token blacklist (defeats stateless benefit)
        // 3. Use token versioning (requires 1 DB lookup)

        // For now, return Ok (token will expire naturally)
        Ok(())
    }

    async fn revoke_all_user_tokens(&self, _user_id: &str) -> ApiResult<u64> {
        // JWT tokens are stateless - cannot track or revoke
        // Would need token versioning or blacklist

        // Return 0 (no tokens revoked)
        Ok(0)
    }

    async fn sign_up(&self, _request: SignUpRequest) -> ApiResult<User> {
        // Sign-up not implemented yet for JWT provider
        // Would be same as DatabaseAuthProvider (create user, return user)
        Err(ApiError::BadRequest(
            "Sign-up not implemented for JWT provider".to_string(),
        ))
    }

    async fn verify_email(&self, _token: &str) -> ApiResult<()> {
        // Email verification not implemented
        Ok(())
    }

    async fn request_password_reset(&self, _request: PasswordResetRequest) -> ApiResult<()> {
        // Password reset not implemented for JWT provider
        // Would be same as DatabaseAuthProvider
        Err(ApiError::BadRequest(
            "Password reset not implemented for JWT provider".to_string(),
        ))
    }

    async fn reset_password(&self, _completion: PasswordResetCompletion) -> ApiResult<()> {
        // Password reset not implemented for JWT provider
        Err(ApiError::BadRequest(
            "Password reset not implemented for JWT provider".to_string(),
        ))
    }

    async fn change_password(
        &self,
        user_id: &str,
        current_password: &str,
        new_password: &str,
        _revoke_other_sessions: bool,
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

        // Note: Cannot revoke JWT tokens (stateless)
        // User needs to wait for tokens to expire or we need a blacklist

        Ok(())
    }

    fn supports_method(&self, method: &AuthMethod) -> bool {
        matches!(method, AuthMethod::Password)
    }

    fn provider_name(&self) -> &str {
        "jwt"
    }
}
