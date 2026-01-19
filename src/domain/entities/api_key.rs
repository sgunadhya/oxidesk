use serde::{Deserialize, Serialize};

/// Request DTO for generating an API key
#[derive(Debug, Deserialize)]
pub struct GenerateApiKeyRequest {
    /// Human-readable description of the API key (3-100 characters)
    pub description: String,
}

/// Response DTO for API key generation (includes secret - returned only once)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    /// 32-character alphanumeric API key
    pub api_key: String,
    /// 64-character alphanumeric secret (only returned on generation)
    pub api_secret: String,
    /// Human-readable description
    pub description: String,
    /// ISO 8601 timestamp of key creation
    pub created_at: String,
}

/// List item for API key listing (excludes secret)
#[derive(Debug, Serialize)]
pub struct ApiKeyListItem {
    /// Agent ID who owns the key
    pub agent_id: String,
    /// 32-character alphanumeric API key
    pub api_key: String,
    /// Human-readable description
    pub description: String,
    /// ISO 8601 timestamp of key creation
    pub created_at: String,
    /// ISO 8601 timestamp of last successful authentication (null if never used)
    pub last_used_at: Option<String>,
}

/// Response for API key listing endpoint
#[derive(Debug, Serialize)]
pub struct ApiKeyListResponse {
    /// List of active API keys
    pub api_keys: Vec<ApiKeyListItem>,
    /// Pagination metadata
    pub pagination: super::user::PaginationMetadata,
}
