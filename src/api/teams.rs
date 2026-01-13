use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::middleware::{ApiResult, AppState, AuthenticatedUser},
    models::*,
    services::TeamService,
};

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddTeamMemberRequest {
    pub user_id: String,
    pub role: TeamMemberRole,
}

// POST /api/teams - Create a new team
pub async fn create_team(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Json(req): Json<CreateTeamRequest>,
) -> ApiResult<(StatusCode, Json<Team>)> {
    let team = Team::new(req.name, req.description);
    let team_service = TeamService::new(state.db.clone());

    let created_team = team_service.create_team(team).await?;

    tracing::info!("Team created: id={}", created_team.id);
    Ok((StatusCode::CREATED, Json(created_team)))
}

// GET /api/teams - List all teams
pub async fn list_teams(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
) -> ApiResult<Json<Vec<Team>>> {
    let team_service = TeamService::new(state.db.clone());
    let teams = team_service.list_teams().await?;

    Ok(Json(teams))
}

// GET /api/teams/:id - Get team by ID
pub async fn get_team(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Team>> {
    let team_service = TeamService::new(state.db.clone());
    let team = team_service.get_team(&team_id).await?;

    Ok(Json(team))
}

// POST /api/teams/:id/members - Add member to team
pub async fn add_team_member(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(team_id): Path<String>,
    Json(req): Json<AddTeamMemberRequest>,
) -> ApiResult<StatusCode> {
    let team_service = TeamService::new(state.db.clone());
    team_service
        .add_member(&team_id, &req.user_id, req.role)
        .await?;

    tracing::info!("User {} added to team {}", req.user_id, team_id);
    Ok(StatusCode::CREATED)
}

// DELETE /api/teams/:id/members/:user_id - Remove member from team
pub async fn remove_team_member(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path((team_id, user_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let team_service = TeamService::new(state.db.clone());
    team_service.remove_member(&team_id, &user_id).await?;

    tracing::info!("User {} removed from team {}", user_id, team_id);
    Ok(StatusCode::NO_CONTENT)
}

// GET /api/teams/:id/members - Get team members
pub async fn get_team_members(
    State(state): State<AppState>,
    axum::Extension(_user): axum::Extension<AuthenticatedUser>,
    Path(team_id): Path<String>,
) -> ApiResult<Json<Vec<User>>> {
    let team_service = TeamService::new(state.db.clone());
    let members = team_service.get_members(&team_id).await?;

    Ok(Json(members))
}
