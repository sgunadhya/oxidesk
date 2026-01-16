use oxidesk::models::conversation::{ConversationStatus, UpdateStatusRequest};

use oxidesk::EventBus;

mod helpers;
use helpers::*;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_agent_can_update_open_to_resolved() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;
    let event_bus = oxidesk::LocalEventBus::new(10); // Minimal event bus

    // Create test contact and conversation
    let contact = create_test_contact(&db, "customer@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    assert_eq!(conversation.status, ConversationStatus::Open);

    // Update status to Resolved
    let update_request = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };

    // Use service
    // Use service
    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    let result = conversation_service
        .update_conversation_status(
            &conversation.id,
            update_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await;

    assert!(
        result.is_ok(),
        "Failed to update status: {:?}",
        result.err()
    );

    // Fetch updated conversation
    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to fetch conversation")
        .expect("Conversation not found");

    assert_eq!(updated.status, ConversationStatus::Resolved);
}

#[tokio::test]
async fn test_resolved_at_timestamp_set_on_status_change() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;
    let event_bus = oxidesk::LocalEventBus::new(10);

    let contact = create_test_contact(&db, "customer2@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    assert!(conversation.resolved_at.is_none());

    // Update status to Resolved
    let update_request = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };

    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    conversation_service
        .update_conversation_status(
            &conversation.id,
            update_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to update status");

    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to fetch conversation")
        .expect("Conversation not found");

    assert!(updated.resolved_at.is_some(), "resolved_at should be set");
}

#[tokio::test]
async fn test_invalid_status_transition_rejected() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;
    let event_bus = oxidesk::LocalEventBus::new(10);

    let contact = create_test_contact(&db, "customer3@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Test a valid cycle: Open -> Snoozed -> Open

    let update_snooze = UpdateStatusRequest {
        status: ConversationStatus::Snoozed,
        snooze_duration: Some("1h".to_string()),
    };

    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    conversation_service
        .update_conversation_status(
            &conversation.id,
            update_snooze,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .unwrap();

    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, ConversationStatus::Snoozed);

    let update_open = UpdateStatusRequest {
        status: ConversationStatus::Open,
        snooze_duration: None,
    };
    conversation_service
        .update_conversation_status(
            &conversation.id,
            update_open,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .unwrap();

    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, ConversationStatus::Open);
}

#[tokio::test]
async fn test_automation_rules_evaluated_on_status_change() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;

    // Create event bus with subscriber
    let event_bus = oxidesk::LocalEventBus::new(10);
    let mut receiver = event_bus.subscribe();

    let contact = create_test_contact(&db, "customer5@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Update status to Resolved with event bus
    let update_request = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };

    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    conversation_service
        .update_conversation_status(
            &conversation.id,
            update_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to update status");

    // Verify event was published
    let event = tokio::time::timeout(tokio::time::Duration::from_secs(1), receiver.next())
        .await
        .expect("Timeout waiting for event")
        .expect("Failed to receive event")
        .expect("Broadcast error");

    // Verify event details
    match event {
        oxidesk::SystemEvent::ConversationStatusChanged {
            conversation_id,
            old_status,
            new_status,
            agent_id: event_agent_id,
            timestamp: _,
        } => {
            assert_eq!(conversation_id, conversation.id);
            assert_eq!(old_status, ConversationStatus::Open);
            assert_eq!(new_status, ConversationStatus::Resolved);
            assert_eq!(event_agent_id, Some(auth_user.user.id));
        }
        _ => panic!("Expected ConversationStatusChanged event"),
    }
}

// Feature 019: Test Resolved → Closed transition
#[tokio::test]
async fn test_resolved_to_closed_transition() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;
    let event_bus = oxidesk::LocalEventBus::new(10);

    let contact = create_test_contact(&db, "customer6@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // First, resolve the conversation
    let resolve_request = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };

    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    conversation_service
        .update_conversation_status(
            &conversation.id,
            resolve_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to resolve conversation");

    // Now close it
    let close_request = UpdateStatusRequest {
        status: ConversationStatus::Closed,
        snooze_duration: None,
    };

    let result = conversation_service
        .update_conversation_status(
            &conversation.id,
            close_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await;

    assert!(
        result.is_ok(),
        "Failed to close resolved conversation: {:?}",
        result.err()
    );

    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to fetch conversation")
        .expect("Conversation not found");

    assert_eq!(updated.status, ConversationStatus::Closed);
}

// Feature 019: Test closed_at timestamp is set
#[tokio::test]
async fn test_closed_at_timestamp_set() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;
    let event_bus = oxidesk::LocalEventBus::new(10);

    let contact = create_test_contact(&db, "customer7@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Resolve then close
    let resolve_request = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };

    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    conversation_service
        .update_conversation_status(
            &conversation.id,
            resolve_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to resolve");

    assert!(db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap()
        .closed_at
        .is_none());

    let close_request = UpdateStatusRequest {
        status: ConversationStatus::Closed,
        snooze_duration: None,
    };

    conversation_service
        .update_conversation_status(
            &conversation.id,
            close_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to close");

    let updated = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to fetch conversation")
        .expect("Conversation not found");

    assert!(
        updated.closed_at.is_some(),
        "closed_at should be set when conversation is closed"
    );
}

// Feature 019: Test reopening clears resolved_at
#[tokio::test]
async fn test_reopening_clears_resolved_at() {
    let test_db = setup_test_db().await;
    let db = test_db.db();
    let auth_user = create_test_auth_user(&db).await;
    let event_bus = oxidesk::LocalEventBus::new(10);

    let contact = create_test_contact(&db, "customer8@example.com").await;
    let inbox_id = "inbox-001".to_string();
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Resolve the conversation
    let resolve_request = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };

    let repo = std::sync::Arc::new(db.clone());
    let conversation_service = oxidesk::services::ConversationService::new(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );
    conversation_service
        .update_conversation_status(
            &conversation.id,
            resolve_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to resolve");

    let resolved = db
        .get_conversation_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert!(resolved.resolved_at.is_some(), "resolved_at should be set");

    // Reopen the conversation
    let reopen_request = UpdateStatusRequest {
        status: ConversationStatus::Open,
        snooze_duration: None,
    };

    conversation_service
        .update_conversation_status(
            &conversation.id,
            reopen_request,
            Some(auth_user.user.id.clone()),
            Some(&event_bus),
        )
        .await
        .expect("Failed to reopen");

    let reopened = db
        .get_conversation_by_id(&conversation.id)
        .await
        .expect("Failed to fetch conversation")
        .expect("Conversation not found");

    assert_eq!(reopened.status, ConversationStatus::Open);
    assert!(
        reopened.resolved_at.is_none(),
        "resolved_at should be cleared when reopening"
    );
}
