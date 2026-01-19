use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
mod helpers;

use helpers::*;
use oxidesk::{
    domain::entities::{CreateAgentRequest, UserType},
    application::services::auth::verify_password,
    application::services::AgentService,
};

#[tokio::test]
async fn test_create_agent_service_flow() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // 1. Create Admin User
    let admin_user = create_test_auth_user(db).await;

    // 2. Request to create a new agent
    let request = CreateAgentRequest {
        email: format!("newagent-{}@example.com", uuid::Uuid::new_v4()),
        first_name: "New".to_string(),
        last_name: Some("Agent".to_string()),
        role_id: None, // Default to Agent role
    };

    // 3. Call create_agent
    let session_service = oxidesk::application::services::SessionService::new(std::sync::Arc::new(db.clone()));
    let agent_service =
        AgentService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::agent_repository::AgentRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::api_key_repository::ApiKeyRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::user_repository::UserRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::role_repository::RoleRepository>,
        session_service,
    );
    let response = agent_service
        .create_agent(&admin_user, request.clone())
        .await
        .expect("Failed to create agent");

    // 4. Verify Response
    assert_eq!(response.email, request.email);
    assert_eq!(response.first_name, "New");
    assert_eq!(response.last_name, Some("Agent".to_string()));
    assert_eq!(response.password.len(), 16); // Password returned
    println!("Generated Password: {}", response.password);

    // 5. Verify Database
    let created_user = db
        .get_user_by_email_and_type(&request.email, &UserType::Agent)
        .await
        .expect("DB error")
        .expect("User not found");

    // Verify password hash
    let agent_details = db
        .get_agent_by_user_id(&created_user.id)
        .await
        .expect("DB error")
        .expect("Agent details not found");

    assert!(
        verify_password(&response.password, &agent_details.password_hash)
            .expect("Verification error")
    );

    // Verify Role
    let roles = db.get_user_roles(&created_user.id).await.expect("DB error");
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "Agent"); // Default role

    teardown_test_db(test_db).await;
}
