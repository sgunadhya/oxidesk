use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use sqlx::Row;

impl Database {
    // ========================================
    // System Configuration Operations
    // ========================================

    /// Get configuration value by key
    pub async fn get_config_value(&self, key: &str) -> ApiResult<Option<String>> {
        let row = sqlx::query("SELECT value FROM system_config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(row.try_get("value")?))
        } else {
            Ok(None)
        }
    }

    /// Set configuration value
    pub async fn set_config_value(
        &self,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO system_config (key, value, description, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET
                 value = excluded.value,
                 description = COALESCE(excluded.description, description),
                 updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(description)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
