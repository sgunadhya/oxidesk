// Integration tests for conversation tagging system (Feature 005)
use oxidesk::domain::entities::conversation::ConversationStatus;

mod helpers;
use helpers::*;

#[tokio::test]
async fn test_tag_crud_operations() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create a tag
    let tag1 = create_test_tag(
        &db,
        "Bug",
        Some("Technical issues".to_string()),
        Some("#FF0000".to_string()),
    )
    .await;
    assert_eq!(tag1.name, "Bug");
    assert_eq!(tag1.description, Some("Technical issues".to_string()));
    assert_eq!(tag1.color, Some("#FF0000".to_string()));

    // Get tag by ID
    let retrieved = db
        .get_tag_by_id(&tag1.id)
        .await
        .expect("Failed to get tag")
        .expect("Tag not found");
    assert_eq!(retrieved.name, "Bug");

    // Get tag by name
    let retrieved_by_name = db
        .get_tag_by_name("Bug")
        .await
        .expect("Failed to get tag")
        .expect("Tag not found");
    assert_eq!(retrieved_by_name.id, tag1.id);

    // List tags
    let (tags, total) = db.list_tags(10, 0).await.expect("Failed to list tags");
    assert_eq!(total, 1);
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "Bug");

    // Update tag
    db.update_tag(
        &tag1.id,
        Some("Critical bugs".to_string()),
        Some("#CC0000".to_string()),
    )
    .await
    .expect("Failed to update tag");

    let updated = db
        .get_tag_by_id(&tag1.id)
        .await
        .expect("Failed to get tag")
        .expect("Tag not found");
    assert_eq!(updated.description, Some("Critical bugs".to_string()));
    assert_eq!(updated.color, Some("#CC0000".to_string()));

    // Delete tag
    db.delete_tag(&tag1.id).await.expect("Failed to delete tag");
    let deleted = db.get_tag_by_id(&tag1.id).await.expect("Failed to get tag");
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_add_tags_to_conversation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let tag1 = create_test_tag(&db, "Bug", None, None).await;
    let tag2 = create_test_tag(&db, "Urgent", None, None).await;

    // Initially no tags
    let tags = db
        .get_conversation_tags(&conversation.id)
        .await
        .expect("Failed to get tags");
    assert_eq!(tags.len(), 0);

    // Add first tag
    db.add_conversation_tag(&conversation.id, &tag1.id, &agent.user_id)
        .await
        .expect("Failed to add tag");

    let tags = db
        .get_conversation_tags(&conversation.id)
        .await
        .expect("Failed to get tags");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "Bug");

    // Add second tag
    db.add_conversation_tag(&conversation.id, &tag2.id, &agent.user_id)
        .await
        .expect("Failed to add tag");

    let tags = db
        .get_conversation_tags(&conversation.id)
        .await
        .expect("Failed to get tags");
    assert_eq!(tags.len(), 2);

    // Add duplicate tag (idempotent - should succeed)
    db.add_conversation_tag(&conversation.id, &tag1.id, &agent.user_id)
        .await
        .expect("Failed to add duplicate tag (should be idempotent)");

    let tags = db
        .get_conversation_tags(&conversation.id)
        .await
        .expect("Failed to get tags");
    assert_eq!(tags.len(), 2); // Still 2, not 3
}

#[tokio::test]
async fn test_remove_tag_from_conversation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let tag1 = create_test_tag(&db, "Bug", None, None).await;
    let tag2 = create_test_tag(&db, "Urgent", None, None).await;

    // Add two tags
    db.add_conversation_tag(&conversation.id, &tag1.id, &agent.user_id)
        .await
        .unwrap();
    db.add_conversation_tag(&conversation.id, &tag2.id, &agent.user_id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 2);

    // Remove one tag
    db.remove_conversation_tag(&conversation.id, &tag1.id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "Urgent");

    // Remove non-existent tag (idempotent - should succeed)
    db.remove_conversation_tag(&conversation.id, &tag1.id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 1); // Still 1
}

#[tokio::test]
async fn test_replace_conversation_tags() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let tag1 = create_test_tag(&db, "Bug", None, None).await;
    let tag2 = create_test_tag(&db, "Urgent", None, None).await;
    let tag3 = create_test_tag(&db, "Feature Request", None, None).await;

    // Add two tags
    db.add_conversation_tag(&conversation.id, &tag1.id, &agent.user_id)
        .await
        .unwrap();
    db.add_conversation_tag(&conversation.id, &tag2.id, &agent.user_id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 2);

    // Replace with single new tag
    db.replace_conversation_tags(&conversation.id, &[tag3.id.clone()], &agent.user_id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "Feature Request");

    // Replace with empty list (remove all)
    db.replace_conversation_tags(&conversation.id, &[], &agent.user_id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 0);
}

#[tokio::test]
async fn test_get_conversations_by_tag() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact1 = create_test_contact(&db, "customer1@example.com").await;
    let contact2 = create_test_contact(&db, "customer2@example.com").await;

    let conv1 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact1.id.clone(),
        ConversationStatus::Open,
    )
    .await;
    let conv2 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact2.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let bug_tag = create_test_tag(&db, "Bug", None, None).await;
    let urgent_tag = create_test_tag(&db, "Urgent", None, None).await;

    // Tag conv1 with Bug
    db.add_conversation_tag(&conv1.id, &bug_tag.id, &agent.user_id)
        .await
        .unwrap();

    // Tag conv2 with Bug and Urgent
    db.add_conversation_tag(&conv2.id, &bug_tag.id, &agent.user_id)
        .await
        .unwrap();
    db.add_conversation_tag(&conv2.id, &urgent_tag.id, &agent.user_id)
        .await
        .unwrap();

    // Get conversations with Bug tag
    let (convs, total) = db
        .get_conversations_by_tag(&bug_tag.id, 10, 0)
        .await
        .unwrap();
    assert_eq!(total, 2);
    assert_eq!(convs.len(), 2);

    // Get conversations with Urgent tag
    let (convs, total) = db
        .get_conversations_by_tag(&urgent_tag.id, 10, 0)
        .await
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(convs.len(), 1);
    assert_eq!(convs[0].id, conv2.id);
}

#[tokio::test]
async fn test_tag_deletion_cascades() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact = create_test_contact(&db, "customer@example.com").await;
    let conversation = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact.id.clone(),
        ConversationStatus::Open,
    )
    .await;

    let tag = create_test_tag(&db, "Bug", None, None).await;

    // Add tag to conversation
    db.add_conversation_tag(&conversation.id, &tag.id, &agent.user_id)
        .await
        .unwrap();

    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 1);

    // Delete tag
    db.delete_tag(&tag.id).await.unwrap();

    // Conversation should have no tags now (cascaded)
    let tags = db.get_conversation_tags(&conversation.id).await.unwrap();
    assert_eq!(tags.len(), 0);
}

#[tokio::test]
async fn test_get_conversations_by_multiple_tags_or() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact1 = create_test_contact(&db, "customer1@example.com").await;
    let contact2 = create_test_contact(&db, "customer2@example.com").await;
    let contact3 = create_test_contact(&db, "customer3@example.com").await;

    let conv1 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact1.id,
        ConversationStatus::Open,
    )
    .await;
    let conv2 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact2.id,
        ConversationStatus::Open,
    )
    .await;
    let conv3 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact3.id,
        ConversationStatus::Open,
    )
    .await;

    let bug_tag = create_test_tag(&db, "Bug", None, None).await;
    let urgent_tag = create_test_tag(&db, "Urgent", None, None).await;

    // conv1: Bug
    db.add_conversation_tag(&conv1.id, &bug_tag.id, &agent.user_id)
        .await
        .unwrap();

    // conv2: Urgent
    db.add_conversation_tag(&conv2.id, &urgent_tag.id, &agent.user_id)
        .await
        .unwrap();

    // conv3: Bug + Urgent
    db.add_conversation_tag(&conv3.id, &bug_tag.id, &agent.user_id)
        .await
        .unwrap();
    db.add_conversation_tag(&conv3.id, &urgent_tag.id, &agent.user_id)
        .await
        .unwrap();

    // Get conversations with Bug OR Urgent (match_all=false)
    let (convs, total) = db
        .get_conversations_by_tags(&[bug_tag.id.clone(), urgent_tag.id.clone()], false, 10, 0)
        .await
        .unwrap();

    assert_eq!(total, 3); // All 3 conversations have at least one tag
    assert_eq!(convs.len(), 3);
}

#[tokio::test]
async fn test_get_conversations_by_multiple_tags_and() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Setup
    let agent = create_test_agent(&db, "agent@example.com", "Agent").await;
    let contact1 = create_test_contact(&db, "customer1@example.com").await;
    let contact2 = create_test_contact(&db, "customer2@example.com").await;
    let contact3 = create_test_contact(&db, "customer3@example.com").await;

    let conv1 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact1.id,
        ConversationStatus::Open,
    )
    .await;
    let conv2 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact2.id,
        ConversationStatus::Open,
    )
    .await;
    let conv3 = create_test_conversation(
        &db,
        "inbox-001".to_string(),
        contact3.id,
        ConversationStatus::Open,
    )
    .await;

    let bug_tag = create_test_tag(&db, "Bug", None, None).await;
    let urgent_tag = create_test_tag(&db, "Urgent", None, None).await;

    // conv1: Bug
    db.add_conversation_tag(&conv1.id, &bug_tag.id, &agent.user_id)
        .await
        .unwrap();

    // conv2: Urgent
    db.add_conversation_tag(&conv2.id, &urgent_tag.id, &agent.user_id)
        .await
        .unwrap();

    // conv3: Bug + Urgent
    db.add_conversation_tag(&conv3.id, &bug_tag.id, &agent.user_id)
        .await
        .unwrap();
    db.add_conversation_tag(&conv3.id, &urgent_tag.id, &agent.user_id)
        .await
        .unwrap();

    // Get conversations with Bug AND Urgent (match_all=true)
    let (convs, total) = db
        .get_conversations_by_tags(&[bug_tag.id.clone(), urgent_tag.id.clone()], true, 10, 0)
        .await
        .unwrap();

    assert_eq!(total, 1); // Only conv3 has both tags
    assert_eq!(convs.len(), 1);
    assert_eq!(convs[0].id, conv3.id);
}
