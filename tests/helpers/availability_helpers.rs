#![allow(dead_code)]
use chrono::{DateTime, Utc};
use oxidesk::{
    database::Database,
    models::{Agent, AgentAvailability},
};
use sqlx::Row;

/// Create a test agent with custom availability status
pub async fn create_test_agent_with_status(
    db: &Database,
    user_id: &str,
    first_name: &str,
    status: AgentAvailability,
) -> Agent {
    let pool = db.pool();
    let _now = Utc::now().to_rfc3339();
    let agent_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO agents (id, user_id, first_name, last_name, password_hash, availability_status)
         VALUES (?, ?, ?, NULL, 'test_hash', ?)"
    )
    .bind(&agent_id)
    .bind(user_id)
    .bind(first_name)
    .bind(status.to_string())
    .execute(pool)
    .await
    .expect("Failed to create test agent");

    Agent {
        id: agent_id,
        user_id: user_id.to_string(),
        first_name: first_name.to_string(),
        last_name: None,
        password_hash: "test_hash".to_string(),
        availability_status: status,
        last_login_at: None,
        last_activity_at: None,
        away_since: None,
        api_key: None,
        api_secret_hash: None,
        api_key_description: None,
        api_key_created_at: None,
        api_key_last_used_at: None,
        api_key_revoked_at: None,
    }
}

/// Update agent's last_activity_at to a specific time
pub async fn set_agent_last_activity(db: &Database, agent_id: &str, timestamp: DateTime<Utc>) {
    let pool = db.pool();
    sqlx::query("UPDATE agents SET last_activity_at = ? WHERE id = ?")
        .bind(timestamp.to_rfc3339())
        .bind(agent_id)
        .execute(pool)
        .await
        .expect("Failed to set agent last activity");
}

/// Update agent's away_since to a specific time
pub async fn set_agent_away_since(db: &Database, agent_id: &str, timestamp: DateTime<Utc>) {
    let pool = db.pool();
    sqlx::query("UPDATE agents SET away_since = ? WHERE id = ?")
        .bind(timestamp.to_rfc3339())
        .bind(agent_id)
        .execute(pool)
        .await
        .expect("Failed to set agent away_since");
}

/// Get count of activity logs for an agent
pub async fn get_activity_log_count(db: &Database, agent_id: &str) -> i64 {
    let pool = db.pool();
    let result: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM agent_activity_logs WHERE agent_id = ?")
            .bind(agent_id)
            .fetch_one(pool)
            .await
            .expect("Failed to count activity logs");
    result.0
}

/// Get most recent activity log for an agent
pub async fn get_latest_activity_log(
    db: &Database,
    agent_id: &str,
) -> Option<(String, Option<String>, Option<String>)> {
    let pool = db.pool();

    // Manual row parsing to handle NULL values properly
    let row = sqlx::query(
        "SELECT event_type, old_status, new_status FROM agent_activity_logs
         WHERE agent_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch latest activity log");

    row.map(|r| {
        let event_type: String = r.get(0);
        let old_status: Option<String> = r.try_get(1).ok();
        let new_status: Option<String> = r.try_get(2).ok();
        (event_type, old_status, new_status)
    })
}

/// Create a conversation assigned to an agent
pub async fn create_assigned_conversation(
    db: &Database,
    inbox_id: &str,
    contact_id: &str,
    assigned_user_id: &str,
) -> String {
    let pool = db.pool();
    let conversation_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO conversations (id, reference_number, inbox_id, contact_id, status, assigned_user_id, assigned_at, created_at, updated_at)
         VALUES (?, (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations), ?, ?, 'open', ?, ?, ?, ?)"
    )
    .bind(&conversation_id)
    .bind(inbox_id)
    .bind(contact_id)
    .bind(assigned_user_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .expect("Failed to create assigned conversation");

    conversation_id
}

/// Get count of open conversations assigned to an agent
pub async fn get_assigned_conversation_count(db: &Database, user_id: &str) -> i64 {
    let pool = db.pool();
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM conversations
         WHERE assigned_user_id = ? AND status IN ('open', 'snoozed')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("Failed to count assigned conversations");
    result.0
}

/// Get system config value
pub async fn get_config_value(db: &Database, key: &str) -> Option<String> {
    let pool = db.pool();
    let result: Option<(String,)> = sqlx::query_as("SELECT value FROM system_config WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .expect("Failed to get config value");
    result.map(|r| r.0)
}

/// Set system config value
pub async fn set_config_value(db: &Database, key: &str, value: &str) {
    let pool = db.pool();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR REPLACE INTO system_config (key, value, updated_at) VALUES (?, ?, ?)")
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(pool)
        .await
        .expect("Failed to set config value");
}
