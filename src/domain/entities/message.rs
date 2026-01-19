use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message type indicating direction of communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Incoming, // From customer to agent
    Outgoing, // From agent to customer
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Incoming => "incoming",
            MessageType::Outgoing => "outgoing",
        }
    }
}

impl From<String> for MessageType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "incoming" => MessageType::Incoming,
            "outgoing" => MessageType::Outgoing,
            _ => MessageType::Incoming, // Default fallback
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Message status representing current delivery state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Received, // Incoming messages only (terminal, immutable)
    Pending,  // Outgoing messages queued for delivery
    Sent,     // Outgoing messages successfully delivered (terminal, immutable)
    Failed,   // Outgoing messages that failed delivery
}

impl MessageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageStatus::Received => "received",
            MessageStatus::Pending => "pending",
            MessageStatus::Sent => "sent",
            MessageStatus::Failed => "failed",
        }
    }

    /// Check if this status represents an immutable message
    pub fn is_immutable(&self) -> bool {
        matches!(self, MessageStatus::Received | MessageStatus::Sent)
    }
}

impl From<String> for MessageStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "received" => MessageStatus::Received,
            "pending" => MessageStatus::Pending,
            "sent" => MessageStatus::Sent,
            "failed" => MessageStatus::Failed,
            _ => MessageStatus::Pending, // Default fallback
        }
    }
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Message entity representing a communication unit between agents and contacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub status: MessageStatus,
    pub content: String,
    pub author_id: String,
    pub is_immutable: bool,
    pub retry_count: i32,
    pub created_at: String,      // ISO 8601 timestamp
    pub sent_at: Option<String>, // ISO 8601 timestamp
    pub updated_at: String,      // ISO 8601 timestamp
}

impl Message {
    /// Create a new incoming message
    pub fn new_incoming(conversation_id: String, content: String, author_id: String) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            message_type: MessageType::Incoming,
            status: MessageStatus::Received,
            content,
            author_id,
            is_immutable: true, // Incoming messages are immediately immutable
            retry_count: 0,
            created_at: now.clone(),
            sent_at: None,
            updated_at: now,
        }
    }

    /// Create a new outgoing message
    pub fn new_outgoing(conversation_id: String, content: String, author_id: String) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            message_type: MessageType::Outgoing,
            status: MessageStatus::Pending,
            content,
            author_id,
            is_immutable: false, // Outgoing messages become immutable after sent
            retry_count: 0,
            created_at: now.clone(),
            sent_at: None,
            updated_at: now,
        }
    }

    /// Validate message content
    pub fn validate_content(content: &str) -> Result<(), String> {
        let len = content.len();
        if len == 0 {
            return Err("Message content cannot be empty".to_string());
        }
        if len > 10_000 {
            return Err(format!(
                "Message content too long: {} characters (max 10,000)",
                len
            ));
        }
        Ok(())
    }

    /// Validate message type immutability (Feature 025: Mutual Exclusion Invariants)
    /// FR-012: Message type cannot be changed after creation
    pub fn validate_type_immutable(&self, new_type: &MessageType) -> Result<(), String> {
        if std::mem::discriminant(&self.message_type) != std::mem::discriminant(new_type) {
            return Err("Message type cannot be changed after creation".to_string());
        }
        Ok(())
    }
}

/// Request to send a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

/// Request to receive an incoming message (webhook)
///
/// Feature 016: Supports automatic contact creation via from_header field.
/// Either contact_id OR from_header must be provided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessageRequest {
    pub conversation_id: String,
    pub content: String,
    /// Contact ID (if already known). If not provided, from_header must be present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    pub inbox_id: String,
    /// Email header for automatic contact creation (Feature 016)
    /// Format: "Display Name <email@example.com>" or just "email@example.com"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
}

/// Response containing message list with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListResponse {
    pub messages: Vec<Message>,
    pub pagination: crate::domain::entities::PaginationMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_serialization() {
        assert_eq!(MessageType::Incoming.as_str(), "incoming");
        assert_eq!(MessageType::Outgoing.as_str(), "outgoing");
    }

    #[test]
    fn test_message_status_serialization() {
        assert_eq!(MessageStatus::Received.as_str(), "received");
        assert_eq!(MessageStatus::Pending.as_str(), "pending");
        assert_eq!(MessageStatus::Sent.as_str(), "sent");
        assert_eq!(MessageStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_message_status_immutability() {
        assert!(MessageStatus::Received.is_immutable());
        assert!(MessageStatus::Sent.is_immutable());
        assert!(!MessageStatus::Pending.is_immutable());
        assert!(!MessageStatus::Failed.is_immutable());
    }

    #[test]
    fn test_new_incoming_message() {
        let msg = Message::new_incoming(
            "conv_123".to_string(),
            "Hello".to_string(),
            "user_456".to_string(),
        );

        assert_eq!(msg.message_type, MessageType::Incoming);
        assert_eq!(msg.status, MessageStatus::Received);
        assert!(msg.is_immutable);
        assert_eq!(msg.retry_count, 0);
    }

    #[test]
    fn test_new_outgoing_message() {
        let msg = Message::new_outgoing(
            "conv_123".to_string(),
            "Hello".to_string(),
            "agent_789".to_string(),
        );

        assert_eq!(msg.message_type, MessageType::Outgoing);
        assert_eq!(msg.status, MessageStatus::Pending);
        assert!(!msg.is_immutable);
        assert_eq!(msg.retry_count, 0);
    }

    #[test]
    fn test_validate_content_empty() {
        let result = Message::validate_content("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Message content cannot be empty");
    }

    #[test]
    fn test_validate_content_too_long() {
        let content = "a".repeat(10_001);
        let result = Message::validate_content(&content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn test_validate_content_valid() {
        let result = Message::validate_content("Hello, world!");
        assert!(result.is_ok());
    }
}
