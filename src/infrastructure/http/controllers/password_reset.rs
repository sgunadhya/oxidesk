use crate::{
    infrastructure::http::middleware::{ApiResult, AppState},
    domain::entities::*,
};
/// Password Reset API Handlers
/// Feature: 017-password-reset
use axum::{extract::State, Json};

/// POST /api/password-reset/request
///
/// Request a password reset for an agent email
///
/// Returns the same response for both existing and non-existing emails
/// to prevent email enumeration attacks.
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(request): Json<RequestPasswordResetRequest>,
) -> ApiResult<Json<RequestPasswordResetResponse>> {
    let response =
        state.password_reset_service.request_password_reset(&request.email).await?;
    Ok(Json(response))
}

/// POST /api/password-reset/reset
///
/// Complete password reset using a valid token
///
/// This endpoint:
/// - Validates the token
/// - Validates password complexity
/// - Updates the agent password
/// - Marks the token as used
/// - Destroys all user sessions
pub async fn reset_password(
    State(state): State<AppState>,
    Json(request): Json<ResetPasswordRequest>,
) -> ApiResult<Json<ResetPasswordResponse>> {
    let response =
        state.password_reset_service.reset_password(&request.token, &request.new_password)
            .await?;

    Ok(Json(response))
}
