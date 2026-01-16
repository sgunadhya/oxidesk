use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    error::ApiError,
    models::{CreateInboxEmailConfigRequest, InboxEmailConfig, UpdateInboxEmailConfigRequest},
};
/// API handlers for inbox email configurations (Feature 021)
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

/// Response DTO with masked passwords
#[derive(Debug, Clone, Serialize)]
pub struct InboxEmailConfigResponse {
    pub id: String,
    pub inbox_id: String,
    pub imap_host: String,
    pub imap_port: i32,
    pub imap_username: String,
    pub imap_password: String, // Will be masked
    pub imap_use_tls: bool,
    pub imap_folder: String,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub smtp_password: String, // Will be masked
    pub smtp_use_tls: bool,
    pub email_address: String,
    pub display_name: String,
    pub poll_interval_seconds: i32,
    pub enabled: bool,
    pub last_poll_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<InboxEmailConfig> for InboxEmailConfigResponse {
    fn from(config: InboxEmailConfig) -> Self {
        Self {
            id: config.id,
            inbox_id: config.inbox_id,
            imap_host: config.imap_host,
            imap_port: config.imap_port,
            imap_username: config.imap_username,
            imap_password: "********".to_string(), // Mask password
            imap_use_tls: config.imap_use_tls,
            imap_folder: config.imap_folder,
            smtp_host: config.smtp_host,
            smtp_port: config.smtp_port,
            smtp_username: config.smtp_username,
            smtp_password: "********".to_string(), // Mask password
            smtp_use_tls: config.smtp_use_tls,
            email_address: config.email_address,
            display_name: config.display_name,
            poll_interval_seconds: config.poll_interval_seconds,
            enabled: config.enabled,
            last_poll_at: config.last_poll_at,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Create inbox email configuration
pub async fn create_inbox_email_config(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(inbox_id): Path<String>,
    Json(request): Json<CreateInboxEmailConfigRequest>,
) -> ApiResult<(StatusCode, Json<InboxEmailConfigResponse>)> {
    // Check if config already exists
    if let Some(_) = state
        .email_service
        .get_inbox_email_config(&inbox_id)
        .await?
    {
        return Err(ApiError::BadRequest(
            "Email configuration already exists for this inbox".to_string(),
        ));
    }

    // Create configuration
    let config = InboxEmailConfig::new(
        inbox_id.clone(),
        request.imap_host,
        request.imap_port,
        request.imap_username,
        request.imap_password,
        request.smtp_host,
        request.smtp_port,
        request.smtp_username,
        request.smtp_password,
        request.email_address,
        request.display_name,
        request.poll_interval_seconds,
    );

    let created = state
        .email_service
        .create_inbox_email_config(&config)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(InboxEmailConfigResponse::from(created)),
    ))
}

/// Get inbox email configuration
pub async fn get_inbox_email_config(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(inbox_id): Path<String>,
) -> ApiResult<Json<InboxEmailConfigResponse>> {
    let config = state
        .email_service
        .get_inbox_email_config(&inbox_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Email configuration not found for inbox {}",
                inbox_id
            ))
        })?;

    Ok(Json(InboxEmailConfigResponse::from(config)))
}

/// Update inbox email configuration
pub async fn update_inbox_email_config(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(inbox_id): Path<String>,
    Json(request): Json<UpdateInboxEmailConfigRequest>,
) -> ApiResult<Json<InboxEmailConfigResponse>> {
    // Get existing config
    let existing = state
        .email_service
        .get_inbox_email_config(&inbox_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Email configuration not found for inbox {}",
                inbox_id
            ))
        })?;

    // Update configuration
    let updated = state
        .email_service
        .update_inbox_email_config(&existing.id, &request)
        .await?;

    Ok(Json(InboxEmailConfigResponse::from(updated)))
}

/// Delete inbox email configuration
pub async fn delete_inbox_email_config(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(inbox_id): Path<String>,
) -> ApiResult<StatusCode> {
    // Get existing config
    let existing = state
        .email_service
        .get_inbox_email_config(&inbox_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Email configuration not found for inbox {}",
                inbox_id
            ))
        })?;

    // Delete configuration
    state
        .email_service
        .delete_inbox_email_config(&existing.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Test inbox email configuration connection
#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub imap_host: String,
    pub imap_port: i32,
    pub imap_username: String,
    pub imap_password: String,
    pub imap_use_tls: bool,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_use_tls: bool,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub imap_success: bool,
    pub imap_error: Option<String>,
    pub smtp_success: bool,
    pub smtp_error: Option<String>,
}

/// Test IMAP and SMTP connection (without saving config)
pub async fn test_inbox_email_config(
    State(_state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Json(_request): Json<TestConnectionRequest>,
) -> ApiResult<Json<TestConnectionResponse>> {
    // TODO: Implement actual connection testing
    // For now, return a placeholder response
    Ok(Json(TestConnectionResponse {
        imap_success: true,
        imap_error: None,
        smtp_success: true,
        smtp_error: None,
    }))
}
