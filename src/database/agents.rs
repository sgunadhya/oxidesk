use async_trait::async_trait;
use crate::api::middleware::error::{ApiError, ApiResult};
use crate::database::Database;
use crate::models::{
    ActivityEventType, Agent, AgentActivityLog, AgentAvailability, User, UserType,
};
use chrono;
use sqlx::Row;
use time;

#[async_trait]
impl AgentRepository for Database {
    // Agent operations
    async fn create_agent(&self, agent: &Agent) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO agents (id, user_id, first_name, last_name, password_hash)
             VALUES (?, ?, ?, ?, ?)",
        )
            .bind(&agent.id)
            .bind(&agent.user_id)
            .bind(&agent.first_name)
            .bind(&agent.last_name)
            .bind(&agent.password_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
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
    ) -> ApiResult<(String, String)> {
        let mut tx = self.pool.begin().await?;

        // Create user
        let user = User::new(email.to_string(), UserType::Agent);
        sqlx::query(
            "INSERT INTO users (id, email, user_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
            .bind(&user.id)
            .bind(&user.email)
            .bind("agent")
            .bind(&user.created_at)
            .bind(&user.updated_at)
            .execute(&mut *tx)
            .await?;

        // Create agent
        let agent = Agent::new(
            user.id.clone(),
            first_name.to_string(),
            last_name.map(|s| s.to_string()),
            password_hash.to_string(),
        );
        sqlx::query(
            "INSERT INTO agents (id, user_id, first_name, last_name, password_hash, availability_status)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
            .bind(&agent.id)
            .bind(&agent.user_id)
            .bind(&agent.first_name)
            .bind(&agent.last_name)
            .bind(&agent.password_hash)
            .bind("offline")
            .execute(&mut *tx)
            .await?;

        // Assign role
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id, created_at)
             VALUES (?, ?, ?)",
        )
            .bind(&user.id)
            .bind(role_id)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok((agent.id, user.id))
    }
    async fn get_agent_by_user_id(&self, user_id: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since,
                    api_key, api_secret_hash, api_key_description,
                    api_key_created_at, api_key_last_used_at, api_key_revoked_at
             FROM agents
             WHERE user_id = ?",
        )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let status_str: String = row
                .try_get("availability_status")
                .unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            Ok(Some(Agent {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: row.try_get("api_key").ok(),
                api_secret_hash: row.try_get("api_secret_hash").ok(),
                api_key_description: row.try_get("api_key_description").ok(),
                api_key_created_at: row.try_get("api_key_created_at").ok(),
                api_key_last_used_at: row.try_get("api_key_last_used_at").ok(),
                api_key_revoked_at: row.try_get("api_key_revoked_at").ok(),
            }))
        } else {
            Ok(None)
        }
    }
    // List agents with pagination (348-401 in view, originally 451)
    async fn list_agents(&self, limit: i64, offset: i64) -> ApiResult<Vec<(User, Agent)>> {
        let rows = sqlx::query(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at, u.deleted_at, u.deleted_by,
                    a.id as agent_id, a.user_id as agent_user_id, a.first_name, a.last_name, a.password_hash,
                    a.availability_status, a.last_login_at, a.last_activity_at, a.away_since
             FROM users u
             INNER JOIN agents a ON a.user_id = u.id
             WHERE u.user_type = 'agent' AND u.deleted_at IS NULL
             ORDER BY u.created_at DESC
             LIMIT ? OFFSET ?",
        )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let user = User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: UserType::Agent,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: row.try_get("deleted_at").ok(),
                deleted_by: row.try_get("deleted_by").ok(),
            };

            let status_str: String = row
                .try_get("availability_status")
                .unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            let agent = Agent {
                id: row.try_get("agent_id")?,
                user_id: row.try_get("agent_user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: None,
                api_secret_hash: None,
                api_key_description: None,
                api_key_created_at: None,
                api_key_last_used_at: None,
                api_key_revoked_at: None,
            };

            results.push((user, agent));
        }

        Ok(results)
    }
    // Count total agents
    async fn count_agents(&self) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM users
             WHERE user_type = 'agent' AND deleted_at IS NULL",
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("count")?)
    }
    // Count admin users (for last admin check)
    async fn count_admin_users(&self) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(DISTINCT ur.user_id) as count
             FROM user_roles ur
             INNER JOIN roles r ON r.id = ur.role_id
             WHERE r.name = 'Admin'",
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("count")?)
    }
    // Agent update operations
    async fn update_agent(&self, agent_id: &str, first_name: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE agents
             SET first_name = ?
             WHERE id = ?",
        )
            .bind(first_name)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_agent_password(
        &self,
        agent_id: &str,
        password_hash: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE agents
             SET password_hash = ?
             WHERE id = ?",
        )
            .bind(password_hash)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Update agent password hash by user_id (for password reset)
    async fn update_agent_password_by_user_id(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE agents
             SET password_hash = ?
             WHERE user_id = ?",
        )
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait]
pub trait AgentRepository:  Send + Sync {
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
    async fn update_agent_password(
        &self,
        agent_id: &str,
        password_hash: &str,
    ) -> ApiResult<()>;
    /// Update agent password hash by user_id (for password reset)
    async fn update_agent_password_by_user_id(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> ApiResult<()>;
}

impl Database {
    // ========================================
    // Agent Availability Operations (Feature 006)
    // ========================================

    pub async fn update_agent_availability(
        &self,
        user_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        sqlx::query("UPDATE agents SET availability_status = ? WHERE user_id = ?")
            .bind(status.to_string())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        tracing::info!("Agent {} availability updated to {}", user_id, status);
        Ok(())
    }

    pub async fn get_agent_availability(&self, user_id: &str) -> ApiResult<AgentAvailability> {
        let row = sqlx::query("SELECT availability_status FROM agents WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("availability_status")?;
            status_str
                .parse()
                .map_err(|e| ApiError::Internal(format!("Invalid availability status: {}", e)))
        } else {
            Err(ApiError::NotFound(format!(
                "Agent not found for user {}",
                user_id
            )))
        }
    }

    /// Update agent availability status with away_since logic
    pub async fn update_agent_availability_with_timestamp(
        &self,
        agent_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Set away_since when transitioning to away/away_manual, clear otherwise
        let away_since = match status {
            AgentAvailability::Away | AgentAvailability::AwayManual => Some(now.clone()),
            _ => None,
        };

        sqlx::query(
            "UPDATE agents
             SET availability_status = ?,
                 away_since = ?,
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(status.to_string())
        .bind(away_since)
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update agent's last_activity_at timestamp
    pub async fn update_agent_activity(&self, agent_id: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE agents
             SET last_activity_at = ?
             WHERE id = ?",
        )
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update agent's last_login_at timestamp
    pub async fn update_agent_last_login(&self, agent_id: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE agents
             SET last_login_at = ?
             WHERE id = ?",
        )
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agents who are online but inactive beyond threshold
    pub async fn get_inactive_online_agents(
        &self,
        inactivity_threshold_seconds: i64,
    ) -> ApiResult<Vec<Agent>> {
        let threshold_time =
            chrono::Utc::now() - chrono::Duration::seconds(inactivity_threshold_seconds);
        let threshold_str = threshold_time.to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since
             FROM agents
             WHERE availability_status = ?
               AND last_activity_at IS NOT NULL
               AND last_activity_at < ?",
        )
        .bind("online")
        .bind(threshold_str)
        .fetch_all(&self.pool)
        .await?;

        let agents = rows
            .into_iter()
            .map(|row| {
                let status_str: String = row
                    .try_get("availability_status")
                    .unwrap_or_else(|_| "offline".to_string());
                let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

                Ok(Agent {
                    id: row.try_get("id")?,
                    user_id: row.try_get("user_id")?,
                    first_name: row.try_get("first_name")?,
                    last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                    password_hash: row.try_get("password_hash")?,
                    availability_status: status,
                    last_login_at: row.try_get("last_login_at").ok(),
                    last_activity_at: row.try_get("last_activity_at").ok(),
                    away_since: row.try_get("away_since").ok(),
                    api_key: None,
                    api_secret_hash: None,
                    api_key_description: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    api_key_revoked_at: None,
                })
            })
            .collect::<ApiResult<Vec<Agent>>>()?;

        Ok(agents)
    }

    /// Get agents who are away/away_manual and idle beyond threshold
    pub async fn get_idle_away_agents(
        &self,
        max_idle_threshold_seconds: i64,
    ) -> ApiResult<Vec<Agent>> {
        let threshold_time =
            chrono::Utc::now() - chrono::Duration::seconds(max_idle_threshold_seconds);
        let threshold_str = threshold_time.to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since
             FROM agents
             WHERE availability_status IN (?, ?)
               AND away_since IS NOT NULL
               AND away_since < ?",
        )
        .bind("away")
        .bind("away_manual")
        .bind(threshold_str)
        .fetch_all(&self.pool)
        .await?;

        let agents = rows
            .into_iter()
            .map(|row| {
                let status_str: String = row
                    .try_get("availability_status")
                    .unwrap_or_else(|_| "offline".to_string());
                let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

                Ok(Agent {
                    id: row.try_get("id")?,
                    user_id: row.try_get("user_id")?,
                    first_name: row.try_get("first_name")?,
                    last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                    password_hash: row.try_get("password_hash")?,
                    availability_status: status,
                    last_login_at: row.try_get("last_login_at").ok(),
                    last_activity_at: row.try_get("last_activity_at").ok(),
                    away_since: row.try_get("away_since").ok(),
                    api_key: None,
                    api_secret_hash: None,
                    api_key_description: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    api_key_revoked_at: None,
                })
            })
            .collect::<ApiResult<Vec<Agent>>>()?;

        Ok(agents)
    }

    // ========================================
    // Agent Activity Log Operations
    // ========================================

    /// Create activity log entry
    pub async fn create_activity_log(&self, log: &AgentActivityLog) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO agent_activity_logs
             (id, agent_id, event_type, old_status, new_status, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.agent_id)
        .bind(log.event_type.to_string())
        .bind(&log.old_status)
        .bind(&log.new_status)
        .bind(&log.metadata)
        .bind(&log.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agent activity logs (paginated)
    pub async fn get_agent_activity_logs(
        &self,
        agent_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<AgentActivityLog>, i64)> {
        // Get total count
        let count_row =
            sqlx::query("SELECT COUNT(*) as count FROM agent_activity_logs WHERE agent_id = ?")
                .bind(agent_id)
                .fetch_one(&self.pool)
                .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get logs
        let rows = sqlx::query(
            "SELECT id, agent_id, event_type, old_status, new_status, metadata, created_at
             FROM agent_activity_logs
             WHERE agent_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(agent_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let logs = rows
            .into_iter()
            .map(|row| {
                let event_type_str: String = row.try_get("event_type")?;
                let event_type = event_type_str
                    .parse()
                    .unwrap_or(ActivityEventType::AvailabilityChanged);

                Ok(AgentActivityLog {
                    id: row.try_get("id")?,
                    agent_id: row.try_get("agent_id")?,
                    event_type,
                    old_status: row.try_get("old_status").ok(),
                    new_status: row.try_get("new_status").ok(),
                    metadata: row.try_get("metadata").ok(),
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect::<ApiResult<Vec<AgentActivityLog>>>()?;

        Ok((logs, total))
    }

    /// Get agent by ID (for API key operations)
    pub async fn get_agent_by_id(&self, agent_id: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since,
                    api_key, api_secret_hash, api_key_description,
                    api_key_created_at, api_key_last_used_at, api_key_revoked_at
             FROM agents
             WHERE id = ?",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row
                .try_get("availability_status")
                .unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            Ok(Some(Agent {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: row.try_get("api_key").ok(),
                api_secret_hash: row.try_get("api_secret_hash").ok(),
                api_key_description: row.try_get("api_key_description").ok(),
                api_key_created_at: row.try_get("api_key_created_at").ok(),
                api_key_last_used_at: row.try_get("api_key_last_used_at").ok(),
                api_key_revoked_at: row.try_get("api_key_revoked_at").ok(),
            }))
        } else {
            Ok(None)
        }
    }
}
