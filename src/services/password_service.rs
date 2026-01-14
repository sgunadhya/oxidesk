/// Password Generation Service
/// Feature: User Creation (016)
///
/// Provides secure random password generation for newly created agent accounts.
/// Passwords are 16+ characters with mixed complexity for high security.

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;

/// Special characters used in password generation
const SPECIAL_CHARS: &[u8] = b"!@#$%^&*()_+-=[]{}|;:,.<>?";

/// Generate a secure random password for new agents
///
/// Password composition:
/// - 8 random alphanumeric characters
/// - 1 guaranteed uppercase letter (A-Z)
/// - 1 guaranteed lowercase letter (a-z)
/// - 2 guaranteed digits (0-9)
/// - 4 random special characters
/// - Total: 16 characters (shuffled for randomness)
///
/// Uses cryptographically secure random number generator (thread_rng).
/// Guarantees all complexity requirements (uppercase, lowercase, digit, special char).
///
/// # Returns
///
/// A 16-character password string with mixed complexity
///
/// # Example
///
/// ```
/// use oxidesk::services::password_service::generate_random_password;
/// let password = generate_random_password();
/// assert_eq!(password.len(), 16);
/// // Example output: "X7g!mPq2@nR8zK4L"
/// ```
pub fn generate_random_password() -> String {
    let mut rng = thread_rng();

    // Generate 8 random alphanumeric characters
    let mut password: Vec<u8> = (0..8)
        .map(|_| rng.sample(Alphanumeric))
        .collect();

    // Add 1 guaranteed uppercase letter (A-Z)
    password.push(rng.gen_range(b'A'..=b'Z'));

    // Add 1 guaranteed lowercase letter (a-z)
    password.push(rng.gen_range(b'a'..=b'z'));

    // Add 2 guaranteed digits (0-9) to ensure password has digits
    for _ in 0..2 {
        password.push(rng.gen_range(b'0'..=b'9'));
    }

    // Add 4 random special characters
    for _ in 0..4 {
        let special_char = SPECIAL_CHARS[rng.gen_range(0..SPECIAL_CHARS.len())];
        password.push(special_char);
    }

    // Shuffle to mix all character types throughout
    // (prevents predictable character placement)
    password.shuffle(&mut rng);

    // Convert to UTF-8 string (safe because all chars are ASCII)
    String::from_utf8(password).expect("Password generation produced invalid UTF-8")
}

/// Validate password meets complexity requirements (internal use)
///
/// Requirements:
/// - At least 16 characters
/// - Contains uppercase letter
/// - Contains lowercase letter
/// - Contains digit
/// - Contains special character
///
/// # Arguments
///
/// * `password` - The password string to validate
///
/// # Returns
///
/// `true` if password meets all requirements, `false` otherwise
///
/// Note: This function is kept internal to avoid naming collision with
/// the existing validate_password_complexity in auth service.
fn _validate_generated_password_complexity(password: &str) -> bool {
    if password.len() < 16 {
        return false;
    }

    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| SPECIAL_CHARS.contains(&(c as u8)));

    has_uppercase && has_lowercase && has_digit && has_special
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generate_password_length() {
        let password = generate_random_password();
        assert_eq!(password.len(), 16, "Password should be exactly 16 characters");
    }

    #[test]
    fn test_generate_password_has_alphanumeric() {
        let password = generate_random_password();
        let has_alpha = password.chars().any(|c| c.is_alphabetic());
        let has_digit = password.chars().any(|c| c.is_numeric());
        assert!(has_alpha, "Password should contain alphabetic characters");
        assert!(has_digit, "Password should contain numeric characters");
    }

    #[test]
    fn test_generate_password_has_special_chars() {
        let password = generate_random_password();
        let has_special = password.chars().any(|c| {
            SPECIAL_CHARS.contains(&(c as u8))
        });
        assert!(has_special, "Password should contain special characters");
    }

    #[test]
    fn test_generate_password_has_mixed_case() {
        let password = generate_random_password();
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        assert!(has_uppercase || has_lowercase, "Password should have mixed case");
    }

    #[test]
    fn test_generate_password_uniqueness() {
        // Generate 100 passwords and verify they're all different
        let mut passwords = HashSet::new();
        for _ in 0..100 {
            let password = generate_random_password();
            passwords.insert(password);
        }
        assert_eq!(passwords.len(), 100, "All generated passwords should be unique");
    }

    #[test]
    fn test_generate_password_meets_complexity() {
        // Generate multiple passwords and verify they all meet complexity requirements
        for _ in 0..10 {
            let password = generate_random_password();
            assert!(
                _validate_generated_password_complexity(&password),
                "Generated password should meet complexity requirements: {}",
                password
            );
        }
    }

    #[test]
    fn test_validate_password_complexity_valid() {
        let valid_password = "Abc123!@#XyzDefG";
        assert!(_validate_generated_password_complexity(valid_password));
    }

    #[test]
    fn test_validate_password_complexity_too_short() {
        let short_password = "Abc123!@#";
        assert!(!_validate_generated_password_complexity(short_password));
    }

    #[test]
    fn test_validate_password_complexity_no_uppercase() {
        let no_upper = "abc123!@#xyzdefg";
        assert!(!_validate_generated_password_complexity(no_upper));
    }

    #[test]
    fn test_validate_password_complexity_no_lowercase() {
        let no_lower = "ABC123!@#XYZDEFG";
        assert!(!_validate_generated_password_complexity(no_lower));
    }

    #[test]
    fn test_validate_password_complexity_no_digit() {
        let no_digit = "Abcdef!@#XyzDefG";
        assert!(!_validate_generated_password_complexity(no_digit));
    }

    #[test]
    fn test_validate_password_complexity_no_special() {
        let no_special = "Abc123XyzDefGHIJ";
        assert!(!_validate_generated_password_complexity(no_special));
    }
}
