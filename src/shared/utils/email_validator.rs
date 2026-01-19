use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};

pub fn validate_and_normalize_email(email: &str) -> ApiResult<String> {
    let trimmed = email.trim();

    if !email_address::EmailAddress::is_valid(trimmed) {
        return Err(ApiError::BadRequest(
            "Invalid email format. Must be in format user@domain.tld".to_string(),
        ));
    }

    // Additional validation: require a TLD (dot after @)
    if let Some(at_pos) = trimmed.find('@') {
        let domain_part = &trimmed[at_pos + 1..];
        if !domain_part.contains('.') {
            return Err(ApiError::BadRequest(
                "Invalid email format. Domain must include a TLD (e.g., .com, .org)".to_string(),
            ));
        }
    }

    // Normalize to lowercase for consistent storage
    Ok(trimmed.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email() {
        let result = validate_and_normalize_email("test@example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");
    }

    #[test]
    fn test_email_normalization() {
        let result = validate_and_normalize_email("Test@Example.COM");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");
    }

    #[test]
    fn test_email_with_whitespace() {
        let result = validate_and_normalize_email("  test@example.com  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");
    }

    #[test]
    fn test_invalid_email_no_at() {
        let result = validate_and_normalize_email("testexample.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_email_no_domain() {
        let result = validate_and_normalize_email("test@");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_email_no_tld() {
        let result = validate_and_normalize_email("test@example");
        assert!(result.is_err());
    }
}
