use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    models::{Agent, Session, User, UserType},
    services::{hash_password, password_reset_service, validate_and_normalize_email},
};
use uuid::Uuid;

#[tokio::test]
async fn test_password_reset_destroys_all_user_sessions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("sessions@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Session Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create multiple sessions for this user
    let session1 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
    let session2 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
    let session3 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);

    db.create_session(&session1).await.unwrap();
    db.create_session(&session2).await.unwrap();
    db.create_session(&session3).await.unwrap();

    // Verify sessions exist
    let sessions_before = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions_before.len(),
        3,
        "Should have 3 sessions before reset"
    );

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Reset password (should destroy all sessions)
    password_reset_service::reset_password(db, token, "NewPass123!")
        .await
        .unwrap();

    // Verify all sessions were destroyed
    let sessions_after = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions_after.len(),
        0,
        "All sessions should be destroyed after password reset"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_only_destroys_user_sessions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create two agent users
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

    // Create sessions for both users
    let session1_user1 = Session::new(user1.id.clone(), Uuid::new_v4().to_string(), 24);
    let session2_user1 = Session::new(user1.id.clone(), Uuid::new_v4().to_string(), 24);
    let session1_user2 = Session::new(user2.id.clone(), Uuid::new_v4().to_string(), 24);

    db.create_session(&session1_user1).await.unwrap();
    db.create_session(&session2_user1).await.unwrap();
    db.create_session(&session1_user2).await.unwrap();

    // Reset password for user1
    password_reset_service::request_password_reset(db, &email1)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user1.id)
        .await
        .unwrap();
    let token = &tokens[0].token;
    password_reset_service::reset_password(db, token, "NewPass123!")
        .await
        .unwrap();

    // User1's sessions should be destroyed
    let user1_sessions = db.get_user_sessions(&user1.id).await.unwrap();
    assert_eq!(
        user1_sessions.len(),
        0,
        "User1 sessions should be destroyed"
    );

    // User2's sessions should remain intact
    let user2_sessions = db.get_user_sessions(&user2.id).await.unwrap();
    assert_eq!(
        user2_sessions.len(),
        1,
        "User2 sessions should remain intact"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_session_destruction_count() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("sessioncount@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Session Count Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create 5 sessions
    for _ in 0..5 {
        let session = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
        db.create_session(&session).await.unwrap();
    }

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Reset password and check session destruction count
    let result = password_reset_service::reset_password(db, token, "NewPass123!").await;
    assert!(result.is_ok());

    // Note: The service logs the count internally, but we verify by checking the database
    let sessions_after = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions_after.len(),
        0,
        "All 5 sessions should be destroyed"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_reset_works_with_no_existing_sessions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user with no sessions
    let email = validate_and_normalize_email("nosessions@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "No Sessions Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Verify no sessions exist
    let sessions_before = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(sessions_before.len(), 0, "Should have no sessions");

    // Request password reset and get token
    password_reset_service::request_password_reset(db, &email)
        .await
        .unwrap();
    let tokens = db
        .get_all_password_reset_tokens_for_user(&user.id)
        .await
        .unwrap();
    let token = &tokens[0].token;

    // Reset password should still succeed (destroying 0 sessions)
    let result = password_reset_service::reset_password(db, token, "NewPass123!").await;
    assert!(
        result.is_ok(),
        "Password reset should succeed even with no sessions"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_sessions_destroyed_synchronously() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent user
    let email = validate_and_normalize_email("sync@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Sync Test".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create sessions
    let session = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 24);
    db.create_session(&session).await.unwrap();

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
    password_reset_service::reset_password(db, token, "NewPass123!")
        .await
        .unwrap();

    // Sessions should be destroyed immediately (synchronously)
    // No need to wait or poll - they should already be gone
    let sessions = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions.len(),
        0,
        "Sessions should be destroyed synchronously"
    );

    teardown_test_db(test_db).await;
}
