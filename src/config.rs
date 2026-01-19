use std::env;

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
}
