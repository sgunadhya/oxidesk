use crate::api::middleware::{ApiError, ApiResult, AuthenticatedUser};
use crate::domain::ports::agent_repository::AgentRepository;
use crate::domain::ports::role_repository::RoleRepository;
use crate::domain::ports::user_repository::UserRepository;
use crate::models::*;
use crate::services::{
    generate_random_password, hash_password, validate_and_normalize_email,
    validate_password_complexity,
};
use std::sync::Arc;

/// Default Agent role ID (seeded in migration 002)
const DEFAULT_AGENT_ROLE_ID: &str = "00000000-0000-0000-0000-000000000002";

#[derive(Clone)]
pub struct AgentService {
    agent_repo: Arc<dyn AgentRepository>,
    user_repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
    session_service: crate::services::SessionService,
}

impl AgentService {
    pub fn new(
        agent_repo: Arc<dyn AgentRepository>,
        user_repo: Arc<dyn UserRepository>,
        role_repo: Arc<dyn RoleRepository>,
        session_service: crate::services::SessionService,
    ) -> Self {
        Self {
            agent_repo,
            user_repo,
            role_repo,
            session_service,
        }
    }

    pub async fn get_agent_by_user_id(&self, user_id: &str) -> ApiResult<Option<Agent>> {
        self.agent_repo.get_agent_by_user_id(user_id).await
    }

    /// Create a new agent with auto-generated password (Feature 016: User Creation)
    pub async fn create_agent(
        &self,
        auth_user: &AuthenticatedUser,
        request: CreateAgentRequest,
    ) -> ApiResult<CreateAgentResponse> {
        // Check permission (admin only)
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'agents:create' permission".to_string(),
            ));
        }

        // Validate email
        let email = validate_and_normalize_email(&request.email)?;

        // Check if email already exists for agents (per-type uniqueness)
        if let Some(_) = self
            .user_repo
            .get_user_by_email_and_type(&email, &UserType::Agent)
            .await?
        {
            return Err(ApiError::Conflict(
                "Email already exists for this user type".to_string(),
            ));
        }

        // Generate random password (16 characters with mixed complexity)
        let password = generate_random_password();
        let password_hash = hash_password(&password)?;

        // Use provided role_id or default to Agent role
        let role_id = request.role_id.as_deref().unwrap_or(DEFAULT_AGENT_ROLE_ID);

        // Verify role exists
        let role = self
            .role_repo
            .get_role_by_id(role_id)
            .await?
            .ok_or_else(|| ApiError::BadRequest(format!("Role not found: {}", role_id)))?;

        // Create agent with role in transaction
        let (agent_id, user_id) = self
            .agent_repo
            .create_agent_with_role(
                &email,
                &request.first_name,
                request.last_name.as_deref(),
                &password_hash,
                role_id,
            )
            .await?;

        // Get created user for timestamp
        let user = self
            .user_repo
            .get_user_by_id(&user_id)
            .await?
            .ok_or_else(|| ApiError::Internal("Failed to retrieve created user".to_string()))?;

        // Get agent details for created_at timestamp
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&user_id)
            .await?
            .ok_or_else(|| ApiError::Internal("Failed to retrieve created agent".to_string()))?;

        Ok(CreateAgentResponse {
            agent_id,
            user_id: user_id.clone(),
            email,
            first_name: request.first_name,
            last_name: request.last_name,
            password, // Plaintext password - shown only once
            password_note: "IMPORTANT: Save this password now. It will not be shown again."
                .to_string(),
            availability_status: agent.availability_status.to_string(),
            enabled: true,
            role_id: role.id.clone(),
            created_at: user.created_at,
        })
    }

    /// Get an agent by ID
    pub async fn get_agent(&self, id: &str) -> ApiResult<AgentResponse> {
        // Get user
        let user = self
            .user_repo
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        // Verify it's an agent
        if !matches!(user.user_type, UserType::Agent) {
            return Err(ApiError::NotFound("Agent not found".to_string()));
        }

        // Get agent
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        // Get roles
        let roles = self.role_repo.get_user_roles(&user.id).await?;

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

        Ok(AgentResponse {
            id: user.id.clone(),
            email: user.email.clone(),
            user_type: user.user_type.clone(),
            first_name: agent.first_name.clone(),
            roles: role_responses,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
        })
    }

    /// Delete an agent
    pub async fn delete(&self, auth_user: &AuthenticatedUser, id: &str) -> ApiResult<()> {
        // Check permission (admin only)
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'agents:delete' permission".to_string(),
            ));
        }

        // Check if user exists and is an agent
        let user = self
            .user_repo
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        if !matches!(user.user_type, UserType::Agent) {
            return Err(ApiError::NotFound("Agent not found".to_string()));
        }

        // Check if this agent has Admin role
        let roles = self.role_repo.get_user_roles(&user.id).await?;
        let is_admin = roles.iter().any(|r| r.name == "Admin");

        if is_admin {
            // Check if this is the last admin (FR-017)
            let admin_count = self.agent_repo.count_admin_users().await?;

            if admin_count <= 1 {
                return Err(ApiError::BadRequest(
                    "Cannot remove last admin agent".to_string(),
                ));
            }
        }

        // Delete user (cascade will delete agent and user_roles)
        // Use database function for soft delete if possible, ensuring it uses standard query
        // Or if there's no repository method for delete user, use db directly (which is what we have)
        self.user_repo.soft_delete_user(id, &auth_user.user.id).await?; // Assuming soft_delete_user exists on Db

        Ok(())
    }

    /// List agents with pagination
    pub async fn list_agents(&self, page: i64, per_page: i64) -> ApiResult<AgentListResponse> {
        // Validate pagination parameters
        let page = if page < 1 { 1 } else { page };
        let per_page = if per_page < 1 {
            20
        } else if per_page > 100 {
            100
        } else {
            per_page
        };

        let offset = (page - 1) * per_page;

        // Get agents with pagination
        let agents_data = self.agent_repo.list_agents(per_page, offset).await?;

        // Get total count for pagination metadata
        let total_count = self.agent_repo.count_agents().await?;
        let total_pages = (total_count + per_page - 1) / per_page;

        // Build agent responses with roles
        let mut agent_responses = Vec::new();
        for (user, agent) in agents_data {
            let roles = self.role_repo.get_user_roles(&user.id).await?;

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

        Ok(AgentListResponse {
            agents: agent_responses,
            pagination: PaginationMetadata {
                page,
                per_page,
                total_count,
                total_pages,
            },
        })
    }

    /// Update an agent
    pub async fn update_agent(
        &self,
        auth_user: &AuthenticatedUser,
        id: &str,
        request: UpdateAgentRequest,
    ) -> ApiResult<AgentResponse> {
        // Check permission (admin only)
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'agents:update' permission".to_string(),
            ));
        }

        // Check if user exists and is an agent
        let user = self
            .user_repo
            .get_user_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        if !matches!(user.user_type, UserType::Agent) {
            return Err(ApiError::NotFound("Agent not found".to_string()));
        }

        // Get agent
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        // Update agent first_name
        self.agent_repo
            .update_agent(&agent.id, &request.first_name)
            .await?;

        // Update roles if provided
        if let Some(role_ids) = request.role_ids {
            if role_ids.is_empty() {
                return Err(ApiError::BadRequest(
                    "Agent must be assigned at least one role".to_string(),
                ));
            }

            // Remove existing roles
            self.role_repo.remove_user_roles(&user.id).await?;

            // Assign new roles
            for role_id in &role_ids {
                let user_role = UserRole::new(user.id.clone(), role_id.clone());
                self.role_repo.assign_role_to_user(&user_role).await?;
            }
        }

        // Get updated roles for response
        let roles = self.role_repo.get_user_roles(&user.id).await?;

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

        Ok(AgentResponse {
            id: user.id.clone(),
            email: user.email.clone(),
            user_type: user.user_type.clone(),
            first_name: request.first_name.clone(),
            roles: role_responses,
            created_at: user.created_at.clone(),
            updated_at: user.updated_at.clone(),
        })
    }

    /// Change an agent's password
    pub async fn change_agent_password(
        &self,
        auth_user: &AuthenticatedUser,
        id: &str,
        request: ChangePasswordRequest,
    ) -> ApiResult<()> {
        // Check permission (admin only)
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'agents:update' permission".to_string(),
            ));
        }

        // Check if user exists and is an agent
        let user = self
            .user_repo
            .get_user_by_id(id)
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
        let agent = self
            .agent_repo
            .get_agent_by_user_id(&user.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

        // Update password
        self.agent_repo
            .update_agent_password(&agent.id, &password_hash)
            .await?;

        // Destroy all active sessions for this agent (security requirement)
        // This forces the agent to re-authenticate with the new password
        let session_count = self.session_service.delete_user_sessions(&user.id).await?;

        tracing::info!(
            "Password changed for agent {} (user_id: {}), destroyed {} sessions",
            agent.id,
            user.id,
            session_count
        );

        Ok(())
    }
}
