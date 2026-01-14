/// Password Encryption Utilities (Feature 021)
///
/// Provides AES-256-GCM encryption for sensitive fields like IMAP/SMTP passwords.
/// Uses a master encryption key from environment variable.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use sha2::{Digest, Sha256};

/// Error type for encryption operations
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Missing encryption key: {0}")]
    MissingKey(String),

    #[error("Invalid encrypted data format")]
    InvalidFormat,
}

pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Get encryption key from environment variable
/// The key should be a 32-byte (256-bit) hex string or base64 encoded string
fn get_encryption_key() -> EncryptionResult<[u8; 32]> {
    // Read from environment variable
    let key_string = std::env::var("ENCRYPTION_KEY").map_err(|_| {
        EncryptionError::MissingKey(
            "ENCRYPTION_KEY environment variable not set. \
             Generate one with: openssl rand -hex 32"
                .to_string(),
        )
    })?;

    // Try to decode as hex first
    if let Ok(bytes) = hex::decode(&key_string) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }

    // Try to decode as base64
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&key_string) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }

    // Fallback: derive key from arbitrary string using SHA-256
    let mut hasher = Sha256::new();
    hasher.update(key_string.as_bytes());
    let result = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    Ok(key)
}

/// Encrypt a password using AES-256-GCM
///
/// Returns a base64-encoded string containing: nonce (12 bytes) + ciphertext
pub fn encrypt_password(plaintext: &str) -> EncryptionResult<String> {
    if plaintext.is_empty() {
        return Err(EncryptionError::EncryptionFailed(
            "Cannot encrypt empty password".to_string(),
        ));
    }

    // Get encryption key
    let key_bytes = get_encryption_key()?;

    // Create cipher
    let cipher = Aes256Gcm::new(&key_bytes.into());

    // Generate random nonce (12 bytes for GCM)
    // Generate random nonce (12 bytes for GCM)
    use rand::RngCore;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

    // Combine nonce + ciphertext and encode as base64
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
}

/// Decrypt a password using AES-256-GCM
///
/// Takes a base64-encoded string containing: nonce (12 bytes) + ciphertext
pub fn decrypt_password(encrypted: &str) -> EncryptionResult<String> {
    if encrypted.is_empty() {
        return Err(EncryptionError::DecryptionFailed(
            "Cannot decrypt empty string".to_string(),
        ));
    }

    // Get encryption key
    let key_bytes = get_encryption_key()?;

    // Create cipher
    let cipher = Aes256Gcm::new(&key_bytes.into());

    // Decode from base64
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|_| EncryptionError::InvalidFormat)?;

    // Split into nonce (12 bytes) and ciphertext
    if combined.len() < 12 {
        return Err(EncryptionError::InvalidFormat);
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

    // Convert to string
    String::from_utf8(plaintext_bytes)
        .map_err(|_| EncryptionError::DecryptionFailed("Invalid UTF-8".to_string()))
}

/// Check if encryption is enabled (ENCRYPTION_KEY is set)
pub fn is_encryption_enabled() -> bool {
    std::env::var("ENCRYPTION_KEY").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_key() {
        std::env::set_var("ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        setup_test_key();

        let original = "my-secret-password";
        let encrypted = encrypt_password(original).unwrap();
        let decrypted = decrypt_password(&encrypted).unwrap();

        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_encrypted_values_differ() {
        setup_test_key();

        let password = "same-password";
        let encrypted1 = encrypt_password(password).unwrap();
        let encrypted2 = encrypt_password(password).unwrap();

        // Should be different due to random nonces
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        assert_eq!(decrypt_password(&encrypted1).unwrap(), password);
        assert_eq!(decrypt_password(&encrypted2).unwrap(), password);
    }

    #[test]
    fn test_empty_password_error() {
        setup_test_key();

        let result = encrypt_password("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_encrypted_data() {
        setup_test_key();

        let result = decrypt_password("not-valid-base64!!!");
        assert!(result.is_err());

        let result = decrypt_password("YWJj"); // "abc" in base64 (too short)
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_key_fails_decryption() {
        std::env::set_var("ENCRYPTION_KEY", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        let password = "secret";
        let encrypted = encrypt_password(password).unwrap();

        // Change the key
        std::env::set_var("ENCRYPTION_KEY", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

        let result = decrypt_password(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_encryption_enabled() {
        std::env::remove_var("ENCRYPTION_KEY");
        assert!(!is_encryption_enabled());

        setup_test_key();
        assert!(is_encryption_enabled());
    }
}
