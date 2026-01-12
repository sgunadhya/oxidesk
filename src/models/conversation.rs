use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationStatus {
    Open,
    Snoozed,
    Resolved,
    Closed,
}

impl fmt::Display for ConversationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversationStatus::Open => write!(f, "open"),
            ConversationStatus::Snoozed => write!(f, "snoozed"),
            ConversationStatus::Resolved => write!(f, "resolved"),
            ConversationStatus::Closed => write!(f, "closed"),
        }
    }
}

// Convert from string (for SQLx)
impl From<String> for ConversationStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "snoozed" => ConversationStatus::Snoozed,
            "resolved" => ConversationStatus::Resolved,
            "closed" => ConversationStatus::Closed,
            _ => ConversationStatus::Open,
        }
    }
}

// Allow reading from DB as string
impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for ConversationStatus {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(ConversationStatus::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for ConversationStatus {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: String,
    pub reference_number: i64,
    pub status: ConversationStatus,
    pub inbox_id: String,
    pub contact_id: String,
    pub subject: Option<String>,
    pub resolved_at: Option<String>, // ISO8601 string from DB
    pub snoozed_until: Option<String>, // ISO8601 string from DB
    pub created_at: String,
    pub updated_at: String,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversation {
    pub inbox_id: String,
    pub contact_id: String,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub id: String,
    pub reference_number: i64,
    pub status: ConversationStatus,
    pub inbox_id: String,
    pub contact_id: String,
    pub subject: Option<String>,
    pub resolved_at: Option<String>,
    pub snoozed_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Conversation> for ConversationResponse {
    fn from(conv: Conversation) -> Self {
        Self {
            id: conv.id,
            reference_number: conv.reference_number,
            status: conv.status,
            inbox_id: conv.inbox_id,
            contact_id: conv.contact_id,
            subject: conv.subject,
            resolved_at: conv.resolved_at,
            snoozed_until: conv.snoozed_until,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: ConversationStatus,
    pub snooze_duration: Option<String>, // e.g. "2h", "30m"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<Conversation>,
    pub pagination: crate::models::PaginationMetadata,
}

// Helper methods for timestamps (converting String <-> DateTime<Utc>)
impl Conversation {
    pub fn resolved_at_datetime(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }

    pub fn snoozed_until_datetime(&self) -> Option<DateTime<Utc>> {
        self.snoozed_until
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }
}
