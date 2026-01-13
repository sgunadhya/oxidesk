mod helpers;

use helpers::*;
use oxidesk::{
    models::{User, UserType, Agent, UserRole},
    services::{validate_password_complexity, validate_and_normalize_email, hash_password},
};

#[tokio::test]
async fn test_admin_initialization_creates_user_with_uuid() {
    let db = setup_test_db().await;

    // Create admin user
    let email = validate_and_normalize_email("admin@example.com").unwrap();
    let password = "SecureAdmin123!";

    validate_password_complexity(password).unwrap();
    let password_hash = hash_password(password).unwrap();

    let user = User::new(email, UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Admin".to_string(), password_hash);

    // Store in database
    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Verify user has UUID (36 characters with hyphens)
    assert_eq!(user.id.len(), 36);
    assert!(user.id.contains('-'));

    // Verify user was stored
    let retrieved = db.get_user_by_id(&user.id).await.unwrap();
    assert!(retrieved.is_some());

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_admin_initialization_assigns_admin_role() {
    let db = setup_test_db().await;

    // Create admin user
    let email = validate_and_normalize_email("admin@example.com").unwrap();
    let password_hash = hash_password("SecureAdmin123!").unwrap();

    let user = User::new(email, UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Admin".to_string(), password_hash);

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Get Admin role from seed data
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();

    // Assign role
    let user_role = UserRole::new(user.id.clone(), admin_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    // Verify user has at least one role
    let roles = db.get_user_roles(&user.id).await.unwrap();
    assert!(!roles.is_empty());
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "Admin");

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_admin_email_uniqueness_per_type() {
    let db = setup_test_db().await;

    // Create first admin agent
    let email = "admin@example.com";
    let normalized_email = validate_and_normalize_email(email).unwrap();
    let password_hash = hash_password("SecureAdmin123!").unwrap();

    let user1 = User::new(normalized_email.clone(), UserType::Agent);
    let agent1 = Agent::new(user1.id.clone(), "Admin".to_string(), password_hash.clone());

    db.create_user(&user1).await.unwrap();
    db.create_agent(&agent1).await.unwrap();

    // Try to create another agent with same email (should fail due to per-type uniqueness)
    let user2 = User::new(normalized_email.clone(), UserType::Agent);
    let result = db.create_user(&user2).await;

    assert!(result.is_err());

    // But creating a contact with same email should succeed (per-type uniqueness)
    let user3 = User::new(normalized_email, UserType::Contact);
    let result = db.create_user(&user3).await;

    assert!(result.is_ok());

    teardown_test_db(db).await;
}


#[tokio::test]
async fn test_admin_initialization_validates_password_complexity() {
    let db = setup_test_db().await;

    // Try with weak password
    let weak_passwords = vec![
        "short",           // Too short
        "nouppercase1!",   // No uppercase
        "NOLOWERCASE1!",   // No lowercase
        "NoDigits!!!",     // No digits
        "NoSpecial123",    // No special char
    ];

    for weak_pass in weak_passwords {
        let result = validate_password_complexity(weak_pass);
        assert!(result.is_err(), "Password '{}' should be rejected", weak_pass);
    }

    // Strong password should pass
    assert!(validate_password_complexity("SecureAdmin123!").is_ok());

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_admin_initialization_idempotent() {
    let db = setup_test_db().await;

    // Create admin user first time
    let email = validate_and_normalize_email("admin@example.com").unwrap();
    let password_hash = hash_password("SecureAdmin123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    db.create_user(&user).await.unwrap();

    let agent = Agent::new(user.id.clone(), "Admin".to_string(), password_hash.clone());
    db.create_agent(&agent).await.unwrap();

    // Check if user exists
    let existing = db.get_user_by_email_and_type(&email, &UserType::Agent).await.unwrap();
    assert!(existing.is_some());

    // Second attempt should detect existing user
    let existing_user = existing.unwrap();
    assert_eq!(existing_user.email, email);

    teardown_test_db(db).await;
}
