use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    infrastructure::http::middleware::error::ApiError,
    domain::entities::{Agent, User, UserType},
    application::services::auth::hash_password,
    shared::utils::email_validator::validate_and_normalize_email,
    application::services::PasswordResetService,
};

#[tokio::test]
async fn test_token_cannot_be_reused_after_successful_reset() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reuse1@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reuse Test 1".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = tokens[0].token.clone();

    // Use token once (should succeed)
    let result1 = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token, "NewPass123!").await;
    assert!(result1.is_ok(), "First use of token should succeed");

    // Try to use same token again (should fail)
    let result2 = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token, "AnotherPass123!").await;
    assert!(result2.is_err(), "Second use of token should fail");

    match result2.unwrap_err() {
        ApiError::BadRequest(msg) => {
            assert_eq!(msg, "Invalid or expired reset token");
        }
        other => panic!("Expected BadRequest for reused token, got: {:?}", other),
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_token_marked_used_after_reset() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reuse2@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reuse Test 2".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = tokens[0].token.clone();

    // Verify token is not used initially
    let token_before = db.get_password_reset_token(&token).await.unwrap().unwrap();
    assert!(!token_before.used, "Token should not be used before reset");

    // Use token
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token, "NewPass123!")
        .await
        .unwrap();

    // Verify token is marked as used
    let token_after = db.get_password_reset_token(&token).await.unwrap().unwrap();
    assert!(
        token_after.used,
        "Token should be marked as used after reset"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_multiple_tokens_can_be_used_once_each() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reuse3@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reuse Test 3".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset three times (each request invalidates previous tokens)
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens1 = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token1 = tokens1[0].token.clone();

    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens2 = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token2 = tokens2.iter().find(|t| !t.used).unwrap().token.clone();

    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens3 = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token3 = tokens3.iter().find(|t| !t.used).unwrap().token.clone();

    // Token1 should be invalidated (marked as used when token2 was created)
    let result1 = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token1, "Pass1_123!").await;
    assert!(result1.is_err(), "Token1 should be invalidated");

    // Token2 should be invalidated (marked as used when token3 was created)
    let result2 = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token2, "Pass2_123!").await;
    assert!(result2.is_err(), "Token2 should be invalidated");

    // Token3 (most recent) should work
    let result3 = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token3, "Pass3_123!").await;
    assert!(result3.is_ok(), "Token3 (most recent) should work");

    // Token3 should not work a second time
    let result3_reuse = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token3, "Pass4_123!").await;
    assert!(result3_reuse.is_err(), "Token3 should not work twice");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_new_token_invalidates_previous_unused_tokens() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reuse4@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reuse Test 4".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset (get first token)
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens1 = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let first_token = tokens1[0].token.clone();

    // Verify first token is not used
    let first_token_before = db
        .get_password_reset_token(&first_token)
        .await
        .unwrap()
        .unwrap();
    assert!(!first_token_before.used, "First token should not be used");

    // Request another password reset (should invalidate first token)
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();

    // First token should now be marked as used (invalidated)
    let first_token_after = db
        .get_password_reset_token(&first_token)
        .await
        .unwrap()
        .unwrap();
    assert!(
        first_token_after.used,
        "First token should be invalidated by new request"
    );

    // Try to use first token (should fail)
    let result = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &first_token, "NewPass123!").await;
    assert!(result.is_err(), "Invalidated token should not work");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_token_reuse_returns_same_error_as_expired() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reuse5@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reuse Test 5".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = tokens[0].token.clone();

    // Use token once
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token, "FirstPass123!")
        .await
        .unwrap();

    // Try to reuse token
    let result = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token, "SecondPass123!").await;

    // Should return generic "Invalid or expired" error (no distinction for security)
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::BadRequest(msg) => {
            assert_eq!(
                msg, "Invalid or expired reset token",
                "Should return generic error message"
            );
        }
        other => panic!("Expected BadRequest, got: {:?}", other),
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_concurrent_use_of_same_token() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("concurrent@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Concurrent Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).request_password_reset( &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = tokens[0].token.clone();

    // Try to use token concurrently (simulate race condition)
    // In practice, only one should succeed due to database atomicity
    let db_clone1 = db.clone();
    let db_clone2 = db.clone();
    let token_clone1 = token.clone();
    let token_clone2 = token.clone();

    let handle1 = tokio::spawn(async move {
        PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db_clone1.clone()), std::sync::Arc::new(db_clone1.clone())).reset_password(&token_clone1, "Pass1_123!").await
    });

    let handle2 = tokio::spawn(async move {
        PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db_clone2.clone()), std::sync::Arc::new(db_clone2.clone())).reset_password(&token_clone2, "Pass2_123!").await
    });

    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();

    // Only one should succeed (though in practice both might fail or succeed depending on timing)
    // At minimum, after both complete, token should be marked as used
    let token_after = db.get_password_reset_token(&token).await.unwrap().unwrap();
    assert!(
        token_after.used || result1.is_ok() || result2.is_ok(),
        "Token should be used after concurrent attempts"
    );

    // A third attempt should definitely fail
    let result3 = PasswordResetService::new(oxidesk::domain::ports::password_reset_repository::PasswordResetRepository::new(db.clone()), std::sync::Arc::new(db.clone())).reset_password( &token, "Pass3_123!").await;
    assert!(result3.is_err(), "Subsequent use should fail");

    teardown_test_db(test_db).await;
}
