/// User Service
/// Feature: User Creation (016)
///
/// Provides user creation business logic including:
/// - Email display name parsing for contact creation
/// - Contact creation from incoming messages (idempotent)

use crate::api::middleware::error::ApiError;
use crate::database::Database;
use regex::Regex;

/// Parse email display name from "From" header
///
/// Extracts email address and optional display name from email headers.
/// Supports two formats:
/// 1. "Display Name <email@example.com>" - with display name
/// 2. "email@example.com" - email only
///
/// Display name is split into first_name (first word) and last_name (remaining words).
///
/// # Arguments
///
/// * `from_header` - Raw "From" header from email (e.g., "John Doe <john@example.com>")
///
/// # Returns
///
/// A tuple of (Option<first_name>, Option<last_name>, email)
/// - first_name: First word of display name, or None if no display name
/// - last_name: Remaining words of display name, or None if single word or no display name
/// - email: Email address (always present)
///
/// # Examples
///
/// ```
/// use oxidesk::services::user_service::parse_email_display_name;
/// let (first, last, email) = parse_email_display_name("John Doe <john@example.com>");
/// assert_eq!(first, Some("John".to_string()));
/// assert_eq!(last, Some("Doe".to_string()));
/// assert_eq!(email, "john@example.com");
///
/// let (first, last, email) = parse_email_display_name("alice@example.com");
/// assert_eq!(first, None);
/// assert_eq!(last, None);
/// assert_eq!(email, "alice@example.com");
/// ```
pub fn parse_email_display_name(from_header: &str) -> (Option<String>, Option<String>, String) {
    // Trim leading/trailing whitespace from input
    let trimmed_header = from_header.trim();

    // Regex to match "Display Name <email@example.com>" format
    let re = Regex::new(r"^([^<]+)\s*<(.+)>$").unwrap();

    if let Some(caps) = re.captures(trimmed_header) {
        // Extract display name and email
        let display_name = caps[1].trim();
        let email = caps[2].trim().to_string();

        // Split display name into first_name and last_name
        let parts: Vec<&str> = display_name.split_whitespace().collect();
        let (first_name, last_name) = match parts.len() {
            0 => (None, None), // No display name (shouldn't happen with regex match)
            1 => (Some(parts[0].to_string()), None), // Single word: first name only
            _ => {
                // Multiple words: first word is first_name, rest is last_name
                let first = parts[0].to_string();
                let last = parts[1..].join(" ");
                (Some(first), Some(last))
            }
        };

        (first_name, last_name, email)
    } else {
        // No display name format matched, treat entire input as email
        (None, None, trimmed_header.to_string())
    }
}

/// Create contact from incoming message (idempotent)
///
/// Automatically creates a contact record when a message arrives from an unknown email address.
/// If contact already exists with the same email, returns existing contact_id (idempotent).
///
/// Creates in single transaction:
/// - User record with type='contact'
/// - Contact record with parsed display name
/// - ContactChannel linking contact to inbox
///
/// # Arguments
///
/// * `db` - Database connection pool
/// * `inbox_id` - ID of inbox that received the message
/// * `from_header` - Raw "From" header from email (e.g., "Alice Johnson <alice@example.com>")
///
/// # Returns
///
/// Result containing contact_id (existing or newly created)
///
/// # Errors
///
/// - ApiError::ValidationError: Invalid email format
/// - ApiError::DatabaseError: Transaction failure
///
/// # Example
///
/// ```rust,ignore
/// let contact_id = create_contact_from_message(
///     &db,
///     "inbox-001",
///     "Alice Johnson <alice@example.com>"
/// ).await?;
/// ```
pub async fn create_contact_from_message(
    db: &Database,
    inbox_id: &str,
    from_header: &str,
) -> Result<String, ApiError> {
    // Parse email and display name
    let (first_name, last_name, email) = parse_email_display_name(from_header);

    // Validate email format
    let normalized_email = crate::services::validate_and_normalize_email(&email)?;

    // Check if contact already exists (idempotent)
    if let Some(existing_contact) = db.get_contact_by_email(&normalized_email).await? {
        tracing::debug!(
            "Contact already exists for email {}, returning existing contact_id: {}",
            normalized_email,
            existing_contact.id
        );
        return Ok(existing_contact.id);
    }

    // Combine first_name and last_name into full_name for contact
    let full_name = match (first_name, last_name) {
        (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
        (Some(first), None) => Some(first),
        (None, Some(last)) => Some(last),
        (None, None) => None,
    };

    // Create contact in transaction (handled by database layer)
    let contact_id = db
        .create_contact_from_message(&normalized_email, full_name.as_deref(), inbox_id)
        .await?;

    tracing::info!(
        "Created contact {} for email {} from message in inbox {}",
        contact_id,
        normalized_email,
        inbox_id
    );

    Ok(contact_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_email_with_full_display_name() {
        let (first, last, email) = parse_email_display_name("John Doe <john@example.com>");
        assert_eq!(first, Some("John".to_string()));
        assert_eq!(last, Some("Doe".to_string()));
        assert_eq!(email, "john@example.com");
    }

    #[test]
    fn test_parse_email_with_single_word_display_name() {
        let (first, last, email) = parse_email_display_name("Alice <alice@example.com>");
        assert_eq!(first, Some("Alice".to_string()));
        assert_eq!(last, None);
        assert_eq!(email, "alice@example.com");
    }

    #[test]
    fn test_parse_email_with_multi_word_display_name() {
        let (first, last, email) = parse_email_display_name("Dr. Robert Smith Jr. <robert@example.com>");
        assert_eq!(first, Some("Dr.".to_string()));
        assert_eq!(last, Some("Robert Smith Jr.".to_string()));
        assert_eq!(email, "robert@example.com");
    }

    #[test]
    fn test_parse_email_only_no_display_name() {
        let (first, last, email) = parse_email_display_name("bob@example.com");
        assert_eq!(first, None);
        assert_eq!(last, None);
        assert_eq!(email, "bob@example.com");
    }

    #[test]
    fn test_parse_email_with_extra_whitespace() {
        let (first, last, email) = parse_email_display_name("  Charlie Brown  <charlie@example.com>  ");
        assert_eq!(first, Some("Charlie".to_string()));
        assert_eq!(last, Some("Brown".to_string()));
        assert_eq!(email, "charlie@example.com");
    }

    #[test]
    fn test_parse_email_with_special_characters_in_name() {
        let (first, last, email) = parse_email_display_name("O'Brien, Jane <jane@example.com>");
        assert_eq!(first, Some("O'Brien,".to_string()));
        assert_eq!(last, Some("Jane".to_string()));
        assert_eq!(email, "jane@example.com");
    }

    #[test]
    fn test_parse_email_with_quoted_display_name() {
        let (first, last, email) = parse_email_display_name("\"Smith, John\" <john@example.com>");
        assert_eq!(first, Some("\"Smith,".to_string()));
        assert_eq!(last, Some("John\"".to_string()));
        assert_eq!(email, "john@example.com");
    }

    #[test]
    fn test_parse_email_edge_case_email_only_with_angle_brackets() {
        let (first, last, email) = parse_email_display_name("<dave@example.com>");
        assert_eq!(first, None);
        assert_eq!(last, None);
        assert_eq!(email, "<dave@example.com>"); // Treated as email (invalid but handled)
    }
}
