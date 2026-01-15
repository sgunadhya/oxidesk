use async_trait::async_trait;
use sqlx::Row;

use crate::{Agent, AgentAvailability, ApiResult, Database};

#[async_trait]
impl ApiKeyRepository for Database {
    async fn create_api_key(
        &self,
        agent_id: &str,
        api_key: &str,
        api_secret_hash: &str,
        description: &str,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE agents
             SET api_key = ?,
                 api_secret_hash = ?,
                 api_key_description = ?,
                 api_key_created_at = ?,
                 api_key_last_used_at = NULL,
                 api_key_revoked_at = NULL
             WHERE id = ?",
        )
            .bind(api_key)
            .bind(api_secret_hash)
            .bind(description)
            .bind(&now)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Get agent by API key (for authentication)
    async fn get_agent_by_api_key(&self, api_key: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since,
                    api_key, api_secret_hash, api_key_description,
                    api_key_created_at, api_key_last_used_at, api_key_revoked_at
             FROM agents
             WHERE api_key = ? AND api_key IS NOT NULL",
        )
            .bind(api_key)
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
    /// Update API key last used timestamp
    async fn update_api_key_last_used(&self, api_key: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE agents
             SET api_key_last_used_at = ?
             WHERE api_key = ?",
        )
            .bind(&now)
            .bind(api_key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
    /// Revoke API key (soft delete with NULL fields)
    async fn revoke_api_key(&self, agent_id: &str) -> ApiResult<bool> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query(
            "UPDATE agents
             SET api_key = NULL,
                 api_secret_hash = NULL,
                 api_key_revoked_at = ?
             WHERE id = ? AND api_key IS NOT NULL",
        )
            .bind(&now)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
    /// List all active API keys with pagination and sorting
    async fn list_api_keys(
        &self,
        limit: i64,
        offset: i64,
        sort_by: &str,
        sort_order: &str,
    ) -> ApiResult<Vec<(String, String, String, String, Option<String>)>> {
        let order_clause = match sort_by {
            "last_used_at" => format!("a.api_key_last_used_at {}", sort_order),
            "description" => format!("a.api_key_description {}", sort_order),
            _ => format!("a.api_key_created_at {}", sort_order),
        };

        let query = format!(
            "SELECT a.id as agent_id, a.api_key, a.api_key_description,
                    a.api_key_created_at, a.api_key_last_used_at
             FROM agents a
             WHERE a.api_key IS NOT NULL
             ORDER BY {}
             LIMIT ? OFFSET ?",
            order_clause
        );

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            results.push((
                row.try_get("agent_id")?,
                row.try_get("api_key")?,
                row.try_get("api_key_description")?,
                row.try_get("api_key_created_at")?,
                row.try_get("api_key_last_used_at").ok(),
            ));
        }

        Ok(results)
    }
    /// Count active API keys
    async fn count_api_keys(&self) -> ApiResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM agents WHERE api_key IS NOT NULL")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("count")?)
    }
}

#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn create_api_key(
        &self,
        agent_id: &str,
        api_key: &str,
        api_secret_hash: &str,
        description: &str,
    ) -> ApiResult<()>;
    /// Get agent by API key (for authentication)
    async fn get_agent_by_api_key(&self, api_key: &str) -> ApiResult<Option<Agent>>;
    /// Update API key last used timestamp
    async fn update_api_key_last_used(&self, api_key: &str) -> ApiResult<()>;
    /// Revoke API key (soft delete with NULL fields)
    async fn revoke_api_key(&self, agent_id: &str) -> ApiResult<bool>;
    /// List all active API keys with pagination and sorting
    async fn list_api_keys(
        &self,
        limit: i64,
        offset: i64,
        sort_by: &str,
        sort_order: &str,
    ) -> ApiResult<Vec<(String, String, String, String, Option<String>)>>;
    /// Count active API keys
    async fn count_api_keys(&self) -> ApiResult<i64>;
}
