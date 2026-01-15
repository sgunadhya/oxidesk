// Feature 023: Cardinality Invariants Integration Tests
//
// Tests for cardinality validation rules:
// - Agent Role Requirement: Agents must have at least one role
// - Conversation Contact Requirement: Conversations must have exactly one contact
// - Message Sender Requirement: Messages must have exactly one sender
// - Webhook Event Subscription: Webhooks must have at least one event
// - Role Permission Requirement: Roles must have at least one permission

use oxidesk::{database::Database, models::*, services::*};

mod helpers;
use helpers::*;

// Helper functions for testing
async fn create_test_inbox(db: &Database, inbox_id: &str) {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let result = sqlx::query(
        "INSERT INTO inboxes (id, name, channel_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(inbox_id)
    .bind("Test Inbox")
    .bind("email")
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await;

    if let Err(e) = result {
        eprintln!("Warning: Failed to insert inbox (may already exist): {}", e);
    }
}

async fn create_test_contact_with_inbox(
    db: &Database,
    inbox_id: &str,
    email: &str,
) -> (User, Contact) {
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: email.to_string(),
        user_type: UserType::Contact,
        created_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        updated_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        deleted_at: None,
        deleted_by: None,
    };

    db.create_user(&user).await.unwrap();

    let contact = Contact {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        first_name: Some("Test".to_string()),
    };

    sqlx::query("INSERT INTO contacts (id, user_id, first_name) VALUES (?, ?, ?)")
        .bind(&contact.id)
        .bind(&contact.user_id)
        .bind("Test")
        .execute(db.pool())
        .await
        .unwrap();

    // Create contact channel
    let channel_id = uuid::Uuid::new_v4().to_string();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    sqlx::query("INSERT INTO contact_channels (id, contact_id, inbox_id, email, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&channel_id)
        .bind(&contact.id)
        .bind(inbox_id)
        .bind(email)
        .bind(&now)
        .bind(&now)
        .execute(db.pool())
        .await
        .unwrap();

    (user, contact)
}

// ============================================================================
// User Story 1: Agent Role Requirement (P1)
// ============================================================================

#[tokio::test]
async fn test_agent_must_have_at_least_one_role_on_update() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // Create an agent with a role
    let agent_email = format!("agent-{}@test.com", uuid::Uuid::new_v4());
    let create_request = CreateAgentRequest {
        email: agent_email.clone(),
        first_name: "Test".to_string(),
        last_name: Some("Agent".to_string()),
        role_id: None, // Will use default role
    };

    let create_response = agent_service::create_agent(&db, &admin, create_request)
        .await
        .expect("Failed to create agent");

    // FR-002: Attempt to update agent with empty role_ids array
    let update_request = UpdateAgentRequest {
        first_name: "Updated".to_string(),
        role_ids: Some(vec![]), // Empty array - violates cardinality
    };

    let result =
        agent_service::update_agent(&db, &admin, &create_response.user_id, update_request).await;

    // FR-003: Verify rejection with specific error message
    assert!(result.is_err(), "Should reject agent update with no roles");
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Agent must be assigned at least one role"),
        "Expected cardinality error, got: {}",
        error
    );
}

// ============================================================================
// User Story 2: Conversation Contact Requirement (P1)
// ============================================================================

#[tokio::test]
async fn test_conversation_must_have_exactly_one_contact() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // Create inbox
    let inbox_id = &uuid::Uuid::new_v4().to_string();
    create_test_inbox(db, inbox_id).await;

    // FR-004: Attempt to create conversation with empty contact_id
    let request = CreateConversation {
        inbox_id: inbox_id.to_string(),
        contact_id: "".to_string(), // Empty string - violates cardinality
        subject: Some("Test".to_string()),
    };

    let result = conversation_service::create_conversation(&db, &admin, request, None).await;

    // FR-006: Verify rejection with specific error message
    assert!(
        result.is_err(),
        "Should reject conversation with empty contact_id"
    );
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Conversation must have exactly one contact"),
        "Expected error to contain 'Conversation must have exactly one contact', got: {}",
        error
    );
}

#[tokio::test]
async fn test_conversation_with_valid_contact_succeeds() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // Create inbox and contact
    let inbox_id = &uuid::Uuid::new_v4().to_string();
    create_test_inbox(db, inbox_id).await;
    let (user, _contact) = create_test_contact_with_inbox(db, inbox_id, "test@contact.com").await;

    // Create conversation with valid contact (use contact.id for FK reference)
    let request = CreateConversation {
        inbox_id: inbox_id.to_string(),
        contact_id: user.id.clone(), // Pass user_id, service converts to contact.id
        subject: Some("Test".to_string()),
    };

    let result = conversation_service::create_conversation(&db, &admin, request, None).await;

    assert!(
        result.is_ok(),
        "Should accept conversation with valid contact, got error: {:?}",
        result.err()
    );
}

// ============================================================================
// User Story 3: Message Sender Requirement (P2)
// ============================================================================

#[tokio::test]
async fn test_message_must_have_exactly_one_sender() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser
    let inbox_id = &uuid::Uuid::new_v4().to_string();
    create_test_inbox(db, inbox_id).await;
    let (user, _contact) = create_test_contact_with_inbox(db, inbox_id, "test@contact.com").await;

    let conv_request = CreateConversation {
        inbox_id: inbox_id.to_string(),
        contact_id: user.id.clone(), // Pass user_id, service converts to contact.id
        subject: Some("Test".to_string()),
    };
    let conversation = conversation_service::create_conversation(&db, &admin, conv_request, None)
        .await
        .expect("Failed to create conversation");

    // FR-007: Attempt to create message without sender (contact_id = None)
    let message_service = MessageService::new(db.clone());
    let incoming_request = IncomingMessageRequest {
        conversation_id: conversation.id.clone(),
        content: "Test message".to_string(),
        contact_id: None, // Missing sender - violates cardinality
        inbox_id: inbox_id.to_string(),
        from_header: None,
        external_id: None,
        received_at: None,
    };

    let result = message_service
        .create_incoming_message(incoming_request)
        .await;

    // FR-008: Verify rejection with specific error message
    assert!(result.is_err(), "Should reject message without sender");
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Message must have exactly one sender"),
        "Expected error to contain 'Message must have exactly one sender', got: {}",
        error
    );
}

#[tokio::test]
async fn test_message_with_valid_sender_succeeds() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser
    let inbox_id = &uuid::Uuid::new_v4().to_string();
    create_test_inbox(db, inbox_id).await;
    let (user, _contact) = create_test_contact_with_inbox(db, inbox_id, "test@contact.com").await;

    let conv_request = CreateConversation {
        inbox_id: inbox_id.to_string(),
        contact_id: user.id.clone(), // Pass user_id, service converts to contact.id
        subject: Some("Test".to_string()),
    };
    let conversation = conversation_service::create_conversation(&db, &admin, conv_request, None)
        .await
        .expect("Failed to create conversation");

    // Create message with valid sender
    let message_service = MessageService::new(db.clone());
    let incoming_request = IncomingMessageRequest {
        conversation_id: conversation.id.clone(),
        content: "Test message".to_string(),
        contact_id: Some(user.id.clone()), // Valid sender (references users.id)
        inbox_id: inbox_id.to_string(),
        from_header: None,
        external_id: None,
        received_at: None,
    };

    let result = message_service
        .create_incoming_message(incoming_request)
        .await;

    assert!(
        result.is_ok(),
        "Should accept message with valid sender, got error: {:?}",
        result.err()
    );
}

// ============================================================================
// User Story 4: Webhook Event Subscription (P2)
// ============================================================================

#[tokio::test]
async fn test_webhook_must_have_at_least_one_event_on_create() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let webhook_service = WebhookService::new(db.clone());

    // FR-009: Attempt to create webhook with empty events array
    let request = CreateWebhookRequest {
        name: "Test Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        subscribed_events: vec![], // Empty array - violates cardinality
        secret: "secret1234567890abcdef".to_string(),
        is_active: Some(true),
    };

    let result = webhook_service.create_webhook(request, "admin-123").await;

    // FR-011: Verify rejection with specific error message
    assert!(result.is_err(), "Should reject webhook with no events");
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Webhook must subscribe to at least one event"),
        "Expected error to contain 'Webhook must subscribe to at least one event', got: {}",
        error
    );
}

#[tokio::test]
#[ignore]
async fn test_webhook_must_have_at_least_one_event_on_update() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user for webhook created_by FK
    let admin = create_test_auth_user(db).await;

    let webhook_service = WebhookService::new(db.clone());

    // Create webhook with valid events
    let create_request = CreateWebhookRequest {
        name: "Test Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        subscribed_events: vec!["conversation.created".to_string()],
        secret: "secret1234567890abcdef".to_string(),
        is_active: Some(true),
    };

    let webhook = webhook_service
        .create_webhook(create_request, &admin.user.id)
        .await
        .expect("Failed to create webhook");

    // FR-010: Attempt to update webhook with empty events array
    let update_request = UpdateWebhookRequest {
        name: None,
        url: None,
        subscribed_events: Some(vec![]), // Empty array - violates cardinality
        secret: None,
        is_active: None,
    };

    let result = webhook_service
        .update_webhook(&webhook.id, update_request)
        .await;

    // FR-011: Verify rejection with specific error message
    assert!(
        result.is_err(),
        "Should reject webhook update with no events"
    );
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Webhook must subscribe to at least one event"),
        "Expected error to contain 'Webhook must subscribe to at least one event', got: {}",
        error
    );
}

// ============================================================================
// User Story 5: Role Permission Requirement (P3)
// ============================================================================

#[tokio::test]
async fn test_role_must_have_at_least_one_permission_on_create() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // FR-012: Attempt to create role with empty permissions array
    let request = CreateRoleRequest {
        name: "Test Role".to_string(),
        description: Some("Test description".to_string()),
        permissions: vec![], // Empty array - violates cardinality
    };

    let result = role_service::create_role(&db, &admin, request).await;

    // FR-014: Verify rejection with specific error message
    assert!(result.is_err(), "Should reject role with no permissions");
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Role must have at least one permission"),
        "Expected error to contain 'Role must have at least one permission', got: {}",
        error
    );
}

#[tokio::test]
async fn test_role_must_have_at_least_one_permission_on_update() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // Create role with valid permissions
    let create_request = CreateRoleRequest {
        name: format!("Test Role {}", uuid::Uuid::new_v4()),
        description: Some("Test description".to_string()),
        permissions: vec!["conversations:read".to_string()],
    };

    let role = role_service::create_role(&db, &admin, create_request)
        .await
        .expect("Failed to create role");

    // FR-013: Attempt to update role with empty permissions array
    let update_request = UpdateRoleRequest {
        name: None,
        description: None,
        permissions: Some(vec![]), // Empty array - violates cardinality
    };

    let result = role_service::update_role(&db, &admin, &role.id, update_request).await;

    // FR-014: Verify rejection with specific error message
    assert!(
        result.is_err(),
        "Should reject role update with no permissions"
    );
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Role must have at least one permission"),
        "Expected error to contain 'Role must have at least one permission', got: {}",
        error
    );
}

#[tokio::test]
async fn test_role_with_valid_permissions_succeeds() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // Create role with valid permissions
    let request = CreateRoleRequest {
        name: format!("Test Role {}", uuid::Uuid::new_v4()),
        description: Some("Test description".to_string()),
        permissions: vec![
            "conversations:read".to_string(),
            "messages:write".to_string(),
        ],
    };

    let result = role_service::create_role(&db, &admin, request).await;

    assert!(result.is_ok(), "Should accept role with valid permissions");
}

// ============================================================================
// Edge Cases (from spec.md)
// ============================================================================

#[tokio::test]
async fn test_entity_deletion_bypasses_cardinality_validation() {
    // Setup
    // Setup test database
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create admin user
    let admin = create_test_auth_user(db).await;
    // admin is already AuthenticatedUser

    // Create an agent with one role
    let agent_email = format!("agent-{}@test.com", uuid::Uuid::new_v4());
    let create_request = CreateAgentRequest {
        email: agent_email,
        first_name: "Test".to_string(),
        last_name: Some("Agent".to_string()),
        role_id: None,
    };

    let create_response = agent_service::create_agent(&db, &admin, create_request)
        .await
        .expect("Failed to create agent");

    // FR-017: Deletion should succeed even though agent has roles
    // (This tests that deletion is exempt from cardinality validation)
    let result = agent_service::delete(&db, &admin, &create_response.user_id).await;

    assert!(
        result.is_ok(),
        "Entity deletion should bypass cardinality checks"
    );
}
