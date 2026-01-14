mod helpers;

use chrono::{Duration, Utc};
use helpers::*;
use oxidesk::{
    database::Database,
    events::EventBus,
    models::{AgentAvailability, User, UserType},
    services::AvailabilityService,
};
use sqlx::Row;

async fn setup() -> (TestDatabase, EventBus, AvailabilityService) {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let event_bus = EventBus::new(100); // Capacity of 100 events
    let availability_service = AvailabilityService::new(db.clone(), event_bus.clone());
    (test_db, event_bus, availability_service)
}

/// Helper to create a test user for agent
async fn create_test_user(db: &Database, user_id: &str, email: &str) -> User {
    let pool = db.pool();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, email, user_type, created_at, updated_at)
         VALUES (?, ?, 'agent', ?, ?)",
    )
    .bind(user_id)
    .bind(email)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .expect("Failed to create test user");

    User {
        id: user_id.to_string(),
        email: email.to_string(),
        user_type: UserType::Agent,
        created_at: now.clone(),
        updated_at: now,
    }
}

#[tokio::test]
async fn test_manual_availability_change_to_online() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-001";
    create_test_user(&db, user_id, "agent1@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent One", AgentAvailability::Offline).await;

    // Set availability to online (service expects user_id, not agent_id)
    availability_service
        .set_availability(&agent.user_id, AgentAvailability::Online)
        .await
        .expect("Failed to set availability");

    // Verify status changed
    let response = availability_service
        .get_availability(&agent.user_id)
        .await
        .expect("Failed to get availability");

    assert_eq!(response.availability_status, AgentAvailability::Online);
    assert!(
        response.last_activity_at.is_some(),
        "last_activity_at should be set when going online"
    );
    assert!(
        response.away_since.is_none(),
        "away_since should be cleared when going online"
    );

    // Verify activity log
    let log_count = get_activity_log_count(&db, &agent.id).await;
    assert_eq!(log_count, 1, "Should have 1 activity log entry");

    let log = get_latest_activity_log(&db, &agent.id).await.unwrap();
    assert_eq!(log.0, "availability_changed");
    assert_eq!(log.1, Some("offline".to_string()));
    assert_eq!(log.2, Some("online".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_manual_availability_change_to_away_manual() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-002";
    create_test_user(&db, user_id, "agent2@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Two", AgentAvailability::Online).await;

    // Set availability to away_manual
    availability_service
        .set_availability(&agent.user_id, AgentAvailability::AwayManual)
        .await
        .expect("Failed to set availability");

    // Verify status changed
    let response = availability_service
        .get_availability(&agent.user_id)
        .await
        .expect("Failed to get availability");

    assert_eq!(response.availability_status, AgentAvailability::AwayManual);
    assert!(
        response.away_since.is_some(),
        "away_since should be set when going away_manual"
    );

    // Verify activity log
    let log = get_latest_activity_log(&db, &agent.id).await.unwrap();
    assert_eq!(log.0, "availability_changed");
    assert_eq!(log.1, Some("online".to_string()));
    assert_eq!(log.2, Some("away_manual".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_return_to_online_from_away() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent in away state
    let user_id = "test-user-003";
    create_test_user(&db, user_id, "agent3@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Three", AgentAvailability::Away).await;
    set_agent_away_since(&db, &agent.id, Utc::now() - Duration::minutes(10)).await;

    // Return to online
    availability_service
        .set_availability(&agent.user_id, AgentAvailability::Online)
        .await
        .expect("Failed to set availability");

    // Verify status changed and away_since cleared
    let response = availability_service
        .get_availability(&agent.user_id)
        .await
        .expect("Failed to get availability");

    assert_eq!(response.availability_status, AgentAvailability::Online);
    assert!(
        response.away_since.is_none(),
        "away_since should be cleared"
    );
    assert!(
        response.last_activity_at.is_some(),
        "last_activity_at should be updated"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_automatic_away_transition() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and online agent with old activity timestamp
    let user_id = "test-user-004";
    create_test_user(&db, user_id, "agent4@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Four", AgentAvailability::Online).await;

    // Set last_activity_at to 10 minutes ago (exceeds default 5-minute threshold)
    set_agent_last_activity(&db, &agent.id, Utc::now() - Duration::minutes(10)).await;

    // Run inactivity check
    let affected = availability_service
        .check_inactivity_timeouts()
        .await
        .expect("Failed to check inactivity timeouts");

    assert_eq!(affected.len(), 1, "Should transition 1 agent to away");
    assert_eq!(affected[0], agent.id);

    // Verify status changed to away
    let response = availability_service
        .get_availability(&agent.user_id)
        .await
        .expect("Failed to get availability");

    assert_eq!(response.availability_status, AgentAvailability::Away);
    assert!(response.away_since.is_some(), "away_since should be set");

    // Verify activity log
    let log = get_latest_activity_log(&db, &agent.id).await.unwrap();
    assert_eq!(log.0, "availability_changed");
    assert_eq!(log.1, Some("online".to_string()));
    assert_eq!(log.2, Some("away".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_conversations_remain_assigned_on_away() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user, agent, and contact
    let user_id = "test-user-005";
    create_test_user(&db, user_id, "agent5@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Five", AgentAvailability::Online).await;

    let contact_user_id = "contact-user-005";
    create_test_user(&db, contact_user_id, "contact5@example.com").await;

    // Create contact
    let pool = db.pool();
    let contact_id = "contact-005";
    let _now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO contacts (id, user_id, first_name) VALUES (?, ?, 'Contact Five')")
        .bind(contact_id)
        .bind(contact_user_id)
        .execute(pool)
        .await
        .expect("Failed to create contact");

    // Create assigned conversation
    let inbox_id = "inbox-001"; // From seed data
    create_assigned_conversation(&db, inbox_id, contact_id, user_id).await;

    // Verify conversation is assigned
    let assigned_count_before = get_assigned_conversation_count(&db, user_id).await;
    assert_eq!(assigned_count_before, 1);

    // Transition to away (manual)
    availability_service
        .set_availability(&agent.user_id, AgentAvailability::Away)
        .await
        .expect("Failed to set availability");

    // Verify conversation is still assigned
    let assigned_count_after = get_assigned_conversation_count(&db, user_id).await;
    assert_eq!(
        assigned_count_after, 1,
        "Conversations should remain assigned when going away"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_max_idle_reassignment() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user, agent, and contact
    let user_id = "test-user-006";
    create_test_user(&db, user_id, "agent6@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Six", AgentAvailability::Away).await;

    let contact_user_id = "contact-user-006";
    create_test_user(&db, contact_user_id, "contact6@example.com").await;

    // Create contact
    let pool = db.pool();
    let contact_id = "contact-006";
    sqlx::query("INSERT INTO contacts (id, user_id, first_name) VALUES (?, ?, 'Contact Six')")
        .bind(contact_id)
        .bind(contact_user_id)
        .execute(pool)
        .await
        .expect("Failed to create contact");

    // Create assigned conversation
    let inbox_id = "inbox-001";
    create_assigned_conversation(&db, inbox_id, contact_id, user_id).await;

    // Set away_since to 35 minutes ago (exceeds default 30-minute threshold)
    set_agent_away_since(&db, &agent.id, Utc::now() - Duration::minutes(35)).await;

    // Verify conversation is assigned before
    let assigned_count_before = get_assigned_conversation_count(&db, user_id).await;
    assert_eq!(assigned_count_before, 1);

    // Run max idle check
    let affected = availability_service
        .check_max_idle_thresholds()
        .await
        .expect("Failed to check max idle thresholds");

    assert_eq!(affected.len(), 1, "Should process 1 agent");
    assert_eq!(affected[0], agent.id);

    // Verify conversations were unassigned
    let assigned_count_after = get_assigned_conversation_count(&db, user_id).await;
    assert_eq!(
        assigned_count_after, 0,
        "Conversations should be unassigned"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_conversations_return_to_team_inbox() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user, agent, and contact
    let user_id = "test-user-007";
    create_test_user(&db, user_id, "agent7@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Seven", AgentAvailability::Away).await;

    let contact_user_id = "contact-user-007";
    create_test_user(&db, contact_user_id, "contact7@example.com").await;

    // Create contact
    let pool = db.pool();
    let contact_id = "contact-007";
    sqlx::query("INSERT INTO contacts (id, user_id, first_name) VALUES (?, ?, 'Contact Seven')")
        .bind(contact_id)
        .bind(contact_user_id)
        .execute(pool)
        .await
        .expect("Failed to create contact");

    // Create assigned conversation
    let inbox_id = "inbox-001";
    let conv_id = create_assigned_conversation(&db, inbox_id, contact_id, user_id).await;

    // Set away_since to 35 minutes ago
    set_agent_away_since(&db, &agent.id, Utc::now() - Duration::minutes(35)).await;

    // Run max idle check (unassigns conversations)
    availability_service
        .check_max_idle_thresholds()
        .await
        .expect("Failed to check max idle thresholds");

    // Verify conversation is no longer assigned to user
    let row = sqlx::query("SELECT assigned_user_id FROM conversations WHERE id = ?")
        .bind(&conv_id)
        .fetch_one(pool)
        .await
        .expect("Failed to fetch conversation");

    let assigned_user_id: Option<String> = row.try_get(0).ok();
    assert!(
        assigned_user_id.is_none(),
        "Conversation should not be assigned to any user (back to team inbox)"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_agent_goes_offline_after_reassignment() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent in away state
    let user_id = "test-user-008";
    create_test_user(&db, user_id, "agent8@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Eight", AgentAvailability::Away).await;

    // Set away_since to 35 minutes ago
    set_agent_away_since(&db, &agent.id, Utc::now() - Duration::minutes(35)).await;

    // Run max idle check
    availability_service
        .check_max_idle_thresholds()
        .await
        .expect("Failed to check max idle thresholds");

    // Verify agent went offline
    let response = availability_service
        .get_availability(&agent.user_id)
        .await
        .expect("Failed to get availability");

    assert_eq!(response.availability_status, AgentAvailability::Offline);

    // Verify activity log shows transition to offline
    let log = get_latest_activity_log(&db, &agent.id).await.unwrap();
    assert_eq!(log.0, "availability_changed");
    assert_eq!(log.2, Some("offline".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_login_event_logging() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-009";
    create_test_user(&db, user_id, "agent9@example.com").await;
    create_test_agent_with_status(&db, user_id, "Agent Nine", AgentAvailability::Offline).await;

    // Handle login
    availability_service
        .handle_login(user_id)
        .await
        .expect("Failed to handle login");

    // Verify agent status
    let agent = db
        .get_agent_by_user_id(user_id)
        .await
        .expect("Failed to get agent")
        .expect("Agent not found");

    assert_eq!(agent.availability_status, AgentAvailability::Online);
    assert!(agent.last_login_at.is_some(), "last_login_at should be set");
    assert!(
        agent.last_activity_at.is_some(),
        "last_activity_at should be set"
    );

    // Verify activity log
    let log = get_latest_activity_log(&db, &agent.id).await.unwrap();
    assert_eq!(log.0, "agent_login");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_logout_event_logging() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-010";
    create_test_user(&db, user_id, "agent10@example.com").await;
    create_test_agent_with_status(&db, user_id, "Agent Ten", AgentAvailability::Online).await;

    // Handle logout
    availability_service
        .handle_logout(user_id)
        .await
        .expect("Failed to handle logout");

    // Verify agent status
    let agent = db
        .get_agent_by_user_id(user_id)
        .await
        .expect("Failed to get agent")
        .expect("Agent not found");

    assert_eq!(agent.availability_status, AgentAvailability::Offline);

    // Verify activity log
    let log = get_latest_activity_log(&db, &agent.id).await.unwrap();
    assert_eq!(log.0, "agent_logout");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_availability_change_logging() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-011";
    create_test_user(&db, user_id, "agent11@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Eleven", AgentAvailability::Online)
            .await;

    // Change status multiple times
    availability_service
        .set_availability(&agent.user_id, AgentAvailability::Away)
        .await
        .expect("Failed to set availability");

    availability_service
        .set_availability(&agent.user_id, AgentAvailability::Online)
        .await
        .expect("Failed to set availability");

    // Verify we have 2 availability_changed logs
    let log_count = get_activity_log_count(&db, &agent.id).await;
    assert_eq!(log_count, 2, "Should have 2 activity log entries");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_activity_tracking() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-012";
    create_test_user(&db, user_id, "agent12@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Twelve", AgentAvailability::Online)
            .await;

    // Record activity
    availability_service
        .record_activity(&agent.id)
        .await
        .expect("Failed to record activity");

    // Verify last_activity_at was updated
    let agent_updated = db
        .get_agent_by_user_id(user_id)
        .await
        .expect("Failed to get agent")
        .expect("Agent not found");

    assert!(
        agent_updated.last_activity_at.is_some(),
        "last_activity_at should be set"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_invalid_status_transition_rejection() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-013";
    create_test_user(&db, user_id, "agent13@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Thirteen", AgentAvailability::Online)
            .await;

    // Try to manually set away_and_reassigning (should fail)
    let result = availability_service
        .set_availability(&agent.user_id, AgentAvailability::AwayAndReassigning)
        .await;

    assert!(
        result.is_err(),
        "Should reject manual setting of away_and_reassigning"
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_activity_logs_pagination() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create test user and agent
    let user_id = "test-user-014";
    create_test_user(&db, user_id, "agent14@example.com").await;
    let agent =
        create_test_agent_with_status(&db, user_id, "Agent Fourteen", AgentAvailability::Online)
            .await;

    // Create multiple activity logs by changing status
    for _ in 0..5 {
        availability_service
            .set_availability(&agent.user_id, AgentAvailability::Away)
            .await
            .expect("Failed to set away");
        availability_service
            .set_availability(&agent.user_id, AgentAvailability::Online)
            .await
            .expect("Failed to set online");
    }

    // Get first page (limit 5)
    let response = availability_service
        .get_activity_logs(&agent.id, 5, 0)
        .await
        .expect("Failed to get activity logs");

    assert_eq!(response.logs.len(), 5);
    assert_eq!(response.total, 10);
    assert_eq!(response.pagination.total_pages, 2);

    // Get second page
    let response_page2 = availability_service
        .get_activity_logs(&agent.id, 5, 5)
        .await
        .expect("Failed to get activity logs");

    assert_eq!(response_page2.logs.len(), 5);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_config_value_loading() {
    let (test_db, _event_bus, _availability_service) = setup().await;
    let db = test_db.db();

    // Verify seeded config values
    let inactivity_timeout = get_config_value(&db, "availability.inactivity_timeout_seconds").await;
    assert_eq!(inactivity_timeout, Some("300".to_string()));

    let max_idle_threshold = get_config_value(&db, "availability.max_idle_threshold_seconds").await;
    assert_eq!(max_idle_threshold, Some("1800".to_string()));

    // Update config value
    set_config_value(&db, "availability.inactivity_timeout_seconds", "600").await;

    // Verify updated value
    let updated_value = get_config_value(&db, "availability.inactivity_timeout_seconds").await;
    assert_eq!(updated_value, Some("600".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_only_online_agents_transition_to_away() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create multiple agents in different states
    let user_id_online = "test-user-015";
    create_test_user(&db, user_id_online, "online@example.com").await;
    let agent_online = create_test_agent_with_status(
        &db,
        user_id_online,
        "Online Agent",
        AgentAvailability::Online,
    )
    .await;
    set_agent_last_activity(&db, &agent_online.id, Utc::now() - Duration::minutes(10)).await;

    let user_id_offline = "test-user-016";
    create_test_user(&db, user_id_offline, "offline@example.com").await;
    let agent_offline = create_test_agent_with_status(
        &db,
        user_id_offline,
        "Offline Agent",
        AgentAvailability::Offline,
    )
    .await;
    set_agent_last_activity(&db, &agent_offline.id, Utc::now() - Duration::minutes(10)).await;

    let user_id_away = "test-user-017";
    create_test_user(&db, user_id_away, "away@example.com").await;
    let agent_away =
        create_test_agent_with_status(&db, user_id_away, "Away Agent", AgentAvailability::Away)
            .await;
    set_agent_last_activity(&db, &agent_away.id, Utc::now() - Duration::minutes(10)).await;

    // Run inactivity check
    let affected = availability_service
        .check_inactivity_timeouts()
        .await
        .expect("Failed to check inactivity timeouts");

    // Only the online agent should be affected
    assert_eq!(affected.len(), 1);
    assert_eq!(affected[0], agent_online.id);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_only_away_agents_transition_to_offline() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create agents in different states
    let user_id_away = "test-user-018";
    create_test_user(&db, user_id_away, "away18@example.com").await;
    let agent_away =
        create_test_agent_with_status(&db, user_id_away, "Away Agent", AgentAvailability::Away)
            .await;
    set_agent_away_since(&db, &agent_away.id, Utc::now() - Duration::minutes(35)).await;

    let user_id_online = "test-user-019";
    create_test_user(&db, user_id_online, "online19@example.com").await;
    let _agent_online = create_test_agent_with_status(
        &db,
        user_id_online,
        "Online Agent",
        AgentAvailability::Online,
    )
    .await;

    // Run max idle check
    let affected = availability_service
        .check_max_idle_thresholds()
        .await
        .expect("Failed to check max idle thresholds");

    // Only the away agent should be affected
    assert_eq!(affected.len(), 1);
    assert_eq!(affected[0], agent_away.id);

    // Verify online agent is still online
    let agent_online_updated = db
        .get_agent_by_user_id(user_id_online)
        .await
        .expect("Failed to get agent")
        .expect("Agent not found");
    assert_eq!(
        agent_online_updated.availability_status,
        AgentAvailability::Online
    );

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_away_manual_agents_also_reassigned() {
    let (test_db, _event_bus, availability_service) = setup().await;
    let db = test_db.db();

    // Create away_manual agent with old away_since
    let user_id = "test-user-020";
    create_test_user(&db, user_id, "away-manual@example.com").await;
    let agent = create_test_agent_with_status(
        &db,
        user_id,
        "Away Manual Agent",
        AgentAvailability::AwayManual,
    )
    .await;
    set_agent_away_since(&db, &agent.id, Utc::now() - Duration::minutes(35)).await;

    // Create contact and conversation
    let contact_user_id = "contact-user-020";
    create_test_user(&db, contact_user_id, "contact20@example.com").await;
    let pool = db.pool();
    let contact_id = "contact-020";
    sqlx::query("INSERT INTO contacts (id, user_id, first_name) VALUES (?, ?, 'Contact Twenty')")
        .bind(contact_id)
        .bind(contact_user_id)
        .execute(pool)
        .await
        .expect("Failed to create contact");

    let inbox_id = "inbox-001";
    create_assigned_conversation(&db, inbox_id, contact_id, user_id).await;

    // Verify conversation is assigned before
    let assigned_count_before = get_assigned_conversation_count(&db, user_id).await;
    assert_eq!(assigned_count_before, 1);

    // Run max idle check
    let affected = availability_service
        .check_max_idle_thresholds()
        .await
        .expect("Failed to check max idle thresholds");

    // away_manual agent should also be processed
    assert_eq!(affected.len(), 1);
    assert_eq!(affected[0], agent.id);

    // Verify conversations were unassigned
    let assigned_count_after = get_assigned_conversation_count(&db, user_id).await;
    assert_eq!(assigned_count_after, 0);

    // Verify agent went offline
    let agent_updated = db
        .get_agent_by_user_id(user_id)
        .await
        .expect("Failed to get agent")
        .expect("Agent not found");
    assert_eq!(
        agent_updated.availability_status,
        AgentAvailability::Offline
    );

    teardown_test_db(test_db).await;
}
