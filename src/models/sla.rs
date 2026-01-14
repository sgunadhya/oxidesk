use serde::{Deserialize, Serialize};

// ===== SLA Policy =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaPolicy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub first_response_time: String, // Format: "2h", "30m", "1d"
    pub resolution_time: String,     // Format: "24h", "2d"
    pub next_response_time: String,  // Format: "4h", "30m"
    pub created_at: String,
    pub updated_at: String,
}

impl SlaPolicy {
    pub fn new(
        name: String,
        description: Option<String>,
        first_response_time: String,
        resolution_time: String,
        next_response_time: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            first_response_time,
            resolution_time,
            next_response_time,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

// ===== Applied SLA =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedSla {
    pub id: String,
    pub conversation_id: String,
    pub sla_policy_id: String,
    pub status: AppliedSlaStatus,
    pub first_response_deadline_at: String,
    pub resolution_deadline_at: String,
    pub applied_at: String,
    pub updated_at: String,
}

impl AppliedSla {
    pub fn new(
        conversation_id: String,
        sla_policy_id: String,
        first_response_deadline_at: String,
        resolution_deadline_at: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            sla_policy_id,
            status: AppliedSlaStatus::Pending,
            first_response_deadline_at,
            resolution_deadline_at,
            applied_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppliedSlaStatus {
    Pending,
    Met,
    Breached,
}

impl std::fmt::Display for AppliedSlaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppliedSlaStatus::Pending => write!(f, "pending"),
            AppliedSlaStatus::Met => write!(f, "met"),
            AppliedSlaStatus::Breached => write!(f, "breached"),
        }
    }
}

impl std::str::FromStr for AppliedSlaStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(AppliedSlaStatus::Pending),
            "met" => Ok(AppliedSlaStatus::Met),
            "breached" => Ok(AppliedSlaStatus::Breached),
            _ => Err(format!("Invalid applied SLA status: {}", s)),
        }
    }
}

// ===== SLA Event =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaEvent {
    pub id: String,
    pub applied_sla_id: String,
    pub event_type: SlaEventType,
    pub status: SlaEventStatus,
    pub deadline_at: String,
    pub met_at: Option<String>,
    pub breached_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl SlaEvent {
    pub fn new(applied_sla_id: String, event_type: SlaEventType, deadline_at: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            applied_sla_id,
            event_type,
            status: SlaEventStatus::Pending,
            deadline_at,
            met_at: None,
            breached_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Validate SLA event status exclusivity (Feature 025: Mutual Exclusion Invariants)
    /// FR-008: SLA event status is exclusive - cannot be both met and breached
    pub fn validate_status_exclusive(&self) -> Result<(), String> {
        if self.met_at.is_some() && self.breached_at.is_some() {
            return Err("SLA event status is exclusive".to_string());
        }

        // Validate timestamp consistency with status
        match self.status {
            SlaEventStatus::Met => {
                if self.met_at.is_none() {
                    return Err("Met status requires met_at timestamp".to_string());
                }
                if self.breached_at.is_some() {
                    return Err("SLA event status is exclusive".to_string());
                }
            }
            SlaEventStatus::Breached => {
                if self.breached_at.is_none() {
                    return Err("Breached status requires breached_at timestamp".to_string());
                }
                if self.met_at.is_some() {
                    return Err("SLA event status is exclusive".to_string());
                }
            }
            SlaEventStatus::Pending => {
                if self.met_at.is_some() || self.breached_at.is_some() {
                    return Err("Pending status cannot have met_at or breached_at timestamps".to_string());
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaEventType {
    FirstResponse,
    Resolution,
    NextResponse,
}

impl std::fmt::Display for SlaEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlaEventType::FirstResponse => write!(f, "first_response"),
            SlaEventType::Resolution => write!(f, "resolution"),
            SlaEventType::NextResponse => write!(f, "next_response"),
        }
    }
}

impl std::str::FromStr for SlaEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "first_response" => Ok(SlaEventType::FirstResponse),
            "resolution" => Ok(SlaEventType::Resolution),
            "next_response" => Ok(SlaEventType::NextResponse),
            _ => Err(format!("Invalid SLA event type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlaEventStatus {
    Pending,
    Met,
    Breached,
}

impl std::fmt::Display for SlaEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlaEventStatus::Pending => write!(f, "pending"),
            SlaEventStatus::Met => write!(f, "met"),
            SlaEventStatus::Breached => write!(f, "breached"),
        }
    }
}

impl std::str::FromStr for SlaEventStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(SlaEventStatus::Pending),
            "met" => Ok(SlaEventStatus::Met),
            "breached" => Ok(SlaEventStatus::Breached),
            _ => Err(format!("Invalid SLA event status: {}", s)),
        }
    }
}

// ===== Duration Parsing Utility =====

use regex::Regex;
use std::sync::OnceLock;

/// Parse duration string like "2h", "30m", "1d" into seconds
pub fn parse_duration(duration_str: &str) -> Result<i64, String> {
    static DURATION_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = DURATION_REGEX
        .get_or_init(|| Regex::new(r"^(\d+)([hmd])$").expect("Invalid duration regex"));

    let caps = re.captures(duration_str).ok_or_else(|| {
        format!(
            "Invalid duration format: {}. Expected format: <number><h|m|d>",
            duration_str
        )
    })?;

    let number: i64 = caps[1]
        .parse()
        .map_err(|_| format!("Invalid number in duration: {}", &caps[1]))?;

    let unit = &caps[2];

    let seconds = match unit {
        "m" => number * 60,           // minutes to seconds
        "h" => number * 60 * 60,      // hours to seconds
        "d" => number * 60 * 60 * 24, // days to seconds
        _ => return Err(format!("Invalid duration unit: {}", unit)),
    };

    if seconds <= 0 {
        return Err("Duration must be greater than 0".to_string());
    }

    Ok(seconds)
}

// ===== DTOs =====

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSlaPolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub first_response_time: String,
    pub resolution_time: String,
    pub next_response_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSlaPolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub first_response_time: Option<String>,
    pub resolution_time: Option<String>,
    pub next_response_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignSlaPolicyRequest {
    pub sla_policy_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationSlaResponse {
    pub conversation_id: String,
    pub applied_sla: Option<AppliedSlaDetails>,
    pub events: Vec<SlaEventDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppliedSlaDetails {
    pub id: String,
    pub policy_name: String,
    pub status: AppliedSlaStatus,
    pub first_response_deadline_at: String,
    pub resolution_deadline_at: String,
    pub applied_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlaEventDetails {
    pub id: String,
    pub event_type: SlaEventType,
    pub status: SlaEventStatus,
    pub deadline_at: String,
    pub met_at: Option<String>,
    pub breached_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlaPolicyListResponse {
    pub policies: Vec<SlaPolicy>,
    pub pagination: crate::models::PaginationMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("2h").unwrap(), 7200);
        assert_eq!(parse_duration("1h").unwrap(), 3600);
        assert_eq!(parse_duration("24h").unwrap(), 86400);
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), 1800);
        assert_eq!(parse_duration("1m").unwrap(), 60);
        assert_eq!(parse_duration("120m").unwrap(), 7200);
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("1d").unwrap(), 86400);
        assert_eq!(parse_duration("2d").unwrap(), 172800);
    }

    #[test]
    fn test_parse_duration_invalid_format() {
        assert!(parse_duration("2x").is_err());
        assert!(parse_duration("h2").is_err());
        assert!(parse_duration("two hours").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_parse_duration_zero() {
        assert!(parse_duration("0h").is_err());
        assert!(parse_duration("0m").is_err());
    }
}
