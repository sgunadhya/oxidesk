use oxidesk::domain::entities::conversation::ConversationStatus;

mod helpers;
use helpers::*;

#[tokio::test]
async fn test_conversation_created_with_open_status() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test contact
    let contact = create_test_contact(&db, "customer@example.com").await;
    let inbox_id = "inbox-001".to_string();

    // Create conversation
    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    assert_eq!(conversation.status, ConversationStatus::Open);
    assert_eq!(conversation.inbox_id, inbox_id);
    assert_eq!(conversation.contact_id, contact.id);
}

#[tokio::test]
async fn test_conversation_has_unique_uuid() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let contact = create_test_contact(&db, "customer1@example.com").await;
    let inbox_id = "inbox-001".to_string();

    // Create two conversations
    let conv1 = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let conv2 = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // UUIDs should be different
    assert_ne!(conv1.id, conv2.id);
    // Both should be valid (non-empty)
    assert!(!conv1.id.is_empty());
    assert!(!conv2.id.is_empty());
}

#[tokio::test]
async fn test_conversation_reference_number_starts_at_100() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let contact = create_test_contact(&db, "customer2@example.com").await;
    let inbox_id = "inbox-001".to_string();

    // Clear any existing conversations to ensure we start fresh
    sqlx::query("DELETE FROM conversations")
        .execute(db.pool())
        .await
        .unwrap();

    // Create first conversation
    let conv1 = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // First conversation should have reference number 100
    assert_eq!(conv1.reference_number, 100);

    // Create second conversation
    let conv2 = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Second should be 101
    assert_eq!(conv2.reference_number, 101);
}

#[tokio::test]
async fn test_conversation_assigned_to_inbox() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let contact = create_test_contact(&db, "customer3@example.com").await;
    let inbox_id = "inbox-001".to_string();

    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Verify inbox assignment
    assert_eq!(conversation.inbox_id, inbox_id);

    // Verify foreign key relationship (would fail if inbox doesn't exist)
    let result = sqlx::query("SELECT id FROM inboxes WHERE id = ?")
        .bind(&conversation.inbox_id)
        .fetch_one(db.pool())
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_conversation_linked_to_contact() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let contact = create_test_contact(&db, "customer4@example.com").await;
    let inbox_id = "inbox-001".to_string();

    let conversation = create_test_conversation(
        &db,
        inbox_id.clone(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    // Verify contact linkage
    assert_eq!(conversation.contact_id, contact.id);

    // Verify foreign key relationship
    let result = sqlx::query("SELECT id FROM contacts WHERE id = ?")
        .bind(&conversation.contact_id)
        .fetch_one(db.pool())
        .await;

    assert!(result.is_ok());

    // Verify we can join conversations with contacts and users
    let joined: (String, String) = sqlx::query_as(
        "SELECT c.id, u.email FROM conversations c
         JOIN contacts con ON c.contact_id = con.id
         JOIN users u ON con.user_id = u.id
         WHERE c.id = ?",
    )
    .bind(&conversation.id)
    .fetch_one(db.pool())
    .await
    .unwrap();

    assert_eq!(joined.0, conversation.id);
    assert_eq!(joined.1, "customer4@example.com");
}
