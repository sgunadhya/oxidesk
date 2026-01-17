use crate::{
    api::middleware::error::ApiResult,
    domain::ports::team_repository::TeamRepository,
    models::{Team, TeamMemberRole, User},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct TeamService {
    team_repo: Arc<dyn TeamRepository>,
}

impl TeamService {
    pub fn new(team_repo: Arc<dyn TeamRepository>) -> Self {
        Self { team_repo }
    }

    pub async fn create_team(&self, team: Team) -> ApiResult<Team> {
        self.team_repo.create_team(&team).await?;
        Ok(team)
    }

    pub async fn get_team(&self, team_id: &str) -> ApiResult<Team> {
        self.team_repo.get_team_by_id(team_id).await?.ok_or_else(|| {
            crate::api::middleware::error::ApiError::NotFound(format!("Team {} not found", team_id))
        })
    }

    pub async fn list_teams(&self) -> ApiResult<Vec<Team>> {
        self.team_repo.list_teams().await
    }

    pub async fn add_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: TeamMemberRole,
    ) -> ApiResult<()> {
        // Verify team exists
        self.get_team(team_id).await?;

        self.team_repo.add_team_member(team_id, user_id, role).await
    }

    pub async fn remove_member(&self, team_id: &str, user_id: &str) -> ApiResult<()> {
        self.team_repo.remove_team_member(team_id, user_id).await
    }

    pub async fn get_members(&self, team_id: &str) -> ApiResult<Vec<User>> {
        self.team_repo.get_team_members(team_id).await
    }

    pub async fn is_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool> {
        self.team_repo.is_team_member(team_id, user_id).await
    }
}
