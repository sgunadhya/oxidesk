use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::{CreateWebhookRequest, DeliveryListResponse, UpdateWebhookRequest},
};

/// Create a new webhook (admin only)
pub async fn create_webhook(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateWebhookRequest>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    let webhook = state.webhook_service.create_webhook(request, &auth_user.user.id).await?;

    Ok((axum::http::StatusCode::CREATED, Json(webhook)))
}

/// List all webhooks with pagination (admin only)
pub async fn list_webhooks(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<ListWebhooksParams>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    let response = state.webhook_service.list_webhooks(params.limit, params.offset).await?;

    Ok(Json(response))
}

/// Get a specific webhook by ID (admin only)
pub async fn get_webhook(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    let webhook = state.webhook_service.get_webhook(&id).await?;

    Ok(Json(webhook))
}

/// Update a webhook (admin only)
pub async fn update_webhook(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateWebhookRequest>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    let webhook = state.webhook_service.update_webhook(&id, request).await?;

    Ok(Json(webhook))
}

/// Delete a webhook (admin only)
pub async fn delete_webhook(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    state.webhook_service.delete_webhook(&id).await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Toggle webhook active/inactive status (admin only)
pub async fn toggle_webhook_status(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    let webhook = state.webhook_service.toggle_webhook_status(&id).await?;

    Ok(Json(webhook))
}

/// Send a test webhook delivery (admin only)
pub async fn test_webhook(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    // Get webhook (full model with secret)
    let webhook = state.webhook_service.get_webhook_full(&id).await?;

    // Create test payload
    let test_payload = serde_json::json!({
        "event_type": "test",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook from oxidesk",
            "webhook_id": webhook.id,
        }
    });

    let payload_str = serde_json::to_string(&test_payload)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize payload: {}", e)))?;

    // Sign the payload
    let signature = crate::services::webhook_signature::sign_payload(&payload_str, &webhook.secret);

    // Attempt delivery using reqwest directly
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError::Internal(format!("Failed to build HTTP client: {}", e)))?;

    let start = std::time::Instant::now();
    let response_result = client
        .post(&webhook.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", signature)
        .header("X-Webhook-Event", "webhook.test")
        .body(payload_str)
        .send()
        .await;

    let response_time_ms = start.elapsed().as_millis() as i64;

    let (success, http_status, error) = match response_result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, Some(status.as_u16() as i32), None)
            } else {
                (
                    false,
                    Some(status.as_u16() as i32),
                    Some(format!("HTTP {}", status)),
                )
            }
        }
        Err(e) => (false, None, Some(e.to_string())),
    };

    let response = crate::models::TestWebhookResponse {
        success,
        status_code: http_status,
        response_time_ms: Some(response_time_ms),
        error,
    };

    Ok(Json(response))
}

/// List deliveries for a webhook (admin only)
pub async fn list_webhook_deliveries(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Query(params): Query<ListDeliveriesParams>,
) -> ApiResult<impl IntoResponse> {
    // Verify admin role
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Administrator role required".to_string(),
        ));
    }

    // Get deliveries from database
    let deliveries = state
        .db
        .get_deliveries_for_webhook(&id, params.limit, params.offset, params.status.as_deref())
        .await?;

    // Get total count
    let total = state
        .db
        .count_deliveries_for_webhook(&id, params.status.as_deref())
        .await?;

    // Convert to response models
    let delivery_responses: Vec<crate::models::DeliveryResponse> = deliveries
        .into_iter()
        .map(crate::models::DeliveryResponse::from)
        .collect();

    let response = DeliveryListResponse {
        deliveries: delivery_responses,
        total,
    };

    Ok(Json(response))
}

// Query parameters for listing webhooks
#[derive(Deserialize)]
pub struct ListWebhooksParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

// Query parameters for listing deliveries
#[derive(Deserialize)]
pub struct ListDeliveriesParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub status: Option<String>,
}
