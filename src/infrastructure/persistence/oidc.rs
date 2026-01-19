use sqlx::Row;

use crate::{domain::entities::OidcProvider, infrastructure::http::middleware::error::{ApiError, ApiResult}, infrastructure::persistence::Database};

impl Database {
    pub async fn create_oidc_provider(&self, provider: &OidcProvider) -> ApiResult<()> {
        let scopes_json = serde_json::to_string(&provider.scopes)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "INSERT INTO oidc_providers (id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.issuer_url)
        .bind(&provider.client_id)
        .bind(&provider.client_secret)
        .bind(&provider.redirect_uri)
        .bind(&scopes_json)
        .bind(provider.enabled)
        .bind(&provider.created_at)
        .bind(&provider.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get OIDC provider by name
    pub async fn get_oidc_provider_by_name(&self, name: &str) -> ApiResult<Option<OidcProvider>> {
        let row = sqlx::query(
            "SELECT id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at
             FROM oidc_providers
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let scopes_json: String = row.try_get("scopes")?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_json)
                .map_err(|e| ApiError::Internal(format!("Failed to parse scopes: {}", e)))?;

            let enabled_val: i32 = row.try_get("enabled")?;
            let enabled = enabled_val != 0;

            Ok(Some(OidcProvider {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                issuer_url: row.try_get("issuer_url")?,
                client_id: row.try_get("client_id")?,
                client_secret: row.try_get("client_secret")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scopes,
                enabled,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List OIDC providers with optional enabled filter
    pub async fn list_oidc_providers(&self, enabled_only: bool) -> ApiResult<Vec<OidcProvider>> {
        let query = if enabled_only {
            sqlx::query(
                "SELECT id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at
                 FROM oidc_providers
                 WHERE enabled = 1
                 ORDER BY name",
            )
        } else {
            sqlx::query(
                "SELECT id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at
                 FROM oidc_providers
                 ORDER BY name",
            )
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut providers = Vec::new();
        for row in rows {
            let scopes_json: String = row.try_get("scopes")?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_json)
                .map_err(|e| ApiError::Internal(format!("Failed to parse scopes: {}", e)))?;

            let enabled_val: i32 = row.try_get("enabled")?;
            let enabled = enabled_val != 0;

            providers.push(OidcProvider {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                issuer_url: row.try_get("issuer_url")?,
                client_id: row.try_get("client_id")?,
                client_secret: row.try_get("client_secret")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scopes,
                enabled,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(providers)
    }

    /// Update an existing OIDC provider
    pub async fn update_oidc_provider(&self, provider: &OidcProvider) -> ApiResult<()> {
        let scopes_json = serde_json::to_string(&provider.scopes)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "UPDATE oidc_providers
             SET name = ?, issuer_url = ?, client_id = ?, client_secret = ?, redirect_uri = ?, scopes = ?, enabled = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&provider.name)
        .bind(&provider.issuer_url)
        .bind(&provider.client_id)
        .bind(&provider.client_secret)
        .bind(&provider.redirect_uri)
        .bind(&scopes_json)
        .bind(provider.enabled)
        .bind(&provider.updated_at)
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete an OIDC provider
    pub async fn delete_oidc_provider(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM oidc_providers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Toggle OIDC provider enabled status
    pub async fn toggle_oidc_provider(&self, id: &str) -> ApiResult<bool> {
        // First get current status
        let row = sqlx::query("SELECT enabled FROM oidc_providers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("OIDC provider {} not found", id)))?;

        let enabled_val: i32 = row.try_get("enabled")?;
        let current_enabled = enabled_val != 0;
        let new_enabled = !current_enabled;

        // Update to opposite
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query("UPDATE oidc_providers SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(new_enabled)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(new_enabled)
    }

    /// Store OIDC state for OAuth2 flow
    pub async fn create_oidc_state(&self, oidc_state: &crate::domain::entities::OidcState) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO oidc_states (state, provider_name, nonce, pkce_verifier, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&oidc_state.state)
        .bind(&oidc_state.provider_name)
        .bind(&oidc_state.nonce)
        .bind(&oidc_state.pkce_verifier)
        .bind(&oidc_state.created_at)
        .bind(&oidc_state.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieve and delete OIDC state (one-time use for security)
    pub async fn consume_oidc_state(
        &self,
        state: &str,
    ) -> ApiResult<Option<crate::domain::entities::OidcState>> {
        let oidc_state = self.get_oidc_state(state).await?;

        if oidc_state.is_some() {
            // Delete the state immediately to prevent replay attacks
            self.delete_oidc_state(state).await?;
        }

        Ok(oidc_state)
    }

    /// Get OIDC state by state parameter
    async fn get_oidc_state(&self, state: &str) -> ApiResult<Option<crate::domain::entities::OidcState>> {
        let row = sqlx::query(
            "SELECT state, provider_name, nonce, pkce_verifier, created_at, expires_at
             FROM oidc_states WHERE state = ?",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| crate::domain::entities::OidcState {
            state: r.get("state"),
            provider_name: r.get("provider_name"),
            nonce: r.get("nonce"),
            pkce_verifier: r.get("pkce_verifier"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
        }))
    }

    /// Delete OIDC state
    async fn delete_oidc_state(&self, state: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM oidc_states WHERE state = ?")
            .bind(state)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clean up expired OIDC states
    pub async fn cleanup_expired_oidc_states(&self) -> ApiResult<u64> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query("DELETE FROM oidc_states WHERE expires_at < ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
