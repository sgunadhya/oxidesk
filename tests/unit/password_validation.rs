use oxidesk::services::validate_password_complexity;

#[test]
fn test_password_minimum_length() {
    // Less than 10 characters should fail
    assert!(validate_password_complexity("Short1!").is_err());

    // Exactly 10 characters should pass
    assert!(validate_password_complexity("Valid1234!").is_ok());
}

#[test]
fn test_password_maximum_length() {
    // 73 characters should fail
    let long_pass = "A".repeat(70) + "bc1!";
    assert!(validate_password_complexity(&long_pass).is_err());

    // 72 characters should pass
    let valid_pass = "A".repeat(68) + "bc1!";
    assert!(validate_password_complexity(&valid_pass).is_ok());
}

#[test]
fn test_password_requires_uppercase() {
    assert!(validate_password_complexity("lowercase123!").is_err());
    assert!(validate_password_complexity("Mixedcase123!").is_ok());
}

#[test]
fn test_password_requires_lowercase() {
    assert!(validate_password_complexity("UPPERCASE123!").is_err());
    assert!(validate_password_complexity("Mixedcase123!").is_ok());
}

#[test]
fn test_password_requires_digit() {
    assert!(validate_password_complexity("NoDigits!!!").is_err());
    assert!(validate_password_complexity("HasDigit1!").is_ok());
}

#[test]
fn test_password_requires_special_character() {
    assert!(validate_password_complexity("NoSpecial123").is_err());
    assert!(validate_password_complexity("HasSpecial1!").is_ok());
}

#[test]
fn test_valid_admin_password() {
    // Admin password from .env.example
    assert!(validate_password_complexity("SuperSecure@dmin123").is_ok());
}

#[test]
fn test_all_special_characters_accepted() {
    let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";

    for ch in special_chars.chars() {
        let password = format!("Password1{}", ch);
        assert!(
            validate_password_complexity(&password).is_ok(),
            "Special character '{}' should be accepted",
            ch
        );
    }
}
