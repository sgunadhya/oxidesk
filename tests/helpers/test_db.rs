use oxidesk::database::Database;
use std::env;

pub async fn setup_test_db() -> Database {
    // Install drivers for AnyPool (required for tests)
    sqlx::any::install_default_drivers();

    // Use file-based SQLite for tests (unique UUID per test for parallel execution)
    use uuid::Uuid;
    let temp_file = format!("test_{}.db", Uuid::new_v4());
    // Use file:// URL scheme for proper SQLite URL format
    let db_url = format!("sqlite://{}?mode=rwc", temp_file);

    let db = Database::connect(&db_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations manually
    setup_schema(&db).await;
    seed_test_data(&db).await;

    db
}

async fn setup_schema(db: &Database) {
    let pool = db.pool();

    // Create users table
    sqlx::query(
        "CREATE TABLE users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            user_type TEXT NOT NULL CHECK(user_type IN ('agent', 'contact')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(email, user_type)
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create users table");

    sqlx::query("CREATE INDEX idx_users_email ON users(email)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_users_type ON users(user_type)")
        .execute(pool)
        .await
        .ok();

    // Create agents table
    sqlx::query(
        "CREATE TABLE agents (
            id TEXT PRIMARY KEY,
            user_id TEXT UNIQUE NOT NULL,
            first_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create agents table");

    // Create contacts table
    sqlx::query(
        "CREATE TABLE contacts (
            id TEXT PRIMARY KEY,
            user_id TEXT UNIQUE NOT NULL,
            first_name TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create contacts table");

    // Create roles table
    sqlx::query(
        "CREATE TABLE roles (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create roles table");

    // Create permissions table
    sqlx::query(
        "CREATE TABLE permissions (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create permissions table");

    // Create user_roles table
    sqlx::query(
        "CREATE TABLE user_roles (
            user_id TEXT NOT NULL,
            role_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (user_id, role_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create user_roles table");

    // Create role_permissions table
    sqlx::query(
        "CREATE TABLE role_permissions (
            role_id TEXT NOT NULL,
            permission_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (role_id, permission_id),
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
            FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create role_permissions table");

    // Create sessions table
    sqlx::query(
        "CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create sessions table");

    // Create contact_channels table
    sqlx::query(
        "CREATE TABLE contact_channels (
            id TEXT PRIMARY KEY,
            contact_id TEXT NOT NULL,
            inbox_id TEXT NOT NULL,
            email TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE,
            UNIQUE(contact_id, inbox_id)
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create contact_channels table");
}

async fn seed_test_data(db: &Database) {
    let pool = db.pool();

    // Insert default roles
    sqlx::query(
        "INSERT INTO roles (id, name, description, created_at, updated_at) VALUES
        ('00000000-0000-0000-0000-000000000001', 'Admin', 'Full system access', datetime('now'), datetime('now')),
        ('00000000-0000-0000-0000-000000000002', 'Agent', 'Standard support agent', datetime('now'), datetime('now'))"
    )
    .execute(pool)
    .await
    .expect("Failed to seed roles");

    // Insert permissions
    sqlx::query(
        "INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
        ('10000000-0000-0000-0000-000000000001', 'users:read', 'View users', datetime('now'), datetime('now')),
        ('10000000-0000-0000-0000-000000000002', 'users:create', 'Create users', datetime('now'), datetime('now')),
        ('11000000-0000-0000-0000-000000000001', 'agents:read', 'View agents', datetime('now'), datetime('now')),
        ('11000000-0000-0000-0000-000000000002', 'agents:create', 'Create agents', datetime('now'), datetime('now'))"
    )
    .execute(pool)
    .await
    .expect("Failed to seed permissions");

    // Assign all permissions to Admin role
    sqlx::query(
        "INSERT INTO role_permissions (role_id, permission_id, created_at)
         SELECT '00000000-0000-0000-0000-000000000001', id, datetime('now')
         FROM permissions"
    )
    .execute(pool)
    .await
    .expect("Failed to assign permissions to Admin role");
}

pub async fn teardown_test_db(db: Database) {
    // Close the connection
    drop(db);
    // Note: Test database files will be cleaned up manually or by .gitignore
}
