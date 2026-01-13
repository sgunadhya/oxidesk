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

    // Create agents table with availability_status and timestamps
    sqlx::query(
        "CREATE TABLE agents (
            id TEXT PRIMARY KEY,
            user_id TEXT UNIQUE NOT NULL,
            first_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            availability_status TEXT NOT NULL DEFAULT 'offline' CHECK(availability_status IN ('offline', 'online', 'away', 'away_manual', 'away_and_reassigning')),
            last_login_at TEXT,
            last_activity_at TEXT,
            away_since TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create agents table");

    sqlx::query("CREATE INDEX idx_agents_availability ON agents(availability_status)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_agents_last_activity ON agents(last_activity_at)")
        .execute(pool)
        .await
        .ok();

    // Create agent_activity_logs table
    sqlx::query(
        "CREATE TABLE agent_activity_logs (
            id TEXT PRIMARY KEY NOT NULL,
            agent_id TEXT NOT NULL,
            event_type TEXT NOT NULL CHECK(event_type IN ('agent_login', 'agent_logout', 'availability_changed')),
            old_status TEXT,
            new_status TEXT,
            metadata TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create agent_activity_logs table");

    sqlx::query("CREATE INDEX idx_activity_logs_agent ON agent_activity_logs(agent_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_activity_logs_created ON agent_activity_logs(created_at DESC)")
        .execute(pool)
        .await
        .ok();

    // Create system_config table
    sqlx::query(
        "CREATE TABLE system_config (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL,
            description TEXT,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create system_config table");

    // Seed availability configuration
    sqlx::query(
        "INSERT INTO system_config (key, value, description, updated_at) VALUES
        ('availability.inactivity_timeout_seconds', '300', 'Time in seconds before online agent goes away due to inactivity', datetime('now')),
        ('availability.max_idle_threshold_seconds', '1800', 'Time in seconds before away agent is reassigned', datetime('now'))"
    )
    .execute(pool)
    .await
    .expect("Failed to seed availability config");

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

    // Create inboxes table (minimal for testing)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS inboxes (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create inboxes table");

    // Create teams table
    sqlx::query(
        "CREATE TABLE teams (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            sla_policy_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create teams table");

    sqlx::query("CREATE INDEX idx_teams_name ON teams(name)")
        .execute(pool)
        .await
        .ok();

    // Create team_memberships table
    sqlx::query(
        "CREATE TABLE team_memberships (
            id TEXT PRIMARY KEY NOT NULL,
            team_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('member', 'lead')),
            joined_at TEXT NOT NULL,
            FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            UNIQUE(team_id, user_id)
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create team_memberships table");

    sqlx::query("CREATE INDEX idx_team_memberships_team ON team_memberships(team_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_team_memberships_user ON team_memberships(user_id)")
        .execute(pool)
        .await
        .ok();

    // Create conversations table with assignment fields
    sqlx::query(
        "CREATE TABLE conversations (
            id TEXT PRIMARY KEY NOT NULL,
            reference_number INTEGER NOT NULL UNIQUE,
            status TEXT NOT NULL CHECK(status IN ('open', 'snoozed', 'resolved', 'closed')) DEFAULT 'open',
            inbox_id TEXT NOT NULL,
            contact_id TEXT NOT NULL,
            subject TEXT,
            resolved_at TEXT,
            snoozed_until TEXT,
            assigned_user_id TEXT,
            assigned_team_id TEXT,
            assigned_at TEXT,
            assigned_by TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE RESTRICT,
            FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE RESTRICT
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create conversations table");

    sqlx::query("CREATE INDEX idx_conversations_assigned_user ON conversations(assigned_user_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_conversations_assigned_team ON conversations(assigned_team_id)")
        .execute(pool)
        .await
        .ok();

    // Create trigger for auto-incrementing reference_number starting from 100
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS set_conversation_reference_number
         AFTER INSERT ON conversations
         WHEN NEW.reference_number IS NULL
         BEGIN
             UPDATE conversations
             SET reference_number = (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations WHERE id != NEW.id)
             WHERE id = NEW.id;
         END"
    )
    .execute(pool)
    .await
    .expect("Failed to create conversations reference_number trigger");

    // Create messages table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            type TEXT NOT NULL CHECK (type IN ('incoming', 'outgoing')),
            status TEXT NOT NULL CHECK (status IN ('received', 'pending', 'sent', 'failed')),
            content TEXT NOT NULL,
            author_id TEXT NOT NULL,
            is_immutable INTEGER NOT NULL DEFAULT 0,
            retry_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            sent_at TEXT,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create messages table");

    // Create indexes for messages
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_status ON messages(status)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(type)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC)")
        .execute(pool)
        .await
        .ok();

    // Add message-related fields to conversations table
    sqlx::query("ALTER TABLE conversations ADD COLUMN last_message_id TEXT")
        .execute(pool)
        .await
        .ok();

    sqlx::query("ALTER TABLE conversations ADD COLUMN last_message_at TEXT")
        .execute(pool)
        .await
        .ok();

    sqlx::query("ALTER TABLE conversations ADD COLUMN last_reply_at TEXT")
        .execute(pool)
        .await
        .ok();

    // Create conversation_participants table
    sqlx::query(
        "CREATE TABLE conversation_participants (
            id TEXT PRIMARY KEY NOT NULL,
            conversation_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            added_at TEXT NOT NULL,
            added_by TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            UNIQUE(conversation_id, user_id)
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create conversation_participants table");

    sqlx::query("CREATE INDEX idx_participants_conversation ON conversation_participants(conversation_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_participants_user ON conversation_participants(user_id)")
        .execute(pool)
        .await
        .ok();

    // Create assignment_history table
    sqlx::query(
        "CREATE TABLE assignment_history (
            id TEXT PRIMARY KEY NOT NULL,
            conversation_id TEXT NOT NULL,
            assigned_user_id TEXT,
            assigned_team_id TEXT,
            assigned_by TEXT NOT NULL,
            assigned_at TEXT NOT NULL,
            unassigned_at TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (assigned_by) REFERENCES users(id) ON DELETE SET NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create assignment_history table");

    sqlx::query("CREATE INDEX idx_assignment_history_conversation ON assignment_history(conversation_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_assignment_history_user ON assignment_history(assigned_user_id)")
        .execute(pool)
        .await
        .ok();

    // Create tags table
    sqlx::query(
        "CREATE TABLE tags (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT UNIQUE NOT NULL,
            description TEXT,
            color TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create tags table");

    sqlx::query("CREATE INDEX idx_tags_name ON tags(name)")
        .execute(pool)
        .await
        .ok();

    // Create conversation_tags table
    sqlx::query(
        "CREATE TABLE conversation_tags (
            conversation_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            added_by TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (conversation_id, tag_id),
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
            FOREIGN KEY (added_by) REFERENCES users(id) ON DELETE SET NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create conversation_tags table");

    sqlx::query("CREATE INDEX idx_conversation_tags_conversation ON conversation_tags(conversation_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_conversation_tags_tag ON conversation_tags(tag_id)")
        .execute(pool)
        .await
        .ok();

    // Create SLA policies table
    sqlx::query(
        "CREATE TABLE sla_policies (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            first_response_time TEXT NOT NULL,
            resolution_time TEXT NOT NULL,
            next_response_time TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create sla_policies table");

    sqlx::query("CREATE INDEX idx_sla_policies_name ON sla_policies(name)")
        .execute(pool)
        .await
        .ok();

    // Create applied SLAs table
    sqlx::query(
        "CREATE TABLE applied_slas (
            id TEXT PRIMARY KEY NOT NULL,
            conversation_id TEXT UNIQUE NOT NULL,
            sla_policy_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
            first_response_deadline_at TEXT NOT NULL,
            resolution_deadline_at TEXT NOT NULL,
            applied_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (sla_policy_id) REFERENCES sla_policies(id) ON DELETE RESTRICT
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create applied_slas table");

    sqlx::query("CREATE UNIQUE INDEX idx_applied_slas_conversation ON applied_slas(conversation_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_applied_slas_policy ON applied_slas(sla_policy_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_applied_slas_status ON applied_slas(status)")
        .execute(pool)
        .await
        .ok();

    // Create SLA events table
    sqlx::query(
        "CREATE TABLE sla_events (
            id TEXT PRIMARY KEY NOT NULL,
            applied_sla_id TEXT NOT NULL,
            event_type TEXT NOT NULL CHECK(event_type IN ('first_response', 'resolution', 'next_response')),
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
            deadline_at TEXT NOT NULL,
            met_at TEXT,
            breached_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (applied_sla_id) REFERENCES applied_slas(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await
    .expect("Failed to create sla_events table");

    sqlx::query("CREATE INDEX idx_sla_events_applied_sla ON sla_events(applied_sla_id)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("CREATE INDEX idx_sla_events_status_deadline ON sla_events(status, deadline_at)")
        .execute(pool)
        .await
        .ok();
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
        ('11000000-0000-0000-0000-000000000002', 'agents:create', 'Create agents', datetime('now'), datetime('now')),
        ('tag-perm-001', 'tags:create', 'Create new tags', datetime('now'), datetime('now')),
        ('tag-perm-002', 'tags:read', 'View tags', datetime('now'), datetime('now')),
        ('tag-perm-003', 'tags:update', 'Update tag properties', datetime('now'), datetime('now')),
        ('tag-perm-004', 'tags:delete', 'Delete tags', datetime('now'), datetime('now')),
        ('tag-perm-005', 'conversations:update_tags', 'Modify conversation tags', datetime('now'), datetime('now'))"
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

    // Insert test inbox
    sqlx::query(
        "INSERT INTO inboxes (id, name) VALUES ('inbox-001', 'Test Inbox')"
    )
    .execute(pool)
    .await
    .expect("Failed to seed test inbox");
}

pub async fn teardown_test_db(db: Database) {
    // Close the connection
    drop(db);
    // Note: Test database files will be cleaned up manually or by .gitignore
}
