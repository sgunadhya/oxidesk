use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use rand::Rng;

/// Generate a new CSRF token (32 random bytes = 64 hex characters)
pub fn generate_csrf_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

/// Validate CSRF token using double-submit cookie pattern
///
/// Compares the token from the request header with the token from the session.
/// Both must be present and match exactly.
pub fn validate_csrf_token(header_token: Option<&str>, session_token: &str) -> Result<(), String> {
    match header_token {
        None => Err("Missing CSRF token in request header".to_string()),
        Some(token) if token.is_empty() => Err("Empty CSRF token in request header".to_string()),
        Some(token) if token != session_token => Err("CSRF token mismatch".to_string()),
        Some(_) => Ok(()),
    }
}

/// CSRF middleware configuration
#[derive(Clone)]
pub struct CsrfConfig {
    /// Header name to check for CSRF token (default: "X-CSRF-Token")
    pub header_name: String,
    /// Whether to require CSRF validation for GET requests (default: false)
    pub validate_get: bool,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            header_name: "X-CSRF-Token".to_string(),
            validate_get: false,
        }
    }
}

/// Extract CSRF token from request headers
pub fn extract_csrf_from_headers(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers
        .get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Check if HTTP method requires CSRF validation
pub fn method_requires_csrf(method: &str, validate_get: bool) -> bool {
    match method {
        "GET" | "HEAD" | "OPTIONS" => validate_get,
        "POST" | "PUT" | "PATCH" | "DELETE" => true,
        _ => false,
    }
}

/// CSRF validation middleware for Axum
///
/// This middleware checks CSRF tokens for state-changing requests.
/// It expects the authenticated user extension to be present (from auth middleware).
pub async fn csrf_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    use crate::infrastructure::http::middleware::AuthenticatedUser;

    let method = request.method().as_str();
    let config = CsrfConfig::default();

    // Skip CSRF validation for safe methods (unless configured otherwise)
    if !method_requires_csrf(method, config.validate_get) {
        return Ok(next.run(request).await);
    }

    // Extract authenticated user from extensions (set by auth middleware)
    let auth_user = request.extensions().get::<AuthenticatedUser>().cloned();

    match auth_user {
        Some(user) => {
            // Extract CSRF token from request header
            let header_token = extract_csrf_from_headers(&headers, &config.header_name);

            // Validate against session's CSRF token
            validate_csrf_token(header_token.as_deref(), &user.session.csrf_token).map_err(
                |err| {
                    tracing::warn!("CSRF validation failed: {}", err);
                    StatusCode::FORBIDDEN
                },
            )?;

            Ok(next.run(request).await)
        }
        None => {
            // No authenticated user - skip CSRF check
            // (unauthenticated endpoints don't need CSRF protection)
            Ok(next.run(request).await)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_csrf_token() {
        let token = generate_csrf_token();

        // Should be 64 hex characters (32 bytes)
        assert_eq!(token.len(), 64);

        // Should be valid hex
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));

        // Should be unique
        let token2 = generate_csrf_token();
        assert_ne!(token, token2);
    }

    #[test]
    fn test_validate_csrf_token_success() {
        let token = "abc123def456";
        let result = validate_csrf_token(Some(token), token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_csrf_token_missing() {
        let result = validate_csrf_token(None, "abc123");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing CSRF token in request header");
    }

    #[test]
    fn test_validate_csrf_token_empty() {
        let result = validate_csrf_token(Some(""), "abc123");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty CSRF token in request header");
    }

    #[test]
    fn test_validate_csrf_token_mismatch() {
        let result = validate_csrf_token(Some("wrong"), "correct");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "CSRF token mismatch");
    }

    #[test]
    fn test_method_requires_csrf() {
        let config = CsrfConfig::default();

        // Safe methods should not require CSRF by default
        assert!(!method_requires_csrf("GET", config.validate_get));
        assert!(!method_requires_csrf("HEAD", config.validate_get));
        assert!(!method_requires_csrf("OPTIONS", config.validate_get));

        // State-changing methods should require CSRF
        assert!(method_requires_csrf("POST", config.validate_get));
        assert!(method_requires_csrf("PUT", config.validate_get));
        assert!(method_requires_csrf("PATCH", config.validate_get));
        assert!(method_requires_csrf("DELETE", config.validate_get));
    }

    #[test]
    fn test_method_requires_csrf_with_get_validation() {
        // With validate_get = true, GET should require CSRF
        assert!(method_requires_csrf("GET", true));
    }
}
