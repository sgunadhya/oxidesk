use std::env;

/// Authentication mode configuration
#[derive(Clone, Debug, PartialEq)]
pub enum AuthMode {
    /// Database-backed sessions (stateful)
    Database,
    /// JWT tokens (stateless, Workers-compatible)
    Jwt,
}

impl AuthMode {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, ConfigError> {
        match s.to_lowercase().as_str() {
            "database" => Ok(AuthMode::Database),
            "jwt" => Ok(AuthMode::Jwt),
            _ => Err(ConfigError::InvalidAuthMode(s.to_string())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub admin_email: String,
    pub admin_password: String,
    pub session_duration_hours: i64,
    pub otel_exporter_endpoint: Option<String>,
    pub service_name: String,
    pub metrics_port: u16,
    pub auth_mode: AuthMode,
    pub jwt_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://oxidesk.db?mode=rwc".to_string());

        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidPort)?;

        let admin_email = env::var("ADMIN_EMAIL").map_err(|_| ConfigError::MissingAdminEmail)?;

        let admin_password =
            env::var("ADMIN_PASSWORD").map_err(|_| ConfigError::MissingAdminPassword)?;

        let session_duration_hours = env::var("SESSION_DURATION_HOURS")
            .unwrap_or_else(|_| "9".to_string())
            .parse()
            .unwrap_or(9);

        let otel_exporter_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

        let service_name = env::var("SERVICE_NAME").unwrap_or_else(|_| "oxidesk".to_string());

        let metrics_port = env::var("METRICS_PORT")
            .unwrap_or_else(|_| "9000".to_string())
            .parse()
            .unwrap_or(9000);

        // Parse auth mode (default to Database for backward compatibility)
        let auth_mode = env::var("AUTH_MODE")
            .ok()
            .map(|s| AuthMode::from_str(&s))
            .transpose()?
            .unwrap_or(AuthMode::Database);

        // Load JWT secret (required if auth_mode is JWT)
        let jwt_secret = env::var("JWT_SECRET").ok();
        if auth_mode == AuthMode::Jwt && jwt_secret.is_none() {
            return Err(ConfigError::MissingJwtSecret);
        }

        // Validate JWT secret length if provided
        if let Some(ref secret) = jwt_secret {
            if secret.len() < 32 {
                return Err(ConfigError::JwtSecretTooShort);
            }
        }

        Ok(Config {
            database_url,
            server_host,
            server_port,
            admin_email,
            admin_password,
            session_duration_hours,
            otel_exporter_endpoint,
            service_name,
            metrics_port,
            auth_mode,
            jwt_secret,
        })
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("ADMIN_EMAIL environment variable not set")]
    MissingAdminEmail,

    #[error("ADMIN_PASSWORD environment variable not set")]
    MissingAdminPassword,

    #[error("Invalid port number")]
    InvalidPort,

    #[error("Invalid AUTH_MODE value: {0} (must be 'database' or 'jwt')")]
    InvalidAuthMode(String),

    #[error("JWT_SECRET environment variable required when AUTH_MODE=jwt")]
    MissingJwtSecret,

    #[error("JWT_SECRET must be at least 32 characters for security")]
    JwtSecretTooShort,
}
