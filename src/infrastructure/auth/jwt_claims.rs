/// JWT Claims Structure
///
/// This defines the payload structure for JWT tokens used in stateless authentication.
/// All user identity and authorization data is encoded in the token itself.

use serde::{Deserialize, Serialize};

/// JWT Claims - Payload embedded in JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    // Standard JWT claims (RFC 7519)
    /// Subject - User ID
    pub sub: String,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// JWT ID - Unique token identifier (for revocation tracking)
    pub jti: String,

    // Custom claims - User identity
    /// User email
    pub email: String,

    /// User type: "agent" or "contact"
    pub user_type: String,

    // Custom claims - Authorization
    /// Role IDs
    pub roles: Vec<String>,

    /// Flattened permissions from all roles
    pub permissions: Vec<String>,

    // Optional metadata
    /// Agent ID (if user_type is agent)
    pub agent_id: Option<String>,

    /// User's first name
    pub first_name: Option<String>,
}

impl JwtClaims {
    /// Create new JWT claims
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: String,
        email: String,
        user_type: String,
        roles: Vec<String>,
        permissions: Vec<String>,
        agent_id: Option<String>,
        first_name: Option<String>,
        duration_hours: i64,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        Self {
            sub: user_id,
            exp: now + (duration_hours * 3600),
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
            email,
            user_type,
            roles,
            permissions,
            agent_id,
            first_name,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        self.exp < now
    }

    /// Get remaining lifetime in seconds
    pub fn remaining_lifetime(&self) -> i64 {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        (self.exp - now).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims_creation() {
        let claims = JwtClaims::new(
            "user_123".to_string(),
            "test@example.com".to_string(),
            "agent".to_string(),
            vec!["role_admin".to_string()],
            vec!["*".to_string()],
            Some("agent_123".to_string()),
            Some("Test".to_string()),
            24,
        );

        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.user_type, "agent");
        assert_eq!(claims.roles.len(), 1);
        assert_eq!(claims.permissions.len(), 1);
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_jwt_claims_expiration() {
        // Create token that expires in -1 hours (already expired)
        let claims = JwtClaims::new(
            "user_123".to_string(),
            "test@example.com".to_string(),
            "agent".to_string(),
            vec![],
            vec![],
            None,
            None,
            -1, // Already expired
        );

        assert!(claims.is_expired());
        assert_eq!(claims.remaining_lifetime(), 0);
    }

    #[test]
    fn test_jwt_claims_remaining_lifetime() {
        let claims = JwtClaims::new(
            "user_123".to_string(),
            "test@example.com".to_string(),
            "agent".to_string(),
            vec![],
            vec![],
            None,
            None,
            1, // 1 hour
        );

        let remaining = claims.remaining_lifetime();
        assert!(remaining > 3500); // Should be close to 3600 seconds
        assert!(remaining <= 3600);
    }
}
