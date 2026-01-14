use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

/// Generate a cryptographically random 32-character alphanumeric API key
pub fn generate_api_key() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Generate a cryptographically random 64-character alphanumeric API secret
pub fn generate_api_secret() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

/// Hash an API secret using bcrypt with cost factor 12
pub fn hash_api_secret(secret: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(secret, 12)
}

/// Verify an API secret against its bcrypt hash
pub fn verify_api_secret(secret: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(secret, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_length() {
        let key = generate_api_key();
        assert_eq!(key.len(), 32, "API key should be exactly 32 characters");
    }

    #[test]
    fn test_generate_api_key_alphanumeric() {
        let key = generate_api_key();
        assert!(
            key.chars().all(|c| c.is_alphanumeric()),
            "API key should contain only alphanumeric characters"
        );
    }

    #[test]
    fn test_generate_api_key_unique() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        assert_ne!(key1, key2, "Generated API keys should be unique");
    }

    #[test]
    fn test_generate_api_secret_length() {
        let secret = generate_api_secret();
        assert_eq!(secret.len(), 64, "API secret should be exactly 64 characters");
    }

    #[test]
    fn test_generate_api_secret_alphanumeric() {
        let secret = generate_api_secret();
        assert!(
            secret.chars().all(|c| c.is_alphanumeric()),
            "API secret should contain only alphanumeric characters"
        );
    }

    #[test]
    fn test_generate_api_secret_unique() {
        let secret1 = generate_api_secret();
        let secret2 = generate_api_secret();
        assert_ne!(secret1, secret2, "Generated API secrets should be unique");
    }

    #[test]
    fn test_hash_api_secret() {
        let secret = "test_secret_1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNO";
        let hash = hash_api_secret(secret).expect("Hashing should succeed");

        assert!(hash.starts_with("$2"), "Hash should be bcrypt format");
        assert!(hash.len() > 50, "Hash should be substantial length");
    }

    #[test]
    fn test_verify_api_secret_valid() {
        let secret = "test_secret_1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNO";
        let hash = hash_api_secret(secret).expect("Hashing should succeed");

        let result = verify_api_secret(secret, &hash).expect("Verification should not error");
        assert!(result, "Valid secret should verify successfully");
    }

    #[test]
    fn test_verify_api_secret_invalid() {
        let secret = "test_secret_1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNO";
        let wrong_secret = "wrong_secret_0987654321zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONML";
        let hash = hash_api_secret(secret).expect("Hashing should succeed");

        let result = verify_api_secret(wrong_secret, &hash).expect("Verification should not error");
        assert!(!result, "Invalid secret should fail verification");
    }
}
