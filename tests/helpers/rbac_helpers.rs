#![allow(dead_code)]
use oxidesk::database::agents::AgentRepository;
use oxidesk::api::middleware::AuthenticatedUser;
use oxidesk::database::Database;
use oxidesk::models::{Agent, Role, User, UserType};
use sqlx::Row;
use uuid::Uuid;

/// Create a test role with specified permissions
pub async fn create_test_role(
    db: &Database,
    name: &str,
    description: Option<&str>,
    permissions: Vec<String>,
) -> Role {
    let role_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let permissions_json = serde_json::to_string(&permissions).unwrap();

    sqlx::query(
        "INSERT INTO roles (id, name, description, permissions, is_protected, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, ?, ?)"
    )
    .bind(&role_id)
    .bind(name)
    .bind(description)
    .bind(&permissions_json)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to create test role");

    Role {
        id: role_id,
        name: name.to_string(),
        description: description.map(|d| d.to_string()),
        permissions,
        is_protected: false,
        created_at: now.clone(),
        updated_at: now,
    }
}

/// Create a test agent with specified email
pub async fn create_test_agent(db: &Database, email: &str, first_name: &str) -> (User, Agent) {
    let user = User::new(email.to_string(), UserType::Agent);
    db.create_user(&user).await.expect("Failed to create user");

    let agent = Agent {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        first_name: first_name.to_string(),
        last_name: None,
        password_hash: "$argon2id$v=19$m=19456,t=2,p=1$test$test".to_string(), // test hash
        availability_status: oxidesk::models::AgentAvailability::Online,
        last_login_at: None,
        last_activity_at: None,
        away_since: None,
        api_key: None,
        api_secret_hash: None,
        api_key_description: None,
        api_key_created_at: None,
        api_key_last_used_at: None,
        api_key_revoked_at: None,
    };
    db.create_agent(&agent)
        .await
        .expect("Failed to create agent");

    (user, agent)
}

/// Assign a role to a user
pub async fn assign_role_to_user(db: &Database, user_id: &str, role_id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO user_roles (user_id, role_id, created_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(role_id)
        .bind(&now)
        .execute(db.pool())
        .await
        .expect("Failed to assign role to user");
}

/// Create an authenticated user with specified roles
pub async fn create_auth_user_with_roles(
    db: &Database,
    email: &str,
    first_name: &str,
    roles: Vec<Role>,
) -> AuthenticatedUser {
    let (user, agent) = create_test_agent(db, email, first_name).await;

    // Assign all roles
    for role in &roles {
        assign_role_to_user(db, &user.id, &role.id).await;
    }

    // Create test session
    let session = oxidesk::models::Session {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        token: format!("test-token-{}", Uuid::new_v4()),
        csrf_token: format!("test-csrf-{}", Uuid::new_v4()),
        expires_at: chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .unwrap()
            .to_rfc3339(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_accessed_at: chrono::Utc::now().to_rfc3339(),
        auth_method: oxidesk::models::AuthMethod::Password,
        provider_name: None,
    };

    // Compute permissions from all roles
    let mut permissions_set = std::collections::HashSet::new();
    for role in &roles {
        for permission in &role.permissions {
            permissions_set.insert(permission.clone());
        }
    }
    let permissions: Vec<String> = permissions_set.into_iter().collect();

    AuthenticatedUser {
        user,
        agent,
        roles,
        permissions,
        session,
        token: format!("test-token-{}", Uuid::new_v4()),
    }
}

/// Ensure Admin role exists and return it
pub async fn ensure_admin_role(db: &Database) -> Role {
    let admin_id = "00000000-0000-0000-0000-000000000001";

    // Try to get existing Admin role
    if let Ok(Some(role)) = db.get_role_by_id(admin_id).await {
        return role;
    }

    // Create Admin role directly via SQL
    let now = chrono::Utc::now().to_rfc3339();
    let permissions = vec![
        "users:read".to_string(),
        "users:create".to_string(),
        "users:update".to_string(),
        "users:delete".to_string(),
        "roles:read".to_string(),
        "roles:manage".to_string(),
    ];
    let permissions_json = serde_json::to_string(&permissions).unwrap();

    sqlx::query(
        "INSERT OR IGNORE INTO roles (id, name, description, permissions, is_protected, created_at, updated_at)
         VALUES (?, 'Admin', 'Full system access', ?, 1, ?, ?)"
    )
    .bind(admin_id)
    .bind(&permissions_json)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to insert Admin role");

    // Return the role
    Role {
        id: admin_id.to_string(),
        name: "Admin".to_string(),
        description: Some("Full system access".to_string()),
        permissions,
        is_protected: true,
        created_at: now.clone(),
        updated_at: now,
    }
}

/// Create a test team
pub async fn create_test_team(db: &Database, name: &str) -> String {
    let team_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO teams (id, name, description, created_at, updated_at)
         VALUES (?, ?, 'Test team', ?, ?)",
    )
    .bind(&team_id)
    .bind(name)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to create test team");

    team_id
}

/// Add user to team
pub async fn add_user_to_team(db: &Database, user_id: &str, team_id: &str) {
    let membership_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO team_memberships (id, team_id, user_id, role, joined_at)
         VALUES (?, ?, ?, 'member', ?)",
    )
    .bind(&membership_id)
    .bind(team_id)
    .bind(user_id)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to add user to team");
}

/// Create test inbox (required for conversations)
pub async fn ensure_test_inbox(db: &Database) -> String {
    let inbox_id = "test-inbox-rbac".to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Try to create inbox, ignore if exists
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO inboxes (id, name, channel_type, created_at, updated_at)
         VALUES (?, 'Test Inbox', 'email', ?, ?)",
    )
    .bind(&inbox_id)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await;

    inbox_id
}

/// Create a conversation assigned to a user
pub async fn create_conversation_assigned_to_user(
    db: &Database,
    contact_id: &str,
    assigned_user_id: &str,
) -> String {
    let inbox_id = ensure_test_inbox(db).await;
    let conv_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id,
         assigned_user_id, assigned_at, created_at, updated_at)
         VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations),
         'open', ?, ?, ?, ?, ?, ?)",
    )
    .bind(&conv_id)
    .bind(&inbox_id)
    .bind(contact_id)
    .bind(assigned_user_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to create conversation");

    conv_id
}

/// Create a conversation assigned to a team
pub async fn create_conversation_assigned_to_team(
    db: &Database,
    contact_id: &str,
    assigned_team_id: &str,
) -> String {
    let inbox_id = ensure_test_inbox(db).await;
    let conv_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id,
         assigned_team_id, assigned_at, created_at, updated_at)
         VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations),
         'open', ?, ?, ?, ?, ?, ?)",
    )
    .bind(&conv_id)
    .bind(&inbox_id)
    .bind(contact_id)
    .bind(assigned_team_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(db.pool())
    .await
    .expect("Failed to create team-assigned conversation");

    conv_id
}

/// Check if an authorization denial event was logged
pub async fn check_auth_denial_logged(
    db: &Database,
    user_id: &str,
    required_permission: &str,
) -> bool {
    let result = sqlx::query(
        "SELECT COUNT(*) as count FROM auth_events
         WHERE user_id = ?
         AND event_type = 'authorization_denied'
         AND error_reason LIKE ?",
    )
    .bind(user_id)
    .bind(format!("%{}%", required_permission))
    .fetch_one(db.pool())
    .await;

    match result {
        Ok(row) => {
            let count: i64 = row.try_get("count").unwrap_or(0);
            count > 0
        }
        Err(_) => false,
    }
}
