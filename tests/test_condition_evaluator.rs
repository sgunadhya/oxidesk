mod helpers;

use oxidesk::{
    domain::entities::{ComparisonOperator, Conversation, ConversationStatus, Priority, RuleCondition},
    domain::services::condition_evaluator::ConditionEvaluator,
};
use serde_json::json;

#[tokio::test]
async fn test_simple_condition_tags_contains() {
    let condition = RuleCondition::Simple {
        attribute: "tags".to_string(),
        comparison: ComparisonOperator::Contains,
        value: json!("Bug"),
    };

    // Create conversation with tags
    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Bug".to_string(), "High Priority".to_string()]);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_simple_condition_tags_not_contains() {
    let condition = RuleCondition::Simple {
        attribute: "tags".to_string(),
        comparison: ComparisonOperator::Contains,
        value: json!("Bug"),
    };

    // Create conversation without Bug tag
    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Feature".to_string()]);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[tokio::test]
async fn test_simple_condition_priority_equals() {
    let condition = RuleCondition::Simple {
        attribute: "priority".to_string(),
        comparison: ComparisonOperator::Equals,
        value: json!("High"),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.priority = Some(Priority::High);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_simple_condition_priority_not_equals() {
    let condition = RuleCondition::Simple {
        attribute: "priority".to_string(),
        comparison: ComparisonOperator::NotEquals,
        value: json!("Low"),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.priority = Some(Priority::High);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_simple_condition_status_equals() {
    let condition = RuleCondition::Simple {
        attribute: "status".to_string(),
        comparison: ComparisonOperator::Equals,
        value: json!("open"),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.status = ConversationStatus::Open;

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_simple_condition_assigned_user_id_equals() {
    let condition = RuleCondition::Simple {
        attribute: "assigned_user_id".to_string(),
        comparison: ComparisonOperator::Equals,
        value: json!("user-123"),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.assigned_user_id = Some("user-123".to_string());

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_simple_condition_assigned_team_id_equals() {
    let condition = RuleCondition::Simple {
        attribute: "assigned_team_id".to_string(),
        comparison: ComparisonOperator::Equals,
        value: json!("team-456"),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.assigned_team_id = Some("team-456".to_string());

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_and_condition_both_true() {
    let condition = RuleCondition::And {
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
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Bug".to_string()]);
    conversation.priority = Some(Priority::High);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_and_condition_one_false() {
    let condition = RuleCondition::And {
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
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Bug".to_string()]);
    conversation.priority = Some(Priority::Low); // Different priority

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[tokio::test]
async fn test_or_condition_one_true() {
    let condition = RuleCondition::Or {
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
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Feature".to_string()]); // No Bug tag
    conversation.priority = Some(Priority::High); // But high priority

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_or_condition_both_false() {
    let condition = RuleCondition::Or {
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
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Feature".to_string()]);
    conversation.priority = Some(Priority::Low);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[tokio::test]
async fn test_not_condition() {
    let condition = RuleCondition::Not {
        condition: Box::new(RuleCondition::Simple {
            attribute: "status".to_string(),
            comparison: ComparisonOperator::Equals,
            value: json!("resolved"),
        }),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.status = ConversationStatus::Open; // Not resolved

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_nested_condition() {
    // (tags contains "Bug" AND priority = "High") OR status = "open"
    let condition = RuleCondition::Or {
        conditions: vec![
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
            RuleCondition::Simple {
                attribute: "status".to_string(),
                comparison: ComparisonOperator::Equals,
                value: json!("open"),
            },
        ],
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.tags = Some(vec!["Feature".to_string()]); // No Bug
    conversation.priority = Some(Priority::Low); // Low priority
    conversation.status = ConversationStatus::Open; // But status is open

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_invalid_attribute() {
    let condition = RuleCondition::Simple {
        attribute: "invalid_field".to_string(),
        comparison: ComparisonOperator::Equals,
        value: json!("value"),
    };

    let conversation = create_test_conversation_minimal();

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_in_operator() {
    let condition = RuleCondition::Simple {
        attribute: "priority".to_string(),
        comparison: ComparisonOperator::In,
        value: json!(["High", "Urgent"]),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.priority = Some(Priority::High);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_not_in_operator() {
    let condition = RuleCondition::Simple {
        attribute: "priority".to_string(),
        comparison: ComparisonOperator::NotIn,
        value: json!(["High", "Urgent"]),
    };

    let mut conversation = create_test_conversation_minimal();
    conversation.priority = Some(Priority::Low);

    let evaluator = ConditionEvaluator::new();
    let result = evaluator.evaluate(&condition, &conversation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

// Helper function to create a minimal test conversation
fn create_test_conversation_minimal() -> Conversation {
    Conversation {
        id: "conv-123".to_string(),
        reference_number: 1001,
        status: ConversationStatus::Open,
        inbox_id: "inbox-001".to_string(),
        contact_id: "contact-001".to_string(),
        subject: Some("Test conversation".to_string()),
        resolved_at: None,
        closed_at: None, // Feature 019
        snoozed_until: None,
        assigned_user_id: None,
        assigned_team_id: None,
        assigned_at: None,
        assigned_by: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        tags: None,
        priority: None,
    }
}
