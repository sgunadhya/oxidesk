use serde::{Deserialize, Serialize};

/// Temporary storage for OIDC authentication flow state
///
/// Stores CSRF state, nonce, and PKCE verifier during OAuth2 authorization.
/// These values are validated on callback to prevent CSRF and replay attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcState {
    pub state: String,
    pub provider_name: String,
    pub nonce: String,
    pub pkce_verifier: String,
    pub created_at: String,
    pub expires_at: String,
}

impl OidcState {
    /// Create a new OIDC state with 10-minute expiration
    pub fn new(state: String, provider_name: String, nonce: String, pkce_verifier: String) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let expires_at = now + time::Duration::minutes(10);

        Self {
            state,
            provider_name,
            nonce,
            pkce_verifier,
            created_at: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            expires_at: expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        }
    }

    /// Check if this state has expired
    pub fn is_expired(&self) -> bool {
        if let Ok(expires_at) = time::OffsetDateTime::parse(
            &self.expires_at,
            &time::format_description::well_known::Rfc3339,
        ) {
            expires_at < time::OffsetDateTime::now_utc()
        } else {
            true
        }
    }
}
