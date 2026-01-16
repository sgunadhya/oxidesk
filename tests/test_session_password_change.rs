use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::session_repository::SessionRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    api::middleware::auth::AuthenticatedUser,
    api::middleware::error::ApiError,
    models::{Agent, ChangePasswordRequest, Session, User, UserType},
    services::{agent_service, hash_password, validate_and_normalize_email},
};
use uuid::Uuid;

#[tokio::test]
async fn test_password_change_destroys_all_user_sessions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user for authentication
    let admin_email = validate_and_normalize_email("admin@example.com").unwrap();
    let admin_password_hash = hash_password("AdminPass123!").unwrap();
    let admin_user = User::new(admin_email.clone(), UserType::Agent);
    let admin_agent = Agent::new(
        admin_user.id.clone(),
        "Admin User".to_string(),
        None,
        admin_password_hash.clone(),
    );

    db.create_user(&admin_user).await.unwrap();
    db.create_agent(&admin_agent).await.unwrap();

    // Get the Admin role from database (seeded role with ID 00000000-0000-0000-0000-000000000001)
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();

    // Assign Admin role to admin user
    let user_role = oxidesk::models::UserRole::new(admin_user.id.clone(), admin_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    // Create admin session for authenticated user
    let admin_session = Session::new(admin_user.id.clone(), Uuid::new_v4().to_string(), 9);
    db.create_session(&admin_session).await.unwrap();

    let roles = db.get_user_roles(&admin_user.id).await.unwrap();
    let permissions = roles
        .iter()
        .flat_map(|r| r.permissions.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let auth_user = AuthenticatedUser {
        user: admin_user.clone(),
        agent: admin_agent.clone(),
        roles,
        permissions,
        session: admin_session.clone(),
        token: admin_session.token.clone(),
    };

    // Create target agent user
    let email = validate_and_normalize_email("agent@example.com").unwrap();
    let password_hash = hash_password("OldPass123!").unwrap();
    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Test Agent".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create multiple sessions for the target agent (simulating multiple devices/browsers)
    let session1 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 9);
    let session2 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 9);
    let session3 = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 9);

    db.create_session(&session1).await.unwrap();
    db.create_session(&session2).await.unwrap();
    db.create_session(&session3).await.unwrap();

    // Verify sessions exist
    let sessions_before = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions_before.len(),
        3,
        "Should have 3 sessions before password change"
    );

    // Change password
    let request = ChangePasswordRequest {
        new_password: "NewPass123!".to_string(),
    };

    let session_service =
        oxidesk::services::SessionService::new(db.clone(), std::sync::Arc::new(db.clone()));
    let agent_service = agent_service::AgentService::new(
        db.clone(),
        std::sync::Arc::new(db.clone()),
        session_service,
    );
    let result = agent_service
        .change_agent_password(&auth_user, &user.id, request)
        .await;
    assert!(result.is_ok(), "Password change should succeed");

    // Verify all sessions were destroyed
    let sessions_after = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(
        sessions_after.len(),
        0,
        "All sessions should be destroyed after password change"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_change_requires_reauthentication() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin_email = validate_and_normalize_email("admin2@example.com").unwrap();
    let admin_password_hash = hash_password("AdminPass123!").unwrap();
    let admin_user = User::new(admin_email.clone(), UserType::Agent);
    let admin_agent = Agent::new(
        admin_user.id.clone(),
        "Admin User 2".to_string(),
        None,
        admin_password_hash.clone(),
    );

    db.create_user(&admin_user).await.unwrap();
    db.create_agent(&admin_agent).await.unwrap();

    // Get and assign Admin role
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();
    let user_role = oxidesk::models::UserRole::new(admin_user.id.clone(), admin_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    let admin_session = Session::new(admin_user.id.clone(), Uuid::new_v4().to_string(), 9);
    db.create_session(&admin_session).await.unwrap();

    let roles = db.get_user_roles(&admin_user.id).await.unwrap();
    let permissions = roles
        .iter()
        .flat_map(|r| r.permissions.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let auth_user = AuthenticatedUser {
        user: admin_user.clone(),
        agent: admin_agent.clone(),
        roles,
        permissions,
        session: admin_session.clone(),
        token: admin_session.token.clone(),
    };

    // Create target agent
    let email = validate_and_normalize_email("agent2@example.com").unwrap();
    let password_hash = hash_password("OldPass123!").unwrap();
    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Test Agent 2".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Create session
    let session = Session::new(user.id.clone(), Uuid::new_v4().to_string(), 9);
    let old_token = session.token.clone();
    db.create_session(&session).await.unwrap();

    // Verify session exists before password change
    let session_before = db
        .get_session_by_token(&old_token)
        .await
        .map_err(|e| {
            eprintln!("Error getting session: {:?}", e);
            e
        })
        .unwrap();
    assert!(
        session_before.is_some(),
        "Session should exist before password change"
    );

    // Change password
    let request = ChangePasswordRequest {
        new_password: "NewPass123!".to_string(),
    };

    let session_service =
        oxidesk::services::SessionService::new(db.clone(), std::sync::Arc::new(db.clone()));
    let agent_service = agent_service::AgentService::new(
        db.clone(),
        std::sync::Arc::new(db.clone()),
        session_service,
    );
    agent_service
        .change_agent_password(&auth_user, &user.id, request)
        .await
        .unwrap();

    // Verify old session no longer exists
    let session_after = db.get_session_by_token(&old_token).await.unwrap();
    assert!(
        session_after.is_none(),
        "Old session should not exist after password change"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_change_does_not_affect_other_users() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin_email = validate_and_normalize_email("admin3@example.com").unwrap();
    let admin_password_hash = hash_password("AdminPass123!").unwrap();
    let admin_user = User::new(admin_email.clone(), UserType::Agent);
    let admin_agent = Agent::new(
        admin_user.id.clone(),
        "Admin User 3".to_string(),
        None,
        admin_password_hash.clone(),
    );

    db.create_user(&admin_user).await.unwrap();
    db.create_agent(&admin_agent).await.unwrap();

    // Get and assign Admin role
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();
    let user_role = oxidesk::models::UserRole::new(admin_user.id.clone(), admin_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    let admin_session = Session::new(admin_user.id.clone(), Uuid::new_v4().to_string(), 9);
    db.create_session(&admin_session).await.unwrap();

    let roles = db.get_user_roles(&admin_user.id).await.unwrap();
    let permissions = roles
        .iter()
        .flat_map(|r| r.permissions.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let auth_user = AuthenticatedUser {
        user: admin_user.clone(),
        agent: admin_agent.clone(),
        roles,
        permissions,
        session: admin_session.clone(),
        token: admin_session.token.clone(),
    };

    // Create two agents
    let email1 = validate_and_normalize_email("agent3a@example.com").unwrap();
    let email2 = validate_and_normalize_email("agent3b@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user1 = User::new(email1.clone(), UserType::Agent);
    let agent1 = Agent::new(
        user1.id.clone(),
        "Agent 1".to_string(),
        None,
        password_hash.clone(),
    );

    let user2 = User::new(email2.clone(), UserType::Agent);
    let agent2 = Agent::new(
        user2.id.clone(),
        "Agent 2".to_string(),
        None,
        password_hash.clone(),
    );

    db.create_user(&user1).await.unwrap();
    db.create_agent(&agent1).await.unwrap();
    db.create_user(&user2).await.unwrap();
    db.create_agent(&agent2).await.unwrap();

    // Create sessions for both agents
    let session1 = Session::new(user1.id.clone(), Uuid::new_v4().to_string(), 9);
    let session2 = Session::new(user2.id.clone(), Uuid::new_v4().to_string(), 9);

    db.create_session(&session1).await.unwrap();
    db.create_session(&session2).await.unwrap();

    // Change password for agent1
    let request = ChangePasswordRequest {
        new_password: "NewPass123!".to_string(),
    };

    let session_service =
        oxidesk::services::SessionService::new(db.clone(), std::sync::Arc::new(db.clone()));
    let agent_service = agent_service::AgentService::new(
        db.clone(),
        std::sync::Arc::new(db.clone()),
        session_service,
    );
    agent_service
        .change_agent_password(&auth_user, &user1.id, request)
        .await
        .unwrap();

    // Verify agent1 sessions are destroyed
    let agent1_sessions = db.get_user_sessions(&user1.id).await.unwrap();
    assert_eq!(
        agent1_sessions.len(),
        0,
        "Agent 1 sessions should be destroyed"
    );

    // Verify agent2 sessions are NOT affected
    let agent2_sessions = db.get_user_sessions(&user2.id).await.unwrap();
    assert_eq!(
        agent2_sessions.len(),
        1,
        "Agent 2 sessions should NOT be affected"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_change_with_no_active_sessions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin_email = validate_and_normalize_email("admin4@example.com").unwrap();
    let admin_password_hash = hash_password("AdminPass123!").unwrap();
    let admin_user = User::new(admin_email.clone(), UserType::Agent);
    let admin_agent = Agent::new(
        admin_user.id.clone(),
        "Admin User 4".to_string(),
        None,
        admin_password_hash.clone(),
    );

    db.create_user(&admin_user).await.unwrap();
    db.create_agent(&admin_agent).await.unwrap();

    // Get and assign Admin role
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();
    let user_role = oxidesk::models::UserRole::new(admin_user.id.clone(), admin_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    let admin_session = Session::new(admin_user.id.clone(), Uuid::new_v4().to_string(), 9);
    db.create_session(&admin_session).await.unwrap();

    let roles = db.get_user_roles(&admin_user.id).await.unwrap();
    let permissions = roles
        .iter()
        .flat_map(|r| r.permissions.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let auth_user = AuthenticatedUser {
        user: admin_user.clone(),
        agent: admin_agent.clone(),
        roles,
        permissions,
        session: admin_session.clone(),
        token: admin_session.token.clone(),
    };

    // Create agent with no sessions
    let email = validate_and_normalize_email("agent4@example.com").unwrap();
    let password_hash = hash_password("OldPass123!").unwrap();
    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Agent No Sessions".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Verify no sessions exist
    let sessions_before = db.get_user_sessions(&user.id).await.unwrap();
    assert_eq!(sessions_before.len(), 0, "Should have no sessions");

    // Change password should succeed even with no sessions
    let request = ChangePasswordRequest {
        new_password: "NewPass123!".to_string(),
    };

    let session_service =
        oxidesk::services::SessionService::new(db.clone(), std::sync::Arc::new(db.clone()));
    let agent_service = agent_service::AgentService::new(
        db.clone(),
        std::sync::Arc::new(db.clone()),
        session_service,
    );
    let result = agent_service
        .change_agent_password(&auth_user, &user.id, request)
        .await;
    assert!(
        result.is_ok(),
        "Password change should succeed even with no active sessions"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_change_permission_required() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create non-admin agent (no permission to change passwords)
    let agent_email = validate_and_normalize_email("agent5@example.com").unwrap();
    let agent_password_hash = hash_password("AgentPass123!").unwrap();
    let agent_user = User::new(agent_email.clone(), UserType::Agent);
    let agent = Agent::new(
        agent_user.id.clone(),
        "Regular Agent".to_string(),
        None,
        agent_password_hash.clone(),
    );

    db.create_user(&agent_user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Get the regular Agent role (not Admin)
    let agent_role = db.get_role_by_name("Agent").await.unwrap().unwrap();
    let user_role = oxidesk::models::UserRole::new(agent_user.id.clone(), agent_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    let agent_session = Session::new(agent_user.id.clone(), Uuid::new_v4().to_string(), 9);
    db.create_session(&agent_session).await.unwrap();

    let roles = db.get_user_roles(&agent_user.id).await.unwrap();
    let permissions = roles
        .iter()
        .flat_map(|r| r.permissions.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let auth_user = AuthenticatedUser {
        user: agent_user.clone(),
        agent: agent.clone(),
        roles,
        permissions,
        session: agent_session.clone(),
        token: agent_session.token.clone(),
    };

    // Create target agent
    let target_email = validate_and_normalize_email("target@example.com").unwrap();
    let target_password_hash = hash_password("OldPass123!").unwrap();
    let target_user = User::new(target_email.clone(), UserType::Agent);
    let target_agent = Agent::new(
        target_user.id.clone(),
        "Target Agent".to_string(),
        None,
        target_password_hash,
    );

    db.create_user(&target_user).await.unwrap();
    db.create_agent(&target_agent).await.unwrap();

    // Attempt to change password without admin permission
    let request = ChangePasswordRequest {
        new_password: "NewPass123!".to_string(),
    };

    let session_service =
        oxidesk::services::SessionService::new(db.clone(), std::sync::Arc::new(db.clone()));
    let agent_service = agent_service::AgentService::new(
        db.clone(),
        std::sync::Arc::new(db.clone()),
        session_service,
    );
    let result = agent_service
        .change_agent_password(&auth_user, &target_user.id, request)
        .await;

    // Should fail with Forbidden error
    assert!(result.is_err(), "Should fail without admin permission");
    match result.unwrap_err() {
        ApiError::Forbidden(_) => {
            // Expected
        }
        other => panic!("Expected Forbidden error, got: {:?}", other),
    }

    teardown_test_db(test_db).await;
}
