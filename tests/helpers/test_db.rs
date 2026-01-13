use oxidesk::database::Database;
use std::fs;
use std::path::PathBuf;

pub struct TestDatabase {
    pub db: Database,
    db_file: PathBuf,
}

impl TestDatabase {
    pub fn db(&self) -> &Database {
        &self.db
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // Clean up database files when TestDatabase is dropped
        let _ = fs::remove_file(&self.db_file);

        // Also remove journal file if it exists
        let mut journal_file = self.db_file.clone();
        journal_file.set_extension("db-journal");
        let _ = fs::remove_file(&journal_file);

        // Remove WAL files if they exist
        let mut wal_file = self.db_file.clone();
        wal_file.set_extension("db-wal");
        let _ = fs::remove_file(&wal_file);

        let mut shm_file = self.db_file.clone();
        shm_file.set_extension("db-shm");
        let _ = fs::remove_file(&shm_file);
    }
}

pub async fn setup_test_db() -> TestDatabase {
    // Install drivers for AnyPool (required for tests)
    sqlx::any::install_default_drivers();

    // Use file-based SQLite for tests (unique UUID per test for parallel execution)
    use uuid::Uuid;
    let temp_file = format!("test_{}.db", Uuid::new_v4());
    let db_file = PathBuf::from(&temp_file);

    // Use file:// URL scheme for proper SQLite URL format
    let db_url = format!("sqlite://{}?mode=rwc", temp_file);

    let db = Database::connect(&db_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations from migrations/sqlite directory
    run_migrations(&db).await;

    TestDatabase { db, db_file }
}

async fn run_migrations(db: &Database) {
    let pool = db.pool();

    // Run all SQLite migrations
    sqlx::migrate!("./migrations/sqlite")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}

/// Teardown test database - cleans up SQLite database files
pub async fn teardown_test_db(test_db: TestDatabase) {
    // Drop will handle cleanup automatically
    drop(test_db);
}
