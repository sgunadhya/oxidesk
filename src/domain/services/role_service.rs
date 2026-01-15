use crate::domain::models::role::Role;
use crate::domain::ports::role_repository::RoleRepository;
use crate::domain::errors::{DomainError, DomainResult};

pub struct RoleDomainService<R: RoleRepository> {
    repository: R,
}

impl<R: RoleRepository> RoleDomainService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list_roles(&self) -> DomainResult<Vec<Role>> {
        self.repository.list_roles().await
    }

    pub async fn get_role(&self, id: &str) -> DomainResult<Role> {
        self.repository
            .get_role_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Role with id {} not found", id)))
    }

    pub async fn create_role(
        &self,
        name: String,
        description: Option<String>,
        permissions: Vec<String>,
    ) -> DomainResult<Role> {
        // Validation
        if name.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Role name cannot be empty".to_string(),
            ));
        }

        if permissions.is_empty() {
            return Err(DomainError::ValidationError(
                "Role must have at least one permission".to_string(),
            ));
        }

        for permission in &permissions {
            if !permission.contains(':') {
                return Err(DomainError::ValidationError(format!(
                    "Invalid permission format: '{}'. Must match pattern 'resource:action'",
                    permission
                )));
            }
        }

        // Check uniqueness
        if self.repository.get_role_by_name(&name).await?.is_some() {
            return Err(DomainError::Conflict("Role name already exists".to_string()));
        }

        // Create
        let role = Role::new(name, description, permissions);
        self.repository.create_role(&role).await?;

        Ok(role)
    }

    pub async fn update_role(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
        permissions: Option<Vec<String>>,
    ) -> DomainResult<Role> {
        let role = self.get_role(id).await?;

        // Domain Rule: Cannot modify protected roles
        if role.is_protected {
            return Err(DomainError::Forbidden("Cannot modify Admin role".to_string()));
        }

        // Validation if permissions are updated
        if let Some(ref perms) = permissions {
             if perms.is_empty() {
                return Err(DomainError::ValidationError(
                    "Role must have at least one permission".to_string(),
                ));
            }

            for permission in perms {
                if !permission.contains(':') {
                    return Err(DomainError::ValidationError(format!(
                        "Invalid permission format: '{}'. Must match pattern 'resource:action'",
                        permission
                    )));
                }
            }
        }

        // Check name uniqueness if changed
        if let Some(ref n) = name {
            if n.trim().is_empty() {
                 return Err(DomainError::ValidationError(
                    "Role name cannot be empty".to_string(),
                ));
            }
            if n != &role.name {
                 if self.repository.get_role_by_name(n).await?.is_some() {
                    return Err(DomainError::Conflict("Role name already exists".to_string()));
                }
            }
        }

        self.repository.update_role(
            id, 
            name.as_deref(), 
            description.as_deref(), 
            permissions.as_deref()
        ).await?;

        self.get_role(id).await
    }

    pub async fn delete_role(&self, id: &str) -> DomainResult<()> {
        let role = self.get_role(id).await?;

        // Domain Rule: Cannot delete protected roles
        if role.is_protected {
            return Err(DomainError::Forbidden("Cannot modify Admin role".to_string()));
        }

        // Domain Rule: Cannot delete if assigned
        let count = self.repository.count_users_with_role(id).await?;
        if count > 0 {
             return Err(DomainError::Conflict(format!(
                "Cannot delete role: {} agents currently assigned",
                count
            )));
        }

        self.repository.delete_role(id).await
    }
}
