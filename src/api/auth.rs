use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::*,
};

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    // Delegate to auth service
    let auth_result = auth::authenticate(
        &state.db,
        &request.email,
        &request.password,
        state.session_duration_hours,
    )
    .await?;

    // Build response from AuthResult
    let role_responses: Vec<RoleResponse> = auth_result.roles
        .iter()
        .map(|r| RoleResponse {
            id: r.id.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            permissions: None,
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
        expires_at: auth_result.session.expires_at,
        user: agent_response,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<StatusCode> {
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
            permissions: None,
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
