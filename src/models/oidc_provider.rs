use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProvider {
    pub id: String,
    pub name: String,
    pub issuer_url: String,
    pub client_id: String,
    #[serde(skip_serializing)]  // Never send client_secret to clients
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOidcProviderRequest {
    pub name: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl CreateOidcProviderRequest {
    pub fn validate(&self) -> Result<(), crate::api::middleware::ApiError> {
        use crate::api::middleware::ApiError;

        if self.name.trim().is_empty() {
            return Err(ApiError::BadRequest("Provider name cannot be empty".to_string()));
        }

        if self.name.len() > 100 {
            return Err(ApiError::BadRequest("Provider name cannot exceed 100 characters".to_string()));
        }

        if self.issuer_url.trim().is_empty() {
            return Err(ApiError::BadRequest("Issuer URL cannot be empty".to_string()));
        }

        if !self.issuer_url.starts_with("https://") {
            return Err(ApiError::BadRequest("Issuer URL must use HTTPS".to_string()));
        }

        if self.client_id.trim().is_empty() {
            return Err(ApiError::BadRequest("Client ID cannot be empty".to_string()));
        }

        if self.client_secret.trim().is_empty() {
            return Err(ApiError::BadRequest("Client secret cannot be empty".to_string()));
        }

        if self.redirect_uri.trim().is_empty() {
            return Err(ApiError::BadRequest("Redirect URI cannot be empty".to_string()));
        }

        if !self.redirect_uri.starts_with("https://") && !self.redirect_uri.starts_with("http://localhost") {
            return Err(ApiError::BadRequest("Redirect URI must use HTTPS (or http://localhost for development)".to_string()));
        }

        if self.scopes.is_empty() {
            return Err(ApiError::BadRequest("At least one scope must be specified".to_string()));
        }

        if !self.scopes.contains(&"openid".to_string()) {
            return Err(ApiError::BadRequest("Scopes must include 'openid' for OIDC".to_string()));
        }

        if !self.scopes.contains(&"email".to_string()) {
            return Err(ApiError::BadRequest("Scopes must include 'email' to identify users".to_string()));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateOidcProviderRequest {
    pub name: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct OidcProviderResponse {
    pub id: String,
    pub name: String,
    pub issuer_url: String,
    pub client_id: String,
    // client_secret is never included in responses for security
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_enabled() -> bool {
    true
}

impl OidcProvider {
    pub fn new(
        name: String,
        issuer_url: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        scopes: Vec<String>,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            issuer_url,
            client_id,
            client_secret,
            redirect_uri,
            scopes,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }

        if self.name.len() > 100 {
            return Err("Provider name cannot exceed 100 characters".to_string());
        }

        if self.issuer_url.trim().is_empty() {
            return Err("Issuer URL cannot be empty".to_string());
        }

        if !self.issuer_url.starts_with("https://") {
            return Err("Issuer URL must use HTTPS".to_string());
        }

        if self.client_id.trim().is_empty() {
            return Err("Client ID cannot be empty".to_string());
        }

        if self.client_secret.trim().is_empty() {
            return Err("Client secret cannot be empty".to_string());
        }

        if self.redirect_uri.trim().is_empty() {
            return Err("Redirect URI cannot be empty".to_string());
        }

        if !self.redirect_uri.starts_with("https://") && !self.redirect_uri.starts_with("http://localhost") {
            return Err("Redirect URI must use HTTPS (or http://localhost for development)".to_string());
        }

        if self.scopes.is_empty() {
            return Err("At least one scope must be specified".to_string());
        }

        if !self.scopes.contains(&"openid".to_string()) {
            return Err("Scopes must include 'openid' for OIDC".to_string());
        }

        if !self.scopes.contains(&"email".to_string()) {
            return Err("Scopes must include 'email' to identify users".to_string());
        }

        Ok(())
    }

    pub fn touch(&mut self) {
        self.updated_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
    }

    pub fn from_request(request: CreateOidcProviderRequest) -> Self {
        Self::new(
            request.name,
            request.issuer_url,
            request.client_id,
            request.client_secret,
            request.redirect_uri,
            request.scopes,
        )
    }

    pub fn update_from_request(&mut self, request: UpdateOidcProviderRequest) -> Result<(), String> {
        if let Some(name) = request.name {
            self.name = name;
        }
        if let Some(issuer_url) = request.issuer_url {
            self.issuer_url = issuer_url;
        }
        if let Some(client_id) = request.client_id {
            self.client_id = client_id;
        }
        if let Some(client_secret) = request.client_secret {
            self.client_secret = client_secret;
        }
        if let Some(redirect_uri) = request.redirect_uri {
            self.redirect_uri = redirect_uri;
        }
        if let Some(scopes) = request.scopes {
            self.scopes = scopes;
        }
        if let Some(enabled) = request.enabled {
            self.enabled = enabled;
        }

        self.touch();
        self.validate()?;

        Ok(())
    }
}

impl From<OidcProvider> for OidcProviderResponse {
    fn from(provider: OidcProvider) -> Self {
        Self {
            id: provider.id,
            name: provider.name,
            issuer_url: provider.issuer_url,
            client_id: provider.client_id,
            redirect_uri: provider.redirect_uri,
            scopes: provider.scopes,
            enabled: provider.enabled,
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}
