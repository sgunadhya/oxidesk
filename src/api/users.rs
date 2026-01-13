use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use crate::{
    api::middleware::{ApiError, ApiResult, AppState, AuthenticatedUser},
    models::*,
};

#[derive(Deserialize)]
pub struct UserListParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    #[serde(default)]
    pub user_type: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

pub async fn list_users(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Query(params): Query<UserListParams>,
) -> ApiResult<Json<UserListResponse>> {
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

    // Parse user_type filter if provided
    let user_type_filter = if let Some(ref type_str) = params.user_type {
        match type_str.to_lowercase().as_str() {
            "agent" => Some(UserType::Agent),
            "contact" => Some(UserType::Contact),
            _ => None,
        }
    } else {
        None
    };

    // Get users with pagination and optional type filter
    let (users, total_count) = state.db.list_users(per_page, offset, user_type_filter).await?;

    let total_pages = (total_count + per_page - 1) / per_page;

    // Build user responses
    let mut user_responses = Vec::new();
    for user in users {
        match user.user_type {
            UserType::Agent => {
                // Get agent details
                if let Some(agent) = state.db.get_agent_by_user_id(&user.id).await? {
                    let roles = state.db.get_user_roles(&user.id).await?;

                    let role_responses: Vec<RoleResponse> = roles
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

                    user_responses.push(UserSummary::Agent {
                        id: user.id.clone(),
                        email: user.email.clone(),
                        first_name: agent.first_name.clone(),
                        roles: role_responses,
                        created_at: user.created_at.clone(),
                    });
                }
            }
            UserType::Contact => {
                // Get contact details
                if let Some(contact) = state.db.get_contact_by_user_id(&user.id).await? {
                    let channels = state.db.get_contact_channels(&contact.id).await?;

                    user_responses.push(UserSummary::Contact {
                        id: user.id.clone(),
                        email: user.email.clone(),
                        first_name: contact.first_name.clone(),
                        channel_count: channels.len(),
                        created_at: user.created_at.clone(),
                    });
                }
            }
        }
    }

    Ok(Json(UserListResponse {
        users: user_responses,
        pagination: PaginationMetadata {
            page,
            per_page,
            total_count,
            total_pages,
        },
    }))
}

pub async fn get_user(
    State(state): State<AppState>,
    axum::Extension(_auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<Json<UserDetail>> {
    // Get user
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    match user.user_type {
        UserType::Agent => {
            // Get agent details
            let agent = state
                .db
                .get_agent_by_user_id(&user.id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Agent details not found".to_string()))?;

            let roles = state.db.get_user_roles(&user.id).await?;

            let role_responses: Vec<RoleResponse> = roles
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

            Ok(Json(UserDetail::Agent(AgentResponse {
                id: user.id.clone(),
                email: user.email.clone(),
                user_type: user.user_type.clone(),
                first_name: agent.first_name.clone(),
                roles: role_responses,
                created_at: user.created_at.clone(),
                updated_at: user.updated_at.clone(),
            })))
        }
        UserType::Contact => {
            // Get contact details
            let contact = state
                .db
                .get_contact_by_user_id(&user.id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Contact details not found".to_string()))?;

            let channels = state.db.get_contact_channels(&contact.id).await?;

            Ok(Json(UserDetail::Contact(ContactResponse {
                id: user.id.clone(),
                email: user.email.clone(),
                user_type: user.user_type.clone(),
                first_name: contact.first_name.clone(),
                channels,
                created_at: user.created_at.clone(),
                updated_at: user.updated_at.clone(),
            })))
        }
    }
}

pub async fn delete_user(
    State(state): State<AppState>,
    axum::Extension(auth_user): axum::Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Check permission (admin only)
    if !auth_user.is_admin() {
        return Err(ApiError::Forbidden(
            "Requires 'users:delete' permission".to_string(),
        ));
    }

    // Get user
    let user = state
        .db
        .get_user_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    match user.user_type {
        UserType::Agent => {
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
        }
        UserType::Contact => {
            // Delete user (cascade will delete contact and contact_channels)
            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(&id)
                .execute(state.db.pool())
                .await?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
