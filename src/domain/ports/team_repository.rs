use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{Team, TeamMemberRole, User};

#[async_trait::async_trait]
pub trait TeamRepository: Send + Sync {
    async fn create_team(&self, team: &Team) -> ApiResult<()>;
    async fn get_team_by_id(&self, id: &str) -> ApiResult<Option<Team>>;
    async fn list_teams(&self) -> ApiResult<Vec<Team>>;

    // Membership
    async fn add_team_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: TeamMemberRole,
    ) -> ApiResult<()>;

    async fn remove_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<()>;

    async fn get_team_members(&self, team_id: &str) -> ApiResult<Vec<User>>;

    async fn is_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool>;

    async fn get_user_teams(&self, user_id: &str) -> ApiResult<Vec<Team>>;

    async fn update_team_sla_policy(
        &self,
        team_id: &str,
        sla_policy_id: Option<&str>,
    ) -> ApiResult<()>;
}
