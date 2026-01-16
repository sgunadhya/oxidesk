use crate::domain::errors::DomainResult;
use crate::domain::models::role::Role;
use async_trait::async_trait;

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn list_roles(&self) -> DomainResult<Vec<Role>>;
    async fn get_role_by_id(&self, id: &str) -> DomainResult<Option<Role>>;
    async fn get_role_by_name(&self, name: &str) -> DomainResult<Option<Role>>;
    async fn create_role(&self, role: &Role) -> DomainResult<()>;
    async fn update_role(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        permissions: Option<&[String]>,
    ) -> DomainResult<()>;
    async fn delete_role(&self, id: &str) -> DomainResult<()>;

    // Additional domain-specific queries
    async fn count_users_with_role(&self, role_id: &str) -> DomainResult<i64>;
    async fn list_permissions(&self) -> DomainResult<Vec<crate::models::Permission>>;
    async fn get_user_roles(&self, user_id: &str) -> DomainResult<Vec<Role>>;
}
