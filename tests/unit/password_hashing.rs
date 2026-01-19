use oxidesk::application::services::auth::{hash_password, verify_password};

#[test]
fn test_hash_password_produces_argon2id_format() {
    let password = "SecureP@ssw0rd123";
    let hash = hash_password(password).unwrap();

    // Argon2id hash should start with $argon2id$
    assert!(hash.starts_with("$argon2id$"));

    // Should contain version and parameters
    assert!(hash.contains("v=19"));
    assert!(hash.contains("m=19456"));
    assert!(hash.contains("t=2"));
    assert!(hash.contains("p=1"));
}

#[test]
fn test_hash_password_generates_unique_hashes() {
    let password = "SecureP@ssw0rd123";
    let hash1 = hash_password(password).unwrap();
    let hash2 = hash_password(password).unwrap();

    // Same password should produce different hashes (due to salt)
    assert_ne!(hash1, hash2);
}

#[test]
fn test_verify_password_correct() {
    let password = "SecureP@ssw0rd123";
    let hash = hash_password(password).unwrap();

    // Correct password should verify
    assert!(verify_password(password, &hash).unwrap());
}

#[test]
fn test_verify_password_incorrect() {
    let password = "SecureP@ssw0rd123";
    let hash = hash_password(password).unwrap();

    // Incorrect password should not verify
    assert!(!verify_password("WrongPassword1!", &hash).unwrap());
}

#[test]
fn test_verify_password_case_sensitive() {
    let password = "SecureP@ssw0rd123";
    let hash = hash_password(password).unwrap();

    // Different case should not verify
    assert!(!verify_password("securep@ssw0rd123", &hash).unwrap());
}

#[test]
fn test_hash_password_performance() {
    use std::time::Instant;

    let password = "SecureP@ssw0rd123";
    let start = Instant::now();
    let _ = hash_password(password).unwrap();
    let duration = start.elapsed();

    // Should complete within reasonable time (< 500ms for tests)
    assert!(duration.as_millis() < 500, "Hashing took {:?}", duration);
}
