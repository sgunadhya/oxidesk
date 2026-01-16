use crate::api::middleware::error::{ApiError, ApiResult};
use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::user_repository::UserRepository;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, ParamsBuilder,
};

/// Validates password complexity requirements (FR-005, FR-006)
/// - 10-72 characters long
/// - Contains uppercase letter
/// - Contains lowercase letter
/// - Contains digit
/// - Contains special character
pub fn validate_password_complexity(password: &str) -> ApiResult<()> {
    let len = password.len();
    if len < 10 || len > 72 {
        return Err(ApiError::BadRequest(
            "Password must be 10-72 characters long".to_string(),
        ));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password
        .chars()
        .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

    if !has_uppercase {
        return Err(ApiError::BadRequest(
            "Password must contain at least one uppercase letter".to_string(),
        ));
    }

    if !has_lowercase {
        return Err(ApiError::BadRequest(
            "Password must contain at least one lowercase letter".to_string(),
        ));
    }

    if !has_digit {
        return Err(ApiError::BadRequest(
            "Password must contain at least one digit".to_string(),
        ));
    }

    if !has_special {
        return Err(ApiError::BadRequest(
            "Password must contain at least one special character (!@#$%^&*()_+-=[]{}|;:,.<>?)"
                .to_string(),
        ));
    }

    Ok(())
}

/// Hash password using Argon2id with parameters:
/// - m_cost = 19456 KiB (19 MiB)
/// - t_cost = 2 iterations
/// - p_cost = 1 thread
pub fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);

    // Configure Argon2id with recommended parameters
    let params = ParamsBuilder::new()
        .m_cost(19456) // 19 MiB
        .t_cost(2) // 2 iterations
        .p_cost(1) // 1 thread
        .build()
        .map_err(|_| ApiError::Internal("Failed to build Argon2 params".to_string()))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?;

    Ok(hash.to_string())
}

/// Verify password against Argon2id hash
pub fn verify_password(password: &str, hash: &str) -> ApiResult<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|_| ApiError::Internal("Invalid password hash format".to_string()))?;

    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate secure random token for sessions (32 bytes = 64 hex characters)
pub fn generate_session_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

/// Result of a successful authentication
pub struct AuthResult {
    pub session: crate::models::Session,
    pub user: crate::models::User,
    pub agent: crate::models::Agent,
    pub roles: Vec<crate::models::Role>,
}

/// Authenticate agent with email and password
/// Performs the full login flow:
/// 1. Normalize/Validate email
/// 2. Find user by email (must be Agent type)
/// 3. Find agent profile
/// 4. Verify password
/// 5. Verify at least one role exists
/// 6. Create session
pub async fn authenticate(
    db: &crate::database::Database,
    session_service: &crate::services::SessionService,
    email: &str,
    password: &str,
    session_duration_hours: i64,
) -> ApiResult<AuthResult> {
    use crate::models::{Session, UserType};
    use crate::services::validate_and_normalize_email;

    // 1. Validate and normalize email
    let email = validate_and_normalize_email(email)?;

    // 2. Get user by email
    let user = db
        .get_user_by_email_and_type(&email, &UserType::Agent)
        .await?
        .ok_or_else(|| {
            // Use generic error for security (timing attacks notwithstanding)
            ApiError::Unauthorized
        })?;

    // 3. Get agent
    let agent = db
        .get_agent_by_user_id(&user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    // 4. Verify password
    let password_valid = verify_password(password, &agent.password_hash)?;

    if !password_valid {
        return Err(ApiError::Unauthorized);
    }

    // 5. Get user roles
    let roles = db.get_user_roles(&user.id).await?;

    if roles.is_empty() {
        return Err(ApiError::Internal("User has no roles assigned".to_string()));
    }

    // 6. Generate session token
    let token = generate_session_token();

    // 7. Create session
    let session = Session::new(user.id.clone(), token, session_duration_hours);
    session_service.create_session(&session).await?;

    Ok(AuthResult {
        session,
        user,
        agent,
        roles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_too_short() {
        let result = validate_password_complexity("Short1!");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_too_long() {
        let long_password = "a".repeat(73) + "A1!";
        let result = validate_password_complexity(&long_password);
        assert!(result.is_err());
    }

    #[test]
    fn test_password_no_uppercase() {
        let result = validate_password_complexity("lowercase123!");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_no_lowercase() {
        let result = validate_password_complexity("UPPERCASE123!");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_no_digit() {
        let result = validate_password_complexity("Lowercase!");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_no_special() {
        let result = validate_password_complexity("Lowercase123");
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_password() {
        let result = validate_password_complexity("SecureP@ssw0rd");
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_and_verify_password() {
        let password = "SecureP@ssw0rd123";
        let hash = hash_password(password).unwrap();

        // Should verify with correct password
        let verify_result = verify_password(password, &hash).unwrap();
        assert!(verify_result);

        // Should not verify with incorrect password
        let verify_wrong = verify_password("WrongPassword1!", &hash).unwrap();
        assert!(!verify_wrong);
    }

    #[test]
    fn test_session_token_generation() {
        let token1 = generate_session_token();
        let token2 = generate_session_token();

        // Should be 64 hex characters
        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        // Should be different
        assert_ne!(token1, token2);

        // Should be valid hex
        assert!(token1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
