use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{ActivityEventType, Agent, AgentActivityLog, AgentAvailability};

/// Repository for agent availability operations
#[async_trait::async_trait]
pub trait AvailabilityRepository: Send + Sync {
    /// Update agent availability status with timestamp
    async fn update_agent_availability_with_timestamp(
        &self,
        agent_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()>;

    /// Update agent activity timestamp
    async fn update_agent_activity(&self, agent_id: &str) -> ApiResult<()>;

    /// Create activity log entry
    async fn create_activity_log(&self, log: &AgentActivityLog) -> ApiResult<()>;

    /// Get agents who are idle (away status) for too long
    async fn get_idle_away_agents(&self, idle_minutes: i64) -> ApiResult<Vec<Agent>>;

    /// Get agents who are online but inactive
    async fn get_inactive_online_agents(&self, inactive_minutes: i64) -> ApiResult<Vec<Agent>>;

    /// Update agent last login timestamp
    async fn update_agent_last_login(&self, agent_id: &str) -> ApiResult<()>;

    /// Get agent activity logs with pagination
    async fn get_agent_activity_logs(
        &self,
        agent_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<AgentActivityLog>, i64)>;

    /// Get config value for availability settings
    async fn get_config_value(&self, key: &str) -> ApiResult<Option<String>>;
}
