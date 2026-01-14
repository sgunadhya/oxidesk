use axum::{
    extract::{Request, State},
    http::header::{AUTHORIZATION, HeaderMap},
    middleware::Next,
    response::Response,
};
use base64::{Engine as _, engine::general_purpose};

use crate::api::middleware::error::ApiError;
use crate::api::middleware::auth::AppState;
use crate::database::Database;
use crate::models::Agent;
use crate::services::api_key_service::verify_api_secret;

/// Extract API key credentials from request headers
/// Supports two methods:
/// 1. Custom headers: X-API-Key and X-API-Secret
/// 2. HTTP Basic Auth: Authorization: Basic <base64(key:secret)>
fn extract_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    // Method 1: Check for custom headers (priority)
    if let (Some(api_key), Some(api_secret)) = (
        headers.get("X-API-Key"),
        headers.get("X-API-Secret"),
    ) {
        if let (Ok(key), Ok(secret)) = (api_key.to_str(), api_secret.to_str()) {
            return Some((key.to_string(), secret.to_string()));
        }
    }

    // Method 2: Check for HTTP Basic Auth
    if let Some(auth_header) = headers.get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(credentials) = auth_str.strip_prefix("Basic ") {
                if let Ok(decoded) = general_purpose::STANDARD.decode(credentials) {
                    if let Ok(decoded_str) = String::from_utf8(decoded) {
                        if let Some((key, secret)) = decoded_str.split_once(':') {
                            return Some((key.to_string(), secret.to_string()));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Validate API key format
/// Key must be exactly 32 alphanumeric characters
/// Secret must be exactly 64 alphanumeric characters
fn validate_credentials_format(api_key: &str, api_secret: &str) -> bool {
    api_key.len() == 32
        && api_key.chars().all(|c| c.is_alphanumeric())
        && api_secret.len() == 64
        && api_secret.chars().all(|c| c.is_alphanumeric())
}

/// Authenticate request with API key
/// Returns the agent if authentication succeeds, None otherwise
pub async fn authenticate_with_api_key(
    db: &Database,
    api_key: &str,
    api_secret: &str,
) -> Result<Option<Agent>, ApiError> {
    // Validate format first (fast rejection)
    if !validate_credentials_format(api_key, api_secret) {
        tracing::debug!("API key/secret format invalid");
        return Ok(None);
    }

    // Lookup API key in database
    let agent = match db.get_agent_by_api_key(api_key).await? {
        Some(agent) => agent,
        None => {
            tracing::debug!("API key not found: {}", &api_key[..10.min(api_key.len())]);
            return Ok(None);
        }
    };

    // Check if secret hash exists (key not revoked)
    let secret_hash = match &agent.api_secret_hash {
        Some(hash) => hash,
        None => {
            tracing::warn!("API key revoked: {}", &api_key[..10.min(api_key.len())]);
            return Ok(None);
        }
    };

    // Verify secret against hash
    match verify_api_secret(api_secret, secret_hash) {
        Ok(true) => {
            tracing::info!("API key authentication successful for agent: {}", agent.id);

            // Update last used timestamp asynchronously (fire and forget)
            let db_clone = db.clone();
            let api_key_clone = api_key.to_string();
            tokio::spawn(async move {
                if let Err(e) = db_clone.update_api_key_last_used(&api_key_clone).await {
                    tracing::error!("Failed to update API key last_used_at: {}", e);
                }
            });

            Ok(Some(agent))
        }
        Ok(false) => {
            tracing::warn!("API key secret verification failed for: {}", &api_key[..10.min(api_key.len())]);
            Ok(None)
        }
        Err(e) => {
            tracing::error!("Bcrypt verification error: {}", e);
            Ok(None)
        }
    }
}

/// Middleware to check for API key authentication
/// This runs BEFORE session authentication in the middleware chain
/// If API key is present and valid, it sets the authentication context
/// If API key is not present, it falls through to session authentication
pub async fn api_key_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Try to extract API key credentials from request headers
    let headers = request.headers().clone();
    if let Some((api_key, api_secret)) = extract_credentials(&headers) {
        // Attempt authentication
        match authenticate_with_api_key(&state.db, &api_key, &api_secret).await? {
            Some(agent) => {
                // Authentication successful - store agent in request extensions
                // The require_auth middleware will use this to build AuthenticatedUser
                request.extensions_mut().insert(agent);
                return Ok(next.run(request).await);
            }
            None => {
                // Authentication failed
                return Err(ApiError::Unauthorized);
            }
        }
    }

    // No API key credentials found - continue to session auth
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_credentials_format_valid() {
        let key = "abcdefghijklmnopqrstuvwxyz012345"; // 32 chars
        let secret = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01"; // 64 chars
        assert!(validate_credentials_format(key, secret));
    }

    #[test]
    fn test_validate_credentials_format_invalid_key_length() {
        let key = "tooshort"; // < 32 chars
        let secret = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01"; // 64 chars
        assert!(!validate_credentials_format(key, secret));
    }

    #[test]
    fn test_validate_credentials_format_invalid_secret_length() {
        let key = "abcdefghijklmnopqrstuvwxyz012345"; // 32 chars
        let secret = "tooshort"; // < 64 chars
        assert!(!validate_credentials_format(key, secret));
    }

    #[test]
    fn test_validate_credentials_format_non_alphanumeric() {
        let key = "abcdefghijklmnopqrstuvwxyz01234!"; // 32 chars but has '!'
        let secret = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01"; // 64 chars
        assert!(!validate_credentials_format(key, secret));
    }

    #[test]
    fn test_extract_credentials_custom_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", "test_key_32_characters_long12".parse().unwrap());
        headers.insert("X-API-Secret", "test_secret_64_characters_long_0123456789_ABCDEFGHIJKLMNOPQRST".parse().unwrap());

        let result = extract_credentials(&headers);
        assert!(result.is_some());
        let (key, secret) = result.unwrap();
        assert_eq!(key, "test_key_32_characters_long12");
        assert_eq!(secret, "test_secret_64_characters_long_0123456789_ABCDEFGHIJKLMNOPQRST");
    }

    #[test]
    fn test_extract_credentials_basic_auth() {
        let mut headers = HeaderMap::new();
        // Base64 encode "mykey:mysecret"
        let credentials = general_purpose::STANDARD.encode("mykey:mysecret");
        headers.insert(AUTHORIZATION, format!("Basic {}", credentials).parse().unwrap());

        let result = extract_credentials(&headers);
        assert!(result.is_some());
        let (key, secret) = result.unwrap();
        assert_eq!(key, "mykey");
        assert_eq!(secret, "mysecret");
    }

    #[test]
    fn test_extract_credentials_no_credentials() {
        let headers = HeaderMap::new();
        let result = extract_credentials(&headers);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_credentials_custom_headers_priority() {
        let mut headers = HeaderMap::new();
        // Add both custom headers and Basic Auth
        headers.insert("X-API-Key", "custom_key".parse().unwrap());
        headers.insert("X-API-Secret", "custom_secret".parse().unwrap());
        let credentials = general_purpose::STANDARD.encode("basic_key:basic_secret");
        headers.insert(AUTHORIZATION, format!("Basic {}", credentials).parse().unwrap());

        let result = extract_credentials(&headers);
        assert!(result.is_some());
        let (key, secret) = result.unwrap();
        // Custom headers should take priority
        assert_eq!(key, "custom_key");
        assert_eq!(secret, "custom_secret");
    }
}
