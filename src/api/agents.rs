use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::*,
};

pub async fn create_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Json(request): Json<CreateAgentRequest>,
) -> ApiResult<(StatusCode, Json<AgentResponse>)> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'agents:create' permission".to_string(),
        ));
    }

    // Validate email
    let email = validate_and_normalize_email(&request.email)?;

    // Check if email already exists for agents (per-type uniqueness)
    if let Some(_) = state
        .db
        .get_user_by_email_and_type(&email, &UserType::Agent)
        .await?
    {
        return Err(ApiError::Conflict("Email already exists".to_string()));
    }

    // Validate password complexity
    validate_password_complexity(&request.password)?;

    // Validate at least one role (FR-007)
    if request.role_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "Agent must be assigned at least one role".to_string(),
        ));
    }

    // Hash password
    let password_hash = hash_password(&request.password)?;

    // Create user
    let user = User::new(email, UserType::Agent);
    state.db.create_user(&user).await?;

    // Create agent
    let agent = Agent::new(user.id.clone(), request.first_name.clone(), password_hash);
    state.db.create_agent(&agent).await?;

    // Assign roles
    for role_id in &request.role_ids {
        let user_role = UserRole::new(user.id.clone(), role_id.clone());
        state.db.assign_role_to_user(&user_role).await?;
    }

    // Get assigned roles for response
    let roles = state.db.get_user_roles(&user.id).await?;

    let role_responses: Vec<RoleResponse> = roles
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
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: agent.first_name.clone(),
        roles: role_responses,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_agent(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentResponse>> {
    // Get user
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    // Verify it's an agent
    if !matches!(user.user_type, UserType::Agent) {
        return Err(ApiError::NotFound("Agent not found".to_string()));
    }

    // Get agent
    let agent = state
        .db
        .get_agent_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    // Get roles
    let roles = state.db.get_user_roles(&user.id).await?;

    let role_responses: Vec<RoleResponse> = roles
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
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: agent.first_name.clone(),
        roles: role_responses,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

    Ok(Json(response))
}

pub async fn delete_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'agents:delete' permission".to_string(),
        ));
    }

    // Check if user exists and is an agent
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    if !matches!(user.user_type, UserType::Agent) {
        return Err(ApiError::NotFound("Agent not found".to_string()));
    }

    // Check if this agent has Admin role
    let roles = state.db.get_user_roles(&user.id).await?;
    let is_admin = roles.iter().any(|r| r.name == "Admin");

    if is_admin {
        // Check if this is the last admin (FR-017)
        let admin_count = state.db.count_admin_users().await?;

        if admin_count <= 1 {
            return Err(ApiError::BadRequest(
                "Cannot remove last admin agent".to_string(),
            ));
        }
    }

    // Delete user (cascade will delete agent and user_roles)
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(state.db.pool())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

pub async fn list_agents(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<AgentListResponse>> {
    // Validate pagination parameters
    let page = if params.page < 1 { 1 } else { params.page };
    let per_page = if params.per_page < 1 {
        20
    } else if params.per_page > 100 {
        100
    } else {
        params.per_page
    };

    let offset = (page - 1) * per_page;

    // Get agents with pagination
    let agents_data = state.db.list_agents(per_page, offset).await?;

    // Get total count for pagination metadata
    let total_count = state.db.count_agents().await?;
    let total_pages = (total_count + per_page - 1) / per_page;

    // Build agent responses with roles
    let mut agent_responses = Vec::new();
    for (user, agent) in agents_data {
        let roles = state.db.get_user_roles(&user.id).await?;

        let role_responses: Vec<RoleResponse> = roles
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

        agent_responses.push(AgentResponse {
            id: user.id.clone(),
            email: user.email.clone(),
            user_type: user.user_type.clone(),
            first_name: agent.first_name.clone(),
            roles: role_responses,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
        });
    }

    Ok(Json(AgentListResponse {
        agents: agent_responses,
        pagination: PaginationMetadata {
            page,
            per_page,
            total_count,
            total_pages,
        },
    }))
}

pub async fn update_agent(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> ApiResult<Json<AgentResponse>> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'agents:update' permission".to_string(),
        ));
    }

    // Check if user exists and is an agent
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    if !matches!(user.user_type, UserType::Agent) {
        return Err(ApiError::NotFound("Agent not found".to_string()));
    }

    // Get agent
    let agent = state
        .db
        .get_agent_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    // Update agent first_name
    state.db.update_agent(&agent.id, &request.first_name).await?;

    // Update roles if provided
    if let Some(role_ids) = request.role_ids {
        if role_ids.is_empty() {
            return Err(ApiError::BadRequest(
                "Agent must be assigned at least one role".to_string(),
            ));
        }

        // Remove existing roles
        state.db.remove_user_roles(&user.id).await?;

        // Assign new roles
        for role_id in &role_ids {
            let user_role = UserRole::new(user.id.clone(), role_id.clone());
            state.db.assign_role_to_user(&user_role).await?;
        }
    }

    // Get updated roles for response
    let roles = state.db.get_user_roles(&user.id).await?;

    let role_responses: Vec<RoleResponse> = roles
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
        id: user.id.clone(),
        email: user.email.clone(),
        user_type: user.user_type.clone(),
        first_name: request.first_name.clone(),
        roles: role_responses,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

    Ok(Json(response))
}

pub async fn change_agent_password(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'agents:update' permission".to_string(),
        ));
    }

    // Check if user exists and is an agent
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    if !matches!(user.user_type, UserType::Agent) {
        return Err(ApiError::NotFound("Agent not found".to_string()));
    }

    // Validate password complexity
    validate_password_complexity(&request.new_password)?;

    // Hash new password
    let password_hash = hash_password(&request.new_password)?;

    // Get agent
    let agent = state
        .db
        .get_agent_by_user_id(&user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    // Update password
    state.db.update_agent_password(&agent.id, &password_hash).await?;

    Ok(StatusCode::NO_CONTENT)
}
