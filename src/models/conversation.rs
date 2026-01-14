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
    pub closed_at: Option<String>, // ISO8601 string from DB (Feature 019)
    pub snoozed_until: Option<String>, // ISO8601 string from DB
    pub assigned_user_id: Option<String>,
    pub assigned_team_id: Option<String>,
    pub assigned_at: Option<String>,
    pub assigned_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i32,
    #[sqlx(skip)]
    pub tags: Option<Vec<String>>,
    pub priority: Option<String>,
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
    pub closed_at: Option<String>, // Feature 019
    pub snoozed_until: Option<String>,
    pub assigned_user_id: Option<String>,
    pub assigned_team_id: Option<String>,
    pub assigned_at: Option<String>,
    pub assigned_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Option<Vec<String>>,  // Fixed: should match Conversation type
    pub priority: Option<String>,
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
            closed_at: conv.closed_at,  // Feature 019
            snoozed_until: conv.snoozed_until,
            assigned_user_id: conv.assigned_user_id,
            assigned_team_id: conv.assigned_team_id,
            assigned_at: conv.assigned_at,
            assigned_by: conv.assigned_by,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
            tags: conv.tags,
            priority: conv.priority,
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
