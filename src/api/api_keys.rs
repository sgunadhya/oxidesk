use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::middleware::error::ApiError;
use crate::api::middleware::auth::{AuthenticatedUser, AppState};
use crate::models::{
    ApiKeyListItem, ApiKeyListResponse, ApiKeyResponse, GenerateApiKeyRequest,
    PaginationMetadata,
};
use crate::services::api_key_service::{
    generate_api_key, generate_api_secret, hash_api_secret,
};

/// Query parameters for API key listing
#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    50
}
fn default_sort_by() -> String {
    "created_at".to_string()
}
fn default_sort_order() -> String {
    "desc".to_string()
}

/// Generate API key for an agent
/// POST /agents/:id/api-key
pub async fn generate_api_key_handler(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    axum::Extension(authenticated_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<GenerateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), ApiError> {
    // Authorization: Admin or self
    let is_admin = authenticated_user
        .permissions
        .iter()
        .any(|p| p == "users:manage" || p == "*");
    let is_self = &authenticated_user.agent.id == &agent_id;

    if !is_admin && !is_self {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to generate API key".to_string(),
        ));
    }

    // Validate description length (3-100 characters)
    let description = request.description.trim();
    if description.len() < 3 || description.len() > 100 {
        return Err(ApiError::BadRequest(
            "Description must be between 3 and 100 characters".to_string(),
        ));
    }

    // Check if agent exists
    let agent = state
        .db
        .get_agent_by_id(&agent_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    // Check if agent already has an API key
    if agent.api_key.is_some() {
        return Err(ApiError::Conflict(
            "Agent already has an API key. Revoke existing key first.".to_string(),
        ));
    }

    // Generate API key and secret
    let api_key = generate_api_key();
    let api_secret = generate_api_secret();

    // Hash the secret
    let api_secret_hash = hash_api_secret(&api_secret).map_err(|e| {
        tracing::error!("Failed to hash API secret: {}", e);
        ApiError::Internal("Failed to generate API key".to_string())
    })?;

    // Store in database
    state
        .db
        .create_api_key(&agent_id, &api_key, &api_secret_hash, description)
        .await?;

    // Get updated agent to retrieve created_at
    let updated_agent = state
        .db
        .get_agent_by_id(&agent_id)
        .await?
        .ok_or_else(|| ApiError::Internal("Failed to retrieve created API key".to_string()))?;

    // Return response with both key and secret (only time secret is returned)
    Ok((
        StatusCode::OK,
        Json(ApiKeyResponse {
            api_key,
            api_secret,
            description: description.to_string(),
            created_at: updated_agent.api_key_created_at.unwrap_or_default(),
        }),
    ))
}

/// Revoke API key for an agent
/// DELETE /agents/:id/api-key
pub async fn revoke_api_key_handler(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    axum::Extension(authenticated_user): axum::Extension<AuthenticatedUser>,
) -> Result<StatusCode, ApiError> {
    // Authorization: Admin or self
    let is_admin = authenticated_user
        .permissions
        .iter()
        .any(|p| p == "users:manage" || p == "*");
    let is_self = &authenticated_user.agent.id == &agent_id;

    if !is_admin && !is_self {
        return Err(ApiError::Forbidden(
            "Insufficient permissions to revoke API key".to_string(),
        ));
    }

    // Check if agent exists
    let agent = state
        .db
        .get_agent_by_id(&agent_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    // Check if agent has an API key to revoke
    if agent.api_key.is_none() {
        return Err(ApiError::NotFound(
            "Agent has no API key to revoke".to_string(),
        ));
    }

    // Revoke the API key
    let revoked = state.db.revoke_api_key(&agent_id).await?;

    if !revoked {
        return Err(ApiError::NotFound(
            "Agent has no API key to revoke".to_string(),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// List all active API keys (admin only)
/// GET /api-keys
pub async fn list_api_keys_handler(
    State(state): State<AppState>,
    Query(query): Query<ListApiKeysQuery>,
    axum::Extension(authenticated_user): axum::Extension<AuthenticatedUser>,
) -> Result<Json<ApiKeyListResponse>, ApiError> {
    // Authorization: Admin only
    let is_admin = authenticated_user
        .permissions
        .iter()
        .any(|p| p == "users:read" || p == "users:manage" || p == "*");

    if !is_admin {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    // Validate and clamp pagination parameters
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Validate sort parameters
    let sort_by = match query.sort_by.as_str() {
        "created_at" | "last_used_at" | "description" => &query.sort_by,
        _ => "created_at",
    };

    let sort_order = match query.sort_order.to_lowercase().as_str() {
        "asc" | "desc" => query.sort_order.to_lowercase(),
        _ => "desc".to_string(),
    };

    // Get API keys from database
    let api_keys_data = state
        .db
        .list_api_keys(per_page, offset, sort_by, &sort_order)
        .await?;

    let api_keys: Vec<ApiKeyListItem> = api_keys_data
        .into_iter()
        .map(
            |(agent_id, api_key, description, created_at, last_used_at)| ApiKeyListItem {
                agent_id,
                api_key,
                description,
                created_at,
                last_used_at,
            },
        )
        .collect();

    // Get total count for pagination metadata
    let total_count = state.db.count_api_keys().await?;
    let total_pages = (total_count + per_page - 1) / per_page;

    Ok(Json(ApiKeyListResponse {
        api_keys,
        pagination: PaginationMetadata {
            page,
            per_page,
            total_count,
            total_pages,
        },
    }))
}
