/// OIDC Repository
///
/// Repository pattern for OIDC provider and state operations.
/// Encapsulates all database access for OIDC functionality.
use crate::{
    infrastructure::http::middleware::error::ApiResult,
    infrastructure::persistence::Database,
    domain::entities::{OidcProvider, OidcState},
};

/// Repository for OIDC provider and state operations
#[derive(Clone)]
pub struct OidcRepository {
    db: Database,
}

impl OidcRepository {
    /// Create a new OIDC repository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // Provider operations

    /// Create a new OIDC provider
    pub async fn create_provider(&self, provider: &OidcProvider) -> ApiResult<()> {
        self.db.create_oidc_provider(provider).await
    }

    /// Get OIDC provider by name
    pub async fn get_provider_by_name(&self, name: &str) -> ApiResult<Option<OidcProvider>> {
        self.db.get_oidc_provider_by_name(name).await
    }

    /// List OIDC providers with optional enabled filter
    pub async fn list_providers(&self, enabled_only: bool) -> ApiResult<Vec<OidcProvider>> {
        self.db.list_oidc_providers(enabled_only).await
    }

    /// Update an existing OIDC provider
    pub async fn update_provider(&self, provider: &OidcProvider) -> ApiResult<()> {
        self.db.update_oidc_provider(provider).await
    }

    /// Delete an OIDC provider
    pub async fn delete_provider(&self, id: &str) -> ApiResult<()> {
        self.db.delete_oidc_provider(id).await
    }

    /// Toggle OIDC provider enabled status
    pub async fn toggle_provider(&self, id: &str) -> ApiResult<bool> {
        self.db.toggle_oidc_provider(id).await
    }

    /// Check if a provider with the given name exists
    pub async fn provider_exists(&self, name: &str) -> ApiResult<bool> {
        Ok(self.get_provider_by_name(name).await?.is_some())
    }

    // State operations

    /// Store OIDC state for OAuth2 flow
    pub async fn create_state(&self, state: &OidcState) -> ApiResult<()> {
        self.db.create_oidc_state(state).await
    }

    /// Retrieve and delete OIDC state (one-time use for security)
    pub async fn consume_state(&self, state: &str) -> ApiResult<Option<OidcState>> {
        self.db.consume_oidc_state(state).await
    }

    /// Clean up expired OIDC states
    pub async fn cleanup_expired_states(&self) -> ApiResult<u64> {
        self.db.cleanup_expired_oidc_states().await
    }
}
