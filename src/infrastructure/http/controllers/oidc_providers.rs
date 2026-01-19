use crate::{
    infrastructure::http::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    domain::entities::*,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

/// List all OIDC providers (admin only)
pub async fn list_oidc_providers(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<OidcProviderResponse>>> {
    // Only admins can list OIDC providers
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden("Admin permission required".to_string()));
    }

    let providers = state.oidc_service.list_providers(false).await?;
    let responses: Vec<OidcProviderResponse> = providers
        .into_iter()
        .map(OidcProviderResponse::from)
        .collect();

    Ok(Json(responses))
}

/// Get OIDC provider by ID (admin only)
pub async fn get_oidc_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<OidcProviderResponse>> {
    // Only admins can view OIDC provider details
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden("Admin permission required".to_string()));
    }

    // Get by name (treating id parameter as name for simplicity)
    let provider = state
        .oidc_service
        .get_provider_by_name(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    Ok(Json(OidcProviderResponse::from(provider)))
}

/// Create OIDC provider (admin only)
pub async fn create_oidc_provider(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateOidcProviderRequest>,
) -> ApiResult<Json<OidcProviderResponse>> {
    // Only admins can create OIDC providers
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden("Admin permission required".to_string()));
    }

    // Validate the request
    request.validate()?;

    // Check if provider with this name already exists
    if state
        .oidc_service
        .provider_exists(&request.name)
        .await?
    {
        return Err(ApiError::Conflict(format!(
            "OIDC provider with name '{}' already exists",
            request.name
        )));
    }

    // Create provider
    let provider = OidcProvider::from_request(request);
    state.oidc_service.create_provider(&provider).await?;

    Ok(Json(OidcProviderResponse::from(provider)))
}

/// Update OIDC provider (admin only)
pub async fn update_oidc_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<UpdateOidcProviderRequest>,
) -> ApiResult<Json<OidcProviderResponse>> {
    // Only admins can update OIDC providers
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden("Admin permission required".to_string()));
    }

    // Get existing provider
    let mut provider = state
        .oidc_service
        .get_provider_by_name(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    // Update fields
    provider
        .update_from_request(request)
        .map_err(|e| ApiError::BadRequest(e))?;

    // Save updates
    state.oidc_service.update_provider(&provider).await?;

    Ok(Json(OidcProviderResponse::from(provider)))
}

/// Delete OIDC provider (admin only)
pub async fn delete_oidc_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<StatusCode> {
    // Only admins can delete OIDC providers
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden("Admin permission required".to_string()));
    }

    // Get provider to verify it exists
    let provider = state
        .oidc_service
        .get_provider_by_name(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    // Delete provider
    state.oidc_service.delete_provider(&provider.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle OIDC provider enabled status (admin only)
pub async fn toggle_oidc_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<OidcProviderResponse>> {
    // Only admins can toggle OIDC providers
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden("Admin permission required".to_string()));
    }

    // Get provider
    let provider = state
        .oidc_service
        .get_provider_by_name(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    // Toggle enabled status
    let _new_status = state.oidc_service.toggle_provider(&provider.id).await?;

    // Get updated provider
    let updated_provider = state
        .oidc_service
        .get_provider_by_name(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    Ok(Json(OidcProviderResponse::from(updated_provider)))
}

/// List enabled OIDC providers (public endpoint for login page)
pub async fn list_enabled_oidc_providers(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<OidcProviderPublicInfo>>> {
    let providers = state.oidc_service.list_providers(true).await?;

    let public_info: Vec<OidcProviderPublicInfo> = providers
        .into_iter()
        .map(|p| OidcProviderPublicInfo {
            name: p.name.clone(),
            display_name: p.name, // Could add display_name field to model
        })
        .collect();

    Ok(Json(public_info))
}

/// Public info about OIDC provider for login page
#[derive(serde::Serialize)]
pub struct OidcProviderPublicInfo {
    pub name: String,
    pub display_name: String,
}
