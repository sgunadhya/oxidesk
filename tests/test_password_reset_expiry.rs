mod helpers;

use helpers::*;
use oxidesk::{
    api::middleware::error::ApiError,
    models::{Agent, User, UserType},
    services::{hash_password, password_reset_service, validate_and_normalize_email},
};

#[tokio::test]
async fn test_expired_token_rejected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("expired1@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Expired Test 1".to_string(),
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

    // Manually expire the token (set expires_at to 2 hours ago)
    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(past_time.to_rfc3339())
        .bind(&token_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Try to use expired token
    let result = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;

    // Should fail with BadRequest
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::BadRequest(msg) => {
            assert_eq!(msg, "Invalid or expired reset token");
        }
        other => panic!("Expected BadRequest for expired token, got: {:?}", other),
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_unchanged_after_expired_token_rejection() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("expired2@example.com").unwrap();
    let original_password = "OriginalPass123!";
    let original_hash = hash_password(original_password).unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Expired Test 2".to_string(),
        None,
        original_hash.clone(),
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

    // Try to use expired token
    let result = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;
    assert!(result.is_err());

    // Verify password remains unchanged
    let agent_after = db.get_agent_by_user_id(&user.id).await.unwrap().unwrap();
    assert_eq!(
        agent_after.password_hash, original_hash,
        "Password should not change when using expired token"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_token_at_exact_expiry_boundary() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("boundary@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Boundary Test".to_string(),
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

    // Set token to expire exactly now (boundary condition)
    let now = chrono::Utc::now();
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(now.to_rfc3339())
        .bind(&token_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Try to use token at exact expiry
    let result = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;

    // Should be rejected (expired tokens are rejected if expires_at <= now)
    assert!(
        result.is_err(),
        "Token at exact expiry boundary should be rejected"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_token_just_before_expiry_works() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("justbefore@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Just Before Test".to_string(),
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

    // Set token to expire in 5 seconds (still valid)
    let future = chrono::Utc::now() + chrono::Duration::seconds(5);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(future.to_rfc3339())
        .bind(&token_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Token should still work
    let result = password_reset_service::reset_password(db, &token_value, "NewPass123!").await;
    assert!(result.is_ok(), "Token just before expiry should work");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_default_expiry_is_1_hour() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("onehour@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "One Hour Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset
    let before_request = chrono::Utc::now();
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let after_request = chrono::Utc::now();

    // Get token and check expiry time
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0];

    // Parse the expires_at string to DateTime for comparison
    let token_expires_at = chrono::DateTime::parse_from_rfc3339(&token.expires_at)
        .unwrap()
        .with_timezone(&chrono::Utc);

    // Expiry should be approximately 1 hour from now (within reasonable tolerance)
    let expected_expiry_min = before_request + chrono::Duration::seconds(3600 - 5); // 5 sec tolerance
    let expected_expiry_max = after_request + chrono::Duration::seconds(3600 + 5);

    assert!(
        token_expires_at >= expected_expiry_min && token_expires_at <= expected_expiry_max,
        "Token expiry should be approximately 1 hour (3600 seconds) from creation"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_multiple_expired_tokens_all_rejected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("multiple@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Multiple Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create multiple tokens and expire them all
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

        expired_tokens.push(latest_token.token.clone());
    }

    // Try to use each expired token - all should fail
    for (i, token) in expired_tokens.iter().enumerate() {
        let result = password_reset_service::reset_password(db, token, "NewPass123!").await;
        assert!(
            result.is_err(),
            "Expired token {} should be rejected",
            i + 1
        );
    }

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_expired_token_error_message_same_as_invalid() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("samemsg@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Same Message Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Request password reset and expire token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token_value = tokens[0].token.clone();
    let token_id = tokens[0].id.clone();

    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    sqlx::query("UPDATE password_reset_tokens SET expires_at = ? WHERE id = ?")
        .bind(past_time.to_rfc3339())
        .bind(&token_id)
        .execute(db.pool())
        .await
        .unwrap();

    // Try to use expired token
    let expired_result =
        password_reset_service::reset_password(db, &token_value, "NewPass123!").await;

    // Try to use non-existent token
    let invalid_result = password_reset_service::reset_password(
        db,
        "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
        "NewPass123!",
    )
    .await;

    // Both should return the same generic error message
    assert!(expired_result.is_err());
    assert!(invalid_result.is_err());

    let expired_msg = match expired_result.unwrap_err() {
        ApiError::BadRequest(msg) => msg,
        _ => panic!("Expected BadRequest"),
    };

    let invalid_msg = match invalid_result.unwrap_err() {
        ApiError::BadRequest(msg) => msg,
        _ => panic!("Expected BadRequest"),
    };

    assert_eq!(
        expired_msg, invalid_msg,
        "Expired and invalid tokens should return same error message"
    );
    assert_eq!(expired_msg, "Invalid or expired reset token");

    teardown_test_db(test_db).await;
}
