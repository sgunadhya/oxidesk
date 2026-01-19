use sqlx::{
    any::{AnyConnectOptions, AnyPoolOptions},
    AnyPool, ConnectOptions,
};
use std::str::FromStr;
use tracing::log::LevelFilter;

pub mod agents;
pub mod api_key;
pub mod auth_event;
mod automation;
pub mod automation_rules;
mod contacts;
mod conversations;
pub mod distributed_lock;
mod email;
mod holiday;
mod inboxes;
mod macros;
mod messages;
mod notification;
mod oidc;
mod password_reset;
mod roles;
mod sessions;
mod sla;
mod system_config;
mod tags;
mod teams;
pub mod templates;
mod users;
mod webhook;
pub struct Database {
    pub(crate) pool: AnyPool,
}

#[cfg(test)]
impl Database {
    pub fn new_mock() -> Self {
        use sqlx::any::AnyPoolOptions;
        let pool = AnyPoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .expect("Failed to create lazy pool");
        Self { pool }
    }
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        // Ensure drivers are installed for AnyPool
        sqlx::any::install_default_drivers();

        let mut connect_options = AnyConnectOptions::from_str(database_url)?;

        // Configure logging
        connect_options = connect_options
            .log_statements(LevelFilter::Info)
            .log_slow_statements(LevelFilter::Warn, std::time::Duration::from_secs(1));

        tracing::info!("Database connection options configured with LevelFilter::Info");

        let pool = AnyPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .connect_with(connect_options)
            .await?;

        // Enable optimizations for SQLite
        if database_url.starts_with("sqlite") {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await?;
            sqlx::query("PRAGMA busy_timeout = 5000")
                .execute(&pool)
                .await?;
            sqlx::query("PRAGMA synchronous = NORMAL")
                .execute(&pool)
                .await?;
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&pool)
                .await?;
        }

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("migrations/sqlite").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
