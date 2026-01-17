use oxidesk::domain::ports::agent_repository::AgentRepository;
use oxidesk::domain::ports::contact_repository::ContactRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
/// Feature 025: Mutual Exclusion Invariants - Integration Tests
///
/// This test suite validates that mutual exclusion constraints are properly enforced:
/// - US1 (P1): User type immutability (agent/contact cannot be changed)
/// - US2 (P1): SLA event status exclusivity (cannot be both met and breached)
/// - US3 (P2): Message type immutability (incoming/outgoing cannot be changed)
/// - US4 (P2): Protected role deletion prevention (Admin role cannot be deleted/modified)
mod helpers;

use helpers::test_db::setup_test_db;
use oxidesk::{database::Database, models::*, services::sla_service::SlaService};
use std::sync::Arc;

// ===== Test Helpers =====

async fn create_test_admin(db: &Database) -> (User, Agent) {
    let user = User::new("admin@test.com".to_string(), UserType::Agent);
    db.create_user(&user).await.unwrap();

    let password_hash = oxidesk::services::hash_password("TestPass123!").unwrap();
    let agent = Agent::new(
        user.id.clone(),
        "Test".to_string(),
        Some("Admin".to_string()),
        password_hash,
    );
    db.create_agent(&agent).await.unwrap();

    // Get Admin role and assign it
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO user_roles (user_id, role_id, created_at) VALUES (?, ?, ?)")
        .bind(&user.id)
        .bind(&admin_role.id)
        .bind(&now)
        .execute(db.pool())
        .await
        .unwrap();

    (user, agent)
}

async fn create_test_inbox(db: &Database) -> Inbox {
    let now = chrono::Utc::now().to_rfc3339();
    let inbox = Inbox {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Test Inbox".to_string(),
        channel_type: "email".to_string(),
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
        deleted_by: None,
    };
    db.create_inbox(&inbox).await.unwrap();
    inbox
}

async fn create_test_contact_with_inbox(db: &Database) -> (User, Contact, Inbox) {
    let inbox = create_test_inbox(db).await;

    let user = User::new("contact@test.com".to_string(), UserType::Contact);
    db.create_user(&user).await.unwrap();

    let contact = Contact::new(user.id.clone(), Some("Test Contact".to_string()));
    db.create_contact(&contact).await.unwrap();

    // Create contact channel
    let channel = ContactChannel::new(contact.id.clone(), inbox.id.clone(), user.email.clone());
    db.create_contact_channel(&channel).await.unwrap();

    (user, contact, inbox)
}

async fn create_test_conversation(db: &Database, contact_id: &str, inbox_id: &str) -> Conversation {
    let request = CreateConversation {
        inbox_id: inbox_id.to_string(),
        subject: Some("Test Conversation".to_string()),
        contact_id: contact_id.to_string(),
    };
    db.create_conversation(&request).await.unwrap()
}

async fn create_test_sla_policy(db: &Database) -> SlaPolicy {
    let policy = SlaPolicy::new(
        "Test SLA Policy".to_string(),
        Some("Test policy for mutual exclusion tests".to_string()),
        "2h".to_string(),
        "24h".to_string(),
        "4h".to_string(),
    );
    db.create_sla_policy(&policy).await.unwrap();
    policy
}

// ===== US2: SLA Event Status Exclusivity Tests (P1) =====

#[tokio::test]
async fn test_sla_event_cannot_be_met_and_breached() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let event_bus = Arc::new(oxidesk::LocalEventBus::new(100));
    let sla_service = SlaService::new(db.clone(), event_bus);

    // Create prerequisite data
    let (_user, contact, test_inbox) = create_test_contact_with_inbox(&db).await;
    let conversation = create_test_conversation(&db, &contact.id, &test_inbox.id).await;
    let policy = create_test_sla_policy(&db).await;

    // Apply SLA policy
    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get the first response event
    let events = db
        .get_sla_events_by_applied_sla(&applied_sla.id)
        .await
        .unwrap();
    let first_response_event = events
        .iter()
        .find(|e| matches!(e.event_type, SlaEventType::FirstResponse))
        .unwrap();

    // Mark the event as met
    let met_at = chrono::Utc::now().to_rfc3339();
    db.mark_sla_event_met(&first_response_event.id, &met_at)
        .await
        .unwrap();

    // Try to mark the same event as breached (should fail)
    let breached_at = chrono::Utc::now().to_rfc3339();
    let result = db
        .mark_sla_event_breached(&first_response_event.id, &breached_at)
        .await;

    assert!(
        result.is_err(),
        "Expected error when marking met event as breached"
    );
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("SLA event status is exclusive"),
        "Expected 'SLA event status is exclusive' error, got: {}",
        error
    );
}

#[tokio::test]
async fn test_sla_status_transition_pending_to_met() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let event_bus = Arc::new(oxidesk::LocalEventBus::new(100));
    let sla_service = SlaService::new(db.clone(), event_bus);

    // Create prerequisite data
    let (_user, contact, test_inbox) = create_test_contact_with_inbox(&db).await;
    let conversation = create_test_conversation(&db, &contact.id, &test_inbox.id).await;
    let policy = create_test_sla_policy(&db).await;

    // Apply SLA policy
    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get the first response event
    let events = db
        .get_sla_events_by_applied_sla(&applied_sla.id)
        .await
        .unwrap();
    let first_response_event = events
        .iter()
        .find(|e| matches!(e.event_type, SlaEventType::FirstResponse))
        .unwrap();

    // Verify event is pending
    assert_eq!(first_response_event.status, SlaEventStatus::Pending);
    assert!(first_response_event.met_at.is_none());
    assert!(first_response_event.breached_at.is_none());

    // Mark the event as met
    let met_at = chrono::Utc::now().to_rfc3339();
    db.mark_sla_event_met(&first_response_event.id, &met_at)
        .await
        .unwrap();

    // Verify event is now met
    let updated_event = db
        .get_sla_event(&first_response_event.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_event.status, SlaEventStatus::Met);
    assert!(updated_event.met_at.is_some());
    assert!(updated_event.breached_at.is_none());
}

#[tokio::test]
async fn test_sla_status_transition_pending_to_breached() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let event_bus = Arc::new(oxidesk::LocalEventBus::new(100));
    let sla_service = SlaService::new(db.clone(), event_bus);

    // Create prerequisite data
    let (_user, contact, test_inbox) = create_test_contact_with_inbox(&db).await;
    let conversation = create_test_conversation(&db, &contact.id, &test_inbox.id).await;
    let policy = create_test_sla_policy(&db).await;

    // Apply SLA policy
    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get the first response event
    let events = db
        .get_sla_events_by_applied_sla(&applied_sla.id)
        .await
        .unwrap();
    let first_response_event = events
        .iter()
        .find(|e| matches!(e.event_type, SlaEventType::FirstResponse))
        .unwrap();

    // Verify event is pending
    assert_eq!(first_response_event.status, SlaEventStatus::Pending);

    // Mark the event as breached
    let breached_at = chrono::Utc::now().to_rfc3339();
    db.mark_sla_event_breached(&first_response_event.id, &breached_at)
        .await
        .unwrap();

    // Verify event is now breached
    let updated_event = db
        .get_sla_event(&first_response_event.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_event.status, SlaEventStatus::Breached);
    assert!(updated_event.met_at.is_none());
    assert!(updated_event.breached_at.is_some());
}

#[tokio::test]
async fn test_sla_breached_cannot_become_met() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let event_bus = Arc::new(oxidesk::LocalEventBus::new(100));
    let sla_service = SlaService::new(db.clone(), event_bus);

    // Create prerequisite data
    let (_user, contact, test_inbox) = create_test_contact_with_inbox(&db).await;
    let conversation = create_test_conversation(&db, &contact.id, &test_inbox.id).await;
    let policy = create_test_sla_policy(&db).await;

    // Apply SLA policy
    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get the resolution event
    let events = db
        .get_sla_events_by_applied_sla(&applied_sla.id)
        .await
        .unwrap();
    let resolution_event = events
        .iter()
        .find(|e| matches!(e.event_type, SlaEventType::Resolution))
        .unwrap();

    // Mark the event as breached first
    let breached_at = chrono::Utc::now().to_rfc3339();
    db.mark_sla_event_breached(&resolution_event.id, &breached_at)
        .await
        .unwrap();

    // Try to mark as met (should fail)
    let met_at = chrono::Utc::now().to_rfc3339();
    let result = db.mark_sla_event_met(&resolution_event.id, &met_at).await;

    assert!(
        result.is_err(),
        "Expected error when marking breached event as met"
    );
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("SLA event status is exclusive"),
        "Expected 'SLA event status is exclusive' error, got: {}",
        error
    );
}

#[tokio::test]
async fn test_sla_status_exclusivity_error_message() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let event_bus = Arc::new(oxidesk::LocalEventBus::new(100));
    let sla_service = SlaService::new(db.clone(), event_bus);

    // Create prerequisite data
    let (_user, contact, test_inbox) = create_test_contact_with_inbox(&db).await;
    let conversation = create_test_conversation(&db, &contact.id, &test_inbox.id).await;
    let policy = create_test_sla_policy(&db).await;

    // Apply SLA policy
    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get an event
    let events = db
        .get_sla_events_by_applied_sla(&applied_sla.id)
        .await
        .unwrap();
    let event = &events[0];

    // Mark as met
    let met_at = chrono::Utc::now().to_rfc3339();
    db.mark_sla_event_met(&event.id, &met_at).await.unwrap();

    // Try to mark as breached
    let breached_at = chrono::Utc::now().to_rfc3339();
    let result = db.mark_sla_event_breached(&event.id, &breached_at).await;

    // Verify error message matches spec requirement (FR-008)
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("SLA event status is exclusive"),
        "Error message should contain 'SLA event status is exclusive', got: {}",
        error
    );
}

#[tokio::test]
async fn test_sla_status_timestamp_consistency() {
    let _db = setup_test_db().await;

    // Create a test SLA event using the model validation
    let applied_sla_id = uuid::Uuid::new_v4().to_string();
    let deadline_at = chrono::Utc::now().to_rfc3339();
    let mut event = SlaEvent::new(applied_sla_id, SlaEventType::FirstResponse, deadline_at);

    // Test 1: Pending status with no timestamps - should be valid
    assert!(event.validate_status_exclusive().is_ok());

    // Test 2: Met status requires met_at timestamp
    event.status = SlaEventStatus::Met;
    assert!(event.validate_status_exclusive().is_err());

    event.met_at = Some(chrono::Utc::now().to_rfc3339());
    assert!(event.validate_status_exclusive().is_ok());

    // Test 3: Breached status requires breached_at timestamp
    event = SlaEvent::new(
        uuid::Uuid::new_v4().to_string(),
        SlaEventType::FirstResponse,
        chrono::Utc::now().to_rfc3339(),
    );
    event.status = SlaEventStatus::Breached;
    assert!(event.validate_status_exclusive().is_err());

    event.breached_at = Some(chrono::Utc::now().to_rfc3339());
    assert!(event.validate_status_exclusive().is_ok());

    // Test 4: Cannot have both timestamps
    event.met_at = Some(chrono::Utc::now().to_rfc3339());
    event.breached_at = Some(chrono::Utc::now().to_rfc3339());
    let result = event.validate_status_exclusive();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "SLA event status is exclusive");
}

// ===== US1: User Type Immutability Tests (P1) =====

#[tokio::test]
async fn test_user_type_immutability_validation() {
    // Create a user with Agent type
    let user = User::new("test@example.com".to_string(), UserType::Agent);

    // Try to change to Contact (should fail validation)
    let result = user.validate_type_immutable(&UserType::Contact);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User type is immutable after creation");

    // Same type should pass
    let result = user.validate_type_immutable(&UserType::Agent);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_user_type_change_error_message() {
    let user = User::new("agent@example.com".to_string(), UserType::Agent);

    let result = user.validate_type_immutable(&UserType::Contact);

    // Verify exact error message matches spec requirement (FR-004)
    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert_eq!(
        error_message, "User type is immutable after creation",
        "Error message should exactly match spec requirement"
    );
}

// ===== US3: Message Type Immutability Tests (P2) =====

#[tokio::test]
async fn test_message_type_immutability_validation() {
    let conversation_id = uuid::Uuid::new_v4().to_string();
    let author_id = uuid::Uuid::new_v4().to_string();

    // Create an incoming message
    let message = Message::new_incoming(conversation_id, "Test message".to_string(), author_id);

    // Try to change to Outgoing (should fail validation)
    let result = message.validate_type_immutable(&MessageType::Outgoing);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Message type cannot be changed after creation"
    );

    // Same type should pass
    let result = message.validate_type_immutable(&MessageType::Incoming);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_message_type_change_error_message() {
    let conversation_id = uuid::Uuid::new_v4().to_string();
    let author_id = uuid::Uuid::new_v4().to_string();

    let message = Message::new_outgoing(conversation_id, "Test message".to_string(), author_id);

    let result = message.validate_type_immutable(&MessageType::Incoming);

    // Verify exact error message matches spec requirement (FR-012)
    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert_eq!(
        error_message, "Message type cannot be changed after creation",
        "Error message should exactly match spec requirement"
    );
}

// ===== US4: Protected Role Deletion Prevention Tests (P2) =====
// Note: Protected role enforcement is already tested in test_rbac_protected_roles.rs
// These tests verify the constraint exists at the model/service layer

#[tokio::test]
async fn test_protected_role_exists_and_is_marked() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Get Admin role
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();

    // Verify it's protected
    assert!(admin_role.is_protected, "Admin role should be protected");
    assert_eq!(
        admin_role.name, "Admin",
        "Protected role should be named Admin"
    );
}

#[tokio::test]
async fn test_admin_role_contains_admin_permissions() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Get Admin role
    let admin_role = db.get_role_by_name("Admin").await.unwrap().unwrap();

    // Verify it has administrative permissions
    assert!(
        !admin_role.permissions.is_empty(),
        "Admin role should have permissions"
    );
    assert!(admin_role.is_protected, "Admin role should be protected");
}
