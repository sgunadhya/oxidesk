use oxidesk::domain::entities::{User, UserType};

#[test]
fn test_user_has_unique_id() {
    let user1 = User::new("test1@example.com".to_string(), UserType::Agent);
    let user2 = User::new("test2@example.com".to_string(), UserType::Agent);

    // UUIDs should be unique
    assert_ne!(user1.id, user2.id);

    // UUID format validation (should be 36 characters with hyphens)
    assert_eq!(user1.id.len(), 36);
    assert!(user1.id.contains('-'));
}

#[test]
fn test_user_email_normalized() {
    let user = User::new("Test@Example.COM".to_string(), UserType::Agent);

    // Email should be normalized to lowercase
    assert_eq!(user.email, "test@example.com");
}

#[test]
fn test_user_timestamps_set() {
    let user = User::new("test@example.com".to_string(), UserType::Agent);

    // Timestamps should be set
    assert!(!user.created_at.is_empty());
    assert!(!user.updated_at.is_empty());

    // Should be equal on creation
    assert_eq!(user.created_at, user.updated_at);
}

#[test]
fn test_user_types() {
    let agent = User::new("agent@example.com".to_string(), UserType::Agent);
    let contact = User::new("contact@example.com".to_string(), UserType::Contact);

    assert!(matches!(agent.user_type, UserType::Agent));
    assert!(matches!(contact.user_type, UserType::Contact));
}
