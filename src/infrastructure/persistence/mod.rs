use sqlx::{any::AnyPoolOptions, AnyPool};

pub mod agents;
pub mod api_key;
pub mod auth_event;
pub mod automation_rules;
mod automation;
mod contacts;
mod conversations;
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

        let pool = AnyPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .connect(database_url)
            .await?;

        // Enable foreign keys for SQLite
        if database_url.starts_with("sqlite") {
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
