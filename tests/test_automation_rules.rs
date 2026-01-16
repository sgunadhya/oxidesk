mod helpers;

use helpers::*;
use oxidesk::models::{
    ActionType, AutomationRule, ComparisonOperator, RuleAction, RuleCondition, RuleType,
};
use serde_json::json;
use std::collections::HashMap;
use oxidesk::automation_rules::AutomationRulesRepository;

#[tokio::test]
async fn test_create_automation_rule() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a simple rule
    let rule = AutomationRule::new(
        "Test Rule".to_string(),
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

    let result = db.create_automation_rule(&rule).await;
    if let Err(e) = &result {
        eprintln!("Error creating automation rule: {:?}", e);
    }
    assert!(result.is_ok());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_automation_rule_by_id() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a rule
    let rule = AutomationRule::new(
        "Test Rule".to_string(),
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

    // Retrieve the rule
    let retrieved = db.get_automation_rule_by_id(&rule.id).await;
    if let Err(e) = &retrieved {
        eprintln!("Error retrieving automation rule: {:?}", e);
    }
    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_some());

    let retrieved_rule = retrieved.unwrap();
    assert_eq!(retrieved_rule.id, rule.id);
    assert_eq!(retrieved_rule.name, rule.name);
    assert_eq!(retrieved_rule.rule_type, rule.rule_type);
    assert_eq!(retrieved_rule.event_subscription, rule.event_subscription);
    assert_eq!(retrieved_rule.priority, rule.priority);
    assert!(retrieved_rule.enabled);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_automation_rule_by_name() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a rule
    let rule = AutomationRule::new(
        "Test Rule With Unique Name".to_string(),
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

    // Retrieve the rule by name
    let retrieved = db
        .get_automation_rule_by_name("Test Rule With Unique Name")
        .await
        .unwrap();
    assert!(retrieved.is_some());

    let retrieved_rule = retrieved.unwrap();
    assert_eq!(retrieved_rule.name, rule.name);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_all_automation_rules() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create multiple rules
    for i in 1..=3 {
        let rule = AutomationRule::new(
            format!("Test Rule {}", i),
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
    }

    // Get all rules
    let rules = db.get_automation_rules(false).await.unwrap();
    assert_eq!(rules.len(), 3);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_enabled_rules_only() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create enabled rule
    let enabled_rule = AutomationRule::new(
        "Enabled Rule".to_string(),
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
    db.create_automation_rule(&enabled_rule).await.unwrap();

    // Create disabled rule
    let mut disabled_rule = AutomationRule::new(
        "Disabled Rule".to_string(),
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
    disabled_rule.enabled = false;
    db.create_automation_rule(&disabled_rule).await.unwrap();

    // Get only enabled rules
    let enabled_rules = db.get_automation_rules(true).await.unwrap();
    assert_eq!(enabled_rules.len(), 1);
    assert_eq!(enabled_rules[0].name, "Enabled Rule");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_get_enabled_rules_for_event() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule that subscribes to tags_changed
    let rule1 = AutomationRule::new(
        "Tags Rule".to_string(),
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
    db.create_automation_rule(&rule1).await.unwrap();

    // Create rule that subscribes to status_change
    let rule2 = AutomationRule::new(
        "Status Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.status.change".to_string()],
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
    db.create_automation_rule(&rule2).await.unwrap();

    // Get rules for tags_changed event
    let rules = db
        .get_enabled_rules_for_event("conversation.tags_changed")
        .await
        .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "Tags Rule");

    // Get rules for status_change event
    let rules = db
        .get_enabled_rules_for_event("conversation.status.change")
        .await
        .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "Status Rule");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_update_automation_rule() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a rule
    let mut rule = AutomationRule::new(
        "Original Name".to_string(),
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

    // Update the rule
    rule.name = "Updated Name".to_string();
    rule.priority = 50;
    rule.updated_at = chrono::Utc::now().to_rfc3339();
    db.update_automation_rule(&rule).await.unwrap();

    // Verify update
    let updated = db
        .get_automation_rule_by_id(&rule.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.priority, 50);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_delete_automation_rule() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a rule
    let rule = AutomationRule::new(
        "Rule To Delete".to_string(),
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

    // Delete the rule
    db.delete_automation_rule(&rule.id).await.unwrap();

    // Verify deletion
    let deleted = db.get_automation_rule_by_id(&rule.id).await.unwrap();
    assert!(deleted.is_none());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_enable_disable_automation_rule() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a rule
    let rule = AutomationRule::new(
        "Rule To Enable/Disable".to_string(),
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

    // Initially enabled
    let retrieved = db
        .get_automation_rule_by_id(&rule.id)
        .await
        .unwrap()
        .unwrap();
    assert!(retrieved.enabled);

    // Disable
    db.disable_automation_rule(&rule.id).await.unwrap();
    let retrieved = db
        .get_automation_rule_by_id(&rule.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!retrieved.enabled);

    // Enable
    db.enable_automation_rule(&rule.id).await.unwrap();
    let retrieved = db
        .get_automation_rule_by_id(&rule.id)
        .await
        .unwrap()
        .unwrap();
    assert!(retrieved.enabled);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_rule_priority_ordering() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rules with different priorities
    let mut rule1 = AutomationRule::new(
        "High Priority Rule".to_string(),
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
    rule1.priority = 10;
    db.create_automation_rule(&rule1).await.unwrap();

    let mut rule2 = AutomationRule::new(
        "Low Priority Rule".to_string(),
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
    rule2.priority = 100;
    db.create_automation_rule(&rule2).await.unwrap();

    // Get all rules (should be ordered by priority)
    let rules = db.get_automation_rules(false).await.unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].name, "High Priority Rule");
    assert_eq!(rules[1].name, "Low Priority Rule");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_complex_condition_serialization() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create rule with AND condition
    let rule = AutomationRule::new(
        "Complex Rule".to_string(),
        RuleType::ConversationUpdate,
        vec!["conversation.tags_changed".to_string()],
        RuleCondition::And {
            conditions: vec![
                RuleCondition::Simple {
                    attribute: "tags".to_string(),
                    comparison: ComparisonOperator::Contains,
                    value: json!("Bug"),
                },
                RuleCondition::Simple {
                    attribute: "priority".to_string(),
                    comparison: ComparisonOperator::Equals,
                    value: json!("High"),
                },
            ],
        },
        RuleAction {
            action_type: ActionType::AddTag,
            parameters: HashMap::from([("tag".to_string(), json!("Escalated"))]),
        },
    );

    db.create_automation_rule(&rule).await.unwrap();

    // Retrieve and verify
    let retrieved = db
        .get_automation_rule_by_id(&rule.id)
        .await
        .unwrap()
        .unwrap();
    match retrieved.condition {
        RuleCondition::And { conditions } => {
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected And condition"),
    }

    teardown_test_db(test_db).await;
}
