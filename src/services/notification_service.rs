use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;

use crate::models::UserNotification;
use crate::database::Database;
use super::connection_manager::{ConnectionManager, NotificationEvent};

/// Notification service for handling user notifications
#[derive(Clone)]
pub struct NotificationService;

impl NotificationService {
    pub fn new() -> Self {
        Self
    }

    /// Stub method for assignment notifications - will be fully implemented in Feature 011
    pub async fn notify_assignment(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<(), String> {
        tracing::info!(
            "STUB: Would notify user {} about conversation {} assignment",
            user_id,
            conversation_id
        );
        // TODO: Feature 011 will implement actual notification delivery
        Ok(())
    }

    /// Extract @mentions from message content
    /// Returns deduplicated usernames for batch verification
    pub fn extract_mentions(content: &str) -> Vec<String> {
        let re = Regex::new(r"@(\w+)").unwrap();
        let mut usernames = HashSet::new();

        for cap in re.captures_iter(content) {
            if let Some(username) = cap.get(1) {
                // Convert to lowercase for case-insensitive deduplication
                usernames.insert(username.as_str().to_lowercase());
            }
        }

        usernames.into_iter().collect()
    }

    /// Send notification via ConnectionManager (best-effort delivery)
    pub async fn send_realtime_notification(
        notification: &UserNotification,
        connection_manager: &Arc<dyn ConnectionManager>,
    ) -> Result<(), String> {
        // Convert UserNotification -> NotificationEvent
        let event = NotificationEvent {
            id: notification.id.clone(),
            type_: notification.notification_type.to_string(),
            created_at: notification.created_at.clone(),
            is_read: notification.is_read,
            conversation_id: notification.conversation_id.clone(),
            message_id: notification.message_id.clone(),
            actor_id: notification.actor_id.clone(),
        };

        // Call connection_manager.send_to_user()
        connection_manager
            .send_to_user(&notification.user_id, event)
            .await
    }

    /// Create assignment notification in transaction
    pub async fn create_assignment_notification(
        db: &Database,
        user_id: &str,
        conversation_id: &str,
        actor_id: Option<String>,
    ) -> Result<UserNotification, String> {
        // Create notification using UserNotification::new_assignment()
        let notification = UserNotification::new_assignment(
            user_id.to_string(),
            conversation_id.to_string(),
            actor_id.ok_or("actor_id is required for assignment notifications")?,
        );

        // Validate the notification
        notification.validate()?;

        // Call db.create_notification()
        db.create_notification(&notification)
            .await
            .map_err(|e| format!("Failed to create notification: {}", e))?;

        // Return the notification
        Ok(notification)
    }

    /// Create mention notifications from message with batch username verification
    /// Parses @mentions, deduplicates, batch verifies usernames, creates notifications
    pub async fn create_mention_notifications(
        db: &Database,
        message_id: &str,
        message_content: &str,
        conversation_id: &str,
        actor_id: &str,
    ) -> Result<Vec<UserNotification>, String> {
        // 1. Extract mentions using existing extract_mentions() function
        let usernames = Self::extract_mentions(message_content);

        // Early return if no mentions
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Batch verify usernames using db.get_users_by_usernames()
        let users = db
            .get_users_by_usernames(&usernames)
            .await
            .map_err(|e| format!("Failed to fetch users: {}", e))?;

        // 3. Filter out self-mentions (user.id == actor_id)
        let valid_users: Vec<_> = users
            .into_iter()
            .filter(|user| user.id != actor_id)
            .collect();

        // 4. Create notifications using UserNotification::new_mention()
        let mut notifications = Vec::new();
        for user in valid_users {
            let notification = UserNotification::new_mention(
                user.id.clone(),
                conversation_id.to_string(),
                message_id.to_string(),
                actor_id.to_string(),
            );

            // Validate before saving
            notification.validate()?;

            notifications.push(notification);
        }

        // 5. Save all notifications to database
        for notification in &notifications {
            db.create_notification(notification)
                .await
                .map_err(|e| format!("Failed to create notification: {}", e))?;
        }

        // 6. Return Vec of created notifications
        Ok(notifications)
    }

    /// Mark a notification as read with authorization check
    pub async fn mark_as_read(
        db: &Database,
        notification_id: &str,
        requesting_user_id: &str,
    ) -> Result<(), String> {
        // Fetch the notification by ID
        let notification = db
            .get_notification_by_id(notification_id)
            .await
            .map_err(|e| format!("Failed to fetch notification: {}", e))?;

        // Return error if notification not found
        let notification = notification.ok_or("Notification not found")?;

        // Check authorization: notification.user_id must match requesting_user_id
        if notification.user_id != requesting_user_id {
            return Err("Cannot mark another agent's notification as read".to_string());
        }

        // Call db.mark_notification_as_read(notification_id)
        db.mark_notification_as_read(notification_id)
            .await
            .map_err(|e| format!("Failed to mark notification as read: {}", e))?;

        // Return Ok(()) on success
        Ok(())
    }

    /// Mark all notifications as read for a user
    /// Returns the count of notifications marked as read
    pub async fn mark_all_as_read(
        db: &Database,
        user_id: &str,
    ) -> Result<i32, String> {
        // Call db.mark_all_notifications_as_read which handles the transaction
        let count = db
            .mark_all_notifications_as_read(user_id)
            .await
            .map_err(|e| format!("Failed to mark all notifications as read: {}", e))?;

        // Return the count of notifications marked as read
        Ok(count)
    }

    /// Cleanup old notifications beyond retention period
    /// Returns the count of notifications deleted
    /// Default retention period is 30 days
    pub async fn cleanup_old_notifications(
        db: &Database,
        retention_days: Option<i32>,
    ) -> Result<i32, String> {
        // Use 30 days as default retention period
        let days = retention_days.unwrap_or(30);

        // Call database method to delete old notifications
        let count = db
            .delete_old_notifications(days)
            .await
            .map_err(|e| format!("Failed to cleanup old notifications: {}", e))?;

        tracing::info!("Cleaned up {} old notifications (older than {} days)", count, days);

        Ok(count)
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mentions_single() {
        let content = "Hey @alice";
        let mentions = NotificationService::extract_mentions(content);
        assert_eq!(mentions, vec!["alice"]);
    }

    #[test]
    fn test_extract_mentions_multiple() {
        let content = "Hey @alice and @bob";
        let mut mentions = NotificationService::extract_mentions(content);
        mentions.sort(); // Sort for consistent comparison
        assert_eq!(mentions, vec!["alice", "bob"]);
    }

    #[test]
    fn test_extract_mentions_duplicates() {
        let content = "@alice @bob @alice";
        let mut mentions = NotificationService::extract_mentions(content);
        mentions.sort(); // Sort for consistent comparison
        assert_eq!(mentions, vec!["alice", "bob"]);
    }

    #[test]
    fn test_extract_mentions_none() {
        let content = "Hello world";
        let mentions = NotificationService::extract_mentions(content);
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_extract_mentions_case_insensitive() {
        let content = "@Alice @ALICE";
        let mentions = NotificationService::extract_mentions(content);
        // Should deduplicate based on case-insensitive comparison
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], "alice");
    }

    use tokio::sync::mpsc::Sender;

    // Mock ConnectionManager for testing
    struct MockConnectionManager {
        should_succeed: bool,
    }

    #[async_trait::async_trait]
    impl ConnectionManager for MockConnectionManager {
        async fn add_connection(&self, _user_id: &str, _sender: Sender<NotificationEvent>) {
            // Mock implementation
        }

        async fn remove_connection(&self, _user_id: &str) {
            // Mock implementation
        }

        async fn send_to_user(&self, _user_id: &str, _event: NotificationEvent) -> Result<(), String> {
            if self.should_succeed {
                Ok(())
            } else {
                Err("Connection failed".to_string())
            }
        }

        async fn is_connected(&self, _user_id: &str) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_send_realtime_notification_success() {
        let notification = UserNotification::new_mention(
            "user123".to_string(),
            "conv456".to_string(),
            "msg789".to_string(),
            "actor012".to_string(),
        );

        let connection_manager = Arc::new(MockConnectionManager {
            should_succeed: true,
        }) as Arc<dyn ConnectionManager>;

        let result = NotificationService::send_realtime_notification(
            &notification,
            &connection_manager,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_realtime_notification_failure() {
        let notification = UserNotification::new_assignment(
            "user123".to_string(),
            "conv456".to_string(),
            "actor789".to_string(),
        );

        let connection_manager = Arc::new(MockConnectionManager {
            should_succeed: false,
        }) as Arc<dyn ConnectionManager>;

        let result = NotificationService::send_realtime_notification(
            &notification,
            &connection_manager,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Connection failed");
    }

    // Test database helper
    use crate::database::Database;
    use std::fs;
    use std::path::PathBuf;

    struct TestDb {
        db: Database,
        db_file: PathBuf,
    }

    impl Drop for TestDb {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.db_file);
            let mut journal_file = self.db_file.clone();
            journal_file.set_extension("db-journal");
            let _ = fs::remove_file(&journal_file);
            let mut wal_file = self.db_file.clone();
            wal_file.set_extension("db-wal");
            let _ = fs::remove_file(&wal_file);
            let mut shm_file = self.db_file.clone();
            shm_file.set_extension("db-shm");
            let _ = fs::remove_file(&shm_file);
        }
    }

    async fn setup_test_db() -> TestDb {
        sqlx::any::install_default_drivers();
        use uuid::Uuid;
        let temp_file = format!("test_{}.db", Uuid::new_v4());
        let db_file = PathBuf::from(&temp_file);
        let db_url = format!("sqlite://{}?mode=rwc", temp_file);

        let db = Database::connect(&db_url)
            .await
            .expect("Failed to connect to database");

        db.run_migrations()
            .await
            .expect("Failed to run migrations");

        TestDb { db, db_file }
    }

    #[tokio::test]
    async fn test_create_assignment_notification_success() {
        use crate::models::{User, UserType};
        use uuid::Uuid;
        let test_db = setup_test_db().await;

        // Create users first to satisfy foreign key constraints
        let user = User::new("user123@test.com".to_string(), UserType::Agent);
        test_db.db.create_user(&user).await.expect("Failed to create user");

        let actor = User::new("actor789@test.com".to_string(), UserType::Agent);
        test_db.db.create_user(&actor).await.expect("Failed to create actor");

        // Create contact user and contact entry for conversation
        let contact_user = User::new("contact@example.com".to_string(), UserType::Contact);
        test_db.db.create_user(&contact_user).await.expect("Failed to create contact user");

        // Create contact entry
        let contact_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO contacts (id, user_id) VALUES (?, ?)"
        )
        .bind(&contact_id)
        .bind(&contact_user.id)
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create contact");

        // Create inbox
        let inbox_id = Uuid::new_v4().to_string();
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        sqlx::query(
            "INSERT INTO inboxes (id, name, channel_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&inbox_id)
        .bind("Test Inbox")
        .bind("email")
        .bind(&now)
        .bind(&now)
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create inbox");

        // Create conversation
        let conv_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, created_at, updated_at, version)
             VALUES (?, 1001, 'open', ?, ?, ?, ?, 0)"
        )
        .bind(&conv_id)
        .bind(&inbox_id)
        .bind(&contact_id)
        .bind(&now)
        .bind(&now)
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create conversation");

        let result = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await;

        if let Err(e) = &result {
            eprintln!("Error creating notification: {}", e);
        }
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let notification = result.unwrap();
        assert_eq!(notification.user_id, user.id);
        assert_eq!(notification.conversation_id, Some(conv_id.clone()));
        assert_eq!(notification.actor_id, Some(actor.id));
        assert_eq!(notification.notification_type, crate::models::NotificationType::Assignment);
        assert!(!notification.is_read);

        // Verify it was actually saved to database
        let saved = test_db.db
            .get_notification_by_id(&notification.id)
            .await
            .expect("Failed to get notification")
            .expect("Notification not found");

        assert_eq!(saved.id, notification.id);
        assert_eq!(saved.user_id, user.id);
    }

    #[tokio::test]
    async fn test_create_assignment_notification_validation() {
        let test_db = setup_test_db().await;

        // Test without actor_id (should fail)
        let result = NotificationService::create_assignment_notification(
            &test_db.db,
            "user123",
            "conv456",
            None,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("actor_id is required"));
    }

    #[tokio::test]
    async fn test_send_realtime_with_mock() {
        let notification = UserNotification::new_assignment(
            "user123".to_string(),
            "conv456".to_string(),
            "actor789".to_string(),
        );

        // Test with successful mock
        let connection_manager_success = Arc::new(MockConnectionManager {
            should_succeed: true,
        }) as Arc<dyn ConnectionManager>;

        let result = NotificationService::send_realtime_notification(
            &notification,
            &connection_manager_success,
        )
        .await;

        assert!(result.is_ok());

        // Test with failing mock
        let connection_manager_fail = Arc::new(MockConnectionManager {
            should_succeed: false,
        }) as Arc<dyn ConnectionManager>;

        let result = NotificationService::send_realtime_notification(
            &notification,
            &connection_manager_fail,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Connection failed");
    }

    // Test helper to create test agents
    async fn create_test_agent(test_db: &TestDb, username: &str) -> crate::models::User {
        use crate::models::{User, UserType, Agent};
        let user = User::new(format!("{}@test.com", username), UserType::Agent);
        test_db.db.create_user(&user).await.expect("Failed to create user");

        let agent = Agent::new(user.id.clone(), username.to_string(), "hash".to_string());
        sqlx::query(
            "INSERT INTO agents (id, user_id, first_name, password_hash, availability_status) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&agent.id)
        .bind(&agent.user_id)
        .bind(&agent.first_name)
        .bind(&agent.password_hash)
        .bind("offline")
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create agent");

        user
    }

    // Test helper to create test conversation
    async fn create_test_conversation(test_db: &TestDb) -> String {
        use uuid::Uuid;
        use crate::models::{User, UserType};

        // Create contact user
        let contact_user = User::new("contact@example.com".to_string(), UserType::Contact);
        test_db.db.create_user(&contact_user).await.expect("Failed to create contact user");

        // Create contact entry
        let contact_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO contacts (id, user_id) VALUES (?, ?)")
            .bind(&contact_id)
            .bind(&contact_user.id)
            .execute(test_db.db.pool())
            .await
            .expect("Failed to create contact");

        // Create inbox
        let inbox_id = Uuid::new_v4().to_string();
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        sqlx::query(
            "INSERT INTO inboxes (id, name, channel_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&inbox_id)
        .bind("Test Inbox")
        .bind("email")
        .bind(&now)
        .bind(&now)
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create inbox");

        // Create conversation
        let conv_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, created_at, updated_at, version)
             VALUES (?, 1001, 'open', ?, ?, ?, ?, 0)"
        )
        .bind(&conv_id)
        .bind(&inbox_id)
        .bind(&contact_id)
        .bind(&now)
        .bind(&now)
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create conversation");

        conv_id
    }

    // Test helper to create test message
    async fn create_test_message(test_db: &TestDb, conversation_id: &str, author_id: &str, content: &str) -> String {
        use uuid::Uuid;
        let message_id = Uuid::new_v4().to_string();
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, updated_at)
             VALUES (?, ?, 'outgoing', 'sent', ?, ?, 0, 0, ?, ?)"
        )
        .bind(&message_id)
        .bind(conversation_id)
        .bind(content)
        .bind(author_id)
        .bind(&now)
        .bind(&now)
        .execute(test_db.db.pool())
        .await
        .expect("Failed to create message");

        message_id
    }

    #[tokio::test]
    async fn test_create_mention_notifications_single() {
        let test_db = setup_test_db().await;
        let alice = create_test_agent(&test_db, "alice").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        let message_content = "Hey @alice, can you check this?";
        let message_id = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await;

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let notifications = result.unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].user_id, alice.id);
        assert_eq!(notifications[0].message_id, Some(message_id.to_string()));
        assert_eq!(notifications[0].conversation_id, Some(conv_id.clone()));
        assert_eq!(notifications[0].actor_id, Some(actor.id));
        assert_eq!(notifications[0].notification_type, crate::models::NotificationType::Mention);
    }

    #[tokio::test]
    async fn test_create_mention_notifications_multiple() {
        let test_db = setup_test_db().await;
        let alice = create_test_agent(&test_db, "alice").await;
        let bob = create_test_agent(&test_db, "bob").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        let message_content = "Hey @alice and @bob, can you check this?";
        let message_id = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await;

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let notifications = result.unwrap();
        assert_eq!(notifications.len(), 2);

        // Verify both users were notified
        let user_ids: Vec<_> = notifications.iter().map(|n| n.user_id.as_str()).collect();
        assert!(user_ids.contains(&alice.id.as_str()));
        assert!(user_ids.contains(&bob.id.as_str()));

        // Verify all notifications have correct metadata
        for notification in &notifications {
            assert_eq!(notification.message_id, Some(message_id.to_string()));
            assert_eq!(notification.conversation_id, Some(conv_id.clone()));
            assert_eq!(notification.actor_id, Some(actor.id.clone()));
            assert_eq!(notification.notification_type, crate::models::NotificationType::Mention);
        }
    }

    #[tokio::test]
    async fn test_create_mention_notifications_self_mention_filtered() {
        let test_db = setup_test_db().await;
        let alice = create_test_agent(&test_db, "alice").await;
        let conv_id = create_test_conversation(&test_db).await;

        let message_content = "@alice mentions herself";
        let message_id = create_test_message(&test_db, &conv_id, &alice.id, message_content).await;

        // Alice is the actor mentioning herself
        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &alice.id, // alice is the actor
        )
        .await;

        assert!(result.is_ok());
        let notifications = result.unwrap();
        // No notification should be created for self-mention
        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_create_mention_notifications_invalid_username() {
        let test_db = setup_test_db().await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        let message_content = "@nonexistent user mentioned";
        let message_id = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await;

        assert!(result.is_ok());
        let notifications = result.unwrap();
        // No notification for non-existent user
        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_create_mention_notifications_duplicate_deduplication() {
        let test_db = setup_test_db().await;
        let alice = create_test_agent(&test_db, "alice").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Alice mentioned multiple times
        let message_content = "@alice @alice @ALICE please check this @alice";
        let message_id = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await;

        assert!(result.is_ok());
        let notifications = result.unwrap();
        // Only one notification despite multiple mentions
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].user_id, alice.id);
    }

    #[tokio::test]
    async fn test_create_mention_notifications_batch_verification() {
        let test_db = setup_test_db().await;
        let alice = create_test_agent(&test_db, "alice").await;
        let bob = create_test_agent(&test_db, "bob").await;
        let charlie = create_test_agent(&test_db, "charlie").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        let message_content = "@alice @bob @charlie @nonexistent, check this out!";
        let message_id = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await;

        assert!(result.is_ok());
        let notifications = result.unwrap();
        // Only valid users get notifications
        assert_eq!(notifications.len(), 3);

        let user_ids: Vec<_> = notifications.iter().map(|n| n.user_id.as_str()).collect();
        assert!(user_ids.contains(&alice.id.as_str()));
        assert!(user_ids.contains(&bob.id.as_str()));
        assert!(user_ids.contains(&charlie.id.as_str()));
    }

    #[tokio::test]
    async fn test_create_mention_notifications_empty_content() {
        let test_db = setup_test_db().await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        let message_content = "No mentions in this message";
        let message_id = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let result = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await;

        assert!(result.is_ok());
        let notifications = result.unwrap();
        // No mentions = no notifications
        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_mark_as_read_success() {
        let test_db = setup_test_db().await;
        let user = create_test_agent(&test_db, "user").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Create notification
        let notification = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification");

        // Verify it starts unread
        assert!(!notification.is_read);

        // Mark as read
        let result = NotificationService::mark_as_read(
            &test_db.db,
            &notification.id,
            &user.id,
        )
        .await;

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);

        // Verify it's now marked as read in database
        let updated = test_db.db
            .get_notification_by_id(&notification.id)
            .await
            .expect("Failed to get notification")
            .expect("Notification not found");

        assert!(updated.is_read);
    }

    #[tokio::test]
    async fn test_mark_as_read_authorization_failure() {
        let test_db = setup_test_db().await;
        let user = create_test_agent(&test_db, "user").await;
        let other_user = create_test_agent(&test_db, "other").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Create notification for user
        let notification = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification");

        // Try to mark as read with other_user's ID
        let result = NotificationService::mark_as_read(
            &test_db.db,
            &notification.id,
            &other_user.id,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot mark another agent's notification as read"
        );

        // Verify it's still unread in database
        let unchanged = test_db.db
            .get_notification_by_id(&notification.id)
            .await
            .expect("Failed to get notification")
            .expect("Notification not found");

        assert!(!unchanged.is_read);
    }

    #[tokio::test]
    async fn test_mark_as_read_idempotent() {
        let test_db = setup_test_db().await;
        let user = create_test_agent(&test_db, "user").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Create notification
        let notification = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification");

        // Mark as read first time
        let result1 = NotificationService::mark_as_read(
            &test_db.db,
            &notification.id,
            &user.id,
        )
        .await;

        assert!(result1.is_ok(), "First mark_as_read failed: {:?}", result1);

        // Mark as read second time (idempotent)
        let result2 = NotificationService::mark_as_read(
            &test_db.db,
            &notification.id,
            &user.id,
        )
        .await;

        assert!(result2.is_ok(), "Second mark_as_read failed: {:?}", result2);

        // Verify it's still marked as read
        let final_state = test_db.db
            .get_notification_by_id(&notification.id)
            .await
            .expect("Failed to get notification")
            .expect("Notification not found");

        assert!(final_state.is_read);
    }

    #[tokio::test]
    async fn test_mark_as_read_flow() {
        let test_db = setup_test_db().await;

        // Create test agents
        let agent1 = create_test_agent(&test_db, "agent1").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Step 1: Create a test notification for user "agent1"
        let notification = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification");

        // Step 2: Verify unread count = 1
        let unread_count = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count");
        assert_eq!(unread_count, 1, "Expected unread count to be 1");

        // Step 3: Call mark_as_read with correct user_id
        NotificationService::mark_as_read(
            &test_db.db,
            &notification.id,
            &agent1.id,
        )
        .await
        .expect("Failed to mark notification as read");

        // Step 4: Fetch notification and verify is_read = true
        let updated_notification = test_db.db
            .get_notification_by_id(&notification.id)
            .await
            .expect("Failed to get notification")
            .expect("Notification not found");
        assert!(updated_notification.is_read, "Expected notification to be marked as read");

        // Step 5: Verify unread count = 0
        let unread_count_after = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count after marking as read");
        assert_eq!(unread_count_after, 0, "Expected unread count to be 0");
    }

    #[tokio::test]
    async fn test_unread_count_after_mark_as_read() {
        let test_db = setup_test_db().await;

        // Create test agents
        let agent1 = create_test_agent(&test_db, "agent1").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Step 1: Create 3 test notifications for user "agent1"
        let notification1 = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification 1");

        let notification2 = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification 2");

        let notification3 = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification 3");

        // Step 2: Verify unread count = 3
        let unread_count = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count");
        assert_eq!(unread_count, 3, "Expected unread count to be 3");

        // Step 3: Mark 2 notifications as read
        NotificationService::mark_as_read(
            &test_db.db,
            &notification1.id,
            &agent1.id,
        )
        .await
        .expect("Failed to mark notification 1 as read");

        NotificationService::mark_as_read(
            &test_db.db,
            &notification2.id,
            &agent1.id,
        )
        .await
        .expect("Failed to mark notification 2 as read");

        // Step 4: Verify unread count = 1
        let unread_count_after_2 = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count after marking 2 as read");
        assert_eq!(unread_count_after_2, 1, "Expected unread count to be 1");

        // Step 5: Mark remaining notification as read
        NotificationService::mark_as_read(
            &test_db.db,
            &notification3.id,
            &agent1.id,
        )
        .await
        .expect("Failed to mark notification 3 as read");

        // Step 6: Verify unread count = 0
        let unread_count_final = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count after marking all as read");
        assert_eq!(unread_count_final, 0, "Expected unread count to be 0");
    }

    #[tokio::test]
    async fn test_mark_all_as_read_multiple() {
        let test_db = setup_test_db().await;
        let user = create_test_agent(&test_db, "user").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Create 3 unread notifications
        let _notification1 = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification 1");

        let _notification2 = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification 2");

        let _notification3 = NotificationService::create_assignment_notification(
            &test_db.db,
            &user.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create notification 3");

        // Verify unread count = 3
        let unread_count_before = test_db.db
            .get_unread_count(&user.id)
            .await
            .expect("Failed to get unread count");
        assert_eq!(unread_count_before, 3, "Expected 3 unread notifications");

        // Mark all as read
        let count = NotificationService::mark_all_as_read(
            &test_db.db,
            &user.id,
        )
        .await
        .expect("Failed to mark all as read");

        // Verify count returned = 3
        assert_eq!(count, 3, "Expected mark_all_as_read to return 3");

        // Verify unread count = 0
        let unread_count_after = test_db.db
            .get_unread_count(&user.id)
            .await
            .expect("Failed to get unread count after marking all as read");
        assert_eq!(unread_count_after, 0, "Expected 0 unread notifications");
    }

    #[tokio::test]
    async fn test_mark_all_as_read_no_unread() {
        let test_db = setup_test_db().await;
        let user = create_test_agent(&test_db, "user").await;

        // Mark all when no unread notifications exist
        let count = NotificationService::mark_all_as_read(
            &test_db.db,
            &user.id,
        )
        .await
        .expect("Failed to mark all as read");

        // Verify count = 0
        assert_eq!(count, 0, "Expected mark_all_as_read to return 0 when no unread notifications");

        // Verify unread count = 0
        let unread_count = test_db.db
            .get_unread_count(&user.id)
            .await
            .expect("Failed to get unread count");
        assert_eq!(unread_count, 0, "Expected 0 unread notifications");
    }

    #[tokio::test]
    async fn test_mark_all_as_read_flow() {
        let test_db = setup_test_db().await;

        // Create test agents
        let agent1 = create_test_agent(&test_db, "agent1").await;
        let actor = create_test_agent(&test_db, "actor").await;
        let conv_id = create_test_conversation(&test_db).await;

        // Step 1: Create 5 test notifications for user "agent1" (mix of assignment and mention types)
        // Create 3 assignment notifications
        let notification1 = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create assignment notification 1");

        let notification2 = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create assignment notification 2");

        let notification3 = NotificationService::create_assignment_notification(
            &test_db.db,
            &agent1.id,
            &conv_id,
            Some(actor.id.clone()),
        )
        .await
        .expect("Failed to create assignment notification 3");

        // Create 2 mention notifications
        let message_content = "@agent1 please check this";
        let message_id1 = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let mention_notifications1 = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id1,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await
        .expect("Failed to create mention notifications 1");

        let notification4 = &mention_notifications1[0];

        let message_id2 = create_test_message(&test_db, &conv_id, &actor.id, message_content).await;

        let mention_notifications2 = NotificationService::create_mention_notifications(
            &test_db.db,
            &message_id2,
            message_content,
            &conv_id,
            &actor.id,
        )
        .await
        .expect("Failed to create mention notifications 2");

        let notification5 = &mention_notifications2[0];

        // Step 2: Verify unread count = 5
        let unread_count = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count");
        assert_eq!(unread_count, 5, "Expected unread count to be 5");

        // Step 3: Call NotificationService::mark_all_as_read for "agent1"
        let count = NotificationService::mark_all_as_read(
            &test_db.db,
            &agent1.id,
        )
        .await
        .expect("Failed to mark all as read");

        // Step 4: Verify the count returned = 5
        assert_eq!(count, 5, "Expected mark_all_as_read to return 5");

        // Step 5: Fetch all 5 notifications and verify all have is_read = true
        let notif1_updated = test_db.db
            .get_notification_by_id(&notification1.id)
            .await
            .expect("Failed to get notification 1")
            .expect("Notification 1 not found");
        assert!(notif1_updated.is_read, "Expected notification 1 to be marked as read");

        let notif2_updated = test_db.db
            .get_notification_by_id(&notification2.id)
            .await
            .expect("Failed to get notification 2")
            .expect("Notification 2 not found");
        assert!(notif2_updated.is_read, "Expected notification 2 to be marked as read");

        let notif3_updated = test_db.db
            .get_notification_by_id(&notification3.id)
            .await
            .expect("Failed to get notification 3")
            .expect("Notification 3 not found");
        assert!(notif3_updated.is_read, "Expected notification 3 to be marked as read");

        let notif4_updated = test_db.db
            .get_notification_by_id(&notification4.id)
            .await
            .expect("Failed to get notification 4")
            .expect("Notification 4 not found");
        assert!(notif4_updated.is_read, "Expected notification 4 to be marked as read");

        let notif5_updated = test_db.db
            .get_notification_by_id(&notification5.id)
            .await
            .expect("Failed to get notification 5")
            .expect("Notification 5 not found");
        assert!(notif5_updated.is_read, "Expected notification 5 to be marked as read");

        // Step 6: Verify unread count = 0
        let unread_count_after = test_db.db
            .get_unread_count(&agent1.id)
            .await
            .expect("Failed to get unread count after marking all as read");
        assert_eq!(unread_count_after, 0, "Expected unread count to be 0");
    }

    #[tokio::test]
    async fn test_cleanup_with_old_notifications() {
        use uuid::Uuid;
        use time::OffsetDateTime;
        use crate::models::NotificationType;

        let test_db = setup_test_db().await;

        // Create a test agent
        let agent1 = create_test_agent(&test_db, "agent1").await;

        // Create a conversation for the notification
        let conversation_id = create_test_conversation(&test_db).await;

        // Create an old notification (40 days ago)
        let old_timestamp = (OffsetDateTime::now_utc() - time::Duration::days(40))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let old_notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: agent1.id.clone(),
            notification_type: NotificationType::Assignment,
            created_at: old_timestamp,
            is_read: false,
            conversation_id: Some(conversation_id.clone()),
            message_id: None,
            actor_id: Some(agent1.id.clone()),
        };

        // Save the old notification
        test_db.db
            .create_notification(&old_notification)
            .await
            .expect("Failed to create old notification");

        // Verify notification exists
        let notif = test_db.db
            .get_notification_by_id(&old_notification.id)
            .await
            .expect("Failed to get notification")
            .expect("Notification not found");
        assert_eq!(notif.id, old_notification.id);

        // Run cleanup with 30-day retention
        let deleted_count = NotificationService::cleanup_old_notifications(&test_db.db, Some(30))
            .await
            .expect("Cleanup failed");

        // Verify 1 notification was deleted
        assert_eq!(deleted_count, 1, "Expected 1 notification to be deleted");

        // Verify notification is gone
        let notif_after_cleanup = test_db.db
            .get_notification_by_id(&old_notification.id)
            .await
            .expect("Failed to get notification");
        assert!(notif_after_cleanup.is_none(), "Expected notification to be deleted");
    }

    #[tokio::test]
    async fn test_cleanup_preserving_recent_notifications() {
        use uuid::Uuid;
        use time::OffsetDateTime;
        use crate::models::NotificationType;

        let test_db = setup_test_db().await;

        // Create a test agent
        let agent1 = create_test_agent(&test_db, "agent1").await;

        // Create a conversation for the notifications
        let conversation_id = create_test_conversation(&test_db).await;

        // Create an old notification (40 days ago)
        let old_timestamp = (OffsetDateTime::now_utc() - time::Duration::days(40))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let old_notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: agent1.id.clone(),
            notification_type: NotificationType::Assignment,
            created_at: old_timestamp,
            is_read: false,
            conversation_id: Some(conversation_id.clone()),
            message_id: None,
            actor_id: Some(agent1.id.clone()),
        };

        // Create a recent notification (10 days ago)
        let recent_timestamp = (OffsetDateTime::now_utc() - time::Duration::days(10))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let recent_notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: agent1.id.clone(),
            notification_type: NotificationType::Assignment,
            created_at: recent_timestamp,
            is_read: false,
            conversation_id: Some(conversation_id.clone()),
            message_id: None,
            actor_id: Some(agent1.id.clone()),
        };

        // Save both notifications
        test_db.db
            .create_notification(&old_notification)
            .await
            .expect("Failed to create old notification");

        test_db.db
            .create_notification(&recent_notification)
            .await
            .expect("Failed to create recent notification");

        // Run cleanup with 30-day retention
        let deleted_count = NotificationService::cleanup_old_notifications(&test_db.db, Some(30))
            .await
            .expect("Cleanup failed");

        // Verify 1 notification was deleted (the old one)
        assert_eq!(deleted_count, 1, "Expected 1 notification to be deleted");

        // Verify old notification is gone
        let old_notif_after = test_db.db
            .get_notification_by_id(&old_notification.id)
            .await
            .expect("Failed to get old notification");
        assert!(old_notif_after.is_none(), "Expected old notification to be deleted");

        // Verify recent notification still exists
        let recent_notif_after = test_db.db
            .get_notification_by_id(&recent_notification.id)
            .await
            .expect("Failed to get recent notification")
            .expect("Recent notification not found");
        assert_eq!(recent_notif_after.id, recent_notification.id, "Expected recent notification to be preserved");
    }

    #[tokio::test]
    async fn test_cleanup_with_no_old_notifications() {
        use uuid::Uuid;
        use time::OffsetDateTime;
        use crate::models::NotificationType;

        let test_db = setup_test_db().await;

        // Create a test agent
        let agent1 = create_test_agent(&test_db, "agent1").await;

        // Create a conversation for the notification
        let conversation_id = create_test_conversation(&test_db).await;

        // Create a recent notification (10 days ago)
        let recent_timestamp = (OffsetDateTime::now_utc() - time::Duration::days(10))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let recent_notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: agent1.id.clone(),
            notification_type: NotificationType::Assignment,
            created_at: recent_timestamp,
            is_read: false,
            conversation_id: Some(conversation_id.clone()),
            message_id: None,
            actor_id: Some(agent1.id.clone()),
        };

        // Save the recent notification
        test_db.db
            .create_notification(&recent_notification)
            .await
            .expect("Failed to create recent notification");

        // Run cleanup with 30-day retention
        let deleted_count = NotificationService::cleanup_old_notifications(&test_db.db, Some(30))
            .await
            .expect("Cleanup failed");

        // Verify 0 notifications were deleted
        assert_eq!(deleted_count, 0, "Expected 0 notifications to be deleted");

        // Verify recent notification still exists
        let recent_notif_after = test_db.db
            .get_notification_by_id(&recent_notification.id)
            .await
            .expect("Failed to get recent notification")
            .expect("Recent notification not found");
        assert_eq!(recent_notif_after.id, recent_notification.id, "Expected recent notification to be preserved");
    }

    #[tokio::test]
    async fn test_cleanup_integration_with_30day_cutoff() {
        use uuid::Uuid;
        use time::OffsetDateTime;
        use crate::models::NotificationType;

        let test_db = setup_test_db().await;

        // Create test agents
        let agent1 = create_test_agent(&test_db, "agent1").await;

        // Create conversation
        let conversation_id = create_test_conversation(&test_db).await;

        // Create notifications at various ages
        let notifications = vec![
            (35, "old1"), // >30 days - should be deleted
            (40, "old2"), // >30 days - should be deleted
            (29, "recent1"), // <30 days - should be preserved
            (15, "recent2"), // <30 days - should be preserved
            (5, "recent3"), // <30 days - should be preserved
        ];

        let mut created_notifications = Vec::new();
        for (days_ago, label) in notifications {
            let timestamp = (OffsetDateTime::now_utc() - time::Duration::days(days_ago))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap();

            let notification = UserNotification {
                id: Uuid::new_v4().to_string(),
                user_id: agent1.id.clone(),
                notification_type: NotificationType::Assignment,
                created_at: timestamp,
                is_read: false,
                conversation_id: Some(conversation_id.clone()),
                message_id: None,
                actor_id: Some(agent1.id.clone()),
            };

            test_db.db
                .create_notification(&notification)
                .await
                .expect(&format!("Failed to create notification {}", label));

            created_notifications.push((notification.id.clone(), days_ago, label));
        }

        // Verify all 5 notifications exist
        for (id, _, label) in &created_notifications {
            let notif = test_db.db
                .get_notification_by_id(id)
                .await
                .expect(&format!("Failed to get notification {}", label))
                .expect(&format!("Notification {} not found", label));
            assert_eq!(notif.id, *id);
        }

        // Run cleanup with 30-day cutoff
        let deleted_count = NotificationService::cleanup_old_notifications(&test_db.db, Some(30))
            .await
            .expect("Cleanup failed");

        // Verify 2 old notifications were deleted
        assert_eq!(deleted_count, 2, "Expected 2 old notifications to be deleted");

        // Verify old notifications (>30 days) are deleted
        for (id, days_ago, label) in &created_notifications {
            if *days_ago > 30 {
                let notif_after = test_db.db
                    .get_notification_by_id(id)
                    .await
                    .expect(&format!("Failed to check notification {}", label));
                assert!(notif_after.is_none(), "Expected old notification {} to be deleted", label);
            }
        }

        // Verify recent notifications (<30 days) are preserved
        for (id, days_ago, label) in &created_notifications {
            if *days_ago <= 30 {
                let notif_after = test_db.db
                    .get_notification_by_id(id)
                    .await
                    .expect(&format!("Failed to check notification {}", label))
                    .expect(&format!("Expected recent notification {} to be preserved", label));
                assert_eq!(notif_after.id, *id);
            }
        }
    }

    #[tokio::test]
    async fn test_cleanup_performance() {
        use uuid::Uuid;
        use time::OffsetDateTime;
        use crate::models::NotificationType;
        use std::time::Instant;

        let test_db = setup_test_db().await;

        // Create a test agent
        let agent1 = create_test_agent(&test_db, "agent1").await;

        // Create conversation
        let conversation_id = create_test_conversation(&test_db).await;

        // Create 100 old notifications (simulating larger dataset)
        // Note: For true performance testing with 10k+ notifications, this would need to be scaled up
        let old_timestamp = (OffsetDateTime::now_utc() - time::Duration::days(40))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        for i in 0..100 {
            let notification = UserNotification {
                id: Uuid::new_v4().to_string(),
                user_id: agent1.id.clone(),
                notification_type: NotificationType::Assignment,
                created_at: old_timestamp.clone(),
                is_read: false,
                conversation_id: Some(conversation_id.clone()),
                message_id: None,
                actor_id: Some(agent1.id.clone()),
            };

            test_db.db
                .create_notification(&notification)
                .await
                .expect(&format!("Failed to create notification {}", i));
        }

        // Measure cleanup performance
        let start = Instant::now();
        let deleted_count = NotificationService::cleanup_old_notifications(&test_db.db, Some(30))
            .await
            .expect("Cleanup failed");
        let duration = start.elapsed();

        // Verify all 100 were deleted
        assert_eq!(deleted_count, 100, "Expected 100 notifications to be deleted");

        // Verify cleanup completed quickly (should be much faster than 5 minutes)
        // For 100 notifications, expect <1 second
        assert!(duration.as_secs() < 5, "Cleanup took too long: {:?}", duration);

        tracing::info!("Cleanup of 100 notifications completed in {:?}", duration);
    }
}
