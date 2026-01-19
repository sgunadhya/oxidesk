use oxidesk::shared::utils::email_validator::validate_and_normalize_email;

#[test]
fn test_valid_email_formats() {
    assert!(validate_and_normalize_email("user@example.com").is_ok());
    assert!(validate_and_normalize_email("user.name@example.com").is_ok());
    assert!(validate_and_normalize_email("user+tag@example.com").is_ok());
    assert!(validate_and_normalize_email("user@subdomain.example.com").is_ok());
}

#[test]
fn test_email_normalization_lowercase() {
    let result = validate_and_normalize_email("User@Example.COM").unwrap();
    assert_eq!(result, "user@example.com");
}

#[test]
fn test_email_normalization_whitespace() {
    let result = validate_and_normalize_email("  user@example.com  ").unwrap();
    assert_eq!(result, "user@example.com");
}

#[test]
fn test_invalid_email_no_at_symbol() {
    assert!(validate_and_normalize_email("userexample.com").is_err());
}

#[test]
fn test_invalid_email_no_domain() {
    assert!(validate_and_normalize_email("user@").is_err());
}

#[test]
fn test_invalid_email_no_local_part() {
    assert!(validate_and_normalize_email("@example.com").is_err());
}

#[test]
fn test_invalid_email_no_tld() {
    assert!(validate_and_normalize_email("user@example").is_err());
}

#[test]
fn test_invalid_email_multiple_at_symbols() {
    assert!(validate_and_normalize_email("user@@example.com").is_err());
}

#[test]
fn test_admin_email_valid() {
    // From .env.example
    assert!(validate_and_normalize_email("admin@example.com").is_ok());
}
