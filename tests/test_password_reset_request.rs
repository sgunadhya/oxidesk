use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    models::{Agent, User, UserType},
    services::{hash_password, password_reset_service, validate_and_normalize_email},
};

#[tokio::test]
async fn test_request_password_reset_registered_email_success() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("alice@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Alice".to_string(), None, password_hash);

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    let result = password_reset_service::request_password_reset(db, &email).await;

    // Should succeed and return generic message
    if let Err(ref e) = result {
        eprintln!("Error: {:?}", e);
    }
    assert!(result.is_ok(), "request_password_reset should succeed");
    let response = result.unwrap();
    assert_eq!(
        response.message,
        "If an account exists with that email, you will receive a password reset link."
    );

    // Verify token was created in database
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token.len(), 32);
    assert!(!tokens[0].used);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_request_password_reset_nonexistent_email_same_response() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Request password reset for email that doesn't exist
    let result =
        password_reset_service::request_password_reset(db, "nonexistent@example.com").await;

    // Should succeed with same generic message (email enumeration prevention)
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(
        response.message,
        "If an account exists with that email, you will receive a password reset link."
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_request_password_reset_token_format() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("bob@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Bob".to_string(), None, password_hash);

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();

    // Verify token format
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    assert_eq!(tokens.len(), 1);

    let token = &tokens[0].token;
    assert_eq!(token.len(), 32, "Token should be 32 characters");
    assert!(
        token.chars().all(|c| c.is_alphanumeric()),
        "Token should be alphanumeric only"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_request_password_reset_token_expiry() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("charlie@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Charlie".to_string(), None, password_hash);

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();

    // Verify token has expiry timestamp (1 hour from now)
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    assert_eq!(tokens.len(), 1);

    let token = &tokens[0];
    assert!(
        !token.is_expired(),
        "Token should not be expired immediately after creation"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_request_password_reset_invalidates_previous_tokens() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("dave@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Dave".to_string(), None, password_hash);

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset twice
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens_after_first = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let first_token = tokens_after_first[0].token.clone();

    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();

    // First token should be marked as used (invalidated)
    let first_token_record = db.get_password_reset_token(&first_token).await.unwrap();
    if let Some(token) = first_token_record {
        assert!(token.used, "Previous token should be invalidated");
    }

    // Should have exactly 1 active token (the new one)
    let all_tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let active_tokens: Vec<_> = all_tokens.iter().filter(|t| !t.used).collect();
    assert_eq!(active_tokens.len(), 1, "Should have exactly 1 active token");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_request_password_reset_email_normalization() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user with lowercase email
    let email = validate_and_normalize_email("emma@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(user.id.clone(), "Emma".to_string(), None, password_hash);

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request with uppercase and spaces (should be normalized)
    let result = password_reset_service::request_password_reset(db, "  EMMA@EXAMPLE.COM  ").await;

    // Should succeed (email normalized internally)
    assert!(result.is_ok());

    // Verify token was created
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    assert_eq!(tokens.len(), 1);

    teardown_test_db(test_db).await;
}
