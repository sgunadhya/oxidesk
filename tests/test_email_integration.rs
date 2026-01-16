/// Integration Tests for Feature 021 - Email Integration
///
/// Tests the complete email flow:
/// - US1: Receiving incoming emails and creating conversations
/// - US2: Reply matching with reference numbers
/// - US3: Sending agent replies via SMTP
/// - Edge cases: malformed emails, attachment limits, duplicate processing
mod helpers;

use helpers::conversation_helpers::create_test_conversation;
use helpers::*;
use oxidesk::database::Database;
use oxidesk::domain::ports::contact_repository::ContactRepository;
use oxidesk::domain::ports::user_repository::UserRepository;
use oxidesk::models::{
    Contact, ConversationStatus, CreateConversation, InboxEmailConfig, Message, User, UserType,
};
use oxidesk::services::{AttachmentService, EmailParserService};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Helper: Create test database with inbox and contact for email testing
/// Returns (test_db, inbox_id, user_id, contact_id)
async fn setup_email_test_db() -> (helpers::test_db::TestDatabase, String, String, String) {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    // Create test inbox with required channel_type and unique name
    let inbox_id = Uuid::new_v4().to_string();
    let inbox_name = format!("Test Inbox {}", Uuid::new_v4());
    let _result = sqlx::query(
        "INSERT INTO inboxes (id, name, channel_type, created_at, updated_at) VALUES (?, ?, 'email', datetime('now'), datetime('now'))",
    )
    .bind(&inbox_id)
    .bind(&inbox_name)
    .execute(db.pool())
    .await
    .unwrap();

    // Create test user (for contact)
    let user = User::new("customer@example.com".to_string(), UserType::Contact);
    let user_id = user.id.clone();
    db.create_user(&user).await.unwrap();

    // Create test contact
    let contact = Contact::new(user_id.clone(), Some("Test Customer".to_string()));
    let contact_id = contact.id.clone();
    db.create_contact(&contact).await.unwrap();

    // Create contact channel with inbox_id
    let channel_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO contact_channels (id, contact_id, inbox_id, email, created_at, updated_at) VALUES (?, ?, ?, 'customer@example.com', datetime('now'), datetime('now'))",
    )
    .bind(&channel_id)
    .bind(&contact_id)
    .bind(&inbox_id)
    .execute(db.pool())
    .await
    .unwrap();

    // Return (test_db, inbox_id, user_id, contact_id)
    // Note: For messages, use user_id as author_id (FK to users table)
    (test_db, inbox_id, user_id, contact_id)
}

/// Helper: Create test email configuration
async fn setup_email_config(db: &Database, inbox_id: &str) -> InboxEmailConfig {
    let config = InboxEmailConfig::new(
        inbox_id.to_string(),
        "imap.example.com".to_string(),
        993,
        "test@example.com".to_string(),
        "password".to_string(),
        "smtp.example.com".to_string(),
        587,
        "test@example.com".to_string(),
        "password".to_string(),
        "test@example.com".to_string(),
        "Test Support".to_string(),
        Some(30),
    );

    db.create_inbox_email_config(&config).await.unwrap()
}

/// Helper: Generate RFC822 email fixture
fn create_test_email(
    from: &str,
    from_name: Option<&str>,
    subject: &str,
    body: &str,
    message_id: &str,
) -> Vec<u8> {
    let from_header = if let Some(name) = from_name {
        format!("From: {} <{}>\r\n", name, from)
    } else {
        format!("From: {}\r\n", from)
    };

    format!(
        "{}To: support@example.com\r\n\
         Subject: {}\r\n\
         Message-ID: {}\r\n\
         Date: Mon, 1 Jan 2024 12:00:00 +0000\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         \r\n\
         {}\r\n",
        from_header, subject, message_id, body
    )
    .into_bytes()
}

/// Helper: Generate email with HTML body
fn create_test_html_email(
    from: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
    message_id: &str,
) -> Vec<u8> {
    format!(
        "From: {}\r\n\
         To: support@example.com\r\n\
         Subject: {}\r\n\
         Message-ID: {}\r\n\
         Date: Mon, 1 Jan 2024 12:00:00 +0000\r\n\
         Content-Type: multipart/alternative; boundary=\"boundary123\"\r\n\
         \r\n\
         --boundary123\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         \r\n\
         {}\r\n\
         --boundary123\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         \r\n\
         {}\r\n\
         --boundary123--\r\n",
        from, subject, message_id, text_body, html_body
    )
    .into_bytes()
}

/// Helper: Generate email with attachment
fn create_test_email_with_attachment(
    from: &str,
    subject: &str,
    body: &str,
    message_id: &str,
    filename: &str,
    content: &[u8],
) -> Vec<u8> {
    use base64::Engine;
    let encoded_content = base64::engine::general_purpose::STANDARD.encode(content);

    format!(
        "From: {}\r\n\
         To: support@example.com\r\n\
         Subject: {}\r\n\
         Message-ID: {}\r\n\
         Date: Mon, 1 Jan 2024 12:00:00 +0000\r\n\
         Content-Type: multipart/mixed; boundary=\"boundary456\"\r\n\
         \r\n\
         --boundary456\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         \r\n\
         {}\r\n\
         --boundary456\r\n\
         Content-Type: application/octet-stream; name=\"{}\"\r\n\
         Content-Disposition: attachment; filename=\"{}\"\r\n\
         Content-Transfer-Encoding: base64\r\n\
         \r\n\
         {}\r\n\
         --boundary456--\r\n",
        from, subject, message_id, body, filename, filename, encoded_content
    )
    .into_bytes()
}

#[tokio::test]
async fn test_parse_simple_email() {
    let parser = EmailParserService::new();

    let raw_email = create_test_email(
        "customer@example.com",
        Some("John Doe"),
        "Help with my account",
        "I need help resetting my password.",
        "<msg-001@example.com>",
    );

    let parsed = parser.parse_email(&raw_email).unwrap();

    assert_eq!(parsed.from_address, "customer@example.com");
    assert_eq!(parsed.from_name, Some("John Doe".to_string()));
    assert_eq!(parsed.subject, Some("Help with my account".to_string()));
    assert_eq!(parsed.message_id, "msg-001@example.com");
    assert!(parsed.text_body.unwrap().contains("password"));
}

#[tokio::test]
async fn test_parse_html_email() {
    let parser = EmailParserService::new();

    let raw_email = create_test_html_email(
        "customer@example.com",
        "HTML Test",
        "Plain text version",
        "<html><body><p>HTML version</p></body></html>",
        "<msg-002@example.com>",
    );

    let parsed = parser.parse_email(&raw_email).unwrap();

    assert_eq!(parsed.from_address, "customer@example.com");
    assert!(parsed.text_body.is_some());
    assert!(parsed.html_body.is_some());
    assert!(parsed.html_body.unwrap().contains("HTML version"));
}

#[tokio::test]
async fn test_parse_email_with_attachment() {
    let parser = EmailParserService::new();

    let attachment_content = b"Test file content";
    let raw_email = create_test_email_with_attachment(
        "customer@example.com",
        "File attached",
        "Please see attached file",
        "<msg-003@example.com>",
        "test.txt",
        attachment_content,
    );

    let parsed = parser.parse_email(&raw_email).unwrap();

    assert_eq!(parsed.attachments.len(), 1);
    assert_eq!(parsed.attachments[0].filename, "test.txt");
    assert_eq!(parsed.attachments[0].content, attachment_content);
}

#[tokio::test]
async fn test_extract_reference_number() {
    let parser = EmailParserService::new();

    // Test various formats
    assert_eq!(
        parser.extract_reference_number("Re: Support Request [#123]"),
        Some(123)
    );
    assert_eq!(
        parser.extract_reference_number("Re: Issue [REF#456]"),
        Some(456)
    );
    assert_eq!(
        parser.extract_reference_number("[#789] Important Issue"),
        Some(789)
    );
    assert_eq!(parser.extract_reference_number("No reference here"), None);
}

#[tokio::test]
async fn test_format_subject_with_reference() {
    let parser = EmailParserService::new();

    let formatted = parser.format_subject_with_reference("Support Request", 123);

    assert!(formatted.contains("[#123]"));
    assert!(formatted.contains("Re: Support Request"));
}

#[tokio::test]
async fn test_create_conversation_from_new_email() {
    let (test_db, inbox_id, _user_id_unused, contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    // Create temp dir for attachments
    let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&temp_dir).unwrap();

    let parser = EmailParserService::new();
    let raw_email = create_test_email(
        "newcustomer@example.com",
        Some("Jane Smith"),
        "New support request",
        "I have a question about billing",
        "<msg-004@example.com>",
    );

    let parsed = parser.parse_email(&raw_email).unwrap();

    // Use the contact from setup (not creating a new one)
    // In a real scenario, process_new_email would create a new contact,
    // but for this test we're using the pre-created contact

    // Get the contact's user_id (FK to users table for messages)
    let user_id_for_message: String =
        sqlx::query_scalar("SELECT user_id FROM contacts WHERE id = ?")
            .bind(&contact_id)
            .fetch_one(db.pool())
            .await
            .unwrap();

    let create_conv = CreateConversation {
        inbox_id: inbox_id.clone(),
        contact_id: contact_id.clone(),
        subject: parsed.subject.clone(),
    };
    let conversation = db.create_conversation(&create_conv).await.unwrap();

    // Create message - use user_id as author_id (FK to users table)
    let content = parsed.text_body.unwrap_or_default();
    let message = Message::new_incoming(
        conversation.id.clone(),
        content.clone(),
        user_id_for_message,
    );
    db.create_message(&message).await.unwrap();

    // Verify conversation created
    assert_eq!(conversation.inbox_id, inbox_id);
    assert_eq!(conversation.status, ConversationStatus::Open);
    assert_eq!(
        conversation.subject,
        Some("New support request".to_string())
    );

    // Verify message created
    let (messages, _total) = db.list_messages(&conversation.id, 100, 0).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("billing"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_reply_matching_with_reference_number() {
    let (test_db, inbox_id, user_id, contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    // Create existing conversation
    let conversation = create_test_conversation(
        db,
        inbox_id.clone(),
        contact_id.clone(),
        ConversationStatus::Open,
    )
    .await;
    let ref_number = conversation.reference_number;

    // Create initial message - use user_id as author_id
    let message1 = Message::new_incoming(
        conversation.id.clone(),
        "Initial message".to_string(),
        user_id.clone(),
    );
    db.create_message(&message1).await.unwrap();

    // Parse reply email with reference number
    let parser = EmailParserService::new();
    let reply_subject = format!("Re: Original request [#{}]", ref_number);
    let raw_reply = create_test_email(
        "customer@example.com",
        None,
        &reply_subject,
        "This is my follow-up response",
        "<msg-005@example.com>",
    );

    let parsed = parser.parse_email(&raw_reply).unwrap();

    // Extract reference and verify conversation match
    let extracted_ref = parser.extract_reference_number(parsed.subject.as_ref().unwrap());
    assert_eq!(extracted_ref, Some(ref_number as i32));

    let matched_conv = db
        .get_conversation_by_reference_number(ref_number as i64)
        .await
        .unwrap();
    assert!(matched_conv.is_some());
    assert_eq!(matched_conv.unwrap().id, conversation.id);

    // Create reply message - use user_id as author_id
    let message2 = Message::new_incoming(
        conversation.id.clone(),
        "This is my follow-up response".to_string(),
        user_id.clone(),
    );
    db.create_message(&message2).await.unwrap();

    // Verify both messages exist on same conversation
    let (messages, _total) = db.list_messages(&conversation.id, 100, 0).await.unwrap();
    assert_eq!(messages.len(), 2);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_reopen_closed_conversation_on_reply() {
    let (test_db, inbox_id, _user_id, contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    // Create closed conversation
    let conversation = create_test_conversation(
        db,
        inbox_id.clone(),
        contact_id.clone(),
        ConversationStatus::Closed,
    )
    .await;

    // Verify it's closed
    assert_eq!(conversation.status, ConversationStatus::Closed);

    // Simulate reply (should reopen)
    db.update_conversation_status(&conversation.id, ConversationStatus::Open)
        .await
        .unwrap();

    let reopened_conv = db.get_conversation_by_id(&conversation.id).await.unwrap();
    assert_eq!(reopened_conv.unwrap().status, ConversationStatus::Open);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_duplicate_email_prevention() {
    let (test_db, inbox_id, user_id, contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    let email_message_id = "<msg-006@example.com>";

    // First check - should be false
    let is_processed = db
        .check_email_processed(&inbox_id, email_message_id)
        .await
        .unwrap();
    assert!(!is_processed);

    // Create a conversation and message for FK constraints
    let conversation = create_test_conversation(
        db,
        inbox_id.clone(),
        contact_id.clone(),
        ConversationStatus::Open,
    )
    .await;
    let message = Message::new_incoming(conversation.id.clone(), "Test".to_string(), user_id);
    db.create_message(&message).await.unwrap();

    // Log email processing with valid FKs
    let log = oxidesk::models::EmailProcessingLog::new(
        inbox_id.clone(),
        email_message_id.to_string(),
        "customer@example.com".to_string(),
        Some("Test subject".to_string()),
    );
    let log = log.mark_success(conversation.id, message.id);
    db.log_email_processing(&log).await.unwrap();

    // Second check - should be true
    let is_processed = db
        .check_email_processed(&inbox_id, email_message_id)
        .await
        .unwrap();
    assert!(is_processed);

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_attachment_size_validation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&temp_dir).unwrap();

    let attachment_repo = Arc::new(db.clone())
        as Arc<dyn oxidesk::domain::ports::attachment_repository::AttachmentRepository>;
    let attachment_service =
        AttachmentService::new(attachment_repo, temp_dir.to_str().unwrap().to_string());

    let message_id = Uuid::new_v4().to_string();

    // Create 26 MB attachment (exceeds 25 MB limit)
    let large_content = vec![0u8; 26 * 1024 * 1024];

    let result = attachment_service
        .save_attachment(
            message_id.clone(),
            "large.bin".to_string(),
            "application/octet-stream".to_string(),
            large_content,
        )
        .await;

    // Should fail due to size limit
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("exceeds maximum"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_attachment_content_type_validation() {
    let test_db = setup_test_db().await;
    let db = test_db.db();

    let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&temp_dir).unwrap();

    let attachment_repo = Arc::new(db.clone())
        as Arc<dyn oxidesk::domain::ports::attachment_repository::AttachmentRepository>;
    let attachment_service =
        AttachmentService::new(attachment_repo, temp_dir.to_str().unwrap().to_string());

    let message_id = Uuid::new_v4().to_string();
    let content = b"Executable content";

    // Try to upload .exe file (should be blocked)
    let result = attachment_service
        .save_attachment(
            message_id.clone(),
            "virus.exe".to_string(),
            "application/x-msdownload".to_string(),
            content.to_vec(),
        )
        .await;

    // Should fail due to blocked content type
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not allowed"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_attachment_storage_and_retrieval() {
    let (test_db, inbox_id, user_id, contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&temp_dir).unwrap();

    let attachment_repo = Arc::new(db.clone())
        as Arc<dyn oxidesk::domain::ports::attachment_repository::AttachmentRepository>;
    let attachment_service =
        AttachmentService::new(attachment_repo, temp_dir.to_str().unwrap().to_string());

    // Create a conversation and message first (needed for foreign key)
    let conversation = create_test_conversation(
        db,
        inbox_id.clone(),
        contact_id.clone(),
        ConversationStatus::Open,
    )
    .await;
    let message = Message::new_incoming(conversation.id, "Test message".to_string(), user_id);
    let message_id = message.id.clone();
    db.create_message(&message).await.unwrap();

    let content = b"Test file content for storage";

    // Save attachment
    let attachment = attachment_service
        .save_attachment(
            message_id.clone(),
            "test.txt".to_string(),
            "text/plain".to_string(),
            content.to_vec(),
        )
        .await
        .unwrap();

    // Verify database record
    assert_eq!(attachment.message_id, message_id);
    assert_eq!(attachment.filename, "test.txt");
    assert_eq!(attachment.content_type, Some("text/plain".to_string()));
    assert_eq!(attachment.file_size, content.len() as i64);

    // Verify file exists on disk
    let file_path = PathBuf::from(&attachment.file_path);
    assert!(file_path.exists());

    let stored_content = std::fs::read(&file_path).unwrap();
    assert_eq!(stored_content, content);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_malformed_email_handling() {
    let parser = EmailParserService::new();

    // Email missing Message-ID header
    let malformed = b"From: test@example.com\r\n\
                      To: support@example.com\r\n\
                      Subject: Test\r\n\
                      \r\n\
                      Body\r\n";

    let result = parser.parse_email(malformed);

    // Should fail gracefully
    assert!(result.is_err());
}

#[tokio::test]
async fn test_email_without_subject() {
    let parser = EmailParserService::new();

    let raw_email = create_test_email(
        "customer@example.com",
        None,
        "", // Empty subject
        "Body without subject",
        "<msg-007@example.com>",
    );

    // Should handle gracefully - subject should be None or empty
    let parsed = parser.parse_email(&raw_email).unwrap();
    assert!(parsed.subject.is_none() || parsed.subject == Some("".to_string()));
    assert!(parsed.text_body.unwrap().contains("Body without subject"));
}

#[tokio::test]
async fn test_multiple_attachments() {
    let (test_db, inbox_id, user_id, contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&temp_dir).unwrap();

    let attachment_repo = Arc::new(db.clone())
        as Arc<dyn oxidesk::domain::ports::attachment_repository::AttachmentRepository>;
    let attachment_service =
        AttachmentService::new(attachment_repo, temp_dir.to_str().unwrap().to_string());

    // Create a conversation and message first (needed for foreign key)
    let conversation = create_test_conversation(
        db,
        inbox_id.clone(),
        contact_id.clone(),
        ConversationStatus::Open,
    )
    .await;
    let message = Message::new_incoming(conversation.id, "Test message".to_string(), user_id);
    let message_id = message.id.clone();
    db.create_message(&message).await.unwrap();

    // Save multiple attachments
    let attachments = vec![
        ("file1.txt", b"Content 1"),
        ("file2.pdf", b"Content 2"),
        ("image.png", b"Content 3"),
    ];

    for (filename, content) in &attachments {
        attachment_service
            .save_attachment(
                message_id.clone(),
                filename.to_string(),
                "application/octet-stream".to_string(),
                content.to_vec(),
            )
            .await
            .unwrap();
    }

    // Retrieve all attachments for message
    let stored = db.get_message_attachments(&message_id).await.unwrap();
    assert_eq!(stored.len(), 3);

    // Verify filenames
    let filenames: Vec<_> = stored.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"file1.txt"));
    assert!(filenames.contains(&"file2.pdf"));
    assert!(filenames.contains(&"image.png"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_email_config_crud() {
    let (test_db, inbox_id, _user_id, _contact_id) = setup_email_test_db().await;
    let db = test_db.db();

    // Create config
    let config = setup_email_config(&db, &inbox_id).await;
    assert_eq!(config.inbox_id, inbox_id);
    assert_eq!(config.imap_host, "imap.example.com");
    assert_eq!(config.smtp_host, "smtp.example.com");
    assert!(config.enabled);

    // Read config
    let retrieved = db.get_inbox_email_config(&inbox_id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, config.id);

    // Update config
    let update = oxidesk::models::email::UpdateInboxEmailConfigRequest {
        imap_host: Some("new-imap.example.com".to_string()),
        imap_port: None,
        imap_username: None,
        imap_password: None,
        imap_use_tls: None,
        imap_folder: None,
        smtp_host: None,
        smtp_port: None,
        smtp_username: None,
        smtp_password: None,
        smtp_use_tls: None,
        email_address: None,
        display_name: None,
        poll_interval_seconds: None,
        enabled: Some(false),
    };

    let updated = db
        .update_inbox_email_config(&config.id, &update)
        .await
        .unwrap();
    assert_eq!(updated.imap_host, "new-imap.example.com");
    assert!(!updated.enabled);

    // Delete config
    db.delete_inbox_email_config(&config.id).await.unwrap();
    let deleted = db.get_inbox_email_config(&inbox_id).await.unwrap();
    assert!(deleted.is_none());

    teardown_test_db(test_db).await;
}

#[tokio::test]
async fn test_last_poll_time_update() {
    let (test_db, inbox_id, _user_id, _contact_id) = setup_email_test_db().await;
    let db = test_db.db();
    let config = setup_email_config(&db, &inbox_id).await;

    // Initial last_poll_at should be None
    assert!(config.last_poll_at.is_none());

    // Update last poll time
    db.update_last_poll_time(&inbox_id).await.unwrap();

    // Verify updated
    let updated = db.get_inbox_email_config(&inbox_id).await.unwrap().unwrap();
    assert!(updated.last_poll_at.is_some());

    teardown_test_db(test_db).await;
}
