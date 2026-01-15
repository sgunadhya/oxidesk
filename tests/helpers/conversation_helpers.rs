#![allow(dead_code)]
use oxidesk::database::agents::AgentRepository;
use oxidesk::api::middleware::AuthenticatedUser;
use oxidesk::database::Database;
use oxidesk::models::conversation::{Conversation, ConversationStatus};
use oxidesk::models::{Agent, Role};
use oxidesk::models::{Contact, User, UserType};
use oxidesk::services::validate_and_normalize_email;
use sqlx::Row;
use uuid::Uuid;

/// Create a test contact with the given email
pub async fn create_test_contact(db: &Database, email: &str) -> Contact {
    let normalized_email = validate_and_normalize_email(email).expect("Invalid email");
    let user = User::new(normalized_email.clone(), UserType::Contact);
    let contact = Contact::new(user.id.clone(), Some(format!("Test User {}", email)));

    db.create_user(&user).await.expect("Failed to create user");
    db.create_contact(&contact)
        .await
        .expect("Failed to create contact");

    // Create contact_channel linking this contact to the test inbox
    let pool = db.pool();
    let channel_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO contact_channels (id, contact_id, inbox_id, email, created_at, updated_at)
         VALUES (?, ?, 'inbox-001', ?, datetime('now'), datetime('now'))",
    )
    .bind(&channel_id)
    .bind(&contact.id)
    .bind(&normalized_email)
    .execute(pool)
    .await
    .expect("Failed to create contact channel");

    contact
}

/// Create a test agent with the given email and name
pub async fn create_test_agent(db: &Database, email: &str, first_name: &str) -> Agent {
    let normalized_email = validate_and_normalize_email(email).expect("Invalid email");
    let user = User::new(normalized_email.clone(), UserType::Agent);
    let agent = Agent::new(
        user.id.clone(),
        first_name.to_string(),
        None,
        "test-password-hash".to_string(),
    );

    db.create_user(&user).await.expect("Failed to create user");
    db.create_agent(&agent)
        .await
        .expect("Failed to create agent");

    agent
}

/// Create a test conversation with specified status
pub async fn create_test_conversation(
    db: &Database,
    inbox_id: String,
    contact_id: String,
    status: ConversationStatus,
) -> Conversation {
    let pool = db.pool();

    // Generate UUID for the conversation
    let conv_id = Uuid::new_v4().to_string();

    // Set resolved_at if status is resolved (required by CHECK constraint)
    let resolved_at = if matches!(status, ConversationStatus::Resolved) {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        None
    };

    let query = r#"
        INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, subject, resolved_at, created_at, updated_at)
        VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations), ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
    "#;

    // Insert the conversation
    sqlx::query(query)
        .bind(&conv_id)
        .bind(status.to_string())
        .bind(&inbox_id)
        .bind(&contact_id)
        .bind("Test conversation")
        .bind(resolved_at)
        .execute(pool)
        .await
        .expect("Failed to insert test conversation");

    // Fetch the created conversation by ID
    let query_select = r#"
        SELECT id, reference_number, status, inbox_id, contact_id, subject,
               resolved_at, snoozed_until, assigned_user_id, assigned_team_id,
               assigned_at, assigned_by, created_at, updated_at, version
        FROM conversations
        WHERE id = ?
    "#;

    let row = sqlx::query(query_select)
        .bind(&conv_id)
        .fetch_one(pool)
        .await
        .expect("Failed to fetch test conversation");

    let status_str: String = row.try_get("status").unwrap();
    Conversation {
        id: row.try_get("id").unwrap(),
        reference_number: row.try_get("reference_number").unwrap(),
        status: ConversationStatus::from(status_str),
        inbox_id: row.try_get("inbox_id").unwrap(),
        contact_id: row.try_get("contact_id").unwrap(),
        subject: row.try_get("subject").ok(),
        resolved_at: row.try_get("resolved_at").ok(),
        closed_at: row.try_get("closed_at").ok(), // Feature 019
        snoozed_until: row.try_get("snoozed_until").ok(),
        assigned_user_id: row.try_get("assigned_user_id").ok(),
        assigned_team_id: row.try_get("assigned_team_id").ok(),
        assigned_at: row.try_get("assigned_at").ok(),
        assigned_by: row.try_get("assigned_by").ok(),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
        version: row.try_get("version").unwrap(),
        tags: None,
        priority: None,
    }
}

/// Create a snoozed conversation expiring at specified time
pub async fn create_snoozed_conversation(
    db: &Database,
    inbox_id: String,
    contact_id: String,
    snoozed_until: String, // ISO 8601 string
) -> Conversation {
    let pool = db.pool();

    // Generate UUID for the conversation
    let conv_id = Uuid::new_v4().to_string();

    let query = r#"
        INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, snoozed_until, created_at, updated_at)
        VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations), ?, ?, ?, ?, datetime('now'), datetime('now'))
    "#;

    sqlx::query(query)
        .bind(&conv_id)
        .bind(ConversationStatus::Snoozed.to_string())
        .bind(&inbox_id)
        .bind(&contact_id)
        .bind(&snoozed_until)
        .execute(pool)
        .await
        .expect("Failed to create snoozed conversation");

    let query_select = r#"
        SELECT id, reference_number, status, inbox_id, contact_id, subject,
               resolved_at, snoozed_until, assigned_user_id, assigned_team_id,
               assigned_at, assigned_by, created_at, updated_at, version
        FROM conversations
        WHERE id = ?
    "#;

    let row = sqlx::query(query_select)
        .bind(&conv_id)
        .fetch_one(pool)
        .await
        .expect("Failed to fetch snoozed conversation");

    let status_str: String = row.try_get("status").unwrap();
    Conversation {
        id: row.try_get("id").unwrap(),
        reference_number: row.try_get("reference_number").unwrap(),
        status: ConversationStatus::from(status_str),
        inbox_id: row.try_get("inbox_id").unwrap(),
        contact_id: row.try_get("contact_id").unwrap(),
        subject: row.try_get("subject").ok(),
        resolved_at: row.try_get("resolved_at").ok(),
        closed_at: row.try_get("closed_at").ok(), // Feature 019
        snoozed_until: row.try_get("snoozed_until").ok(),
        assigned_user_id: row.try_get("assigned_user_id").ok(),
        assigned_team_id: row.try_get("assigned_team_id").ok(),
        assigned_at: row.try_get("assigned_at").ok(),
        assigned_by: row.try_get("assigned_by").ok(),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
        version: row.try_get("version").unwrap(),
        tags: None,
        priority: None,
    }
}

/// Get conversation by ID
pub async fn get_conversation_by_id(
    db: &Database,
    conversation_id: String,
) -> Option<Conversation> {
    db.get_conversation_by_id(&conversation_id)
        .await
        .ok()
        .flatten()
}

/// Create a test authenticated user (Agent)
pub async fn create_test_auth_user(db: &Database) -> AuthenticatedUser {
    let email = format!("agent-{}@example.com", Uuid::new_v4());
    let user = User::new(email.clone(), UserType::Agent);
    db.create_user(&user).await.expect("Failed to create user");

    let agent = Agent {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        first_name: "Test Agent".to_string(),
        last_name: None,
        password_hash: "hash".to_string(),
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

    // Assign Admin role
    let role = Role {
        id: "00000000-0000-0000-0000-000000000001".to_string(), // seeded admin role
        name: "Admin".to_string(),
        description: Some("Full system access".to_string()),
        permissions: vec![], // Admin has all permissions
        is_protected: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    // Implement assign_role if needed or raw query.
    // db.assign_role(...) might verify role existence.
    sqlx::query("INSERT INTO user_roles (user_id, role_id, created_at) VALUES (?, ?, ?)")
        .bind(&user.id)
        .bind(&role.id)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .expect("Failed to assign role");

    // Create test session
    let session = oxidesk::models::Session {
        id: Uuid::new_v4().to_string(),
        user_id: user.id.clone(),
        token: "test-token".to_string(),
        csrf_token: "test-csrf".to_string(),
        expires_at: chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .unwrap()
            .to_rfc3339(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_accessed_at: chrono::Utc::now().to_rfc3339(),
        auth_method: oxidesk::models::AuthMethod::Password,
        provider_name: None,
    };

    AuthenticatedUser {
        user,
        agent,
        roles: vec![role],
        permissions: vec!["*".to_string()], // Admin has all permissions
        session,
        token: "test-token".to_string(),
    }
}
