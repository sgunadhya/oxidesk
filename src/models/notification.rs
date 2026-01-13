use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Notification type representing the kind of notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationType {
    Assignment,
    Mention,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Assignment => "assignment",
            NotificationType::Mention => "mention",
        }
    }
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for NotificationType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "assignment" => NotificationType::Assignment,
            "mention" => NotificationType::Mention,
            _ => NotificationType::Assignment, // Default fallback
        }
    }
}

/// UserNotification entity representing a notification sent to a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNotification {
    pub id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub created_at: String,  // ISO 8601 timestamp
    pub is_read: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
}

impl UserNotification {
    /// Create a new assignment notification
    pub fn new_assignment(
        user_id: String,
        conversation_id: String,
        actor_id: String,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            notification_type: NotificationType::Assignment,
            created_at: now,
            is_read: false,
            conversation_id: Some(conversation_id),
            message_id: None,
            actor_id: Some(actor_id),
        }
    }

    /// Create a new mention notification
    pub fn new_mention(
        user_id: String,
        conversation_id: String,
        message_id: String,
        actor_id: String,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            notification_type: NotificationType::Mention,
            created_at: now,
            is_read: false,
            conversation_id: Some(conversation_id),
            message_id: Some(message_id),
            actor_id: Some(actor_id),
        }
    }

    /// Validate notification fields based on type
    pub fn validate(&self) -> Result<(), String> {
        match self.notification_type {
            NotificationType::Assignment => {
                // Assignment notifications MUST have conversation_id
                if self.conversation_id.is_none() {
                    return Err("Assignment notification must have conversation_id".to_string());
                }
            }
            NotificationType::Mention => {
                // Mention notifications MUST have conversation_id, message_id, AND actor_id
                if self.conversation_id.is_none() {
                    return Err("Mention notification must have conversation_id".to_string());
                }
                if self.message_id.is_none() {
                    return Err("Mention notification must have message_id".to_string());
                }
                if self.actor_id.is_none() {
                    return Err("Mention notification must have actor_id".to_string());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_serialization() {
        assert_eq!(NotificationType::Assignment.as_str(), "assignment");
        assert_eq!(NotificationType::Mention.as_str(), "mention");
    }

    #[test]
    fn test_notification_type_display() {
        assert_eq!(NotificationType::Assignment.to_string(), "assignment");
        assert_eq!(NotificationType::Mention.to_string(), "mention");
    }

    #[test]
    fn test_valid_assignment_notification() {
        let notification = UserNotification::new_assignment(
            "user_123".to_string(),
            "conv_456".to_string(),
            "actor_789".to_string(),
        );

        assert_eq!(notification.notification_type, NotificationType::Assignment);
        assert_eq!(notification.user_id, "user_123");
        assert_eq!(notification.conversation_id, Some("conv_456".to_string()));
        assert_eq!(notification.actor_id, Some("actor_789".to_string()));
        assert_eq!(notification.message_id, None);
        assert!(!notification.is_read);

        // Should validate successfully
        assert!(notification.validate().is_ok());
    }

    #[test]
    fn test_valid_mention_notification() {
        let notification = UserNotification::new_mention(
            "user_123".to_string(),
            "conv_456".to_string(),
            "msg_789".to_string(),
            "actor_012".to_string(),
        );

        assert_eq!(notification.notification_type, NotificationType::Mention);
        assert_eq!(notification.user_id, "user_123");
        assert_eq!(notification.conversation_id, Some("conv_456".to_string()));
        assert_eq!(notification.message_id, Some("msg_789".to_string()));
        assert_eq!(notification.actor_id, Some("actor_012".to_string()));
        assert!(!notification.is_read);

        // Should validate successfully
        assert!(notification.validate().is_ok());
    }

    #[test]
    fn test_assignment_without_conversation_id_fails() {
        let notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: "user_123".to_string(),
            notification_type: NotificationType::Assignment,
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            is_read: false,
            conversation_id: None,  // Missing required field
            message_id: None,
            actor_id: Some("actor_789".to_string()),
        };

        let result = notification.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Assignment notification must have conversation_id"
        );
    }

    #[test]
    fn test_mention_without_message_id_fails() {
        let notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: "user_123".to_string(),
            notification_type: NotificationType::Mention,
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            is_read: false,
            conversation_id: Some("conv_456".to_string()),
            message_id: None,  // Missing required field
            actor_id: Some("actor_789".to_string()),
        };

        let result = notification.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Mention notification must have message_id"
        );
    }

    #[test]
    fn test_mention_without_actor_id_fails() {
        let notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: "user_123".to_string(),
            notification_type: NotificationType::Mention,
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            is_read: false,
            conversation_id: Some("conv_456".to_string()),
            message_id: Some("msg_789".to_string()),
            actor_id: None,  // Missing required field
        };

        let result = notification.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Mention notification must have actor_id"
        );
    }

    #[test]
    fn test_mention_without_conversation_id_fails() {
        let notification = UserNotification {
            id: Uuid::new_v4().to_string(),
            user_id: "user_123".to_string(),
            notification_type: NotificationType::Mention,
            created_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            is_read: false,
            conversation_id: None,  // Missing required field
            message_id: Some("msg_789".to_string()),
            actor_id: Some("actor_012".to_string()),
        };

        let result = notification.validate();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Mention notification must have conversation_id"
        );
    }
}
