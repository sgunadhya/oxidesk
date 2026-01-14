use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inbox {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub created_at: String,
    pub updated_at: String,
}
