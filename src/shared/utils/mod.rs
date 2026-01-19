/// Utility modules
pub mod email_validator;
pub mod encryption;

/// Utility functions for password reset feature
use rand::{distributions::Alphanumeric, Rng};

/// Generate a cryptographically secure reset token
///
/// Generates a 32-character alphanumeric token using thread_rng() which is
/// cryptographically secure (uses OS random number generator).
///
/// Token format: [a-zA-Z0-9]{32}
/// Entropy: 190 bits (62^32 possibilities)
///
/// # Examples
///
/// ```
/// use oxidesk::utils::generate_reset_token;
/// let token = generate_reset_token();
/// assert_eq!(token.len(), 32);
/// assert!(token.chars().all(|c| c.is_alphanumeric()));
/// ```
pub fn generate_reset_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reset_token_length() {
        let token = generate_reset_token();
        assert_eq!(token.len(), 32, "Token should be exactly 32 characters");
    }

    #[test]
    fn test_generate_reset_token_alphanumeric() {
        let token = generate_reset_token();
        assert!(
            token.chars().all(|c| c.is_alphanumeric()),
            "Token should only contain alphanumeric characters"
        );
    }

    #[test]
    fn test_generate_reset_token_uniqueness() {
        let token1 = generate_reset_token();
        let token2 = generate_reset_token();
        assert_ne!(
            token1, token2,
            "Consecutive tokens should be different (extremely high probability)"
        );
    }

    #[test]
    fn test_generate_reset_token_no_special_chars() {
        let token = generate_reset_token();
        assert!(
            !token.contains(|c: char| !c.is_alphanumeric()),
            "Token should not contain special characters"
        );
    }
}
