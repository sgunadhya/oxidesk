use crate::api::middleware::error::{ApiError, ApiResult};
use crate::database::Database;
use crate::models::{Team, TeamMemberRole, TeamMembership, User, UserType};
use sqlx::Row;

impl Database {
    // ========== Team Operations (T021-T023) ==========

    pub async fn create_team(&self, team: &Team) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO teams (id, name, description, sla_policy_id, business_hours, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&team.id)
        .bind(&team.name)
        .bind(&team.description)
        .bind(&team.sla_policy_id)
        .bind(&team.business_hours)
        .bind(&team.created_at)
        .bind(&team.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                ApiError::BadRequest(format!("Team with name '{}' already exists", team.name))
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

        tracing::info!("Team created: id={}, name={}", team.id, team.name);
        Ok(())
    }

    pub async fn get_team_by_id(&self, id: &str) -> ApiResult<Option<Team>> {
        let row = sqlx::query(
            "SELECT id, name, description, sla_policy_id, business_hours, created_at, updated_at
             FROM teams WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Team {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                sla_policy_id: row.try_get("sla_policy_id").ok(),
                business_hours: row.try_get("business_hours").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_teams(&self) -> ApiResult<Vec<Team>> {
        let rows = sqlx::query(
            "SELECT id, name, description, sla_policy_id, business_hours, created_at, updated_at
             FROM teams ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut teams = Vec::new();
        for row in rows {
            teams.push(Team {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                sla_policy_id: row.try_get("sla_policy_id").ok(),
                business_hours: row.try_get("business_hours").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(teams)
    }

    // ========== Team Membership Operations (T024-T028) ==========

    pub async fn add_team_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: TeamMemberRole,
    ) -> ApiResult<()> {
        let membership = TeamMembership::new(team_id.to_string(), user_id.to_string(), role);

        sqlx::query(
            "INSERT INTO team_memberships (id, team_id, user_id, role, joined_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&membership.id)
        .bind(&membership.team_id)
        .bind(&membership.user_id)
        .bind(membership.role.to_string())
        .bind(&membership.joined_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                ApiError::BadRequest("User is already a member of this team".to_string())
            } else if e.to_string().contains("FOREIGN KEY") {
                ApiError::NotFound("Team or user not found".to_string())
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

        tracing::info!(
            "Team member added: team={}, user={}, role={}",
            team_id,
            user_id,
            role
        );
        Ok(())
    }

    pub async fn remove_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM team_memberships WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        tracing::info!("Team member removed: team={}, user={}", team_id, user_id);
        Ok(())
    }

    pub async fn get_team_members(&self, team_id: &str) -> ApiResult<Vec<User>> {
        let rows = sqlx::query(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at
             FROM users u
             INNER JOIN team_memberships tm ON u.id = tm.user_id
             WHERE tm.team_id = ?
             ORDER BY u.email ASC",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: match user_type_str.as_str() {
                    "agent" => UserType::Agent,
                    "contact" => UserType::Contact,
                    _ => UserType::Agent,
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: None,
                deleted_by: None,
            });
        }

        Ok(users)
    }

    pub async fn is_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM team_memberships WHERE team_id = ? AND user_id = ?",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count > 0)
    }

    pub async fn get_user_teams(&self, user_id: &str) -> ApiResult<Vec<Team>> {
        let rows = sqlx::query(
            "SELECT t.id, t.name, t.description, t.sla_policy_id, t.business_hours, t.created_at, t.updated_at
             FROM teams t
             INNER JOIN team_memberships tm ON t.id = tm.team_id
             WHERE tm.user_id = ?
             ORDER BY t.name ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut teams = Vec::new();
        for row in rows {
            teams.push(Team {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                sla_policy_id: row.try_get("sla_policy_id").ok(),
                business_hours: row.try_get("business_hours").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(teams)
    }

    /// Update team's SLA policy
    pub async fn update_team_sla_policy(
        &self,
        team_id: &str,
        sla_policy_id: Option<&str>,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query("UPDATE teams SET sla_policy_id = ?, updated_at = ? WHERE id = ?")
            .bind(sla_policy_id)
            .bind(now)
            .bind(team_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
