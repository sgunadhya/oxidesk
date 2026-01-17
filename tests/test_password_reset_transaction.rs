use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::session_repository::SessionRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;
use uuid::Uuid;

use helpers::*;
use oxidesk::{
    models::{Agent, Session, User, UserType},
    services::{hash_password, PasswordResetService, validate_and_normalize_email},
};

#[tokio::test]
async fn test_invalid_token_does_not_change_password() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("rollback1@example.com").unwrap();
    let original_password = "OriginalPass123!";
    let original_hash = hash_password(original_password).unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Rollback Test 1".to_string(),
        None,
        original_hash.clone(),
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Try to reset with invalid token
    let invalid_token = "invalidtokenformat123456789012";
    let result = PasswordResetService::new(db.clone()).reset_password(invalid_token, "NewPass123!").await;

    // Should fail
    assert!(result.is_err());

    // Password should remain unchanged
    let agent_after = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_eq!(
        agent_after.password_hash, original_hash,
        "Password should not change on failed reset"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_weak_password_does_not_mark_token_as_used() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("rollback2@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Rollback Test 2".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(db.clone()).request_password_reset(&email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Try to reset with weak password
    let result = PasswordResetService::new(db.clone()).reset_password(token, "weak").await;

    // Should fail
    assert!(result.is_err());

    // Token should still be unused
    let token_after = db.get_password_reset_token(token).await.unwrap().unwrap();
    assert!(
        !token_after.used,
        "Token should not be marked as used on password validation failure"
    );

    // Token should still be usable
    let result2 = PasswordResetService::new(db.clone()).reset_password(token, "StrongPass123!").await;
    assert!(
        result2.is_ok(),
        "Token should still be usable after failed attempt"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_failed_reset_does_not_destroy_sessions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("rollback3@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Rollback Test 3".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create sessions
    let session1 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
    let session2 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
    db.create_session(&session1).await.unwrap();
    db.create_session(&session2).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(db.clone()).request_password_reset(&email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Try to reset with weak password (should fail)
    let result = PasswordResetService::new(db.clone()).reset_password(token, "weak").await;
    assert!(result.is_err());

    // Sessions should remain intact
    let sessions_after = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions_after.len(),
        2,
        "Sessions should not be destroyed on failed reset"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_nonexistent_token_does_not_affect_database() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("rollback4@example.com").unwrap();
    let original_hash = hash_password("OriginalPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Rollback Test 4".to_string(),
        None,
        original_hash.clone(),
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create session
    let session = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
    db.create_session(&session).await.unwrap();

    // Try to reset with non-existent token
    let nonexistent_token = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6";
    let result = PasswordResetService::new(db.clone()).reset_password(nonexistent_token, "NewPass123!").await;

    // Should fail
    assert!(result.is_err());

    // Password should remain unchanged
    let agent_after = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_eq!(agent_after.password_hash, original_hash);

    // Session should remain intact
    let sessions_after = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(sessions_after.len(), 1);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_expired_token_does_not_change_password() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("expired@example.com").unwrap();
    let original_password = "OriginalPass123!";
    let original_hash = hash_password(original_password).unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Expired Test".to_string(),
        None,
        original_hash.clone(),
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    PasswordResetService::new(db.clone()).request_password_reset(&email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token_value = tokens[0].token.clone();

    // Manually update token to be expired (set expires_at to past)
    let expired_token = db
        .get_password_reset_token(&token_value)
        .await
        .unwrap()
        .unwrap();
    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(past_time.to_rfc3339())
        .bind(&expired_token.id)
        .execute(db.pool())
        .await
        .unwrap();

    // Try to reset with expired token
    let result = PasswordResetService::new(db.clone()).reset_password(&token_value, "NewPass123!").await;

    // Should fail
    assert!(result.is_err());

    // Password should remain unchanged
    let agent_after = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_eq!(
        agent_after.password_hash, original_hash,
        "Password should not change with expired token"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_used_token_does_not_change_password_again() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("reuse@example.com").unwrap();
    let original_hash = hash_password("OriginalPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Reuse Test".to_string(),
        None,
        original_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and get token
    PasswordResetService::new(db.clone()).request_password_reset(&email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Use token once
    PasswordResetService::new(db.clone()).reset_password(token, "FirstNewPass123!")
        .await
        .unwrap();

    // Get the new password hash
    let agent_after_first = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    let first_new_hash = agent_after_first.password_hash.clone();

    // Try to use same token again
    let result = PasswordResetService::new(db.clone()).reset_password(token, "SecondNewPass123!").await;

    // Should fail
    assert!(result.is_err());

    // Password should remain as first reset (not changed to second password)
    let agent_after_second = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_eq!(
        agent_after_second.password_hash, first_new_hash,
        "Password should not change on second use of same token"
    );

    teardown_test_db(test_db).await;
}
