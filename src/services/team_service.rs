use crate::{
    api::middleware::error::ApiResult,
    database::Database,
    models::{Team, TeamMemberRole, User},
};

pub struct TeamService {
    db: Database,
}

impl TeamService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create_team(&self, team: Team) -> ApiResult<Team> {
        self.db.create_team(&team).await?;
        Ok(team)
    }

    pub async fn get_team(&self, team_id: &str) -> ApiResult<Team> {
        self.db.get_team_by_id(team_id).await?.ok_or_else(|| {
            crate::api::middleware::error::ApiError::NotFound(format!("Team {} not found", team_id))
        })
    }

    pub async fn list_teams(&self) -> ApiResult<Vec<Team>> {
        self.db.list_teams().await
    }

    pub async fn add_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: TeamMemberRole,
    ) -> ApiResult<()> {
        // Verify team exists
        self.get_team(team_id).await?;

        self.db.add_team_member(team_id, user_id, role).await
    }

    pub async fn remove_member(&self, team_id: &str, user_id: &str) -> ApiResult<()> {
        self.db.remove_team_member(team_id, user_id).await
    }

    pub async fn get_members(&self, team_id: &str) -> ApiResult<Vec<User>> {
        self.db.get_team_members(team_id).await
    }

    pub async fn is_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool> {
        self.db.is_team_member(team_id, user_id).await
    }
}
