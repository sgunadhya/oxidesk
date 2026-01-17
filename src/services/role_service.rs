use crate::api::middleware::{ApiError, ApiResult, AuthenticatedUser};

use crate::domain::errors::DomainError;
use crate::domain::services::role_service::RoleDomainService;
use crate::models::{CreateRoleRequest, PermissionResponse, RoleResponse, UpdateRoleRequest};

// Helper to map DomainError to ApiError
impl From<DomainError> for ApiError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::NotFound(msg) => ApiError::NotFound(msg),
            DomainError::ValidationError(msg) => ApiError::BadRequest(msg),
            DomainError::Conflict(msg) => ApiError::Conflict(msg),
            DomainError::Forbidden(msg) => ApiError::Forbidden(msg),
            DomainError::Internal(msg) => ApiError::Internal(msg),
        }
    }
}

use crate::domain::ports::role_repository::RoleRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct RoleService {
    domain_service: RoleDomainService,
}

impl RoleService {
    pub fn new(repository: Arc<dyn RoleRepository>) -> Self {
        Self {
            domain_service: RoleDomainService::new(repository),
        }
    }

    /// List all roles
    pub async fn list_roles(&self) -> ApiResult<Vec<RoleResponse>> {
        let roles = self.domain_service.list_roles().await?;

        let responses: Vec<RoleResponse> = roles
            .into_iter()
            .map(|r| RoleResponse {
                id: r.id,
                name: r.name,
                description: r.description,
                permissions: r.permissions,
                is_protected: r.is_protected,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect();

        Ok(responses)
    }

    /// Get a role by ID
    pub async fn get_role(&self, id: &str) -> ApiResult<RoleResponse> {
        let role = self.domain_service.get_role(id).await?;

        Ok(RoleResponse {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: role.permissions,
            is_protected: role.is_protected,
            created_at: role.created_at,
            updated_at: role.updated_at,
        })
    }

    /// Create a new role
    pub async fn create_role(
        &self,
        auth_user: &AuthenticatedUser,
        request: CreateRoleRequest,
    ) -> ApiResult<RoleResponse> {
        // Authorization Check (Application Layer concern)
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'roles:manage' permission".to_string(),
            ));
        }

        let role = self
            .domain_service
            .create_role(request.name, request.description, request.permissions)
            .await?;

        Ok(RoleResponse {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: role.permissions,
            is_protected: role.is_protected,
            created_at: role.created_at,
            updated_at: role.updated_at,
        })
    }

    /// Update a role
    pub async fn update_role(
        &self,
        auth_user: &AuthenticatedUser,
        id: &str,
        request: UpdateRoleRequest,
    ) -> ApiResult<RoleResponse> {
        // Authorization Check
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'roles:manage' permission".to_string(),
            ));
        }

        let role = self
            .domain_service
            .update_role(id, request.name, request.description, request.permissions)
            .await?;

        Ok(RoleResponse {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions: role.permissions,
            is_protected: role.is_protected,
            created_at: role.created_at,
            updated_at: role.updated_at,
        })
    }

    /// Delete a role
    pub async fn delete(&self, auth_user: &AuthenticatedUser, id: &str) -> ApiResult<()> {
        // Authorization Check
        if !auth_user.is_admin() {
            return Err(ApiError::Forbidden(
                "Requires 'roles:manage' permission".to_string(),
            ));
        }

        self.domain_service.delete_role(id).await?;

        Ok(())
    }

    /// List all permissions
    pub async fn list_permissions(&self) -> ApiResult<Vec<PermissionResponse>> {
        let permissions = self.domain_service.list_permissions().await?;

        let responses: Vec<PermissionResponse> = permissions
            .iter()
            .map(|p| PermissionResponse {
                id: p.id.clone(),
                name: p.name.clone(),
                description: p.description.clone(),
                created_at: p.created_at.clone(),
                updated_at: p.updated_at.clone(),
            })
            .collect();

        Ok(responses)
    }

    /// Get user roles
    pub async fn get_user_roles(&self, user_id: &str) -> ApiResult<Vec<crate::models::Role>> {
        let roles = self.domain_service.get_user_roles(user_id).await?;

        // Convert domain roles to API roles
        Ok(roles.into_iter().map(|r| crate::models::Role {
            id: r.id,
            name: r.name,
            description: r.description,
            permissions: r.permissions,
            is_protected: r.is_protected,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }

    /// Count users with a specific role
    pub async fn count_users_with_role(&self, role_id: &str) -> ApiResult<i64> {
        self.domain_service.count_users_with_role(role_id).await.map_err(|e| e.into())
    }

    /// Get raw list of roles (for web pages)
    pub async fn list_roles_raw(&self) -> ApiResult<Vec<crate::models::Role>> {
        let roles = self.domain_service.list_roles().await?;

        Ok(roles.into_iter().map(|r| crate::models::Role {
            id: r.id,
            name: r.name,
            description: r.description,
            permissions: r.permissions,
            is_protected: r.is_protected,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }

    /// Get role permissions (for web pages)
    pub async fn get_role_permissions(&self, role_id: &str) -> ApiResult<Vec<crate::models::Permission>> {
        // For now, we'll get permissions from the role itself
        let role = self.domain_service.get_role(role_id).await?;

        // Get all permissions and filter by role's permission list
        let all_permissions = self.domain_service.list_permissions().await?;
        let role_permission_names: std::collections::HashSet<_> = role.permissions.iter().collect();

        Ok(all_permissions.into_iter()
            .filter(|p| role_permission_names.contains(&p.name))
            .collect())
    }
}
