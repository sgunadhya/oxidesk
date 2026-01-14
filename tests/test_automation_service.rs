mod helpers;

use helpers::*;
use oxidesk::{
    models::{
        AutomationRule, RuleType, RuleCondition, RuleAction, ActionType, ComparisonOperator,
        ConversationStatus, Priority,
    },
    services::automation_service::{AutomationService, AutomationConfig},
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_rule_matching_by_event_type() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule that subscribes to tags_changed event
    let rule = AutomationRule::new(
        "Tag Changed Rule".to_string(),
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

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation with Bug tag
    let contact = create_test_contact(db, "test@example.com").await;
    let mut conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;
    conversation.tags = Some(vec!["Bug".to_string()]);

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event
    let result = service
        .handle_conversation_event("conversation.tags_changed", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify action was executed
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, Some(Priority::High));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_priority_based_rule_ordering() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rules with different priorities
    let rule1 = AutomationRule {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Low Priority Rule".to_string(),
        description: None,
        enabled: true,
        rule_type: RuleType::ConversationUpdate,
        event_subscription: vec!["conversation.created".to_string()],
        condition: RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        action: RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("Low"))]),
        },
        priority: 500, // Lower priority (higher number)
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let rule2 = AutomationRule {
        id: uuid::Uuid::new_v4().to_string(),
        name: "High Priority Rule".to_string(),
        description: None,
        enabled: true,
        rule_type: RuleType::ConversationUpdate,
        event_subscription: vec!["conversation.created".to_string()],
        condition: RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        action: RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        },
        priority: 100, // Higher priority (lower number)
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    db.create_automation_rule(&rule1).await.unwrap();
    db.create_automation_rule(&rule2).await.unwrap();

    // Create conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event - high priority rule should execute last and win
    let result = service
        .handle_conversation_event("conversation.created", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify high priority rule's action was applied (executed last)
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, Some(Priority::High));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_disabled_rules_are_skipped() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create disabled rule
    let mut rule = AutomationRule::new(
        "Disabled Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.status_changed".to_string()],
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
    rule.enabled = false;

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event
    let result = service
        .handle_conversation_event("conversation.status_changed", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify action was NOT executed (rule disabled)
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, None);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_condition_true_executes_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule with condition that will be true
    let rule = AutomationRule::new(
        "Condition True Rule".to_string(),
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

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation with status=open (condition will be true)
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event
    let result = service
        .handle_conversation_event("conversation.created", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify action was executed
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, Some(Priority::Medium));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_condition_false_skips_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule with condition that will be false
    let rule = AutomationRule::new(
        "Condition False Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.created".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("resolved"), // Conversation will be open, not resolved
        },
        RuleAction {
            action_type: ActionType::SetPriority,
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        },
    );

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation with status=open (condition will be false)
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event
    let result = service
        .handle_conversation_event("conversation.created", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify action was NOT executed (condition false)
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, None);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_cascade_depth_limiting() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule
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
            parameters: HashMap::from([("priority".to_string(), json!("High"))]),
        },
    );

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service with max depth = 0 (no cascading allowed)
    let config = AutomationConfig {
        cascade_max_depth: 0,
        ..Default::default()
    };
    let service = AutomationService::new(std::sync::Arc::new(db.clone()), config);

    // Handle event at depth 1 (should be blocked)
    let result = service
        .handle_conversation_event_with_depth("conversation.created", &conversation, "test-user", 1)
        .await;

    assert!(result.is_ok(), "Event handling should succeed but skip rules");

    // Verify action was NOT executed (depth limit exceeded)
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, None);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_evaluation_logging() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule
    let rule = AutomationRule::new(
        "Logging Test Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.created".to_string()],
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

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event
    let result = service
        .handle_conversation_event("conversation.created", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify evaluation log was created
    let logs = db
        .get_rule_evaluation_logs_by_rule(&rule.id, 10, 0)
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].rule_id, rule.id);
    assert_eq!(logs[0].event_type, "conversation.created");
    assert_eq!(logs[0].conversation_id, Some(conversation.id.clone()));
    assert!(logs[0].matched);
    assert!(logs[0].action_executed);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_multiple_rules_matching_same_event() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create multiple rules for same event
    let rule1 = AutomationRule::new(
        "Rule 1".to_string(),
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

    let rule2 = AutomationRule::new(
        "Rule 2".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.created".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        RuleAction {
            action_type: ActionType::ChangeStatus,
            parameters: HashMap::from([("status".to_string(), json!("resolved"))]),
        },
    );

    db.create_automation_rule(&rule1).await.unwrap();
    db.create_automation_rule(&rule2).await.unwrap();

    // Create conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event
    let result = service
        .handle_conversation_event("conversation.created", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed");

    // Verify both actions were executed
    let updated = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated.priority, Some(Priority::Medium));
    assert_eq!(updated.status, ConversationStatus::Resolved);

    // Verify both logs were created
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

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_rule_evaluation_error_does_not_crash() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule with invalid action parameters (will cause error)
    let rule = AutomationRule::new(
        "Invalid Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.created".to_string()],
        RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("open"),
        },
        RuleAction {
            action_type: ActionType::AssignToUser,
            parameters: HashMap::from([("user_id".to_string(), json!("non-existent-user"))]),
        },
    );

    db.create_automation_rule(&rule).await.unwrap();

    // Create conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create automation service
    let service = AutomationService::new(
        std::sync::Arc::new(db.clone()),
        AutomationConfig::default(),
    );

    // Handle event - should not crash despite error
    let result = service
        .handle_conversation_event("conversation.created", &conversation, "test-user")
        .await;

    assert!(result.is_ok(), "Event handling should succeed even with action error");

    // Verify evaluation log shows error
    let logs = db
        .get_rule_evaluation_logs_by_rule(&rule.id, 10, 0)
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert!(logs[0].matched);
    assert!(!logs[0].action_executed); // Action failed
    assert!(logs[0].error_message.is_some());

    teardown_test_db(test_db).await;
}
