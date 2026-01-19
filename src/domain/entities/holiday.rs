use serde::{Deserialize, Serialize};

/// Holiday calendar entry for SLA business hours calculation (Feature 029)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Holiday {
    pub id: String,
    pub name: String,
    pub date: String, // Date in YYYY-MM-DD format
    pub recurring: bool, // If true, repeats annually on same month-day
    pub created_at: String,
    pub updated_at: String,
}

impl Holiday {
    pub fn new(name: String, date: String, recurring: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            date,
            recurring,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// DTO for creating a holiday
#[derive(Debug, Deserialize)]
pub struct CreateHolidayRequest {
    pub name: String,
    pub date: String, // YYYY-MM-DD
    pub recurring: bool,
}

/// DTO for updating a holiday
#[derive(Debug, Deserialize)]
pub struct UpdateHolidayRequest {
    pub name: Option<String>,
    pub date: Option<String>,
    pub recurring: Option<bool>,
}

/// DTO for holiday list response
#[derive(Debug, Serialize)]
pub struct HolidayListResponse {
    pub holidays: Vec<Holiday>,
    pub count: i64,
}
