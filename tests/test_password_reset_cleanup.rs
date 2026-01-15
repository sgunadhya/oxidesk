use oxidesk::database::agents::AgentRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    models::{Agent, User, UserType},
    services::{hash_password, password_reset_service, validate_and_normalize_email},
};

#[tokio::test]
async fn test_expired_token_deleted_on_validation_failure() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("cleanup1@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Cleanup Test 1".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token_value = tokens[0].token.clone();
    let token_id = tokens[0].id.clone();

    // Manually expire the token
    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(past_time.to_rfc3339())
        .bind(&token_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Verify token exists before validation attempt
    let token_before = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_before.is_some(),
        "Token should exist before validation"
    );

    // Try to use expired token (should trigger lazy cleanup)
    let result = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;
    assert!(result.is_err(), "Expired token should be rejected");

    // Verify token was deleted (lazy cleanup)
    let token_after = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_after.is_none(),
        "Expired token should be deleted after validation failure"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_valid_token_not_deleted_on_validation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("cleanup2@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Cleanup Test 2".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token_value = tokens[0].token.clone();

    // Verify token exists and is valid
    let token_before = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(token_before.is_some(), "Token should exist");
    assert!(
        !token_before.as_ref().unwrap().is_expired(),
        "Token should not be expired"
    );

    // Use valid token (should not trigger cleanup)
    let result = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;
    assert!(result.is_ok(), "Valid token should work");

    // Verify token still exists (marked as used, but not deleted)
    let token_after = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_after.is_some(),
        "Token should still exist after successful use"
    );
    assert!(token_after.unwrap().used, "Token should be marked as used");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_multiple_expired_tokens_cleaned_up_individually() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("cleanup3@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Cleanup Test 3".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create multiple tokens and expire them
    let mut expired_tokens = Vec::new();
    for _ in 0..3 {
        password_reset_service::request_password_reset(db, &email)
            .await
            .unwrap();
        let tokens = db
            .get_all_password_reset_tokens_for_user(&user.id)
            .await
            .unwrap();
        let latest_token = tokens.iter().find(|t| !t.used).unwrap();

        // Expire this token
        let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
        sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
            .bind(past_time.to_rfc3339())
            .bind(&latest_token.id)
            .execute(db.pool())
            .await
            .unwrap();

        expired_tokens.push((latest_token.token.clone(), latest_token.id.clone()));
    }

    // All tokens should exist before cleanup
    let all_tokens_before = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    assert!(
        all_tokens_before.len() >= 3,
        "Should have at least 3 tokens"
    );

    // Try to use each expired token (each should trigger cleanup for that specific token)
    for (i, (token_value, _)) in expired_tokens.iter().enumerate() {
        let result = password_reset_service::reset_password(db, token_value, "NewPass123!").await;
        assert!(result.is_err(), "Expired token {} should fail", i + 1);

        // This specific token should be deleted
        let token_after = db.get_password_reset_token(token_value).await.unwrap();
        assert!(
            token_after.is_none(),
            "Expired token {} should be deleted after validation",
            i + 1
        );
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_lazy_cleanup_only_on_expired_not_used() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("cleanup4@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Cleanup Test 4".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token_value = tokens[0].token.clone();

    // Use the token successfully
    password_reset_service::reset_password(db, &token_value, "NewPass123!")
        .await
        .unwrap();

    // Token should be marked as used but not deleted
    let token_after_use = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_after_use.is_some(),
        "Used token should not be deleted"
    );
    assert!(
        token_after_use.unwrap().used,
        "Token should be marked as used"
    );

    // Try to use the token again (should fail because it's used, not because it's expired)
    let result = password_reset_service::reset_password(db, &token_value, "AnotherPass123!").await;
    assert!(result.is_err(), "Used token should fail");

    // Token should still exist (lazy cleanup only deletes expired tokens, not used ones)
    let token_after_reuse = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_after_reuse.is_some(),
        "Used token should not be deleted by lazy cleanup"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_cleanup_does_not_affect_other_users_tokens() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create two agent users
    let email1 = validate_and_normalize_email("user1@cleanup.com").unwrap();
    let email2 = validate_and_normalize_email("user2@cleanup.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user1 = User::new(email1.clone(), UserType::Agent);
    let agent1 = Agent::new(
        user1.id.clone(),
        "User 1".to_string(),
        None,
        password_hash.clone(),
    );

    let user2 = User::new(email2.clone(), UserType::Agent);
    let agent2 = Agent::new(
        user2.id.clone(),
        "User 2".to_string(),
        None,
        password_hash.clone(),
    );

    db.create_user(&user1).await.unwrap();
    db.create_agent(&agent1).await.unwrap();
    db.create_user(&user2).await.unwrap();
    db.create_agent(&agent2).await.unwrap();

    // Create tokens for both users
    password_reset_service::request_password_reset(db, &email1)
        .await
        .unwrap();
    password_reset_service::request_password_reset(db, &email2)
        .await
        .unwrap();

    let tokens1 = db
        .get_all_password_reset_tokens_for_user(&user1.id)
        .await
        .unwrap();
    let tokens2 = db
        .get_all_password_reset_tokens_for_user(&user2.id)
        .await
        .unwrap();

    let token1_value = tokens1[0].token.clone();
    let token1_id = tokens1[0].id.clone();
    let token2_value = tokens2[0].token.clone();

    // Expire user1's token
    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(past_time.to_rfc3339())
        .bind(&token1_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Try to use user1's expired token (should trigger cleanup)
    let result1 = password_reset_service::reset_password(db, &token1_value, "NewPass123!").await;
    assert!(result1.is_err(), "User1's expired token should fail");

    // User1's token should be deleted
    let token1_after = db.get_password_reset_token(&token1_value).await.unwrap();
    assert!(
        token1_after.is_none(),
        "User1's expired token should be deleted"
    );

    // User2's token should still exist and be valid
    let token2_after = db.get_password_reset_token(&token2_value).await.unwrap();
    assert!(
        token2_after.is_some(),
        "User2's token should not be affected by User1's cleanup"
    );
    assert!(
        !token2_after.unwrap().is_expired(),
        "User2's token should still be valid"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_no_background_cleanup_job() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("background@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Background Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token_id = tokens[0].id.clone();
    let token_value = tokens[0].token.clone();

    // Expire the token
    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(past_time.to_rfc3339())
        .bind(&token_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Wait a bit (simulating time passing)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Token should still exist (no automatic background cleanup)
    let token_still_there = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_still_there.is_some(),
        "Expired token should remain in database until lazy cleanup is triggered"
    );

    // Only when we try to use it should it be deleted
    let _ = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;

    let token_after_use = db.get_password_reset_token(&token_value).await.unwrap();
    assert!(
        token_after_use.is_none(),
        "Token should be deleted only after validation attempt (lazy cleanup)"
    );

    teardown_test_db(test_db).await;
}
