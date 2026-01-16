use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::user_repository::UserRepository;
use crate::{
    api::middleware::ApiError,
    database::Database,
    models::{AuthMethod, OidcProvider, Session, User, UserType},
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};

/// OIDC service for handling OAuth2/OIDC authentication flows
pub struct OidcService;

/// Authorization request with PKCE
pub struct OidcAuthRequest {
    pub authorize_url: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

impl OidcService {
    /// Initiate OIDC login flow
    ///
    /// Returns the authorization URL to redirect the user to, along with
    /// state, nonce, and PKCE verifier that must be stored for the callback.
    pub async fn initiate_login(provider: &OidcProvider) -> Result<OidcAuthRequest, ApiError> {
        // Create OIDC client
        let client = Self::create_client(provider).await?;

        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate CSRF token and nonce
        let (authorize_url, csrf_state, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(pkce_challenge)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        Ok(OidcAuthRequest {
            authorize_url: authorize_url.to_string(),
            state: csrf_state.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
        })
    }

    /// Handle OIDC callback and complete authentication
    ///
    /// Exchanges authorization code for tokens, validates ID token,
    /// and creates or updates user and session.
    pub async fn handle_callback(
        db: &Database,
        session_service: &crate::services::SessionService,
        provider: &OidcProvider,
        authorization_code: String,
        state: String,
        expected_state: String,
        pkce_verifier: String,
        session_duration_hours: i64,
    ) -> Result<CallbackResult, ApiError> {
        // Verify state matches (CSRF protection)
        if state != expected_state {
            return Err(ApiError::BadRequest("Invalid state parameter".to_string()));
        }

        // Create OIDC client
        let client = Self::create_client(provider).await?;

        // Exchange authorization code for tokens
        let pkce_verifier = PkceCodeVerifier::new(pkce_verifier);
        let token_response = client
            .exchange_code(AuthorizationCode::new(authorization_code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to exchange authorization code: {}", e))
            })?;

        // Get ID token claims
        let id_token = token_response
            .id_token()
            .ok_or_else(|| ApiError::Internal("No ID token in response".to_string()))?;

        let claims = id_token
            .claims(&client.id_token_verifier(), &Nonce::new_random())
            .map_err(|e| ApiError::Internal(format!("Failed to verify ID token: {}", e)))?;

        // Extract email from claims
        let email = claims
            .email()
            .ok_or_else(|| ApiError::Internal("No email in ID token".to_string()))?
            .as_str()
            .to_string();

        // Get or create user
        let user = match db
            .get_user_by_email_and_type(&email, &UserType::Agent)
            .await?
        {
            Some(existing_user) => existing_user,
            None => {
                // Auto-provision user if they don't exist
                // This requires the email to be pre-approved or have auto-provisioning enabled
                return Err(ApiError::Unauthorized);
            }
        };

        // Get agent
        let agent = db
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // Get roles
        let roles = db.get_user_roles(&user.id).await?;

        if roles.is_empty() {
            return Err(ApiError::Internal("User has no roles assigned".to_string()));
        }

        // Generate session token
        let token = crate::services::auth::generate_session_token();

        // Create session with OIDC auth method
        let session = Session::new_with_method(
            user.id.clone(),
            token,
            session_duration_hours,
            AuthMethod::Oidc,
            Some(provider.name.clone()),
        );

        session_service.create_session(&session).await?;

        Ok(CallbackResult {
            session,
            user,
            agent,
            roles,
        })
    }

    /// Create OIDC client from provider configuration
    async fn create_client(provider: &OidcProvider) -> Result<CoreClient, ApiError> {
        // Discover provider metadata
        let issuer_url = IssuerUrl::new(provider.issuer_url.clone())
            .map_err(|e| ApiError::Internal(format!("Invalid issuer URL: {}", e)))?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to discover provider metadata: {}", e))
            })?;

        // Create client
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(provider.client_id.clone()),
            Some(ClientSecret::new(provider.client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(provider.redirect_uri.clone())
                .map_err(|e| ApiError::Internal(format!("Invalid redirect URI: {}", e)))?,
        );

        Ok(client)
    }
}

/// Result of successful OIDC callback
pub struct CallbackResult {
    pub session: Session,
    pub user: User,
    pub agent: crate::models::Agent,
    pub roles: Vec<crate::models::Role>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oidc_auth_request_has_required_fields() {
        let auth_request = OidcAuthRequest {
            authorize_url: "https://example.com/authorize".to_string(),
            state: "random_state".to_string(),
            nonce: "random_nonce".to_string(),
            pkce_verifier: "random_verifier".to_string(),
        };

        assert!(!auth_request.authorize_url.is_empty());
        assert!(!auth_request.state.is_empty());
        assert!(!auth_request.nonce.is_empty());
        assert!(!auth_request.pkce_verifier.is_empty());
    }
}
