use crate::api::middleware::error::ApiResult;
use crate::models::{Agent, User};
use async_trait::async_trait;

#[async_trait]
pub trait AgentRepository: Send + Sync {
    // Agent operations
    async fn create_agent(&self, agent: &Agent) -> ApiResult<()>;
    /// Create agent with role assignment in transaction (Feature 016: User Creation)
    /// Creates user + agent + role assignment atomically
    /// Returns the created agent_id and user_id
    async fn create_agent_with_role(
        &self,
        email: &str,
        first_name: &str,
        last_name: Option<&str>,
        password_hash: &str,
        role_id: &str,
    ) -> ApiResult<(String, String)>;
    async fn get_agent_by_user_id(&self, user_id: &str) -> ApiResult<Option<Agent>>;
    // List agents with pagination (348-401 in view, originally 451)
    async fn list_agents(&self, limit: i64, offset: i64) -> ApiResult<Vec<(User, Agent)>>;
    // Count total agents
    async fn count_agents(&self) -> ApiResult<i64>;
    // Count admin users (for last admin check)
    async fn count_admin_users(&self) -> ApiResult<i64>;
    // Agent update operations
    async fn update_agent(&self, agent_id: &str, first_name: &str) -> ApiResult<()>;
    async fn update_agent_password(&self, agent_id: &str, password_hash: &str) -> ApiResult<()>;
    /// Update agent password hash by user_id (for password reset)
    async fn update_agent_password_by_user_id(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> ApiResult<()>;
}
