use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::*,
};

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    // Extract IP address and user agent for logging
    let ip_address = auth_logger::extract_ip_address(&headers);
    let user_agent = auth_logger::extract_user_agent(&headers);

    // Check rate limit
    if let Err(wait_duration) = state.rate_limiter.check(&request.email).await {
        // Log rate limit exceeded event
        let _ = AuthLogger::log_rate_limit_exceeded(
            &state.db,
            request.email.clone(),
            AuthMethod::Password,
            ip_address,
            user_agent,
            wait_duration.as_secs(),
        )
        .await;

        return Err(ApiError::TooManyRequests(format!(
            "Too many failed login attempts. Please try again in {} seconds.",
            wait_duration.as_secs()
        )));
    }

    // Attempt authentication
    let auth_result = match auth::authenticate(
        &state.db,
        &request.email,
        &request.password,
        state.session_duration_hours,
    )
    .await
    {
        Ok(result) => {
            // Reset rate limiter on successful login
            state.rate_limiter.reset(&request.email).await;

            // Log successful login
            let _ = AuthLogger::log_login_success(
                &state.db,
                result.user.id.clone(),
                request.email.clone(),
                AuthMethod::Password,
                None, // no provider for password auth
                ip_address.clone(),
                user_agent.clone(),
            )
            .await;

            result
        }
        Err(e) => {
            // Record failed attempt for rate limiting
            let _ = state.rate_limiter.record_failure(&request.email).await;

            // Log failed login attempt
            let error_reason = match &e {
                ApiError::Unauthorized => "Invalid email or password".to_string(),
                _ => format!("{:?}", e),
            };

            let _ = AuthLogger::log_login_failure(
                &state.db,
                request.email.clone(),
                AuthMethod::Password,
                None,
                ip_address,
                user_agent,
                error_reason,
            )
            .await;

            return Err(e);
        }
    };

    // Build response from AuthResult
    let role_responses: Vec<RoleResponse> = auth_result.roles
        .iter()
        .map(|r| RoleResponse {
            id: r.id.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            permissions: r.permissions.clone(),
            is_protected: r.is_protected,
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
        })
        .collect();

    let agent_response = AgentResponse {
        id: auth_result.user.id,
        email: auth_result.user.email,
        user_type: auth_result.user.user_type,
        first_name: auth_result.agent.first_name,
        roles: role_responses,
        created_at: auth_result.user.created_at,
        updated_at: auth_result.user.updated_at,
    };

    Ok(Json(LoginResponse {
        token: auth_result.session.token,
        csrf_token: auth_result.session.csrf_token,
        expires_at: auth_result.session.expires_at,
        user: agent_response,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<StatusCode> {
    // Extract IP address and user agent for logging
    let ip_address = auth_logger::extract_ip_address(&headers);
    let user_agent = auth_logger::extract_user_agent(&headers);

    // Log logout event
    let _ = AuthLogger::log_logout(
        &state.db,
        auth_user.user.id.clone(),
        auth_user.user.email.clone(),
        auth_user.session.auth_method.clone(),
        auth_user.session.provider_name.clone(),
        ip_address,
        user_agent,
    )
    .await;

    // Delete the session using the token from authenticated user
    state.db.delete_session(&auth_user.token).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_session(
    auth_user: axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<AgentResponse>> {
    let role_responses: Vec<RoleResponse> = auth_user
        .roles
        .iter()
        .map(|r| RoleResponse {
            id: r.id.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            permissions: r.permissions.clone(),
            is_protected: r.is_protected,
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
        })
        .collect();

    let response = AgentResponse {
        id: auth_user.user.id.clone(),
        email: auth_user.user.email.clone(),
        user_type: auth_user.user.user_type.clone(),
        first_name: auth_user.agent.first_name.clone(),
        roles: role_responses,
        created_at: auth_user.user.created_at.clone(),
        updated_at: auth_user.user.updated_at.clone(),
    };

    Ok(Json(response))
}

/// Get authentication events for the current user
pub async fn get_my_auth_events(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<AuthEventListResponse>> {
    // Default pagination: 50 events
    let events = AuthLogger::get_user_events(&state.db, &auth_user.user.id, 50, 0).await?;

    let event_responses: Vec<AuthEventResponse> = events
        .into_iter()
        .map(AuthEventResponse::from)
        .collect();

    let total = event_responses.len() as i64;

    Ok(Json(AuthEventListResponse {
        events: event_responses,
        total,
    }))
}

/// Get recent authentication events (admin only)
pub async fn get_recent_auth_events(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<AuthEventListResponse>> {
    // Check if user is admin
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Admin permission required".to_string(),
        ));
    }

    // Default pagination: 100 recent events
    let events = AuthLogger::get_recent_events(&state.db, 100, 0).await?;

    let event_responses: Vec<AuthEventResponse> = events
        .into_iter()
        .map(AuthEventResponse::from)
        .collect();

    let total = event_responses.len() as i64;

    Ok(Json(AuthEventListResponse {
        events: event_responses,
        total,
    }))
}

/// Initiate OIDC login flow
///
/// Redirects the user to the OIDC provider's authorization endpoint.
/// Stores state, nonce, and PKCE verifier in session for callback validation.
pub async fn oidc_login(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
) -> ApiResult<axum::response::Redirect> {
    // Get provider configuration
    let provider = state
        .db
        .get_oidc_provider_by_name(&provider_name)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    // Check if provider is enabled
    if !provider.enabled {
        return Err(ApiError::BadRequest("OIDC provider is disabled".to_string()));
    }

    // Initiate OIDC flow
    let auth_request = OidcService::initiate_login(&provider).await?;

    // Store state, nonce, and PKCE verifier in database for validation on callback
    // This prevents CSRF and replay attacks
    let oidc_state = OidcState::new(
        auth_request.state.clone(),
        provider_name,
        auth_request.nonce.clone(),
        auth_request.pkce_verifier.clone(),
    );

    state.db.create_oidc_state(&oidc_state).await?;

    // Redirect to provider's authorization URL
    Ok(axum::response::Redirect::temporary(&auth_request.authorize_url))
}

/// Handle OIDC callback
///
/// Receives authorization code from provider, exchanges it for tokens,
/// validates the ID token, and creates a session.
pub async fn oidc_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<OidcCallbackParams>,
) -> ApiResult<Json<LoginResponse>> {
    // Extract IP address and user agent for logging
    let ip_address = auth_logger::extract_ip_address(&headers);
    let user_agent = auth_logger::extract_user_agent(&headers);

    // Check for error from provider
    if let Some(error) = params.error {
        let error_description = params.error_description.unwrap_or_else(|| error.clone());

        tracing::warn!("OIDC callback error: {}", error_description);

        return Err(ApiError::BadRequest(format!(
            "OIDC authentication failed: {}",
            error_description
        )));
    }

    // Get authorization code
    let code = params
        .code
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".to_string()))?;

    let state_param = params
        .state
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".to_string()))?;

    // Retrieve and consume stored OIDC state (one-time use for security)
    let stored_state = state
        .db
        .consume_oidc_state(&state_param)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest("Invalid or expired state parameter".to_string())
        })?;

    // Verify state hasn't expired
    if stored_state.is_expired() {
        return Err(ApiError::BadRequest("OIDC state has expired".to_string()));
    }

    // Verify state matches (CSRF protection)
    if stored_state.state != state_param {
        return Err(ApiError::BadRequest("State mismatch - possible CSRF attack".to_string()));
    }

    // Extract validated values
    let provider_name = stored_state.provider_name;
    let expected_state = stored_state.state;
    let pkce_verifier = stored_state.pkce_verifier;

    // Get provider configuration
    let provider = state
        .db
        .get_oidc_provider_by_name(&provider_name)
        .await?
        .ok_or_else(|| ApiError::NotFound("OIDC provider not found".to_string()))?;

    // Handle callback and create session
    let callback_result = match OidcService::handle_callback(
        &state.db,
        &provider,
        code,
        state_param,
        expected_state,
        pkce_verifier,
        state.session_duration_hours,
    )
    .await
    {
        Ok(result) => {
            // Log successful OIDC login
            let _ = AuthLogger::log_login_success(
                &state.db,
                result.user.id.clone(),
                result.user.email.clone(),
                AuthMethod::Oidc,
                Some(provider.name.clone()),
                ip_address,
                user_agent,
            )
            .await;

            result
        }
        Err(e) => {
            // Log failed OIDC login
            let _ = AuthLogger::log_login_failure(
                &state.db,
                "unknown".to_string(), // email not available on failure
                AuthMethod::Oidc,
                Some(provider.name),
                ip_address,
                user_agent,
                format!("{:?}", e),
            )
            .await;

            return Err(e);
        }
    };

    // Build response
    let role_responses: Vec<RoleResponse> = callback_result
        .roles
        .iter()
        .map(|r| RoleResponse {
            id: r.id.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            permissions: r.permissions.clone(),
            is_protected: r.is_protected,
            created_at: r.created_at.clone(),
            updated_at: r.updated_at.clone(),
        })
        .collect();

    let agent_response = AgentResponse {
        id: callback_result.user.id,
        email: callback_result.user.email,
        user_type: callback_result.user.user_type,
        first_name: callback_result.agent.first_name,
        roles: role_responses,
        created_at: callback_result.user.created_at,
        updated_at: callback_result.user.updated_at,
    };

    Ok(Json(LoginResponse {
        token: callback_result.session.token,
        csrf_token: callback_result.session.csrf_token,
        expires_at: callback_result.session.expires_at,
        user: agent_response,
    }))
}

/// OIDC callback parameters
#[derive(serde::Deserialize)]
pub struct OidcCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}
