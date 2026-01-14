// Feature 021: Email Integration Models
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Email configuration for an inbox (IMAP receiving + SMTP sending)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InboxEmailConfig {
    pub id: String,
    pub inbox_id: String,

    // IMAP configuration (receiving)
    pub imap_host: String,
    pub imap_port: i32,
    pub imap_username: String,
    pub imap_password: String, // Encrypted at rest using AES-256-GCM
    pub imap_use_tls: bool,
    pub imap_folder: String,

    // SMTP configuration (sending)
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub smtp_password: String, // Encrypted at rest using AES-256-GCM
    pub smtp_use_tls: bool,

    // Email identity
    pub email_address: String,
    pub display_name: String,

    // Polling configuration
    pub poll_interval_seconds: i32,
    pub enabled: bool,
    pub last_poll_at: Option<String>, // ISO8601

    // Timestamps
    pub created_at: String, // ISO8601
    pub updated_at: String, // ISO8601
}

impl InboxEmailConfig {
    /// Create a new inbox email configuration
    pub fn new(
        inbox_id: String,
        imap_host: String,
        imap_port: i32,
        imap_username: String,
        imap_password: String,
        smtp_host: String,
        smtp_port: i32,
        smtp_username: String,
        smtp_password: String,
        email_address: String,
        display_name: String,
        poll_interval_seconds: Option<i32>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            inbox_id,
            imap_host,
            imap_port,
            imap_username,
            imap_password,
            imap_use_tls: true,
            imap_folder: "INBOX".to_string(),
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            smtp_use_tls: true,
            email_address,
            display_name,
            poll_interval_seconds: poll_interval_seconds.unwrap_or(30),
            enabled: true,
            last_poll_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Email attachment metadata
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageAttachment {
    pub id: String,
    pub message_id: String,
    pub filename: String,
    pub content_type: Option<String>, // MIME type
    pub file_size: i64,               // bytes
    pub file_path: String,            // absolute path on disk
    pub created_at: String,           // ISO8601
}

impl MessageAttachment {
    /// Create a new message attachment
    pub fn new(
        message_id: String,
        filename: String,
        content_type: Option<String>,
        file_size: i64,
        file_path: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_id,
            filename,
            content_type,
            file_size,
            file_path,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Processing status for email ingestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    Success,
    Failed,
    Duplicate,
}

impl std::fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingStatus::Success => write!(f, "success"),
            ProcessingStatus::Failed => write!(f, "failed"),
            ProcessingStatus::Duplicate => write!(f, "duplicate"),
        }
    }
}

impl From<String> for ProcessingStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "success" => ProcessingStatus::Success,
            "failed" => ProcessingStatus::Failed,
            "duplicate" => ProcessingStatus::Duplicate,
            _ => ProcessingStatus::Failed,
        }
    }
}

/// Audit log for email processing
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailProcessingLog {
    pub id: String,
    pub inbox_id: String,

    // Email identifiers
    pub email_message_id: String,  // RFC 5322 Message-ID
    pub email_uid: Option<String>, // IMAP UID

    // Email metadata
    pub from_address: String,
    pub subject: Option<String>,

    // Processing result
    pub processing_status: String, // Will be converted to/from ProcessingStatus enum
    pub error_message: Option<String>,

    // Created entities
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,

    // Timestamp
    pub processed_at: String, // ISO8601
}

impl EmailProcessingLog {
    /// Create a new processing log entry
    pub fn new(
        inbox_id: String,
        email_message_id: String,
        from_address: String,
        subject: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            inbox_id,
            email_message_id,
            email_uid: None,
            from_address,
            subject,
            processing_status: ProcessingStatus::Success.to_string(),
            error_message: None,
            conversation_id: None,
            message_id: None,
            processed_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Mark as successful with created entities
    pub fn mark_success(mut self, conversation_id: String, message_id: String) -> Self {
        self.processing_status = ProcessingStatus::Success.to_string();
        self.conversation_id = Some(conversation_id);
        self.message_id = Some(message_id);
        self
    }

    /// Mark as failed with error message
    pub fn mark_failed(mut self, error: String) -> Self {
        self.processing_status = ProcessingStatus::Failed.to_string();
        self.error_message = Some(error);
        self
    }

    /// Mark as duplicate
    pub fn mark_duplicate(mut self) -> Self {
        self.processing_status = ProcessingStatus::Duplicate.to_string();
        self
    }

    /// Get processing status as enum
    pub fn status(&self) -> ProcessingStatus {
        ProcessingStatus::from(self.processing_status.clone())
    }
}

/// Request to create inbox email configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CreateInboxEmailConfigRequest {
    pub imap_host: String,
    pub imap_port: i32,
    pub imap_username: String,
    pub imap_password: String,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub smtp_password: String,
    pub email_address: String,
    pub display_name: String,
    pub poll_interval_seconds: Option<i32>,
}

/// Request to update inbox email configuration
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInboxEmailConfigRequest {
    pub imap_host: Option<String>,
    pub imap_port: Option<i32>,
    pub imap_username: Option<String>,
    pub imap_password: Option<String>,
    pub imap_use_tls: Option<bool>,
    pub imap_folder: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_use_tls: Option<bool>,
    pub email_address: Option<String>,
    pub display_name: Option<String>,
    pub poll_interval_seconds: Option<i32>,
    pub enabled: Option<bool>,
}
