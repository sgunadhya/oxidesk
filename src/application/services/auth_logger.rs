use crate::infrastructure::persistence::auth_event::AuthEventRepository;
use crate::{
    infrastructure::persistence::Database,
    domain::entities::{AuthEvent, AuthEventType, AuthMethod},
};

/// Authentication logger service
///
/// Provides centralized logging for all authentication events including
/// successful logins, failures, logouts, session expirations, and rate limiting.
pub struct AuthLogger;

impl AuthLogger {
    /// Log a successful login event
    pub async fn log_login_success(
        db: &Database,
        user_id: String,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: Option<String>,
    ) -> Result<(), crate::infrastructure::http::middleware::ApiError> {
        let event = AuthEvent::new(
            AuthEventType::LoginSuccess,
            Some(user_id),
            email,
            auth_method,
            provider_name,
            ip_address,
            user_agent,
            None, // no error reason
        );

        db.create_auth_event(&event).await?;
        tracing::info!(
            event_type = "login_success",
            user_id = event.user_id.as_deref().unwrap_or("unknown"),
            email = %event.email,
            auth_method = ?event.auth_method,
            "Authentication successful"
        );

        Ok(())
    }

    /// Log a failed login attempt
    pub async fn log_login_failure(
        db: &Database,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: Option<String>,
        error_reason: String,
    ) -> Result<(), crate::infrastructure::http::middleware::ApiError> {
        let event = AuthEvent::new(
            AuthEventType::LoginFailure,
            None, // no user_id for failed attempts
            email,
            auth_method,
            provider_name,
            ip_address,
            user_agent,
            Some(error_reason.clone()),
        );

        db.create_auth_event(&event).await?;
        tracing::warn!(
            event_type = "login_failure",
            email = %event.email,
            auth_method = ?event.auth_method,
            error_reason = %error_reason,
            "Authentication failed"
        );

        Ok(())
    }

    /// Log a logout event
    pub async fn log_logout(
        db: &Database,
        user_id: String,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: Option<String>,
    ) -> Result<(), crate::infrastructure::http::middleware::ApiError> {
        let event = AuthEvent::new(
            AuthEventType::Logout,
            Some(user_id),
            email,
            auth_method,
            provider_name,
            ip_address,
            user_agent,
            None,
        );

        db.create_auth_event(&event).await?;
        tracing::info!(
            event_type = "logout",
            user_id = event.user_id.as_deref().unwrap_or("unknown"),
            email = %event.email,
            "User logged out"
        );

        Ok(())
    }

    /// Log a session expiration event
    pub async fn log_session_expired(
        db: &Database,
        user_id: String,
        email: String,
        auth_method: AuthMethod,
        provider_name: Option<String>,
        ip_address: String,
        user_agent: Option<String>,
    ) -> Result<(), crate::infrastructure::http::middleware::ApiError> {
        let event = AuthEvent::new(
            AuthEventType::SessionExpired,
            Some(user_id),
            email,
            auth_method,
            provider_name,
            ip_address,
            user_agent,
            None,
        );

        db.create_auth_event(&event).await?;
        tracing::info!(
            event_type = "session_expired",
            user_id = event.user_id.as_deref().unwrap_or("unknown"),
            email = %event.email,
            "Session expired"
        );

        Ok(())
    }

    /// Log a rate limit exceeded event
    pub async fn log_rate_limit_exceeded(
        db: &Database,
        email: String,
        auth_method: AuthMethod,
        ip_address: String,
        user_agent: Option<String>,
        retry_after_seconds: u64,
    ) -> Result<(), crate::infrastructure::http::middleware::ApiError> {
        let error_reason = format!(
            "Rate limit exceeded. Retry after {} seconds",
            retry_after_seconds
        );

        let event = AuthEvent::new(
            AuthEventType::RateLimitExceeded,
            None, // no user_id
            email,
            auth_method,
            None, // no provider for rate limiting
            ip_address,
            user_agent,
            Some(error_reason.clone()),
        );

        db.create_auth_event(&event).await?;
        tracing::warn!(
            event_type = "rate_limit_exceeded",
            email = %event.email,
            retry_after_seconds = %retry_after_seconds,
            "Rate limit exceeded"
        );

        Ok(())
    }

    /// Get authentication events for a specific user
    pub async fn get_user_events(
        db: &Database,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuthEvent>, crate::infrastructure::http::middleware::ApiError> {
        db.get_auth_events_by_user(user_id, limit, offset).await
    }

    /// Get recent authentication events (admin view)
    pub async fn get_recent_events(
        db: &Database,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuthEvent>, crate::infrastructure::http::middleware::ApiError> {
        db.get_recent_auth_events(limit, offset).await
    }

    /// Log an authorization denial event (RBAC System)
    /// Called when user lacks required permission for an action
    pub async fn log_authorization_denied(
        db: &Database,
        user_id: String,
        email: String,
        auth_method: AuthMethod,
        ip_address: String,
        user_agent: Option<String>,
        required_permission: String,
        resource_id: Option<String>,
    ) -> Result<(), crate::infrastructure::http::middleware::ApiError> {
        let error_reason = if let Some(res_id) = resource_id {
            format!(
                "Required permission '{}' for resource '{}'",
                required_permission, res_id
            )
        } else {
            format!("Required permission '{}'", required_permission)
        };

        let event = AuthEvent::new(
            AuthEventType::AuthorizationDenied,
            Some(user_id.clone()),
            email.clone(),
            auth_method,
            None, // no provider for authorization
            ip_address,
            user_agent,
            Some(error_reason.clone()),
        );

        db.create_auth_event(&event).await?;
        tracing::warn!(
            event_type = "authorization_denied",
            user_id = %user_id,
            email = %email,
            required_permission = %required_permission,
            error_reason = %error_reason,
            "Authorization denied"
        );

        Ok(())
    }
}

/// Helper function to extract IP address from request
///
/// Checks X-Forwarded-For, X-Real-IP headers, or falls back to connection info.
pub fn extract_ip_address(headers: &axum::http::HeaderMap) -> String {
    // Check X-Forwarded-For (proxy/load balancer)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            // Take the first IP in the chain
            if let Some(ip) = value.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP (nginx)
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            return value.to_string();
        }
    }

    // Fallback to unknown
    "unknown".to_string()
}

/// Helper function to extract User-Agent from request
pub fn extract_user_agent(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ip_from_forwarded_for() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.1, 10.0.0.1".parse().unwrap());

        let ip = extract_ip_address(&headers);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_ip_from_real_ip() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "192.168.1.100".parse().unwrap());

        let ip = extract_ip_address(&headers);
        assert_eq!(ip, "192.168.1.100");
    }

    #[test]
    fn test_extract_ip_fallback() {
        let headers = axum::http::HeaderMap::new();
        let ip = extract_ip_address(&headers);
        assert_eq!(ip, "unknown");
    }

    #[test]
    fn test_extract_user_agent() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("user-agent", "Mozilla/5.0 (Test)".parse().unwrap());

        let ua = extract_user_agent(&headers);
        assert_eq!(ua, Some("Mozilla/5.0 (Test)".to_string()));
    }

    #[test]
    fn test_extract_user_agent_missing() {
        let headers = axum::http::HeaderMap::new();
        let ua = extract_user_agent(&headers);
        assert_eq!(ua, None);
    }
}
