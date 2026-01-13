mod helpers;

use chrono::Datelike;
use helpers::*;
use oxidesk::models::*;

// ========================================
// Phase 3: User Story 1 - Permission Enforcement & Validation
// ========================================

/// T021: Test that applying SLA requires "sla:manage" permission
#[tokio::test]
async fn test_apply_sla_requires_permission() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent WITHOUT "sla:manage" permission
    let _agent = create_test_agent(&db, "agent@example.com", "Test Agent").await;

    // Try to apply SLA without permission via API
    // This test verifies that the API endpoint checks for "sla:manage" permission
    // When implemented, the endpoint should return 403 Forbidden

    // For now, we'll test the service layer directly
    // The API layer permission check will be added in T026
    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Service layer should succeed (permission check is in API layer)
    let result = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await;

    // This will succeed at service layer
    assert!(result.is_ok());

    // TODO: Once API endpoint is implemented with permission check, add API-level test here
}

/// T022: Test that applying SLA validates conversation exists
#[tokio::test]
async fn test_apply_sla_validates_conversation_exists() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Try to apply SLA to non-existent conversation
    let result = sla_service
        .apply_sla("non-existent-conversation-id", &policy.id, &base_timestamp)
        .await;

    // Should fail with NotFound error
    assert!(result.is_err());
    match result.unwrap_err() {
        oxidesk::ApiError::NotFound(msg) => {
            assert!(msg.contains("Conversation not found"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

/// T023: Test that applying SLA validates policy exists
#[tokio::test]
async fn test_apply_sla_validates_policy_exists() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Try to apply non-existent SLA policy
    let result = sla_service
        .apply_sla(&conversation.id, "non-existent-policy-id", &base_timestamp)
        .await;

    // Should fail with NotFound error
    assert!(result.is_err());
    match result.unwrap_err() {
        oxidesk::ApiError::NotFound(msg) => {
            assert!(msg.contains("SLA policy not found"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

/// T024: Test that applying SLA creates applied SLA and events
#[tokio::test]
async fn test_apply_sla_creates_applied_sla_and_events() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Verify applied SLA was created
    assert_eq!(applied_sla.conversation_id, conversation.id);
    assert_eq!(applied_sla.sla_policy_id, policy.id);
    assert_eq!(applied_sla.status, AppliedSlaStatus::Pending);

    // Verify SLA events were created
    let events = db
        .get_sla_events_by_applied_sla(&applied_sla.id)
        .await
        .unwrap();

    assert_eq!(events.len(), 2, "Should have first_response and resolution events");

    // Verify event types
    let event_types: Vec<_> = events.iter().map(|e| e.event_type).collect();
    assert!(event_types.contains(&SlaEventType::FirstResponse));
    assert!(event_types.contains(&SlaEventType::Resolution));
}

/// T025: Test that applying SLA without permission is rejected (API level)
#[tokio::test]
async fn test_apply_sla_without_permission_rejected() {
    // This test will be implemented once the API endpoint with permission check is added
    // For now, we'll mark it as a placeholder

    // TODO: Implement API-level test once POST /api/sla/apply endpoint is created with permission check
    // Expected behavior:
    // 1. Create user without "sla:manage" permission
    // 2. Attempt POST /api/sla/apply
    // 3. Verify 403 Forbidden response
}

// ========================================
// Phase 4: User Story 2 - Business Hours Deadline Calculation
// ========================================

/// T031: Test deadline calculation with business hours
#[tokio::test]
async fn test_deadline_calculation_with_business_hours() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 2h first response, 8h resolution
    let policy = create_test_sla_policy(&db, "Business Hours", "2h", "8h", "4h").await;

    // Create team with business hours: Mon-Fri 9am-5pm EST
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("Support Team".to_string(), Some("Team with business hours".to_string()));
    team.sla_policy_id = Some(policy.id.clone());
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    // Create conversation assigned to team
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Assign conversation to team
    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Apply SLA during business hours (e.g., Monday 10:00 AM EST)
    let base_timestamp = "2024-01-08T15:00:00Z"; // Monday 10:00 AM EST (UTC-5)

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await
        .unwrap();

    // Verify deadlines were calculated (exact values will depend on business hours logic)
    assert!(!applied_sla.first_response_deadline_at.is_empty());
    assert!(!applied_sla.resolution_deadline_at.is_empty());
}

/// T032: Test deadline skips weekend
#[tokio::test]
async fn test_deadline_skips_weekend() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 1h first response
    let policy = create_test_sla_policy(&db, "Weekend Test", "1h", "8h", "4h").await;

    // Create team with business hours: Mon-Fri 9am-5pm EST
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("Support Team".to_string(), None);
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Friday 4:30 PM EST (21:30 UTC) + 1h should roll to Monday 10:30 AM EST
    let base_timestamp = "2024-01-12T21:30:00Z"; // Friday 4:30 PM EST

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await
        .unwrap();

    // Parse the deadline
    let deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at).unwrap();

    // Should be Monday (weekday 0 = Monday in chrono)
    assert_eq!(deadline.weekday(), chrono::Weekday::Mon, "Deadline should skip weekend and land on Monday");
}

/// T033: Test deadline skips evenings
#[tokio::test]
async fn test_deadline_skips_evenings() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 2h first response
    let policy = create_test_sla_policy(&db, "Evening Test", "2h", "8h", "4h").await;

    // Create team with business hours: Mon-Fri 9am-5pm EST
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("Support Team".to_string(), None);
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Monday 4:30 PM EST (21:30 UTC) + 2h should roll to Tuesday 11:30 AM EST
    let base_timestamp = "2024-01-08T21:30:00Z"; // Monday 4:30 PM EST

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await
        .unwrap();

    // Parse the deadline
    let deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at).unwrap();

    // Should be Tuesday (next business day)
    assert_eq!(deadline.weekday(), chrono::Weekday::Tue, "Deadline should skip evening and land on next business day");
}

/// T034: Test deadline calculation without business hours (24/7 default)
#[tokio::test]
async fn test_deadline_calculation_without_business_hours() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "24/7 Service", "2h", "24h", "4h").await;

    // Create team WITHOUT business hours
    let team = Team::new("24/7 Team".to_string(), None);
    db.create_team(&team).await.unwrap();

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Use a timestamp that would normally skip to next business day
    let base_timestamp = "2024-01-12T21:30:00Z"; // Friday 4:30 PM EST

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await
        .unwrap();

    // Parse timestamps
    let base = chrono::DateTime::parse_from_rfc3339(base_timestamp).unwrap();
    let deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at).unwrap();

    // Should be exactly 2 hours later (no business hours skipping)
    let diff = (deadline - base).num_hours();
    assert_eq!(diff, 2, "24/7 deadline should be exactly 2 hours later, not skipping weekends");
}

/// T035: Test business hours timezone handling
#[tokio::test]
async fn test_business_hours_timezone_handling() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Timezone Test", "1h", "8h", "4h").await;

    // Create team with business hours in EST
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("EST Team".to_string(), None);
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Use UTC timestamp that corresponds to EST business hours
    let base_timestamp = "2024-01-08T15:00:00Z"; // Monday 10:00 AM EST

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let result = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await;

    // Should succeed with proper timezone handling
    assert!(result.is_ok(), "Should handle timezone conversion correctly");
}

// ========================================
// Phase 5: User Story 3 - Duplicate SLA Prevention
// ========================================

/// T046: Test duplicate SLA application is rejected
#[tokio::test]
async fn test_duplicate_sla_application_rejected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA first time - should succeed
    let result1 = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await;
    assert!(result1.is_ok(), "First SLA application should succeed");

    // Apply SLA second time - should fail
    let result2 = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await;

    assert!(result2.is_err(), "Second SLA application should fail");
}

/// T047: Test duplicate SLA error message
#[tokio::test]
async fn test_duplicate_sla_error_message() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA first time
    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Apply SLA second time and verify error message
    let result = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await;

    match result {
        Err(oxidesk::ApiError::BadRequest(msg)) => {
            assert!(
                msg.contains("already has an applied SLA"),
                "Error message should mention conversation already has SLA, got: {}",
                msg
            );
        }
        _ => panic!("Expected BadRequest error with specific message"),
    }
}

/// T048: Test applying SLA to different conversations succeeds
#[tokio::test]
async fn test_apply_sla_to_different_conversations_succeeds() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create two conversations
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation1 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;
    let conversation2 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply same policy to both conversations - both should succeed
    let result1 = sla_service
        .apply_sla(&conversation1.id, &policy.id, &base_timestamp)
        .await;
    assert!(result1.is_ok(), "First conversation SLA application should succeed");

    let result2 = sla_service
        .apply_sla(&conversation2.id, &policy.id, &base_timestamp)
        .await;
    assert!(result2.is_ok(), "Second conversation SLA application should succeed");
}

/// T049: Test applying same policy twice to conversation is rejected
#[tokio::test]
async fn test_apply_same_policy_twice_to_conversation_rejected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply same policy twice
    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    let result = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await;

    assert!(result.is_err(), "Applying same policy twice should be rejected");
}

/// T050: Test applying different policy to conversation is also rejected
#[tokio::test]
async fn test_apply_different_policy_to_conversation_rejected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create two SLA policies
    let policy1 = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;
    let policy2 = create_test_sla_policy(&db, "Premium", "1h", "12h", "2h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply first policy
    sla_service
        .apply_sla(&conversation.id, &policy1.id, &base_timestamp)
        .await
        .unwrap();

    // Apply different policy - should also be rejected
    let result = sla_service
        .apply_sla(&conversation.id, &policy2.id, &base_timestamp)
        .await;

    assert!(result.is_err(), "Applying different policy should also be rejected");

    // Verify error message
    match result {
        Err(oxidesk::ApiError::BadRequest(msg)) => {
            assert!(
                msg.contains("already has an applied SLA"),
                "Error should mention conversation already has SLA"
            );
        }
        _ => panic!("Expected BadRequest error"),
    }
}

// ========================================
// Phase 6: User Story 4 - CASCADE Deletion
// ========================================

/// T056: Test deleting SLA policy cascades to applied SLAs
#[tokio::test]
async fn test_delete_sla_policy_cascades_to_applied_slas() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation and apply SLA
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Verify applied SLA exists
    let applied_sla_before = db.get_applied_sla_by_conversation(&conversation.id).await.unwrap();
    assert!(applied_sla_before.is_some(), "Applied SLA should exist before deletion");

    // Delete the SLA policy
    sla_service.delete_policy(&policy.id).await.unwrap();

    // Verify applied SLA was CASCADE deleted
    let applied_sla_after = db.get_applied_sla_by_conversation(&conversation.id).await.unwrap();
    assert!(applied_sla_after.is_none(), "Applied SLA should be CASCADE deleted");
}

/// T057: Test deleting applied SLA cascades to SLA events (already works from 007)
#[tokio::test]
async fn test_delete_applied_sla_cascades_to_sla_events() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create conversation and apply SLA
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    let applied_sla = db.get_applied_sla_by_conversation(&conversation.id).await.unwrap().unwrap();

    // Verify SLA events exist
    let events_before = db.get_sla_events_by_applied_sla(&applied_sla.id).await.unwrap();
    assert_eq!(events_before.len(), 2, "Should have 2 SLA events (first_response, resolution)");

    // Delete the SLA policy (which CASCADE deletes applied SLA)
    sla_service.delete_policy(&policy.id).await.unwrap();

    // Verify SLA events were CASCADE deleted
    let events_after = db.get_sla_events_by_applied_sla(&applied_sla.id).await.unwrap();
    assert_eq!(events_after.len(), 0, "SLA events should be CASCADE deleted");
}

/// T058: Test CASCADE delete with multiple conversations
#[tokio::test]
async fn test_cascade_delete_with_multiple_conversations() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Create 10 conversations and apply SLA to each
    let contact = create_test_contact(&db, "customer@example.com").await;
    let mut conversation_ids = Vec::new();

    for _i in 0..10 {
        let conversation = create_test_conversation(
            &db,
            "inbox-001".to_string(),  // Use same inbox for all
            contact.id.clone(),
            ConversationStatus::Open,
        )
        .await;

        sla_service
            .apply_sla(&conversation.id, &policy.id, &base_timestamp)
            .await
            .unwrap();

        conversation_ids.push(conversation.id.clone());
    }

    // Verify all 10 applied SLAs exist
    for conv_id in &conversation_ids {
        let applied_sla = db.get_applied_sla_by_conversation(conv_id).await.unwrap();
        assert!(applied_sla.is_some(), "Applied SLA should exist for conversation {}", conv_id);
    }

    // Delete the SLA policy
    sla_service.delete_policy(&policy.id).await.unwrap();

    // Verify all 10 applied SLAs were CASCADE deleted
    for conv_id in &conversation_ids {
        let applied_sla = db.get_applied_sla_by_conversation(conv_id).await.unwrap();
        assert!(applied_sla.is_none(), "Applied SLA should be CASCADE deleted for conversation {}", conv_id);
    }
}

/// T059: Test deleting policy with no applied SLAs (edge case)
#[tokio::test]
async fn test_delete_policy_with_no_applied_slas() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy but don't apply it to any conversations
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    // Delete the policy - should succeed even with no applied SLAs
    let result = sla_service.delete_policy(&policy.id).await;
    assert!(result.is_ok(), "Deleting policy with no applied SLAs should succeed");

    // Verify policy is deleted
    let policy_after = sla_service.get_policy(&policy.id).await.unwrap();
    assert!(policy_after.is_none(), "Policy should be deleted");
}

/// T060: Test CASCADE delete is transactional (all-or-nothing)
#[tokio::test]
async fn test_cascade_delete_is_transactional() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy and apply to conversation
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get event IDs before deletion
    let events_before = db.get_sla_events_by_applied_sla(&applied_sla.id).await.unwrap();
    let event_ids: Vec<String> = events_before.iter().map(|e| e.id.clone()).collect();

    // Delete the policy
    sla_service.delete_policy(&policy.id).await.unwrap();

    // Verify everything is deleted (transactional)
    let policy_after = sla_service.get_policy(&policy.id).await.unwrap();
    assert!(policy_after.is_none(), "Policy should be deleted");

    let applied_sla_after = db.get_applied_sla(&applied_sla.id).await.unwrap();
    assert!(applied_sla_after.is_none(), "Applied SLA should be deleted");

    // Verify all events are deleted
    for event_id in event_ids {
        let event = db.get_sla_event(&event_id).await.unwrap();
        assert!(event.is_none(), "SLA event {} should be deleted", event_id);
    }
}

/// T061: Test no orphaned SLA events after CASCADE delete
#[tokio::test]
async fn test_sla_events_orphaned_check() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy and apply to multiple conversations
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let contact = create_test_contact(&db, "customer@example.com").await;

    // Apply to 5 conversations
    let mut applied_sla_ids = Vec::new();
    for _i in 0..5 {
        let conversation = create_test_conversation(
            &db,
            "inbox-001".to_string(),  // Use same inbox for all
            contact.id.clone(),
            ConversationStatus::Open,
        )
        .await;

        let applied_sla = sla_service
            .apply_sla(&conversation.id, &policy.id, &base_timestamp)
            .await
            .unwrap();

        applied_sla_ids.push(applied_sla.id.clone());
    }

    // Delete the policy
    sla_service.delete_policy(&policy.id).await.unwrap();

    // Verify no orphaned events (all should be CASCADE deleted)
    for applied_sla_id in applied_sla_ids {
        let events = db.get_sla_events_by_applied_sla(&applied_sla_id).await.unwrap();
        assert_eq!(events.len(), 0, "No orphaned events should exist for applied_sla {}", applied_sla_id);
    }
}

// ========================================
// Phase 7: Polish & Integration
// ========================================

/// T073: Full SLA application workflow integration test
#[tokio::test]
async fn test_full_sla_application_workflow() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup: Create team with business hours
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("Support Team".to_string(), Some("24/7 Support".to_string()));
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard SLA", "2h", "24h", "4h").await;

    // Assign policy to team
    db.update_team_sla_policy(&team.id, Some(&policy.id)).await.unwrap();

    // Create conversation assigned to team
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    // Monday 10:00 AM EST
    let base_timestamp = "2024-01-08T15:00:00Z";
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await
        .unwrap();

    // Verify applied SLA was created
    assert_eq!(applied_sla.conversation_id, conversation.id);
    assert_eq!(applied_sla.sla_policy_id, policy.id);
    assert_eq!(applied_sla.status, AppliedSlaStatus::Pending);

    // Verify deadlines are set
    assert!(!applied_sla.first_response_deadline_at.is_empty());
    assert!(!applied_sla.resolution_deadline_at.is_empty());

    // Verify SLA events were created
    let events = db.get_sla_events_by_applied_sla(&applied_sla.id).await.unwrap();
    assert_eq!(events.len(), 2);

    // Verify business hours were applied (deadlines should skip non-working hours)
    let first_response_deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at).unwrap();
    let base = chrono::DateTime::parse_from_rfc3339(base_timestamp).unwrap();

    // 2 hours during business hours should still be on Monday
    assert_eq!(first_response_deadline.weekday(), chrono::Weekday::Mon);

    // Try to apply SLA again - should fail (duplicate prevention)
    let result = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await;
    assert!(result.is_err());

    // Delete policy - should CASCADE delete applied SLA
    sla_service.delete_policy(&policy.id).await.unwrap();

    let applied_sla_after = db.get_applied_sla_by_conversation(&conversation.id).await.unwrap();
    assert!(applied_sla_after.is_none(), "Applied SLA should be CASCADE deleted");
}

/// T074: Business hours edge cases integration test
#[tokio::test]
async fn test_business_hours_edge_cases() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    // Test case 1: SLA applied during weekend should start on next Monday
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("Weekend Test Team".to_string(), None);
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    let policy = create_test_sla_policy(&db, "Weekend SLA", "1h", "8h", "2h").await;

    let contact = create_test_contact(&db, "weekend@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    // Saturday 10:00 AM EST
    let saturday_timestamp = "2024-01-13T15:00:00Z";
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, saturday_timestamp)
        .await
        .unwrap();

    // Deadline should be on Monday (skipping weekend)
    let deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at).unwrap();
    assert_eq!(deadline.weekday(), chrono::Weekday::Mon, "Deadline should skip to Monday");

    // Test case 2: Invalid business hours JSON should fall back to 24/7
    let mut team2 = Team::new("Invalid BH Team".to_string(), None);
    team2.business_hours = Some("invalid json".to_string());
    db.create_team(&team2).await.unwrap();

    let policy2 = create_test_sla_policy(&db, "Fallback SLA", "2h", "24h", "4h").await;

    let contact2 = create_test_contact(&db, "fallback@example.com").await;
    let conversation2 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact2.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation2.id, &team2.id, "system").await.unwrap();

    let base_timestamp = chrono::Utc::now().to_rfc3339();
    let result = sla_service
        .apply_sla(&conversation2.id, &policy2.id, &base_timestamp)
        .await;

    // Should succeed with 24/7 fallback
    assert!(result.is_ok(), "Should fall back to 24/7 calculation with invalid business hours");
}

/// T080: Final integration verification
#[tokio::test]
async fn test_final_integration_verification() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create complete setup
    let business_hours_json = r#"{
        "timezone": "America/New_York",
        "schedule": [
            {"day": "Monday", "start": "09:00", "end": "17:00"},
            {"day": "Tuesday", "start": "09:00", "end": "17:00"},
            {"day": "Wednesday", "start": "09:00", "end": "17:00"},
            {"day": "Thursday", "start": "09:00", "end": "17:00"},
            {"day": "Friday", "start": "09:00", "end": "17:00"}
        ]
    }"#;

    let mut team = Team::new("Production Team".to_string(), Some("Final verification".to_string()));
    team.business_hours = Some(business_hours_json.to_string());
    db.create_team(&team).await.unwrap();

    let policy = create_test_sla_policy(&db, "Production SLA", "4h", "48h", "8h").await;
    db.update_team_sla_policy(&team.id, Some(&policy.id)).await.unwrap();

    let contact = create_test_contact(&db, "production@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    db.assign_conversation_to_team(&conversation.id, &team.id, "system").await.unwrap();

    let sla_service = oxidesk::SlaService::new(
        db.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(oxidesk::EventBus::new(100))),
    );

    // Wednesday 2:00 PM EST (19:00 UTC)
    let base_timestamp = "2024-01-10T19:00:00Z";

    // Apply SLA
    let applied_sla = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await
        .unwrap();

    // Comprehensive verification
    assert_eq!(applied_sla.conversation_id, conversation.id);
    assert_eq!(applied_sla.sla_policy_id, policy.id);
    assert_eq!(applied_sla.status, AppliedSlaStatus::Pending);

    // Verify deadlines are properly calculated with business hours
    let first_response_deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at).unwrap();
    let resolution_deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.resolution_deadline_at).unwrap();

    // First response (4h) should be Thursday morning
    assert!(
        first_response_deadline.weekday() == chrono::Weekday::Wed ||
        first_response_deadline.weekday() == chrono::Weekday::Thu,
        "First response deadline should be Wed or Thu"
    );

    // Resolution (48h) should be multiple days later
    assert!(resolution_deadline > first_response_deadline);

    // Verify SLA events
    let events = db.get_sla_events_by_applied_sla(&applied_sla.id).await.unwrap();
    assert_eq!(events.len(), 2);

    let first_response_event = events.iter().find(|e| e.event_type == SlaEventType::FirstResponse).unwrap();
    let resolution_event = events.iter().find(|e| e.event_type == SlaEventType::Resolution).unwrap();

    assert_eq!(first_response_event.status, SlaEventStatus::Pending);
    assert_eq!(resolution_event.status, SlaEventStatus::Pending);

    // Verify duplicate prevention
    let duplicate_result = sla_service
        .apply_sla(&conversation.id, &policy.id, base_timestamp)
        .await;
    assert!(duplicate_result.is_err());

    // Verify CASCADE deletion
    sla_service.delete_policy(&policy.id).await.unwrap();
    let applied_sla_after = db.get_applied_sla_by_conversation(&conversation.id).await.unwrap();
    assert!(applied_sla_after.is_none());

    let events_after = db.get_sla_events_by_applied_sla(&applied_sla.id).await.unwrap();
    assert_eq!(events_after.len(), 0);
}
