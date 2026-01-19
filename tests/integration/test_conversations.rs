use crate::common::{TestApp, TestUser};
use oxidesk::domain::entities::{Conversation, ConversationStatus, CreateConversation, UserType, UpdateStatusRequest};
use axum::http::StatusCode;

#[tokio::test]
async fn test_conversation_lifecycle() {
    let app = TestApp::new().await;
    
    // 1. Setup: Create Agent (Admin), Inbox, and Contact
    let admin = app.create_admin_agent().await;
    let token = admin.token.clone();
    
    // Create Inbox (assuming helper exists or manual DB insert)
    // Since we don't have inbox endpoints or helpers yet, we insert into DB directly
    let inbox_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO inboxes (id, name, email_address, created_at, updated_at) VALUES (?, 'Test Inbox', 'test@example.com', datetime('now'), datetime('now'))")
        .bind(&inbox_id)
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Create Contact (using service or API helper? we have contact_service)
    // We can use the API if exposed, or just insert DB.
    // Let's use the API if possible, but simpler to use DB helper for setup
    let contact_id = uuid::Uuid::new_v4().to_string();
    let contact_user_id = uuid::Uuid::new_v4().to_string();
    
    sqlx::query("INSERT INTO users (id, email, user_type, created_at, updated_at) VALUES (?, 'contact@example.com', 'contact', datetime('now'), datetime('now'))")
        .bind(&contact_user_id)
        .execute(&app.db_pool)
        .await.unwrap();
        
    sqlx::query("INSERT INTO contacts (id, user_id, first_name) VALUES (?, ?, 'John Doe')")
        .bind(&contact_id)
        .bind(&contact_user_id)
        .execute(&app.db_pool)
        .await.unwrap();

    // 2. Create Conversation
    let create_payload = CreateConversation {
        inbox_id: inbox_id.clone(),
        contact_id: contact_user_id.clone(), // Service expects User ID of contact
        subject: Some("Help needed".to_string()),
        channel: "email".to_string(),
    };

    let response = app.post("/api/conversations", &create_payload, &token).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    let conversation: Conversation = response.json().await;
    assert_eq!(conversation.subject.as_deref(), Some("Help needed"));
    assert_eq!(conversation.status, ConversationStatus::Open);
    
    let conversation_id = conversation.id;

    // 3. Update Status -> Snoozed
    let update_snooze = UpdateStatusRequest {
        status: ConversationStatus::Snoozed,
        snooze_duration: Some("1h".to_string()),
    };
    
    let response = app.patch(&format!("/api/conversations/{}/status", conversation_id), &update_snooze, &token).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    let updated: Conversation = response.json().await;
    assert_eq!(updated.status, ConversationStatus::Snoozed);
    assert!(updated.snoozed_until.is_some());

    // 4. Update Status -> Open (unsnooze)
    let update_open = UpdateStatusRequest {
        status: ConversationStatus::Open,
        snooze_duration: None,
    };
    let response = app.patch(&format!("/api/conversations/{}/status", conversation_id), &update_open, &token).await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Conversation = response.json().await;
    assert_eq!(updated.status, ConversationStatus::Open);
    assert!(updated.snoozed_until.is_none());

    // 5. Update Status -> Resolved
    let update_resolve = UpdateStatusRequest {
        status: ConversationStatus::Resolved,
        snooze_duration: None,
    };
    let response = app.patch(&format!("/api/conversations/{}/status", conversation_id), &update_resolve, &token).await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Conversation = response.json().await;
    assert_eq!(updated.status, ConversationStatus::Resolved);
    assert!(updated.resolved_at.is_some());
    
    // 6. Verify Reopen (Resolved -> Open)
    let response = app.patch(&format!("/api/conversations/{}/status", conversation_id), &update_open, &token).await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Conversation = response.json().await;
    assert_eq!(updated.status, ConversationStatus::Open);
    assert!(updated.resolved_at.is_none()); // Should be cleared
}
