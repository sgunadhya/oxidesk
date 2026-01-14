mod helpers;

use helpers::*;
use oxidesk::{
    models::{RuleAction, ActionType, ConversationStatus, Priority},
    services::action_executor::ActionExecutor,
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_execute_set_priority_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create action
    let action = RuleAction {
        action_type: ActionType::SetPriority,
        parameters: HashMap::from([("priority".to_string(), json!("High"))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, "automation-system").await;

    assert!(result.is_ok(), "Set priority action should succeed");

    // Verify priority was set
    let updated_conv = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated_conv.priority, Some(Priority::High));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_assign_to_user_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data
    let contact = create_test_contact(db, "test@example.com").await;
    let agent = create_test_agent(db, "agent@example.com", "Test Agent").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create action
    let action = RuleAction {
        action_type: ActionType::AssignToUser,
        parameters: HashMap::from([("user_id".to_string(), json!(agent.user_id))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, "automation-system").await;

    assert!(result.is_ok(), "Assign to user action should succeed");

    // Verify assignment
    let updated_conv = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated_conv.assigned_user_id, Some(agent.user_id.clone()));
    assert_eq!(updated_conv.assigned_by, Some("automation-system".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_assign_to_team_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create a team
    let team_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO teams (id, name, created_at, updated_at) VALUES (?, ?, datetime('now'), datetime('now'))"
    )
    .bind(&team_id)
    .bind("Test Team")
    .execute(db.pool())
    .await
    .unwrap();

    // Create action
    let action = RuleAction {
        action_type: ActionType::AssignToTeam,
        parameters: HashMap::from([("team_id".to_string(), json!(team_id))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, "automation-system").await;

    assert!(result.is_ok(), "Assign to team action should succeed");

    // Verify assignment
    let updated_conv = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated_conv.assigned_team_id, Some(team_id));
    assert_eq!(updated_conv.assigned_by, Some("automation-system".to_string()));

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_add_tag_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data
    let contact = create_test_contact(db, "test@example.com").await;
    let agent = create_test_agent(db, "agent@example.com", "Test Agent").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create a tag
    let tag_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tags (id, name, created_at, updated_at) VALUES (?, ?, datetime('now'), datetime('now'))"
    )
    .bind(&tag_id)
    .bind("Bug")
    .execute(db.pool())
    .await
    .unwrap();

    // Create action
    let action = RuleAction {
        action_type: ActionType::AddTag,
        parameters: HashMap::from([("tag".to_string(), json!("Bug"))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, agent.user_id.as_str()).await;

    assert!(result.is_ok(), "Add tag action should succeed");

    // Verify tag was added
    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "Bug");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_remove_tag_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test data
    let contact = create_test_contact(db, "test@example.com").await;
    let agent = create_test_agent(db, "agent@example.com", "Test Agent").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create a tag and add it to conversation
    let tag_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tags (id, name, created_at, updated_at) VALUES (?, ?, datetime('now'), datetime('now'))"
    )
    .bind(&tag_id)
    .bind("Bug")
    .execute(db.pool())
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO conversation_tags (conversation_id, tag_id, added_by, added_at) VALUES (?, ?, ?, datetime('now'))"
    )
    .bind(&conversation.id)
    .bind(&tag_id)
    .bind(&agent.user_id)
    .execute(db.pool())
    .await
    .unwrap();

    // Verify tag is present
    let tags_before = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags_before.len(), 1);

    // Create action
    let action = RuleAction {
        action_type: ActionType::RemoveTag,
        parameters: HashMap::from([("tag".to_string(), json!("Bug"))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, agent.user_id.as_str()).await;

    assert!(result.is_ok(), "Remove tag action should succeed");

    // Verify tag was removed
    let tags_after = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags_after.len(), 0);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_change_status_action() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create action
    let action = RuleAction {
        action_type: ActionType::ChangeStatus,
        parameters: HashMap::from([("status".to_string(), json!("resolved"))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, "automation-system").await;

    assert!(result.is_ok(), "Change status action should succeed");

    // Verify status was changed
    let updated_conv = db.get_conversation_by_id(&conversation.id).await.unwrap().unwrap();
    assert_eq!(updated_conv.status, ConversationStatus::Resolved);
    assert!(updated_conv.resolved_at.is_some());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_action_with_invalid_parameters() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create action with missing required parameter
    let action = RuleAction {
        action_type: ActionType::SetPriority,
        parameters: HashMap::new(), // Missing "priority" parameter
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, "automation-system").await;

    assert!(result.is_err(), "Action with invalid parameters should fail");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_action_user_not_found() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test conversation
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Create action with non-existent user
    let action = RuleAction {
        action_type: ActionType::AssignToUser,
        parameters: HashMap::from([("user_id".to_string(), json!("non-existent-user"))]),
    };

    // Execute action
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, &conversation.id, "automation-system").await;

    assert!(result.is_err(), "Action with non-existent user should fail");

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_execute_action_conversation_not_found() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create action
    let action = RuleAction {
        action_type: ActionType::SetPriority,
        parameters: HashMap::from([("priority".to_string(), json!("High"))]),
    };

    // Execute action on non-existent conversation
    let executor = ActionExecutor::new(std::sync::Arc::new(db.clone()));
    let result = executor.execute(&action, "non-existent-conversation", "automation-system").await;

    assert!(result.is_err(), "Action on non-existent conversation should fail");

    teardown_test_db(test_db).await;
}
