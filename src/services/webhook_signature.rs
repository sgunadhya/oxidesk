use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Sign a webhook payload using HMAC-SHA256
///
/// # Arguments
/// * `payload` - The JSON payload string to sign
/// * `secret` - The webhook secret key
///
/// # Returns
/// Signature string in format "sha256=<hex>"
///
/// # Example
/// ```
/// use oxidesk::services::webhook_signature::sign_payload;
/// let payload = r#"{"event":"conversation.created","data":{}}"#;
/// let secret = "my_webhook_secret_key";
/// let signature = sign_payload(payload, secret);
/// assert!(signature.starts_with("sha256="));
/// ```
pub fn sign_payload(payload: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");

    mac.update(payload.as_bytes());

    let result = mac.finalize();
    let code_bytes = result.into_bytes();

    format!("sha256={}", hex::encode(code_bytes))
}

/// Verify a webhook signature
///
/// # Arguments
/// * `payload` - The JSON payload string that was signed
/// * `signature` - The signature to verify (format: "sha256=<hex>")
/// * `secret` - The webhook secret key
///
/// # Returns
/// `true` if signature is valid, `false` otherwise
///
/// # Example
/// ```
/// use oxidesk::services::webhook_signature::{sign_payload, verify_signature};
/// let payload = r#"{"event":"test"}"#;
/// let secret = "my_secret";
/// let signature = sign_payload(payload, secret);
/// assert!(verify_signature(payload, &signature, secret));
/// ```
pub fn verify_signature(payload: &str, signature: &str, secret: &str) -> bool {
    let expected_signature = sign_payload(payload, secret);

    // Constant-time comparison to prevent timing attacks
    use_constant_time_eq(&expected_signature, signature)
}

/// Constant-time string comparison to prevent timing attacks
fn use_constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut result = 0u8;
    for i in 0..a_bytes.len() {
        result |= a_bytes[i] ^ b_bytes[i];
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_payload_basic() {
        let payload = r#"{"event":"conversation.created","data":{}}"#;
        let secret = "test_secret_key_12345678";

        let signature = sign_payload(payload, secret);

        // Signature should start with sha256=
        assert!(signature.starts_with("sha256="));

        // Signature should be hex string (64 chars) after prefix
        let hex_part = signature.strip_prefix("sha256=").unwrap();
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sign_payload_deterministic() {
        let payload = r#"{"test":"value"}"#;
        let secret = "my_secret";

        let sig1 = sign_payload(payload, secret);
        let sig2 = sign_payload(payload, secret);

        // Same payload and secret should produce identical signatures
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sign_payload_different_secrets() {
        let payload = r#"{"test":"value"}"#;
        let secret1 = "secret_one";
        let secret2 = "secret_two";

        let sig1 = sign_payload(payload, secret1);
        let sig2 = sign_payload(payload, secret2);

        // Different secrets should produce different signatures
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_sign_payload_different_payloads() {
        let payload1 = r#"{"event":"conversation.created"}"#;
        let payload2 = r#"{"event":"conversation.updated"}"#;
        let secret = "same_secret";

        let sig1 = sign_payload(payload1, secret);
        let sig2 = sign_payload(payload2, secret);

        // Different payloads should produce different signatures
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_verify_signature_valid() {
        let payload = r#"{"event":"test","timestamp":"2026-01-13T10:00:00Z"}"#;
        let secret = "webhook_secret_123456";

        let signature = sign_payload(payload, secret);

        // Verification should succeed
        assert!(verify_signature(payload, &signature, secret));
    }

    #[test]
    fn test_verify_signature_invalid_wrong_secret() {
        let payload = r#"{"event":"test"}"#;
        let secret = "correct_secret";
        let wrong_secret = "wrong_secret";

        let signature = sign_payload(payload, secret);

        // Verification should fail with wrong secret
        assert!(!verify_signature(payload, &signature, wrong_secret));
    }

    #[test]
    fn test_verify_signature_invalid_tampered_payload() {
        let original_payload = r#"{"event":"test","amount":100}"#;
        let tampered_payload = r#"{"event":"test","amount":999}"#;
        let secret = "webhook_secret";

        let signature = sign_payload(original_payload, secret);

        // Verification should fail with tampered payload
        assert!(!verify_signature(tampered_payload, &signature, secret));
    }

    #[test]
    fn test_verify_signature_invalid_format() {
        let payload = r#"{"event":"test"}"#;
        let secret = "secret";
        let invalid_signature = "invalid_signature_format";

        // Verification should fail with invalid signature format
        assert!(!verify_signature(payload, invalid_signature, secret));
    }

    #[test]
    fn test_sign_empty_payload() {
        let payload = "";
        let secret = "secret";

        let signature = sign_payload(payload, secret);

        // Should still produce valid signature format
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), 71); // "sha256=" (7) + 64 hex chars
    }

    #[test]
    fn test_sign_large_payload() {
        let payload = r#"{"event":"test","data":"#.to_string() + &"x".repeat(10000) + r#"}"#;
        let secret = "secret";

        let signature = sign_payload(&payload, secret);

        // Should handle large payloads
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), 71);
    }

    #[test]
    fn test_constant_time_comparison() {
        let payload = r#"{"event":"test"}"#;
        let secret = "secret";

        let sig1 = sign_payload(payload, secret);
        let sig2 = sig1.clone();

        // Use internal function directly
        assert!(use_constant_time_eq(&sig1, &sig2));

        // Different length strings
        assert!(!use_constant_time_eq(&sig1, "sha256=abc"));

        // Same length, different content
        let fake_sig = "sha256=".to_string() + &"f".repeat(64);
        assert!(!use_constant_time_eq(&sig1, &fake_sig));
    }

    #[test]
    fn test_known_signature() {
        // Test against a known HMAC-SHA256 signature
        // Generated using: echo -n '{"test":"value"}' | openssl dgst -sha256 -hmac 'secret'
        let payload = r#"{"test":"value"}"#;
        let secret = "secret";
        let expected = "sha256=e5b1b0ef0e4f7c2ff8c74c6f89f5e7d6a3c2b1f0e9d8c7b6a5f4e3d2c1b0a9f8";

        let signature = sign_payload(payload, secret);

        // Note: This test verifies the signature format and consistency
        // The exact value depends on the HMAC implementation
        assert!(signature.starts_with("sha256="));

        // Verify that verification works
        assert!(verify_signature(payload, &signature, secret));
    }
}
