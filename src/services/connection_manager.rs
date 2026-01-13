use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Mutex};

/// Represents a notification event to be sent to a connected user
#[derive(Debug, Clone, Serialize)]
pub struct NotificationEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String, // Renamed from 'type' (Rust keyword)
    pub created_at: String,
    pub is_read: bool,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub actor_id: Option<String>,
}

/// Trait for managing real-time connections and delivering notifications
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    /// Add a new connection for a user
    async fn add_connection(&self, user_id: &str, sender: Sender<NotificationEvent>);

    /// Remove a connection for a user
    async fn remove_connection(&self, user_id: &str);

    /// Send a notification event to a specific user
    async fn send_to_user(&self, user_id: &str, event: NotificationEvent) -> Result<(), String>;

    /// Check if a user is currently connected
    async fn is_connected(&self, user_id: &str) -> bool;
}

/// In-memory implementation of ConnectionManager using a HashMap
pub struct InMemoryConnectionManager {
    connections: Arc<Mutex<HashMap<String, Sender<NotificationEvent>>>>,
}

impl InMemoryConnectionManager {
    /// Create a new InMemoryConnectionManager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConnectionManager for InMemoryConnectionManager {
    async fn add_connection(&self, user_id: &str, sender: Sender<NotificationEvent>) {
        let mut connections = self.connections.lock().await;
        connections.insert(user_id.to_string(), sender);
    }

    async fn remove_connection(&self, user_id: &str) {
        let mut connections = self.connections.lock().await;
        connections.remove(user_id);
    }

    async fn send_to_user(&self, user_id: &str, event: NotificationEvent) -> Result<(), String> {
        let connections = self.connections.lock().await;
        if let Some(sender) = connections.get(user_id) {
            sender
                .send(event)
                .await
                .map_err(|e| format!("Failed to send notification: {}", e))?;
            Ok(())
        } else {
            Err(format!("User {} is not connected", user_id))
        }
    }

    async fn is_connected(&self, user_id: &str) -> bool {
        let connections = self.connections.lock().await;
        connections.contains_key(user_id)
    }
}

/// Mock implementation of ConnectionManager for testing
/// Records all notifications sent instead of actually sending them
pub struct MockConnectionManager {
    sent_notifications: Arc<Mutex<Vec<(String, NotificationEvent)>>>,
}

impl MockConnectionManager {
    /// Create a new MockConnectionManager
    pub fn new() -> Self {
        Self {
            sent_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Retrieve all notifications that have been sent
    pub async fn get_sent_notifications(&self) -> Vec<(String, NotificationEvent)> {
        let notifications = self.sent_notifications.lock().await;
        notifications.clone()
    }
}

impl Default for MockConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConnectionManager for MockConnectionManager {
    async fn add_connection(&self, _user_id: &str, _sender: Sender<NotificationEvent>) {
        // Mock implementation - do nothing
    }

    async fn remove_connection(&self, _user_id: &str) {
        // Mock implementation - do nothing
    }

    async fn send_to_user(&self, user_id: &str, event: NotificationEvent) -> Result<(), String> {
        let mut notifications = self.sent_notifications.lock().await;
        notifications.push((user_id.to_string(), event));
        Ok(())
    }

    async fn is_connected(&self, _user_id: &str) -> bool {
        // Mock implementation - always return true for simplicity
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_in_memory_add_connection() {
        let manager = InMemoryConnectionManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager.add_connection("user1", tx).await;
        assert!(manager.is_connected("user1").await);
        assert!(!manager.is_connected("user2").await);
    }

    #[tokio::test]
    async fn test_in_memory_remove_connection() {
        let manager = InMemoryConnectionManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager.add_connection("user1", tx).await;
        assert!(manager.is_connected("user1").await);

        manager.remove_connection("user1").await;
        assert!(!manager.is_connected("user1").await);
    }

    #[tokio::test]
    async fn test_in_memory_send_to_user() {
        let manager = InMemoryConnectionManager::new();
        let (tx, mut rx) = mpsc::channel(10);

        manager.add_connection("user1", tx).await;

        let event = NotificationEvent {
            id: "notif1".to_string(),
            type_: "message_received".to_string(),
            created_at: "2026-01-13T00:00:00Z".to_string(),
            is_read: false,
            conversation_id: Some("conv1".to_string()),
            message_id: Some("msg1".to_string()),
            actor_id: Some("user2".to_string()),
        };

        let result = manager.send_to_user("user1", event.clone()).await;
        assert!(result.is_ok());

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, "notif1");
        assert_eq!(received.type_, "message_received");
    }

    #[tokio::test]
    async fn test_in_memory_send_to_disconnected_user() {
        let manager = InMemoryConnectionManager::new();

        let event = NotificationEvent {
            id: "notif1".to_string(),
            type_: "message_received".to_string(),
            created_at: "2026-01-13T00:00:00Z".to_string(),
            is_read: false,
            conversation_id: None,
            message_id: None,
            actor_id: None,
        };

        let result = manager.send_to_user("user1", event).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[tokio::test]
    async fn test_mock_records_notifications() {
        let manager = MockConnectionManager::new();

        let event1 = NotificationEvent {
            id: "notif1".to_string(),
            type_: "message_received".to_string(),
            created_at: "2026-01-13T00:00:00Z".to_string(),
            is_read: false,
            conversation_id: Some("conv1".to_string()),
            message_id: Some("msg1".to_string()),
            actor_id: Some("user2".to_string()),
        };

        let event2 = NotificationEvent {
            id: "notif2".to_string(),
            type_: "assignment_changed".to_string(),
            created_at: "2026-01-13T00:01:00Z".to_string(),
            is_read: false,
            conversation_id: Some("conv2".to_string()),
            message_id: None,
            actor_id: Some("user3".to_string()),
        };

        manager.send_to_user("user1", event1).await.unwrap();
        manager.send_to_user("user2", event2).await.unwrap();

        let sent = manager.get_sent_notifications().await;
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].0, "user1");
        assert_eq!(sent[0].1.id, "notif1");
        assert_eq!(sent[1].0, "user2");
        assert_eq!(sent[1].1.id, "notif2");
    }

    #[tokio::test]
    async fn test_mock_is_always_connected() {
        let manager = MockConnectionManager::new();
        assert!(manager.is_connected("user1").await);
        assert!(manager.is_connected("user2").await);
    }

    #[tokio::test]
    async fn test_in_memory_multiple_connections() {
        let manager = InMemoryConnectionManager::new();
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);

        manager.add_connection("user1", tx1).await;
        manager.add_connection("user2", tx2).await;

        assert!(manager.is_connected("user1").await);
        assert!(manager.is_connected("user2").await);

        let event1 = NotificationEvent {
            id: "notif1".to_string(),
            type_: "test".to_string(),
            created_at: "2026-01-13T00:00:00Z".to_string(),
            is_read: false,
            conversation_id: None,
            message_id: None,
            actor_id: None,
        };

        let event2 = NotificationEvent {
            id: "notif2".to_string(),
            type_: "test".to_string(),
            created_at: "2026-01-13T00:00:00Z".to_string(),
            is_read: false,
            conversation_id: None,
            message_id: None,
            actor_id: None,
        };

        manager.send_to_user("user1", event1).await.unwrap();
        manager.send_to_user("user2", event2).await.unwrap();

        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        assert_eq!(received1.id, "notif1");
        assert_eq!(received2.id, "notif2");
    }
}
