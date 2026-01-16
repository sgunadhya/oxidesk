use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    api::middleware::error::ApiError,
    models::{Agent, User, UserType},
    services::auth::verify_password,
    services::{hash_password, password_reset_service, validate_and_normalize_email},
};

#[tokio::test]
async fn test_successful_password_reset_with_valid_token() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reset@example.com").unwrap();
    let old_password = "OldPass123!";
    let old_password_hash = hash_password(old_password).unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reset Test".to_string(),
        None,
        old_password_hash.clone(),
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Reset password with token
    let new_password = "NewPass123!";
    let result = password_reset_service::reset_password(db, token, new_password).await;

    // Should succeed
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(
        response.message,
        "Password has been reset successfully. Please log in with your new password."
    );

    // Verify password was updated by getting agent and checking password
    let updated_agent = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert!(
        verify_password(new_password, &updated_agent.password_hash).unwrap(),
        "New password should verify successfully"
    );
    assert!(
        !verify_password(old_password, &updated_agent.password_hash).unwrap(),
        "Old password should no longer verify"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_marks_token_as_used() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("tokenused@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Token Used Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Token should not be used initially
    let token_before = db.get_password_reset_token(token).await.unwrap().unwrap();
    assert!(!token_before.used, "Token should not be used before reset");

    // Reset password
    password_reset_service::reset_password(db, token, "NewPass123!")
        .await
        .unwrap();

    // Token should now be marked as used
    let token_after = db.get_password_reset_token(token).await.unwrap().unwrap();
    assert!(
        token_after.used,
        "Token should be marked as used after reset"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_rejects_invalid_token_format() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Try to reset with invalid token formats
    let long_token = "a".repeat(40);
    let invalid_tokens = vec![
        "short",                              // Too short
        &long_token,                          // Too long
        "invalid_chars_!@#$%^&*()_+{}[]",     // Invalid characters
        "spaces in token string not allowed", // Spaces
        "",                                   // Empty
    ];

    for invalid_token in &invalid_tokens {
        let result = password_reset_service::reset_password(db, invalid_token, "NewPass123!").await;

        assert!(
            result.is_err(),
            "Should reject invalid token format: {}",
            invalid_token
        );

        match result.unwrap_err() {
            ApiError::BadRequest(msg) => {
                assert_eq!(msg, "Invalid token format");
            }
            other => panic!(
                "Expected BadRequest for invalid token format, got: {:?}",
                other
            ),
        }
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_rejects_nonexistent_token() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Try to reset with valid format but non-existent token
    let nonexistent_token = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6";

    let result = password_reset_service::reset_password(db, nonexistent_token, "NewPass123!").await;

    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::BadRequest(msg) => {
            assert_eq!(msg, "Invalid or expired reset token");
        }
        other => panic!(
            "Expected BadRequest for non-existent token, got: {:?}",
            other
        ),
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_rejects_weak_passwords() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("weakpass@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Weak Pass Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Try various weak passwords
    let weak_passwords = vec![
        "short",            // Too short
        "nouppercase123!",  // No uppercase
        "NOLOWERCASE123!",  // No lowercase
        "NoDigitsHere!",    // No digits
        "NoSpecialChar123", // No special character
    ];

    for weak_pass in &weak_passwords {
        let result = password_reset_service::reset_password(db, token, weak_pass).await;

        assert!(
            result.is_err(),
            "Should reject weak password: {}",
            weak_pass
        );

        match result.unwrap_err() {
            ApiError::BadRequest(_) => {
                // Expected - password validation error
            }
            other => panic!("Expected BadRequest for weak password, got: {:?}", other),
        }
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_updates_password_hash() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("hashupdate@example.com").unwrap();
    let old_password = "OldPass123!";
    let old_password_hash = hash_password(old_password).unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Hash Update Test".to_string(),
        None,
        old_password_hash.clone(),
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Verify old password hash
    let agent_before = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_eq!(agent_before.password_hash, old_password_hash);

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Reset password
    let new_password = "NewPass123!";
    password_reset_service::reset_password(db, token, new_password)
        .await
        .unwrap();

    // Verify password hash was updated
    let agent_after = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_ne!(
        agent_after.password_hash, old_password_hash,
        "Password hash should be different after reset"
    );
    assert!(
        verify_password(new_password, &agent_after.password_hash).unwrap(),
        "New password should verify with new hash"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_returns_success_message() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("message@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Message Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Reset password
    let result = password_reset_service::reset_password(db, token, "NewPass123!").await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(
        response.message,
        "Password has been reset successfully. Please log in with your new password."
    );

    teardown_test_db(test_db).await;
}
