mod helpers;

use helpers::*;
use oxidesk::{
    infrastructure::persistence::automation_rules::AutomationRulesRepository,
    domain::ports::{
        agent_repository::AgentRepository, automation_repository::AutomationRepository,
        conversation_repository::ConversationRepository,
        conversation_tag_repository::ConversationTagRepository, tag_repository::TagRepository,
        team_repository::TeamRepository, user_repository::UserRepository,
    },
    domain::entities::{
        ActionType, AutomationRule, ComparisonOperator, ConversationStatus, Priority, RuleAction,
        RuleCondition, RuleType,
    },
    domain::services::action_executor::ActionExecutor,
    application::services::automation_service::{AutomationConfig, AutomationService},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Helper function to create AutomationService with all required repositories
fn create_automation_service(db: &oxidesk::Database, config: AutomationConfig) -> AutomationService {
    let tag_repo = TagRepository::new(db.clone());
    let action_executor = ActionExecutor::new(
        Arc::new(db.clone()) as Arc<dyn ConversationRepository>,
        Arc::new(db.clone()) as Arc<dyn UserRepository>,
        Arc::new(db.clone()) as Arc<dyn AgentRepository>,
        Arc::new(db.clone()) as Arc<dyn TeamRepository>,
        tag_repo,
        Arc::new(db.clone()) as Arc<dyn ConversationTagRepository>,
    );
    AutomationService::new(
        Arc::new(db.clone()) as Arc<dyn AutomationRepository>,
        action_executor,
        config,
    )
}

/// User Story 1: Event-Triggered Rule Execution
/// When a conversation is tagged with "Bug", automatically set priority to "High"
#[tokio::test]
async fn test_event_triggered_rule_execution() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup: Create a rule that triggers on tags_changed event
    let rule = AutomationRule::new(
        "Bug Priority Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.tags_changed".to_string()],
        RuleCondition::Simple {
            attribute: "tags".to_string(),
            comparison: ComparisonOperator::Contains,
            value: json!("Bug"),
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        },
    );

    AutomationRulesRepository::create_automation_rule(db, &rule).await.unwrap();

    // Create test data
    let contact = create_test_contact(db, "user@example.com").await;
    let mut conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Initially no priority set
    assert_eq!(conversation.priority, None);

    // Action: Add "Bug" tag and trigger automation
    conversation.tags = Some(vec!["Bug".to_string()]);

    let service =
        create_automation_service(db, AutomationConfig::default());

    service
        .handle_conversation_event("conversation.tags_changed", &conversation, "test-user")
        .await
        .unwrap();

    // Verify: Priority was set to "High"
    let updated_conversation = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_conversation.priority, Some(Priority::High));

    // Verify: Evaluation log was created
    let logs = db
        .get_rule_evaluation_logs_by_rule(&rule.id, 10, 0)
        .await
        .unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].rule_id, rule.id);
    assert_eq!(logs[0].event_type, "conversation.tags_changed");
    assert!(logs[0].matched);
    assert!(logs[0].action_executed);

    teardown_test_db(test_db).await;
}

/// User Story 2: Conditional Rule Evaluation
/// Rule should only execute when condition is true
#[tokio::test]
async fn test_conditional_rule_evaluation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup: Create a rule with a specific condition
    let rule = AutomationRule::new(
        "High Priority Assignment".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.priority_changed".to_string()],
        RuleCondition::Simple {
            attribute: "priority".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("High"),
        },
        RuleAction {
            action_type: ActionType::ChangeStatus,
            parameters: HashMap::from([("status".to_string(), json!("open"))]),
        },
    );

    AutomationRulesRepository::create_automation_rule(db, &rule).await.unwrap();

    let service =
        create_automation_service(db, AutomationConfig::default());

    // Test Case 1: Condition matches (priority is "Urgent")
    let contact1 = create_test_contact(db, "user1@example.com").await;
    let mut conversation1 = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact1.id.clone(),
        ConversationStatus::Open,
    )
    .await;
    conversation1.priority = Some(Priority::High);
    db.set_conversation_priority(&conversation1.id, &Priority::High)
        .await
        .unwrap();

    service
        .handle_conversation_event("conversation.priority_changed", &conversation1, "test-user")
        .await
        .unwrap();

    // Verify: Action was executed
    let logs1 = db
        .get_evaluation_logs_by_conversation(&conversation1.id)
        .await
        .unwrap();
    assert_eq!(logs1.len(), 1);
    assert!(logs1[0].action_executed);

    // Test Case 2: Condition does not match (priority is "Low")
    let contact2 = create_test_contact(db, "user2@example.com").await;
    let mut conversation2 = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact2.id.clone(),
        ConversationStatus::Open,
    )
    .await;
    conversation2.priority = Some(Priority::Low);
    db.set_conversation_priority(&conversation2.id, &Priority::Low)
        .await
        .unwrap();

    service
        .handle_conversation_event("conversation.priority_changed", &conversation2, "test-user")
        .await
        .unwrap();

    // Verify: Action was NOT executed (condition false)
    let logs2 = db
        .get_evaluation_logs_by_conversation(&conversation2.id)
        .await
        .unwrap();
    assert_eq!(logs2.len(), 1);
    assert!(!logs2[0].action_executed);

    teardown_test_db(test_db).await;
}

/// User Story 3: Enabled/Disabled Rule Control
/// Disabled rules should not be evaluated
#[tokio::test]
async fn test_enabled_disabled_rule_control() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup: Create a rule
    let rule = AutomationRule::new(
        "Test Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.created".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("Medium"))]),
        },
    );

    AutomationRulesRepository::create_automation_rule(db, &rule).await.unwrap();

    let service =
        create_automation_service(db, AutomationConfig::default());

    // Test Case 1: Rule is enabled
    let contact1 = create_test_contact(db, "user1@example.com").await;
    let conversation1 = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact1.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    service
        .handle_conversation_event("conversation.created", &conversation1, "test-user")
        .await
        .unwrap();

    let updated1 = db
        .get_conversation_by_id(&conversation1.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated1.priority, Some(Priority::Medium));

    // Action: Disable the rule
    AutomationRulesRepository::disable_automation_rule(db, &rule.id).await.unwrap();

    // Test Case 2: Rule is disabled
    let contact2 = create_test_contact(db, "user2@example.com").await;
    let conversation2 = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact2.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    service
        .handle_conversation_event("conversation.created", &conversation2, "test-user")
        .await
        .unwrap();

    // Verify: Action was NOT executed (rule disabled)
    let updated2 = db
        .get_conversation_by_id(&conversation2.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated2.priority, None);

    // Action: Re-enable the rule
    AutomationRulesRepository::enable_automation_rule(db, &rule.id).await.unwrap();

    // Test Case 3: Rule is re-enabled
    let contact3 = create_test_contact(db, "user3@example.com").await;
    let conversation3 = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact3.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    service
        .handle_conversation_event("conversation.created", &conversation3, "test-user")
        .await
        .unwrap();

    // Verify: Action was executed (rule re-enabled)
    let updated3 = db
        .get_conversation_by_id(&conversation3.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated3.priority, Some(Priority::Medium));

    teardown_test_db(test_db).await;
}

/// User Story 4: Rule Type and Event Subscription
/// Rules should only evaluate for events they subscribe to
#[tokio::test]
async fn test_rule_type_and_event_subscription() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup: Create rules with different event subscriptions
    let rule1 = AutomationRule::new(
        "Tags Changed Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.tags_changed".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        },
    );

    let rule2 = AutomationRule::new(
        "Status Changed Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.status_changed".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("resolved"),
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("Low"))]),
        },
    );

    let rule3 = AutomationRule::new(
        "Assignment Changed Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.assignment_changed".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("Medium"))]),
        },
    );

    AutomationRulesRepository::create_automation_rule(db, &rule1).await.unwrap();
    AutomationRulesRepository::create_automation_rule(db, &rule2).await.unwrap();
    AutomationRulesRepository::create_automation_rule(db, &rule3).await.unwrap();

    let service =
        create_automation_service(db, AutomationConfig::default());

    // Test: Trigger "tags_changed" event
    let contact = create_test_contact(db, "user@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    service
        .handle_conversation_event("conversation.tags_changed", &conversation, "test-user")
        .await
        .unwrap();

    // Verify: Only rule1 was evaluated (subscribed to tags_changed)
    let logs1 = db
        .get_rule_evaluation_logs_by_rule(&rule1.id, 10, 0)
        .await
        .unwrap();
    let logs2 = db
        .get_rule_evaluation_logs_by_rule(&rule2.id, 10, 0)
        .await
        .unwrap();
    let logs3 = db
        .get_rule_evaluation_logs_by_rule(&rule3.id, 10, 0)
        .await
        .unwrap();

    assert_eq!(logs1.len(), 1, "Rule1 should have been evaluated");
    assert_eq!(logs2.len(), 0, "Rule2 should NOT have been evaluated");
    assert_eq!(logs3.len(), 0, "Rule3 should NOT have been evaluated");

    // Verify: Priority set by rule1
    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.priority, Some(Priority::High));

    teardown_test_db(test_db).await;
}

/// Integration Test: Complex workflow with multiple rules
#[tokio::test]
async fn test_complex_workflow_with_multiple_rules() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup: Create a workflow with multiple rules
    // Rule 1: When Bug tag is added, set priority to High
    let rule1 = AutomationRule::new(
        "Bug Priority Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.tags_changed".to_string()],
        RuleCondition::Simple {
            attribute: "tags".to_string(),
            comparison: ComparisonOperator::Contains,
            value: json!("Bug"),
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        },
    );

    // Rule 2: When priority is High and status is open, assign to user
    let agent = create_test_agent(db, "agent@example.com", "Test Agent").await;
    let rule2 = AutomationRule::new(
        "High Priority Assignment".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.priority_changed".to_string()],
        RuleCondition::And {
            conditions: vec![
                RuleCondition::Simple {
                    attribute: "priority".to_string(),
                    comparison: ComparisonOperator::Equals,
                    value: json!("High"),
                },
                RuleCondition::Simple {
                    attribute: "status".to_string(),
                    comparison: ComparisonOperator::Equals,
                    value: json!("open"),
                },
            ],
        },
        RuleAction {
            action_type: ActionType::AssignToUser,
            parameters: HashMap::from([("user_id".to_string(), json!(agent.user_id))]),
        },
    );

    AutomationRulesRepository::create_automation_rule(db, &rule1).await.unwrap();
    AutomationRulesRepository::create_automation_rule(db, &rule2).await.unwrap();

    let service =
        create_automation_service(db, AutomationConfig::default());

    // Action: Create conversation and add Bug tag
    let contact = create_test_contact(db, "user@example.com").await;
    let mut conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Step 1: Add Bug tag (triggers rule1)
    conversation.tags = Some(vec!["Bug".to_string()]);
    service
        .handle_conversation_event("conversation.tags_changed", &conversation, "system")
        .await
        .unwrap();

    // Verify: Priority was set
    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.priority, Some(Priority::High));

    // Step 2: Trigger priority_changed event (triggers rule2)
    conversation.priority = Some(Priority::High);
    service
        .handle_conversation_event("conversation.priority_changed", &conversation, "system")
        .await
        .unwrap();

    // Verify: Conversation was assigned
    let final_conversation = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        final_conversation.assigned_user_id,
        Some(agent.user_id.clone())
    );
    assert_eq!(final_conversation.priority, Some(Priority::High));

    // Verify: Both rules have evaluation logs
    let logs1 = db
        .get_rule_evaluation_logs_by_rule(&rule1.id, 10, 0)
        .await
        .unwrap();
    let logs2 = db
        .get_rule_evaluation_logs_by_rule(&rule2.id, 10, 0)
        .await
        .unwrap();
    assert_eq!(logs1.len(), 1);
    assert_eq!(logs2.len(), 1);
    assert!(logs1[0].action_executed);
    assert!(logs2[0].action_executed);

    teardown_test_db(test_db).await;
}
