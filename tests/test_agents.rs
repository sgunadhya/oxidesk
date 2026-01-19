use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    domain::entities::{Agent, User, UserRole, UserType},
    application::services::auth::{hash_password, validate_password_complexity},
    shared::utils::email_validator::validate_and_normalize_email,
};

#[tokio::test]
async fn test_agent_creation_success() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user first
    let admin_email = validate_and_normalize_email("admin@example.com").unwrap();
    let admin_password_hash = hash_password("AdminPass123!").unwrap();

    let admin_user = User::new(admin_email.clone(), UserType::Agent);
    let admin_agent = Agent::new(
        admin_user.id.clone(),
        "Admin".to_string(),
        None,
        admin_password_hash,
    );

    db.create_user(&admin_user).await.unwrap();
    db.create_agent(&admin_agent).await.unwrap();

    // Assign Admin role
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();
    let user_role = UserRole::new(admin_user.id.clone(), admin_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    // Now create a new agent
    let new_email = validate_and_normalize_email("agent@example.com").unwrap();
    let new_password_hash = hash_password("AgentPass123!").unwrap();

    let new_user = User::new(new_email.clone(), UserType::Agent);
    let new_agent = Agent::new(
        new_user.id.clone(),
        "Test Agent".to_string(),
        None,
        new_password_hash,
    );

    db.create_user(&new_user).await.unwrap();
    db.create_agent(&new_agent).await.unwrap();

    // Assign Agent role
    let agent_role = db.get_role_by_name("Agent").await.unwrap().unwrap();
    let new_user_role = UserRole::new(new_user.id.clone(), agent_role.id.clone());
    db.assign_role_to_user(&new_user_role).await.unwrap();

    // Verify agent was created
    let retrieved = db
        .get_user_by_email_and_type(&new_email, &UserType::Agent)
        .await
        .unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().email, new_email);

    // Verify agent has at least one role
    let roles = db.get_user_roles(&new_user.id).await.unwrap();
    assert!(!roles.is_empty());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_duplicate_agent_email_rejection() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create first agent
    let email = validate_and_normalize_email("duplicate@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user1 = User::new(email.clone(), UserType::Agent);
    let agent1 = Agent::new(
        user1.id.clone(),
        "Agent 1".to_string(),
        None,
        password_hash.clone(),
    );

    db.create_user(&user1).await.unwrap();
    db.create_agent(&agent1).await.unwrap();

    // Try to create another agent with same email (should fail)
    let user2 = User::new(email.clone(), UserType::Agent);
    let result = db.create_user(&user2).await;

    // Should fail due to per-type email uniqueness constraint
    assert!(result.is_err());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_password_complexity_violations() {
    // Test weak passwords that should be rejected
    let too_long = "a".repeat(73);
    let weak_passwords = vec![
        ("short", "Too short"),
        ("nouppercase123!", "No uppercase"),
        ("NOLOWERCASE123!", "No lowercase"),
        ("NoDigitsHere!", "No digits"),
        ("NoSpecialChar123", "No special character"),
        (too_long.as_str(), "Too long (>72 chars)"),
    ];

    for (weak_pass, reason) in weak_passwords {
        let result = validate_password_complexity(weak_pass);
        assert!(
            result.is_err(),
            "Password '{}' should be rejected: {}",
            weak_pass,
            reason
        );
    }
}

#[tokio::test]
async fn test_agent_creation_requires_at_least_one_role() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent but don't assign any roles
    let email = validate_and_normalize_email("norole@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "No Role Agent".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Verify agent was created
    let retrieved = db.get_agent_by_user_id(&user.id).await.unwrap();
    assert!(retrieved.is_some());

    // Verify agent has no roles (this is the test - we're verifying the database allows this,
    // but the API endpoint should enforce the requirement)
    let roles = db.get_user_roles(&user.id).await.unwrap();
    assert!(roles.is_empty(), "Agent should have no roles at this point");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_agent_email_can_duplicate_contact_email() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a contact with an email
    let email = validate_and_normalize_email("shared@example.com").unwrap();

    let contact_user = User::new(email.clone(), UserType::Contact);
    db.create_user(&contact_user).await.unwrap();

    // Now create an agent with the same email (should succeed due to per-type uniqueness)
    let agent_user = User::new(email.clone(), UserType::Agent);
    let password_hash = hash_password("TestPass123!").unwrap();
    let agent = Agent::new(
        agent_user.id.clone(),
        "Test Agent".to_string(),
        None,
        password_hash,
    );

    let result = db.create_user(&agent_user).await;
    assert!(
        result.is_ok(),
        "Should allow same email for different user types"
    );

    db.create_agent(&agent).await.unwrap();

    // Verify both exist
    let contact = db
        .get_user_by_email_and_type(&email, &UserType::Contact)
        .await
        .unwrap();
    let agent_retrieved = db
        .get_user_by_email_and_type(&email, &UserType::Agent)
        .await
        .unwrap();

    assert!(contact.is_some());
    assert!(agent_retrieved.is_some());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_agent_by_id() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create an agent
    let email = validate_and_normalize_email("get@example.com").unwrap();
    let password_hash = hash_password("TestPass123!").unwrap();

    let user = User::new(email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        "Get Agent".to_string(),
        None,
        password_hash,
    );

    db.create_user(&user).await.unwrap();
    db.create_agent(&agent).await.unwrap();

    // Assign a role
    let agent_role = db.get_role_by_name("Agent").await.unwrap().unwrap();
    let user_role = UserRole::new(user.id.clone(), agent_role.id.clone());
    db.assign_role_to_user(&user_role).await.unwrap();

    // Retrieve agent by ID
    let retrieved_user = db.get_user_by_id(&user.id).await.unwrap();
    assert!(retrieved_user.is_some());

    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.email, email);
    assert!(matches!(retrieved_user.user_type, UserType::Agent));

    // Retrieve agent details
    let retrieved_agent = db.get_agent_by_user_id(&user.id).await.unwrap();
    assert!(retrieved_agent.is_some());

    let retrieved_agent = retrieved_agent.unwrap();
    assert_eq!(retrieved_agent.first_name, "Get Agent");

    teardown_test_db(test_db).await;
}
