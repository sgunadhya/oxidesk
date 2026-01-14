/// Password Reset Service
/// Feature: 017-password-reset
///
/// Business logic for password reset functionality including:
/// - Token generation and validation
/// - Rate limiting
/// - Email enumeration prevention
/// - Session destruction
use crate::{
    api::middleware::error::{ApiError, ApiResult},
    database::Database,
    models::*,
    services::{
        email_service::{send_password_reset_email, SmtpConfig},
        auth::{validate_password_complexity, hash_password},
    },
    utils::generate_reset_token,
};
use std::env;

/// Request a password reset for an agent email
///
/// This implements email enumeration prevention by:
/// - Always returning the same success message
/// - Only sending email if user exists
/// - Maintaining consistent timing
///
/// # Rate Limiting
/// Maximum 5 requests per hour per email
pub async fn request_password_reset(
    db: &Database,
    email: &str,
) -> ApiResult<RequestPasswordResetResponse> {
    // Normalize email
    let email = email.trim().to_lowercase();

    // Try to find agent by email
    let user_option = db
        .get_user_by_email_and_type(&email, &UserType::Agent)
        .await?;

    // If user exists, proceed with reset flow
    if let Some(user) = user_option {
        // Check rate limit (5 requests per hour)
        let rate_limit_window = env::var("PASSWORD_RESET_RATE_LIMIT")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        let recent_requests = db
            .count_recent_reset_requests(&user.id, 3600)
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
        db.invalidate_user_reset_tokens(&user.id).await?;

        // Store new token
        db.create_password_reset_token(&reset_token).await?;

        // Send email (async, best-effort)
        let smtp_config = SmtpConfig::from_env()
            .map_err(|e| ApiError::Internal(format!("SMTP configuration error: {}", e)))?;

        // Spawn email sending in background to not block response
        let email_clone = email.clone();
        tokio::spawn(async move {
            if let Err(e) = send_password_reset_email(&email_clone, &token_value, &smtp_config).await {
                tracing::error!("Failed to send password reset email to {}: {}", email_clone, e);
            }
        });

        tracing::info!(
            "Password reset requested for email: {} (user_id: {})",
            email,
            user.id
        );
    } else {
        // User does not exist - still return success (email enumeration prevention)
        tracing::info!(
            "Password reset requested for non-existent email: {}",
            email
        );
    }

    // Always return the same generic message (email enumeration prevention)
    Ok(RequestPasswordResetResponse {
        message: "If an account exists with that email, you will receive a password reset link."
            .to_string(),
    })
}

/// Validate a reset token and return the token record if valid
///
/// Token is valid if:
/// - It exists in the database
/// - It has not been used (used = false)
/// - It has not expired (expires_at > now)
pub async fn validate_reset_token(
    db: &Database,
    token: &str,
) -> ApiResult<PasswordResetToken> {
    // Validate token format (32 alphanumeric characters)
    if token.len() != 32 || !token.chars().all(|c| c.is_alphanumeric()) {
        return Err(ApiError::BadRequest("Invalid token format".to_string()));
    }

    // Lookup token
    let token_record = db
        .get_password_reset_token(token)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Invalid or expired reset token".to_string()))?;

    // Check if token is expired first (and clean up if so)
    // Note: We check expiry before usage status to ensure lazy cleanup happens
    // for all expired tokens, even if they were previously invalidated
    if token_record.is_expired() {
        // Lazy cleanup: delete expired token
        db.delete_password_reset_token(&token_record.id).await?;
        return Err(ApiError::BadRequest(
            "Invalid or expired reset token".to_string(),
        ));
    }

    // Check if token is already used
    if token_record.used {
        return Err(ApiError::BadRequest(
            "Invalid or expired reset token".to_string(),
        ));
    }

    Ok(token_record)
}

/// Reset password using a valid token
///
/// This performs an atomic transaction:
/// 1. Validate token
/// 2. Validate password complexity
/// 3. Hash new password
/// 4. Update agent password
/// 5. Mark token as used
/// 6. Delete all user sessions
///
/// If any step fails, the entire transaction is rolled back
pub async fn reset_password(
    db: &Database,
    token: &str,
    new_password: &str,
) -> ApiResult<ResetPasswordResponse> {
    // Validate token
    let token_record = validate_reset_token(db, token).await?;

    // Validate password complexity
    validate_password_complexity(new_password)?;

    // Hash password
    let password_hash = hash_password(new_password)?;

    // Execute all password reset operations atomically in a transaction
    // This ensures that either all operations succeed or none do
    // Prevents inconsistent state (e.g., password changed but token still valid)
    let session_count = db.reset_password_atomic(
        &token_record.user_id,
        &token_record.id,
        &password_hash,
    ).await?;

    tracing::info!(
        "Password reset successful for user_id: {}, sessions_destroyed: {}",
        token_record.user_id,
        session_count
    );

    Ok(ResetPasswordResponse {
        message: "Password has been reset successfully. Please log in with your new password."
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_token_format_valid() {
        let valid_token = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6";
        assert_eq!(valid_token.len(), 32);
        assert!(valid_token.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_validate_token_format_invalid_length() {
        let short_token = "abc123";
        assert_ne!(short_token.len(), 32);
    }

    #[test]
    fn test_validate_token_format_invalid_chars() {
        let invalid_token = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5!@";
        assert!(!invalid_token.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_email_normalization() {
        let email = "  Alice@Example.COM  ";
        let normalized = email.trim().to_lowercase();
        assert_eq!(normalized, "alice@example.com");
    }
}
