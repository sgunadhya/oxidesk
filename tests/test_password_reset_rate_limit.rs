use oxidesk::database::agents::AgentRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    api::middleware::error::ApiError,
    models::{Agent, User, UserType},
    services::{hash_password, password_reset_service, validate_and_normalize_email},
};

// Consolidate all rate limit tests into a single sequential test
// This is necessary because some tests modify the global environment variable (PASSWORD_RESET_RATE_LIMIT)
// which causes race conditions and failures when running in parallel with other tests.
#[tokio::test]
async fn test_rate_limiting_scenarios() {
    // 1. Test: Rate limit enforces 5 requests per hour (Default)
    {
        let test_db = setup_test_db().await;
        let db = test_db.db();

        let email = validate_and_normalize_email("ratelimit@example.com").unwrap();
        let password_hash = hash_password("TestPass123!").unwrap();

        let user = User::new(email.clone(), UserType::Agent);
        let agent = Agent::new(
            user.id.clone(),
            "Rate Limit Test".to_string(),
            None,
            password_hash,
        );

        db.create_user(&user).await.unwrap();
        db.create_agent(&agent).await.unwrap();

        // Make 5 requests (should all succeed)
        for i in 1..=5 {
            let result = password_reset_service::request_password_reset(db, &email).await;
            assert!(result.is_ok(), "Request {} should succeed", i);
        }

        // 6th request should fail with TooManyRequests
        let result = password_reset_service::request_password_reset(db, &email).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApiError::TooManyRequests(msg) => {
                assert!(msg.contains("Too many password reset requests"));
            }
            other => panic!("Expected TooManyRequests error, got: {:?}", other),
        }

        teardown_test_db(test_db).await;
    }

    // 2. Test: Rate limiting is per-email
    {
        let test_db = setup_test_db().await;
        let db = test_db.db();

        let email1 = validate_and_normalize_email("user1@example.com").unwrap();
        let email2 = validate_and_normalize_email("user2@example.com").unwrap();
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

        // Make 5 requests for user1
        for _ in 0..5 {
            password_reset_service::request_password_reset(db, &email1)
                .await
                .unwrap();
        }

        // 6th request for user1 should fail
        let result1 = password_reset_service::request_password_reset(db, &email1).await;
        assert!(result1.is_err(), "User1 should be rate limited");

        // But user2 should still be able to make requests (rate limiting is per-email)
        let result2 = password_reset_service::request_password_reset(db, &email2).await;
        assert!(result2.is_ok(), "User2 should not be rate limited");

        teardown_test_db(test_db).await;
    }

    // 3. Test: Rate limit window is 1 hour
    {
        let test_db = setup_test_db().await;
        let db = test_db.db();

        let email = validate_and_normalize_email("window@example.com").unwrap();
        let password_hash = hash_password("TestPass123!").unwrap();

        let user = User::new(email.clone(), UserType::Agent);
        let agent = Agent::new(
            user.id.clone(),
            "Window Test".to_string(),
            None,
            password_hash,
        );

        db.create_user(&user).await.unwrap();
        db.create_agent(&agent).await.unwrap();

        // Make 5 requests
        for _ in 0..5 {
            password_reset_service::request_password_reset(db, &email)
                .await
                .unwrap();
        }

        // 6th request should fail
        let result = password_reset_service::request_password_reset(db, &email).await;
        assert!(result.is_err());

        // Check that we're counting requests in a 3600 second (1 hour) window
        let count = db
            .count_recent_reset_requests(&user.id, 3600)
            .await
            .unwrap();
        assert_eq!(count, 5, "Should count 5 requests in the 1-hour window");

        teardown_test_db(test_db).await;
    }

    // 4. Test: Rate limit respects environment variable
    {
        let test_db = setup_test_db().await;
        let db = test_db.db();

        // Set custom rate limit for test
        // SAFE: We are running sequentially now
        std::env::set_var("PASSWORD_RESET_RATE_LIMIT", "3");

        let email = validate_and_normalize_email("custom@example.com").unwrap();
        let password_hash = hash_password("TestPass123!").unwrap();

        let user = User::new(email.clone(), UserType::Agent);
        let agent = Agent::new(
            user.id.clone(),
            "Custom Limit".to_string(),
            None,
            password_hash,
        );

        db.create_user(&user).await.unwrap();
        db.create_agent(&agent).await.unwrap();

        // Make 3 requests (should all succeed)
        for i in 1..=3 {
            let result = password_reset_service::request_password_reset(db, &email).await;
            assert!(
                result.is_ok(),
                "Request {} should succeed with custom limit of 3",
                i
            );
        }

        // 4th request should fail
        let result = password_reset_service::request_password_reset(db, &email).await;
        assert!(
            result.is_err(),
            "4th request should fail with custom limit of 3"
        );

        // Clean up env var
        std::env::set_var("PASSWORD_RESET_RATE_LIMIT", "5");

        teardown_test_db(test_db).await;
    }

    // 5. Test: Rate limit does not affect non-existent emails
    {
        let test_db = setup_test_db().await;
        let db = test_db.db();

        // Request password reset for non-existent email multiple times
        // Should always succeed with generic message (no rate limiting for non-existent users)
        for i in 1..=10 {
            let result =
                password_reset_service::request_password_reset(db, "nonexistent@example.com").await;
            assert!(
                result.is_ok(),
                "Request {} should succeed for non-existent email (no rate limiting)",
                i
            );
        }

        teardown_test_db(test_db).await;
    }
}
