use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use uuid::Uuid;

// ============================================================================
// DeliveryStatus Enum
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
    Queued,
    Success,
    Failed,
}

impl fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeliveryStatus::Queued => write!(f, "queued"),
            DeliveryStatus::Success => write!(f, "success"),
            DeliveryStatus::Failed => write!(f, "failed"),
        }
    }
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryStatus::Queued => "queued",
            DeliveryStatus::Success => "success",
            DeliveryStatus::Failed => "failed",
        }
    }
}

// Convert from string (for SQLx)
impl From<String> for DeliveryStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "success" => DeliveryStatus::Success,
            "failed" => DeliveryStatus::Failed,
            _ => DeliveryStatus::Queued,
        }
    }
}

// Allow reading from DB as string (SQLite)
impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DeliveryStatus {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(DeliveryStatus::from(s))
    }
}

impl sqlx::Type<sqlx::Sqlite> for DeliveryStatus {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

// ============================================================================
// Webhook Model
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Webhook {
    pub id: String,
    pub name: String,
    pub url: String,
    #[sqlx(skip)]
    pub subscribed_events: Vec<String>,  // Stored as JSON in DB
    #[serde(skip_serializing)]  // Don't expose secret in API responses
    pub secret: String,
    pub is_active: bool,
    pub created_at: String,  // ISO 8601
    pub updated_at: String,  // ISO 8601
    pub created_by: String,
}

impl Webhook {
    /// Create a new webhook with generated ID and timestamps
    pub fn new(
        name: String,
        url: String,
        subscribed_events: Vec<String>,
        secret: String,
        created_by: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            url,
            subscribed_events,
            secret,
            is_active: true,  // Default active
            created_at: now.clone(),
            updated_at: now,
            created_by,
        }
    }

    /// Validate webhook fields
    pub fn validate(&self) -> Result<(), String> {
        // Name validation
        if self.name.is_empty() || self.name.len() > 255 {
            return Err("Webhook name must be 1-255 characters".to_string());
        }

        // URL validation
        if self.url.is_empty() || self.url.len() > 2048 {
            return Err("Webhook URL must be 1-2048 characters".to_string());
        }

        // Validate URL format
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err("Webhook URL must be HTTP or HTTPS".to_string());
        }

        // Subscribed events validation
        if self.subscribed_events.is_empty() {
            return Err("Webhook must subscribe to at least one event".to_string());
        }

        for event in &self.subscribed_events {
            if event.is_empty() || event.len() > 100 {
                return Err("Event type must be 1-100 characters".to_string());
            }
        }

        // Secret validation
        if self.secret.len() < 16 || self.secret.len() > 255 {
            return Err("Webhook secret must be 16-255 characters".to_string());
        }

        Ok(())
    }

    /// Check if webhook matches event type and is active
    pub fn matches_event(&self, event_type: &str) -> bool {
        self.is_active && self.subscribed_events.contains(&event_type.to_string())
    }

    /// Update timestamp to current time
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

// ============================================================================
// WebhookDelivery Model
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookDelivery {
    pub id: String,
    pub webhook_id: String,
    pub event_type: String,
    pub payload: String,  // JSON string
    pub signature: String,  // Format: "sha256=..."
    pub status: DeliveryStatus,
    pub http_status_code: Option<i32>,
    pub retry_count: i32,
    pub next_retry_at: Option<String>,  // ISO 8601
    pub attempted_at: Option<String>,  // ISO 8601
    pub completed_at: Option<String>,  // ISO 8601
    pub error_message: Option<String>,
}

impl WebhookDelivery {
    /// Create a new delivery record in queued status
    pub fn new(
        webhook_id: String,
        event_type: String,
        payload: String,
        signature: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            webhook_id,
            event_type,
            payload,
            signature,
            status: DeliveryStatus::Queued,
            http_status_code: None,
            retry_count: 0,
            next_retry_at: None,
            attempted_at: None,
            completed_at: None,
            error_message: None,
        }
    }

    /// Mark delivery as successful
    pub fn mark_success(&mut self, http_status: i32) {
        let now = chrono::Utc::now().to_rfc3339();
        self.status = DeliveryStatus::Success;
        self.http_status_code = Some(http_status);
        self.completed_at = Some(now.clone());
        self.attempted_at = Some(now);
        self.next_retry_at = None;
        self.error_message = None;
    }

    /// Mark delivery as failed and calculate retry schedule
    pub fn mark_failed(&mut self, http_status: Option<i32>, error: String) {
        let now = chrono::Utc::now();
        self.retry_count += 1;
        self.http_status_code = http_status;
        self.error_message = Some(error);
        self.attempted_at = Some(now.to_rfc3339());

        if self.retry_count >= 5 {
            // Permanent failure after 5 attempts
            self.status = DeliveryStatus::Failed;
            self.completed_at = Some(now.to_rfc3339());
            self.next_retry_at = None;
        } else {
            // Schedule retry with exponential backoff
            self.status = DeliveryStatus::Queued;
            self.next_retry_at = Some(self.calculate_next_retry());
        }
    }

    /// Calculate next retry time using exponential backoff
    /// Delay: 1min, 2min, 4min, 8min, 16min (capped at 16 minutes)
    fn calculate_next_retry(&self) -> String {
        const INITIAL_DELAY_SECS: i64 = 60;  // 1 minute
        let delay_secs = INITIAL_DELAY_SECS * 2i64.pow((self.retry_count - 1) as u32);
        let delay_secs = delay_secs.min(960);  // Cap at 16 minutes

        let next_retry = chrono::Utc::now() + chrono::Duration::seconds(delay_secs);
        next_retry.to_rfc3339()
    }

    /// Check if delivery is ready for retry
    pub fn is_ready_for_retry(&self) -> bool {
        if self.status != DeliveryStatus::Queued {
            return false;
        }

        match &self.next_retry_at {
            None => self.retry_count == 0,  // Initial attempt
            Some(retry_at) => {
                // Parse retry_at and check if it's in the past
                if let Ok(retry_time) = chrono::DateTime::parse_from_rfc3339(retry_at) {
                    retry_time <= chrono::Utc::now()
                } else {
                    false
                }
            }
        }
    }
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub subscribed_events: Vec<String>,
    pub secret: String,
    #[serde(default)]
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebhookRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub subscribed_events: Option<Vec<String>>,
    pub secret: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub subscribed_events: Vec<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
    // Note: secret is intentionally excluded for security
}

impl From<Webhook> for WebhookResponse {
    fn from(webhook: Webhook) -> Self {
        Self {
            id: webhook.id,
            name: webhook.name,
            url: webhook.url,
            subscribed_events: webhook.subscribed_events,
            is_active: webhook.is_active,
            created_at: webhook.created_at,
            updated_at: webhook.updated_at,
            created_by: webhook.created_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWebhookResponse {
    pub success: bool,
    pub status_code: Option<i32>,
    pub response_time_ms: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookListResponse {
    pub webhooks: Vec<WebhookResponse>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryResponse {
    pub id: String,
    pub webhook_id: String,
    pub event_type: String,
    pub status: String,
    pub http_status_code: Option<i32>,
    pub retry_count: i32,
    pub next_retry_at: Option<String>,
    pub attempted_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

impl From<WebhookDelivery> for DeliveryResponse {
    fn from(delivery: WebhookDelivery) -> Self {
        Self {
            id: delivery.id,
            webhook_id: delivery.webhook_id,
            event_type: delivery.event_type,
            status: delivery.status.to_string(),
            http_status_code: delivery.http_status_code,
            retry_count: delivery.retry_count,
            next_retry_at: delivery.next_retry_at,
            attempted_at: delivery.attempted_at,
            completed_at: delivery.completed_at,
            error_message: delivery.error_message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryListResponse {
    pub deliveries: Vec<DeliveryResponse>,
    pub total: i64,
}
