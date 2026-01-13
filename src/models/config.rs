use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub updated_at: String,
}
