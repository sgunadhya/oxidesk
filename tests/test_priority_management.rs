// Feature 020: Conversation Priority Management Tests
mod helpers;

use helpers::*;
use oxidesk::{
    application::services::conversation_priority_service::ConversationPriorityService,
    domain::entities::{ConversationStatus, Priority},
    domain::ports::event_bus::EventBus,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_update_priority_from_none_to_low() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    // Create a conversation with no priority
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    assert!(
        conversation.priority.is_none(),
        "Initial priority should be none"
    );

    // Update priority to Low
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );
    let updated = priority_service
        .update_conversation_priority(&conversation.id, Some(Priority::Low), "user-123")
        .await
        .expect("Failed to update priority");

    assert_eq!(updated.priority, Some(Priority::Low));
}

#[tokio::test]
async fn test_update_priority_from_low_to_high() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    // Create a conversation with Low priority
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Set initial priority to Low
    db.set_conversation_priority(&conversation.id, &Priority::Low)
        .await
        .expect("Failed to set initial priority");

    // Update priority to High
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );
    let updated = priority_service
        .update_conversation_priority(&conversation.id, Some(Priority::High), "user-123")
        .await
        .expect("Failed to update priority");

    assert_eq!(updated.priority, Some(Priority::High));
}

#[tokio::test]
async fn test_remove_priority() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    // Create a conversation with High priority
    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Set initial priority to High
    db.set_conversation_priority(&conversation.id, &Priority::High)
        .await
        .expect("Failed to set initial priority");

    // Remove priority
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );
    let updated = priority_service
        .update_conversation_priority(&conversation.id, None, "user-123")
        .await
        .expect("Failed to remove priority");

    assert!(updated.priority.is_none(), "Priority should be removed");
}

#[test]
fn test_priority_ordering() {
    assert!(Priority::Low < Priority::Medium);
    assert!(Priority::Medium < Priority::High);
    assert!(Priority::Low < Priority::High);
}
#[tokio::test]
async fn test_same_priority_idempotence() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Set initial priority to Medium
    db.set_conversation_priority(&conversation.id, &Priority::Medium)
        .await
        .expect("Failed to set initial priority");

    // Get the updated_at timestamp
    let before_update = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("DB error")
        .expect("Conversation not found");

    // Sleep for 1 second to ensure different timestamp (SQLite datetime('now') has second precision)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Update to the same priority
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );
    let updated = priority_service
        .update_conversation_priority(&conversation.id, Some(Priority::Medium), "user-123")
        .await
        .expect("Failed to update priority");

    assert_eq!(updated.priority, Some(Priority::Medium));
    // updated_at should still change even for same-priority updates (per spec)
    assert_ne!(updated.updated_at, before_update.updated_at);
}

#[tokio::test]
async fn test_priority_update_triggers_automation_event() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;
    let event_bus = std::sync::Arc::new(oxidesk::LocalEventBus::new(100));
    let mut rx = event_bus.subscribe();

    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Update priority with event bus
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        Some(event_bus.clone()),
    );
    let _updated = priority_service
        .update_conversation_priority(&conversation.id, Some(Priority::High), "user-123")
        .await
        .expect("Failed to update priority");

    // Receive the event
    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.next())
        .await
        .expect("Timeout waiting for event")
        .expect("Failed to receive event")
        .expect("Broadcast error");

    match event {
        oxidesk::events::SystemEvent::ConversationPriorityChanged {
            conversation_id,
            previous_priority,
            new_priority,
            updated_by,
            ..
        } => {
            assert_eq!(conversation_id, conversation.id);
            assert_eq!(previous_priority, None);
            assert_eq!(new_priority, Some("High".to_string()));
            assert_eq!(updated_by, "user-123");
        }
        _ => panic!("Expected ConversationPriorityChanged event"),
    }
}

#[tokio::test]
async fn test_same_priority_no_automation_trigger() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;
    let event_bus = std::sync::Arc::new(oxidesk::LocalEventBus::new(100));
    let mut rx = event_bus.subscribe();

    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Set initial priority
    db.set_conversation_priority(&conversation.id, &Priority::Medium)
        .await
        .expect("Failed to set initial priority");

    // Update to same priority (idempotent)
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        Some(event_bus.clone()),
    );
    let _updated = priority_service
        .update_conversation_priority(&conversation.id, Some(Priority::Medium), "user-123")
        .await
        .expect("Failed to update priority");

    // Should NOT receive an event (idempotent - no change)
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.next()).await;
    assert!(
        result.is_err(),
        "Should not trigger event for same-priority update"
    );
}

#[tokio::test]
async fn test_priority_removal_triggers_automation_event() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;
    let event_bus = std::sync::Arc::new(oxidesk::LocalEventBus::new(100));
    let mut rx = event_bus.subscribe();

    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Set initial priority
    db.set_conversation_priority(&conversation.id, &Priority::High)
        .await
        .expect("Failed to set initial priority");

    // Remove priority
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        Some(event_bus.clone()),
    );
    let _updated = priority_service
        .update_conversation_priority(&conversation.id, None, "user-123")
        .await
        .expect("Failed to remove priority");

    // Receive the event
    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.next())
        .await
        .expect("Timeout waiting for event")
        .expect("Failed to receive event")
        .expect("Broadcast error");

    match event {
        oxidesk::events::SystemEvent::ConversationPriorityChanged {
            conversation_id,
            previous_priority,
            new_priority,
            updated_by,
            ..
        } => {
            assert_eq!(conversation_id, conversation.id);
            assert_eq!(previous_priority, Some("High".to_string()));
            assert_eq!(new_priority, None);
            assert_eq!(updated_by, "user-123");
        }
        _ => panic!("Expected ConversationPriorityChanged event"),
    }
}

#[tokio::test]
async fn test_priority_update_on_resolved_conversation() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    let contact = create_test_contact(db, "test@example.com").await;
    let conversation = create_test_conversation(
        db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Resolved,
    )
    .await;

    // Should be able to update priority even on resolved conversation
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );
    let updated = priority_service
        .update_conversation_priority(&conversation.id, Some(Priority::High), "user-123")
        .await
        .expect("Failed to update priority on resolved conversation");

    assert_eq!(updated.priority, Some(Priority::High));
    assert_eq!(updated.status, ConversationStatus::Resolved);
}

#[tokio::test]
async fn test_priority_update_nonexistent_conversation() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );
    let result = priority_service
        .update_conversation_priority("nonexistent-id", Some(Priority::High), "user-123")
        .await;

    assert!(result.is_err(), "Should fail for nonexistent conversation");
    match result {
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("not found"),
                "Error should mention conversation not found"
            );
        }
        Ok(_) => panic!("Expected error for nonexistent conversation"),
    }
}

#[tokio::test]
async fn test_all_valid_priority_values() {
    let test_db = setup_test_db().await;
    let db = &test_db.db;

    let contact = create_test_contact(db, "test@example.com").await;

    let valid_priorities = vec![Priority::Low, Priority::Medium, Priority::High];
    let priority_service = ConversationPriorityService::new(
        std::sync::Arc::new(db.clone())
            as std::sync::Arc<
                dyn oxidesk::domain::ports::conversation_repository::ConversationRepository,
            >,
        None,
    );

    for priority in valid_priorities {
        let conversation = create_test_conversation(
            db,
            "inbox-001".to_string(),
            contact.id.clone(),
            ConversationStatus::Open,
        )
        .await;

        let updated = priority_service
            .update_conversation_priority(&conversation.id, Some(priority), "user-123")
            .await
            .expect(&format!("Failed to set priority to {}", priority));

        assert_eq!(updated.priority, Some(priority));
    }
}
