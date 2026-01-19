mod helpers;

use chrono::Utc;
use helpers::*;
use oxidesk::domain::entities::*;

// ========================================
// Phase 3: User Story 1 - SLA Application Tests
// ========================================

#[tokio::test]
async fn test_apply_sla_on_team_assignment() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "2h", "24h", "4h").await;

    // Create team and assign SLA policy
    let team = Team::new(
        "Support Team".to_string(),
        Some("Main support team".to_string()),
    );
    db.create_team(&team).await.unwrap();
    db.update_team_sla_policy(&team.id, Some(&policy.id))
        .await
        .unwrap();

    // Create conversation using helper
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Get the team with SLA policy
    let team_with_sla = db.get_team_by_id(&team.id).await.unwrap().unwrap();
    assert_eq!(team_with_sla.sla_policy_id, Some(policy.id.clone()));

    // Apply SLA (this would normally be called by assignment service)
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Verify applied SLA was created
    let applied_sla = get_applied_sla(&db, &conversation.id).await;
    assert!(applied_sla.is_some(), "Applied SLA should be created");

    let applied_sla = applied_sla.unwrap();
    assert_eq!(applied_sla.conversation_id, conversation.id);
    assert_eq!(applied_sla.sla_policy_id, policy.id);
    assert_eq!(applied_sla.status, AppliedSlaStatus::Pending);
}

#[tokio::test]
async fn test_first_response_deadline_calculation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 2 hour first response time
    let policy = create_test_sla_policy(&db, "Fast Response", "2h", "24h", "4h").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get applied SLA and verify first response deadline
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Calculate expected deadline
    let expected_deadline = calculate_deadline(
        chrono::DateTime::parse_from_rfc3339(&base_timestamp)
            .unwrap()
            .with_timezone(&Utc),
        "2h",
    );

    let actual_deadline =
        chrono::DateTime::parse_from_rfc3339(&applied_sla.first_response_deadline_at)
            .unwrap()
            .with_timezone(&Utc);

    // Allow 1 second tolerance for timing differences
    let diff = (expected_deadline - actual_deadline).num_seconds().abs();
    assert!(
        diff <= 1,
        "First response deadline should be conversation created_at + 2h"
    );
}

#[tokio::test]
async fn test_resolution_deadline_calculation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 24 hour resolution time
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

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get applied SLA and verify resolution deadline
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Calculate expected deadline
    let expected_deadline = calculate_deadline(
        chrono::DateTime::parse_from_rfc3339(&base_timestamp)
            .unwrap()
            .with_timezone(&Utc),
        "24h",
    );

    let actual_deadline = chrono::DateTime::parse_from_rfc3339(&applied_sla.resolution_deadline_at)
        .unwrap()
        .with_timezone(&Utc);

    // Allow 1 second tolerance
    let diff = (expected_deadline - actual_deadline).num_seconds().abs();
    assert!(
        diff <= 1,
        "Resolution deadline should be conversation created_at + 24h"
    );
}

#[tokio::test]
async fn test_sla_events_created_as_pending() {
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

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Get SLA events
    let events = get_sla_events(&db, &applied_sla.id).await;

    // Verify two events were created (first_response and resolution)
    assert_eq!(events.len(), 2, "Should create 2 SLA events");

    // Verify first response event
    let first_response_event = events
        .iter()
        .find(|e| e.event_type == SlaEventType::FirstResponse)
        .expect("First response event should exist");

    assert_eq!(first_response_event.status, SlaEventStatus::Pending);
    assert_eq!(
        first_response_event.deadline_at,
        applied_sla.first_response_deadline_at
    );
    assert!(first_response_event.met_at.is_none());
    assert!(first_response_event.breached_at.is_none());

    // Verify resolution event
    let resolution_event = events
        .iter()
        .find(|e| e.event_type == SlaEventType::Resolution)
        .expect("Resolution event should exist");

    assert_eq!(resolution_event.status, SlaEventStatus::Pending);
    assert_eq!(
        resolution_event.deadline_at,
        applied_sla.resolution_deadline_at
    );
    assert!(resolution_event.met_at.is_none());
    assert!(resolution_event.breached_at.is_none());
}

#[tokio::test]
async fn test_no_sla_if_team_has_no_policy() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create team without SLA policy
    let team = Team::new(
        "Support Team".to_string(),
        Some("Main support team".to_string()),
    );
    db.create_team(&team).await.unwrap();

    // Verify team has no SLA policy
    let team_from_db = db.get_team_by_id(&team.id).await.unwrap().unwrap();
    assert!(team_from_db.sla_policy_id.is_none());

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Assign conversation to team (this would normally trigger SLA application)
    // But since team has no SLA policy, no SLA should be applied

    // Verify no applied SLA exists
    let applied_sla = get_applied_sla(&db, &conversation.id).await;
    assert!(
        applied_sla.is_none(),
        "No SLA should be applied when team has no policy"
    );
}

// ========================================
// Phase 4: User Story 2 - First Response Met Tests
// ========================================

#[tokio::test]
async fn test_first_response_met_on_agent_message() {
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

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Create agent user
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    // Agent sends a message (simulating first response)
    let message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &message_timestamp)
        .await
        .unwrap();

    // Get first response event
    let events = get_sla_events(&db, &applied_sla.id).await;
    let first_response_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");

    // Verify event is marked as met
    assert_eq!(first_response_event.status, oxidesk::SlaEventStatus::Met);
    assert!(
        first_response_event.met_at.is_some(),
        "met_at should be set"
    );
    assert!(
        first_response_event.breached_at.is_none(),
        "breached_at should not be set"
    );
}

#[tokio::test]
async fn test_first_response_met_timestamp_recorded() {
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

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Create agent user
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    // Record the time when message is sent
    let message_timestamp = chrono::Utc::now().to_rfc3339();

    // Agent sends a message
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &message_timestamp)
        .await
        .unwrap();

    // Get first response event
    let events = get_sla_events(&db, &applied_sla.id).await;
    let first_response_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");

    // Verify met_at timestamp
    assert!(first_response_event.met_at.is_some());
    let met_at =
        chrono::DateTime::parse_from_rfc3339(&first_response_event.met_at.clone().unwrap())
            .unwrap()
            .with_timezone(&Utc);
    let expected_met_at = chrono::DateTime::parse_from_rfc3339(&message_timestamp)
        .unwrap()
        .with_timezone(&Utc);

    // Allow 1 second tolerance
    let diff = (expected_met_at - met_at).num_seconds().abs();
    assert!(diff <= 1, "met_at should be set to the message timestamp");
}

#[tokio::test]
async fn test_applied_sla_remains_pending_after_first_response() {
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

    // Use current time in RFC3339 format as base timestamp
    let base_timestamp = chrono::Utc::now().to_rfc3339();

    // Apply SLA
    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &base_timestamp)
        .await
        .unwrap();

    // Get applied SLA before first response
    let applied_sla_before = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_before.status,
        oxidesk::AppliedSlaStatus::Pending
    );

    // Create agent user
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    // Agent sends a message
    let message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &message_timestamp)
        .await
        .unwrap();

    // Get applied SLA after first response
    let applied_sla_after = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Applied SLA should remain pending because resolution is not met yet
    assert_eq!(applied_sla_after.status, oxidesk::AppliedSlaStatus::Pending);
}

// ========================================
// Phase 5: User Story 3 - Breach Detection Tests
// ========================================

#[tokio::test]
async fn test_first_response_breach_detected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with very short deadline (1 minute)
    let policy = create_test_sla_policy(&db, "Urgent", "1m", "10m", "2m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Apply SLA with a timestamp in the past
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Check for breaches
    sla_service.check_breaches().await.unwrap();

    // Get first response event after breach check
    let events = get_sla_events(&db, &applied_sla.id).await;
    let first_response_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");

    // Verify event is marked as breached
    assert_eq!(
        first_response_event.status,
        oxidesk::SlaEventStatus::Breached
    );
    assert!(
        first_response_event.breached_at.is_some(),
        "breached_at should be set"
    );
    assert!(
        first_response_event.met_at.is_none(),
        "met_at should not be set"
    );
}

#[tokio::test]
async fn test_first_response_breached_at_timestamp() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 1 minute deadline
    let policy = create_test_sla_policy(&db, "Urgent", "1m", "10m", "2m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Apply SLA with timestamp in the past
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Get the deadline before checking breaches
    let events_before = get_sla_events(&db, &applied_sla.id).await;
    let first_response_event_before = events_before
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");
    let deadline = first_response_event_before.deadline_at.clone();

    // Check for breaches
    sla_service.check_breaches().await.unwrap();

    // Get event after breach check
    let events_after = get_sla_events(&db, &applied_sla.id).await;
    let first_response_event_after = events_after
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");

    // Verify breached_at is set to the deadline time
    assert!(first_response_event_after.breached_at.is_some());
    let breached_at = first_response_event_after.breached_at.as_ref().unwrap();

    // breached_at should be close to the deadline (allowing some processing time)
    let deadline_time = chrono::DateTime::parse_from_rfc3339(&deadline)
        .unwrap()
        .with_timezone(&Utc);
    let breached_time = chrono::DateTime::parse_from_rfc3339(breached_at)
        .unwrap()
        .with_timezone(&Utc);

    // Allow up to 2 seconds difference for processing
    let diff = (breached_time - deadline_time).num_seconds().abs();
    assert!(diff <= 2, "breached_at should be close to deadline");
}

#[tokio::test]
async fn test_resolution_breach_detected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with short resolution time
    let policy = create_test_sla_policy(&db, "Urgent", "10m", "1m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Apply SLA with timestamp in the past
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Check for breaches
    sla_service.check_breaches().await.unwrap();

    // Get resolution event
    let events = get_sla_events(&db, &applied_sla.id).await;
    let resolution_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::Resolution)
        .expect("Resolution event should exist");

    // Verify event is marked as breached
    assert_eq!(resolution_event.status, oxidesk::SlaEventStatus::Breached);
    assert!(resolution_event.breached_at.is_some());
}

#[tokio::test]
async fn test_resolution_breach_updates_applied_sla_status() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Urgent", "10m", "1m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Apply SLA with timestamp in the past
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Verify applied SLA starts as pending
    let applied_sla_before = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_before.status,
        oxidesk::AppliedSlaStatus::Pending
    );

    // Check for breaches
    sla_service.check_breaches().await.unwrap();

    // Verify applied SLA is now breached
    let applied_sla_after = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_after.status,
        oxidesk::AppliedSlaStatus::Breached
    );
}

#[tokio::test]
async fn test_multiple_breaches_on_same_sla() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with both deadlines in the past
    let policy = create_test_sla_policy(&db, "Urgent", "1m", "2m", "1m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Apply SLA with timestamp well in the past
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Check for breaches
    sla_service.check_breaches().await.unwrap();

    // Get all events
    let events = get_sla_events(&db, &applied_sla.id).await;

    // Verify both first response and resolution are breached
    let first_response_breached = events.iter().any(|e| {
        e.event_type == oxidesk::SlaEventType::FirstResponse
            && e.status == oxidesk::SlaEventStatus::Breached
    });
    let resolution_breached = events.iter().any(|e| {
        e.event_type == oxidesk::SlaEventType::Resolution
            && e.status == oxidesk::SlaEventStatus::Breached
    });

    assert!(first_response_breached, "First response should be breached");
    assert!(resolution_breached, "Resolution should be breached");
}

#[tokio::test]
async fn test_agent_reply_after_breach_stays_breached() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Urgent", "1m", "10m", "2m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Apply SLA with timestamp in the past
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Check for breaches - this should mark first response as breached
    sla_service.check_breaches().await.unwrap();

    // Verify first response is breached
    let events_after_breach = get_sla_events(&db, &applied_sla.id).await;
    let fr_event_after_breach = events_after_breach
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");
    assert_eq!(
        fr_event_after_breach.status,
        oxidesk::SlaEventStatus::Breached
    );

    // Now agent replies (after breach)
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &message_timestamp)
        .await
        .unwrap();

    // Verify event stays breached (doesn't change to met)
    let events_after_reply = get_sla_events(&db, &applied_sla.id).await;
    let fr_event_after_reply = events_after_reply
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");

    assert_eq!(
        fr_event_after_reply.status,
        oxidesk::SlaEventStatus::Breached,
        "Event should stay breached even after agent reply"
    );
    assert!(fr_event_after_reply.breached_at.is_some());
    assert!(fr_event_after_reply.met_at.is_none());
}

// =====================================================
// Phase 6: User Story 4 - Resolution Met
// =====================================================

#[tokio::test]
async fn test_resolution_met_on_conversation_resolved() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent for first response
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Handle first response
    let first_response_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &first_response_timestamp)
        .await
        .unwrap();

    // Resolve conversation (before resolution deadline)
    let resolution_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_conversation_resolved(&conversation.id, &resolution_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Get resolution event
    let events = get_sla_events(&db, &applied_sla.id).await;
    let resolution_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::Resolution)
        .expect("Resolution event should exist");

    // Verify resolution event is marked as met
    assert_eq!(resolution_event.status, oxidesk::SlaEventStatus::Met);
    assert!(resolution_event.met_at.is_some());
    assert!(resolution_event.breached_at.is_none());
}

#[tokio::test]
async fn test_resolution_met_timestamp_recorded() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Handle first response
    let first_response_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &first_response_timestamp)
        .await
        .unwrap();

    // Resolve conversation with specific timestamp
    let resolution_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_conversation_resolved(&conversation.id, &resolution_timestamp)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Get resolution event
    let events = get_sla_events(&db, &applied_sla.id).await;
    let resolution_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::Resolution)
        .expect("Resolution event should exist");

    // Verify met_at timestamp is recorded
    assert!(resolution_event.met_at.is_some());
    let met_at = resolution_event.met_at.as_ref().unwrap();

    // Verify met_at is close to the resolution timestamp
    let met_time = chrono::DateTime::parse_from_rfc3339(met_at)
        .unwrap()
        .with_timezone(&Utc);
    let resolution_time = chrono::DateTime::parse_from_rfc3339(&resolution_timestamp)
        .unwrap()
        .with_timezone(&Utc);

    // Allow up to 2 seconds difference for processing
    let diff = (met_time - resolution_time).num_seconds().abs();
    assert!(diff <= 2, "met_at should be close to resolution timestamp");
}

#[tokio::test]
async fn test_applied_sla_status_met_when_all_events_met() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Verify applied SLA starts as pending
    let applied_sla_before = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_before.status,
        oxidesk::AppliedSlaStatus::Pending
    );

    // Handle first response
    let first_response_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &first_response_timestamp)
        .await
        .unwrap();

    // Applied SLA should still be pending (only first response met, resolution still pending)
    let applied_sla_after_first = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_after_first.status,
        oxidesk::AppliedSlaStatus::Pending
    );

    // Resolve conversation
    let resolution_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_conversation_resolved(&conversation.id, &resolution_timestamp)
        .await
        .unwrap();

    // Now applied SLA should be met (both events met)
    let applied_sla_after_resolution = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_after_resolution.status,
        oxidesk::AppliedSlaStatus::Met
    );

    // Verify both events are met
    let events = get_sla_events(&db, &applied_sla_after_resolution.id).await;
    let first_response_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .expect("First response event should exist");
    let resolution_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::Resolution)
        .expect("Resolution event should exist");

    assert_eq!(first_response_event.status, oxidesk::SlaEventStatus::Met);
    assert_eq!(resolution_event.status, oxidesk::SlaEventStatus::Met);
}

// =====================================================
// Phase 7: User Story 5 - Next Response Events
// =====================================================

#[tokio::test]
async fn test_next_response_event_created_on_contact_reply() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Initially, only first_response and resolution events exist
    let events_before = get_sla_events(&db, &applied_sla.id).await;
    assert_eq!(events_before.len(), 2);

    // Agent sends first message
    let agent_message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_timestamp)
        .await
        .unwrap();

    // Contact replies
    let contact_message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_contact_message(&conversation.id, &contact.id, &contact_message_timestamp)
        .await
        .unwrap();

    // Verify next_response event was created
    let events_after = get_sla_events(&db, &applied_sla.id).await;
    assert_eq!(events_after.len(), 3);

    let next_response_event = events_after
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::NextResponse)
        .expect("Next response event should be created");

    assert_eq!(next_response_event.status, oxidesk::SlaEventStatus::Pending);
}

#[tokio::test]
async fn test_next_response_deadline_calculation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with 5 minute next response time
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Agent sends message
    let agent_message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_timestamp)
        .await
        .unwrap();

    // Contact replies
    let contact_message_timestamp = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_contact_message(&conversation.id, &contact.id, &contact_message_timestamp)
        .await
        .unwrap();

    // Verify next_response event deadline is contact_message_timestamp + 5 minutes
    let events = get_sla_events(&db, &applied_sla.id).await;
    let next_response_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::NextResponse)
        .expect("Next response event should exist");

    let contact_time = chrono::DateTime::parse_from_rfc3339(&contact_message_timestamp)
        .unwrap()
        .with_timezone(&Utc);
    let deadline_time = chrono::DateTime::parse_from_rfc3339(&next_response_event.deadline_at)
        .unwrap()
        .with_timezone(&Utc);

    let expected_deadline = contact_time + chrono::Duration::minutes(5);
    let diff = (deadline_time - expected_deadline).num_seconds().abs();

    // Allow up to 2 seconds difference for processing
    assert!(
        diff <= 2,
        "Deadline should be 5 minutes after contact message"
    );
}

#[tokio::test]
async fn test_only_one_pending_next_response_event() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Agent sends message
    let agent_message_1 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_1)
        .await
        .unwrap();

    // Contact replies (creates first next_response event)
    let contact_message_1 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_contact_message(&conversation.id, &contact.id, &contact_message_1)
        .await
        .unwrap();

    // Verify one next_response event exists
    let events_after_first = get_sla_events(&db, &applied_sla.id).await;
    let next_response_events_1: Vec<_> = events_after_first
        .iter()
        .filter(|e| e.event_type == oxidesk::SlaEventType::NextResponse)
        .collect();
    assert_eq!(next_response_events_1.len(), 1);
    assert_eq!(
        next_response_events_1[0].status,
        oxidesk::SlaEventStatus::Pending
    );

    // Agent replies (should mark the next_response event as met)
    let agent_message_2 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_2)
        .await
        .unwrap();

    // Contact replies again (creates second next_response event)
    let contact_message_2 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_contact_message(&conversation.id, &contact.id, &contact_message_2)
        .await
        .unwrap();

    // Verify two next_response events exist, but only one is pending
    let events_after_second = get_sla_events(&db, &applied_sla.id).await;
    let next_response_events_2: Vec<_> = events_after_second
        .iter()
        .filter(|e| e.event_type == oxidesk::SlaEventType::NextResponse)
        .collect();
    assert_eq!(next_response_events_2.len(), 2);

    let pending_count = next_response_events_2
        .iter()
        .filter(|e| e.status == oxidesk::SlaEventStatus::Pending)
        .count();
    assert_eq!(
        pending_count, 1,
        "Only one next_response event should be pending"
    );

    let met_count = next_response_events_2
        .iter()
        .filter(|e| e.status == oxidesk::SlaEventStatus::Met)
        .count();
    assert_eq!(met_count, 1, "One next_response event should be met");
}

#[tokio::test]
async fn test_next_response_met_on_agent_reply() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Apply SLA
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Get applied SLA
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();

    // Agent sends message (marks first response as met)
    let agent_message_1 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_1)
        .await
        .unwrap();

    // Contact replies (creates next_response event)
    let contact_message = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_contact_message(&conversation.id, &contact.id, &contact_message)
        .await
        .unwrap();

    // Verify next_response event is pending
    let events_before = get_sla_events(&db, &applied_sla.id).await;
    let next_response_event_before = events_before
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::NextResponse)
        .expect("Next response event should exist");
    assert_eq!(
        next_response_event_before.status,
        oxidesk::SlaEventStatus::Pending
    );

    // Agent replies (should mark next_response as met)
    let agent_message_2 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_2)
        .await
        .unwrap();

    // Verify next_response event is now met
    let events_after = get_sla_events(&db, &applied_sla.id).await;
    let next_response_event_after = events_after
        .iter()
        .find(|e| {
            e.event_type == oxidesk::SlaEventType::NextResponse
                && e.id == next_response_event_before.id
        })
        .expect("Next response event should still exist");
    assert_eq!(
        next_response_event_after.status,
        oxidesk::SlaEventStatus::Met
    );
    assert!(next_response_event_after.met_at.is_some());
}

// =====================================================
// Phase 8: Integration Tests
// =====================================================

#[tokio::test]
async fn test_full_sla_lifecycle() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy
    let policy = create_test_sla_policy(&db, "Standard", "10m", "30m", "5m").await;

    // Create conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create agent
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;

    let sla_service = oxidesk::SlaService::new(
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Step 1: Apply SLA to conversation
    let now = chrono::Utc::now().to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &now)
        .await
        .unwrap();

    // Verify SLA was applied
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(applied_sla.sla_policy_id, policy.id);

    // Verify initial events created
    let events = get_sla_events(&db, &applied_sla.id).await;
    assert_eq!(events.len(), 2); // first_response and resolution

    // Step 2: Agent sends first response
    let agent_message_1 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_1)
        .await
        .unwrap();

    // Verify first response is met
    let events_after_first_response = get_sla_events(&db, &applied_sla.id).await;
    let first_response_event = events_after_first_response
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .unwrap();
    assert_eq!(first_response_event.status, oxidesk::SlaEventStatus::Met);

    // Step 3: Contact replies
    let contact_message = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_contact_message(&conversation.id, &contact.id, &contact_message)
        .await
        .unwrap();

    // Verify next_response event created
    let events_after_contact = get_sla_events(&db, &applied_sla.id).await;
    assert_eq!(events_after_contact.len(), 3); // first_response, resolution, next_response

    // Step 4: Agent replies again
    let agent_message_2 = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message_2)
        .await
        .unwrap();

    // Verify next_response is met
    let events_after_agent_2 = get_sla_events(&db, &applied_sla.id).await;
    let next_response_event = events_after_agent_2
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::NextResponse)
        .unwrap();
    assert_eq!(next_response_event.status, oxidesk::SlaEventStatus::Met);

    // Step 5: Resolve conversation
    let resolution_time = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_conversation_resolved(&conversation.id, &resolution_time)
        .await
        .unwrap();

    // Verify resolution is met
    let events_final = get_sla_events(&db, &applied_sla.id).await;
    let resolution_event = events_final
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::Resolution)
        .unwrap();
    assert_eq!(resolution_event.status, oxidesk::SlaEventStatus::Met);

    // Verify applied SLA status is Met
    let applied_sla_final = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(applied_sla_final.status, oxidesk::AppliedSlaStatus::Met);
}

#[tokio::test]
async fn test_sla_breach_workflow() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create SLA policy with short deadlines
    let policy = create_test_sla_policy(&db, "Urgent", "1m", "2m", "1m").await;

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
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::sla_repository::SlaRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::conversation_repository::ConversationRepository>,
        std::sync::Arc::new(db.clone()) as std::sync::Arc<dyn oxidesk::domain::ports::team_repository::TeamRepository>,
        std::sync::Arc::new(oxidesk::LocalEventBus::new(100)),
    );

    // Step 1: Apply SLA with timestamp in the past (5 minutes ago)
    let past_timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
    sla_service
        .apply_sla(&conversation.id, &policy.id, &past_timestamp)
        .await
        .unwrap();

    // Verify SLA was applied
    let applied_sla = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(applied_sla.status, oxidesk::AppliedSlaStatus::Pending);

    // Step 2: Run breach detection
    sla_service.check_breaches().await.unwrap();

    // Verify both events are breached (deadlines were in the past)
    let events = get_sla_events(&db, &applied_sla.id).await;

    let first_response_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .unwrap();
    assert_eq!(
        first_response_event.status,
        oxidesk::SlaEventStatus::Breached
    );
    assert!(first_response_event.breached_at.is_some());

    let resolution_event = events
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::Resolution)
        .unwrap();
    assert_eq!(resolution_event.status, oxidesk::SlaEventStatus::Breached);
    assert!(resolution_event.breached_at.is_some());

    // Verify applied SLA status is Breached
    let applied_sla_after_breach = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_after_breach.status,
        oxidesk::AppliedSlaStatus::Breached
    );

    // Step 3: Agent tries to respond (should not change breached status)
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let agent_message = chrono::Utc::now().to_rfc3339();
    sla_service
        .handle_agent_message(&conversation.id, &agent.user_id, &agent_message)
        .await
        .unwrap();

    // Verify first_response stays breached (doesn't change to met)
    let events_after_agent = get_sla_events(&db, &applied_sla.id).await;
    let first_response_after = events_after_agent
        .iter()
        .find(|e| e.event_type == oxidesk::SlaEventType::FirstResponse)
        .unwrap();
    assert_eq!(
        first_response_after.status,
        oxidesk::SlaEventStatus::Breached
    );
    assert!(first_response_after.met_at.is_none());

    // Verify applied SLA status stays Breached
    let applied_sla_final = get_applied_sla(&db, &conversation.id).await.unwrap();
    assert_eq!(
        applied_sla_final.status,
        oxidesk::AppliedSlaStatus::Breached
    );
}
