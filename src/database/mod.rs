use sqlx::{any::AnyPoolOptions, AnyPool, Row};
use time;
use crate::{
    api::middleware::error::{ApiError, ApiResult},
    models::*,
};

pub struct Database {
    pool: AnyPool,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
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
        sqlx::migrate!("migrations/sqlite")
            .run(&self.pool)
            .await?;
        Ok(())
    }

    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    // User operations
    pub async fn create_user(&self, user: &User) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO users (id, email, user_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(match user.user_type {
            UserType::Agent => "agent",
            UserType::Contact => "contact",
        })
        .bind(&user.created_at)
        .bind(&user.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_by_email_and_type(
        &self,
        email: &str,
        user_type: &UserType,
    ) -> ApiResult<Option<User>> {
        let user_type_str = match user_type {
            UserType::Agent => "agent",
            UserType::Contact => "contact",
        };

        let row = sqlx::query(
            "SELECT id, email, user_type, created_at, updated_at
             FROM users
             WHERE email = ? AND user_type = ?",
        )
        .bind(email)
        .bind(user_type_str)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if row.try_get::<String, _>("user_type")? == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_by_id(&self, id: &str) -> ApiResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, email, user_type, created_at, updated_at
             FROM users
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if row.try_get::<String, _>("user_type")? == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    // Agent operations
    pub async fn create_agent(&self, agent: &Agent) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO agents (id, user_id, first_name, last_name, password_hash)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&agent.id)
        .bind(&agent.user_id)
        .bind(&agent.first_name)
        .bind(&agent.last_name)
        .bind(&agent.password_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create agent with role assignment in transaction (Feature 016: User Creation)
    /// Creates user + agent + role assignment atomically
    /// Returns the created agent_id and user_id
    pub async fn create_agent_with_role(
        &self,
        email: &str,
        first_name: &str,
        last_name: Option<&str>,
        password_hash: &str,
        role_id: &str,
    ) -> ApiResult<(String, String)> {
        let mut tx = self.pool.begin().await?;

        // Create user
        let user = User::new(email.to_string(), UserType::Agent);
        sqlx::query(
            "INSERT INTO users (id, email, user_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind("agent")
        .bind(&user.created_at)
        .bind(&user.updated_at)
        .execute(&mut *tx)
        .await?;

        // Create agent
        let agent = Agent::new(
            user.id.clone(),
            first_name.to_string(),
            last_name.map(|s| s.to_string()),
            password_hash.to_string(),
        );
        sqlx::query(
            "INSERT INTO agents (id, user_id, first_name, last_name, password_hash, availability_status)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&agent.id)
        .bind(&agent.user_id)
        .bind(&agent.first_name)
        .bind(&agent.last_name)
        .bind(&agent.password_hash)
        .bind("offline")
        .execute(&mut *tx)
        .await?;

        // Assign role
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(&user.id)
        .bind(role_id)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((agent.id, user.id))
    }

    pub async fn get_agent_by_user_id(&self, user_id: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since,
                    api_key, api_secret_hash, api_key_description,
                    api_key_created_at, api_key_last_used_at, api_key_revoked_at
             FROM agents
             WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("availability_status").unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            Ok(Some(Agent {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: row.try_get("api_key").ok(),
                api_secret_hash: row.try_get("api_secret_hash").ok(),
                api_key_description: row.try_get("api_key_description").ok(),
                api_key_created_at: row.try_get("api_key_created_at").ok(),
                api_key_last_used_at: row.try_get("api_key_last_used_at").ok(),
                api_key_revoked_at: row.try_get("api_key_revoked_at").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    // Role operations
    pub async fn get_role_by_name(&self, name: &str) -> ApiResult<Option<Role>> {
        let row = sqlx::query(
            "SELECT id, name, description, permissions, CAST(is_protected AS INTEGER) as is_protected, created_at, updated_at
             FROM roles
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.try_get("permissions")?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_json)
                .unwrap_or_else(|_| Vec::new());

            Ok(Some(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                permissions,
                is_protected: row.try_get::<i32, _>("is_protected")? != 0,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn assign_role_to_user(&self, user_role: &UserRole) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(&user_role.user_id)
        .bind(&user_role.role_id)
        .bind(&user_role.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_roles(&self, user_id: &str) -> ApiResult<Vec<Role>> {
        let rows = sqlx::query(
            "SELECT r.id, r.name, r.description, r.permissions, CAST(r.is_protected AS INTEGER) as is_protected, r.created_at, r.updated_at
             FROM roles r
             INNER JOIN user_roles ur ON r.id = ur.role_id
             WHERE ur.user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut roles = Vec::new();
        for row in rows {
            let permissions_json: String = row.try_get("permissions")?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_json)
                .unwrap_or_else(|_| Vec::new());

            roles.push(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                permissions,
                is_protected: row.try_get::<i32, _>("is_protected")? != 0,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(roles)
    }

    // Session operations
    pub async fn create_session(&self, session: &Session) -> ApiResult<()> {
        let auth_method_str = match session.auth_method {
            crate::models::AuthMethod::Password => "password",
            crate::models::AuthMethod::Oidc => "oidc",
            crate::models::AuthMethod::ApiKey => "apikey",
        };

        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, csrf_token, expires_at, created_at, last_accessed_at, auth_method, provider_name)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.token)
        .bind(&session.csrf_token)
        .bind(&session.expires_at)
        .bind(&session.created_at)
        .bind(&session.last_accessed_at)
        .bind(auth_method_str)
        .bind(&session.provider_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_session_by_token(&self, token: &str) -> ApiResult<Option<Session>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, csrf_token, expires_at, created_at, last_accessed_at, auth_method, provider_name
             FROM sessions
             WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => crate::models::AuthMethod::Password,
                "oidc" => crate::models::AuthMethod::Oidc,
                _ => crate::models::AuthMethod::Password,
            };

            Ok(Some(Session {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                csrf_token: row.try_get("csrf_token")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                last_accessed_at: row.try_get("last_accessed_at")?,
                auth_method,
                provider_name: row.try_get("provider_name")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_session(&self, token: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) -> ApiResult<u64> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // List agents with pagination
    pub async fn list_agents(&self, limit: i64, offset: i64) -> ApiResult<Vec<(User, Agent)>> {
        let rows = sqlx::query(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at,
                    a.id as agent_id, a.user_id as agent_user_id, a.first_name, a.last_name, a.password_hash,
                    a.availability_status, a.last_login_at, a.last_activity_at, a.away_since
             FROM users u
             INNER JOIN agents a ON a.user_id = u.id
             WHERE u.user_type = 'agent'
             ORDER BY u.created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let user = User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: UserType::Agent,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };

            let status_str: String = row.try_get("availability_status").unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            let agent = Agent {
                id: row.try_get("agent_id")?,
                user_id: row.try_get("agent_user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: None,
                api_secret_hash: None,
                api_key_description: None,
                api_key_created_at: None,
                api_key_last_used_at: None,
                api_key_revoked_at: None,
            };

            results.push((user, agent));
        }

        Ok(results)
    }

    // Count total agents
    pub async fn count_agents(&self) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM users
             WHERE user_type = 'agent'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }

    // Count admin users (for last admin check)
    pub async fn count_admin_users(&self) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(DISTINCT ur.user_id) as count
             FROM user_roles ur
             INNER JOIN roles r ON r.id = ur.role_id
             WHERE r.name = 'Admin'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }

    // Contact operations
    pub async fn create_contact(&self, contact: &Contact) -> ApiResult<()> {
        // Handle Option<String> for first_name
        let first_name_value: Option<&str> = contact.first_name.as_deref();

        sqlx::query(
            "INSERT INTO contacts (id, user_id, first_name)
             VALUES (?, ?, ?)",
        )
        .bind(&contact.id)
        .bind(&contact.user_id)
        .bind(first_name_value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_contact_by_user_id(&self, user_id: &str) -> ApiResult<Option<Contact>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name
             FROM contacts
             WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            // Handle NULL for first_name
            let first_name: Option<String> = row.try_get("first_name").ok();

            Ok(Some(Contact {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_contact_channel(&self, channel: &ContactChannel) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO contact_channels (id, contact_id, inbox_id, email, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&channel.id)
        .bind(&channel.contact_id)
        .bind(&channel.inbox_id)
        .bind(&channel.email)
        .bind(&channel.created_at)
        .bind(&channel.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get contact by email (Feature 016: User Creation)
    /// Used for idempotent contact creation - check if contact exists before creating
    pub async fn get_contact_by_email(&self, email: &str) -> ApiResult<Option<Contact>> {
        let row = sqlx::query(
            "SELECT c.id, c.user_id, c.first_name
             FROM contacts c
             JOIN users u ON u.id = c.user_id
             WHERE u.email = ? AND u.user_type = 'contact'",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Contact {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Create contact from incoming message (Feature 016: User Creation)
    /// Creates user + contact + contact_channel in a single transaction
    /// Returns the created contact_id
    pub async fn create_contact_from_message(
        &self,
        email: &str,
        full_name: Option<&str>,
        inbox_id: &str,
    ) -> ApiResult<String> {
        let mut tx = self.pool.begin().await?;

        // Create user
        let user = User::new(email.to_string(), UserType::Contact);
        sqlx::query(
            "INSERT INTO users (id, email, user_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind("contact")
        .bind(&user.created_at)
        .bind(&user.updated_at)
        .execute(&mut *tx)
        .await?;

        // Create contact
        let contact = Contact::new(user.id.clone(), full_name.map(|s| s.to_string()));
        sqlx::query(
            "INSERT INTO contacts (id, user_id, first_name)
             VALUES (?, ?, ?)",
        )
        .bind(&contact.id)
        .bind(&contact.user_id)
        .bind(&contact.first_name)
        .execute(&mut *tx)
        .await?;

        // Create contact channel
        let channel = ContactChannel::new(contact.id.clone(), inbox_id.to_string(), email.to_string());
        sqlx::query(
            "INSERT INTO contact_channels (id, contact_id, inbox_id, email, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&channel.id)
        .bind(&channel.contact_id)
        .bind(&channel.inbox_id)
        .bind(&channel.email)
        .bind(&channel.created_at)
        .bind(&channel.updated_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(contact.id)
    }

    pub async fn get_contact_channels(&self, contact_id: &str) -> ApiResult<Vec<ContactChannel>> {
        let rows = sqlx::query(
            "SELECT id, contact_id, inbox_id, email, created_at, updated_at
             FROM contact_channels
             WHERE contact_id = ?",
        )
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        let mut channels = Vec::new();
        for row in rows {
            channels.push(ContactChannel {
                id: row.try_get("id")?,
                contact_id: row.try_get("contact_id")?,
                inbox_id: row.try_get("inbox_id")?,
                email: row.try_get("email")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(channels)
    }

    // List contacts with pagination
    pub async fn list_contacts(&self, limit: i64, offset: i64) -> ApiResult<Vec<(User, Contact)>> {
        let rows = sqlx::query(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at,
                    c.id as contact_id, c.user_id as contact_user_id, c.first_name
             FROM users u
             INNER JOIN contacts c ON c.user_id = u.id
             WHERE u.user_type = 'contact'
             ORDER BY u.created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let user = User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: UserType::Contact,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };

            // Handle NULL for first_name
            let first_name: Option<String> = row.try_get("first_name").ok();

            let contact = Contact {
                id: row.try_get("contact_id")?,
                user_id: row.try_get("contact_user_id")?,
                first_name,
            };

            results.push((user, contact));
        }

        Ok(results)
    }

    // Count total contacts
    pub async fn count_contacts(&self) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM users
             WHERE user_type = 'contact'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }

    // Conversation operations
    pub async fn create_conversation(&self, create: &CreateConversation) -> ApiResult<Conversation> {
        // Handle Option<String> for subject
        let subject_value: Option<&str> = create.subject.as_deref();

        tracing::debug!(
            "Creating conversation for inbox_id={}, contact_id={}",
            create.inbox_id,
            create.contact_id
        );

        // Insert the conversation
        let result = sqlx::query(
            "INSERT INTO conversations (id, reference_number, status, inbox_id, contact_id, subject, created_at, updated_at)
             VALUES (lower(hex(randomblob(16))), (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations), 'open', ?, ?, ?, datetime('now'), datetime('now'))",
        )
        .bind(&create.inbox_id)
        .bind(&create.contact_id)
        .bind(subject_value)
        .execute(&self.pool)
        .await?;

        // Fetch the created conversation using rowid
        let row = sqlx::query(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, created_at, updated_at, version
             FROM conversations
             WHERE rowid = ?",
        )
        .bind(result.last_insert_id())
        .fetch_one(&self.pool)
        .await?;

        let status_str: String = row.try_get("status")?;
        let conversation = Conversation {
            id: row.try_get("id")?,
            reference_number: row.try_get("reference_number")?,
            status: ConversationStatus::from(status_str),
            inbox_id: row.try_get("inbox_id")?,
            contact_id: row.try_get("contact_id")?,
            subject: row.try_get("subject").ok(),
            resolved_at: row.try_get("resolved_at").ok(),
            snoozed_until: row.try_get("snoozed_until").ok(),
            assigned_user_id: row.try_get("assigned_user_id").ok(),
            assigned_team_id: row.try_get("assigned_team_id").ok(),
            assigned_at: row.try_get("assigned_at").ok(),
            assigned_by: row.try_get("assigned_by").ok(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            version: row.try_get("version")?,
            tags: None,
            priority: None,
        };

        tracing::info!(
            "Conversation created: id={}, reference_number={}, status={:?}",
            conversation.id,
            conversation.reference_number,
            conversation.status
        );

        Ok(conversation)
    }

    pub async fn get_conversation_by_id(&self, id: &str) -> ApiResult<Option<Conversation>> {
        let row = sqlx::query(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, assigned_user_id, assigned_team_id,
                    assigned_at, assigned_by, created_at, updated_at, version, priority
             FROM conversations
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            let conversation = Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: row.try_get::<Option<String>, _>("priority").ok().flatten(),
            };
            Ok(Some(conversation))
        } else {
            Ok(None)
        }
    }

    pub async fn get_conversation_by_reference_number(&self, reference_number: i64) -> ApiResult<Option<Conversation>> {
        let row = sqlx::query(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, created_at, updated_at, version
             FROM conversations
             WHERE reference_number = ?",
        )
        .bind(reference_number)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            let conversation = Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: row.try_get::<Option<String>, _>("priority").ok().flatten(),
            };
            Ok(Some(conversation))
        } else {
            Ok(None)
        }
    }

    pub async fn update_conversation_fields(
        &self,
        id: &str,
        status: ConversationStatus,
        resolved_at: Option<String>,
        snoozed_until: Option<String>,
    ) -> ApiResult<Conversation> {
        // Optimistic locking not strictly enforced here as previous version isn't passed, 
        // but can be added if we pass expected_version.
        // For now, simple update.
        
        sqlx::query(
            "UPDATE conversations 
             SET status = ?, resolved_at = ?, snoozed_until = ?, version = version + 1
             WHERE id = ?"
        )
        .bind(status.to_string())
        .bind(resolved_at)
        .bind(snoozed_until)
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        self.get_conversation_by_id(id).await?.ok_or_else(|| {
              crate::api::middleware::ApiError::NotFound("Conversation not found after update".to_string())
        })
    }
    
    pub async fn get_conversation_by_reference(&self, ref_num: i64) -> ApiResult<Option<Conversation>> {
        let row = sqlx::query("SELECT * FROM conversations WHERE reference_number = ?")
            .bind(ref_num)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
             use sqlx::Row;
             let conversation = Conversation {
                id: row.try_get("id")?,
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                status: ConversationStatus::from(row.try_get::<String, _>("status")?),
                reference_number: row.try_get("reference_number")?,
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: row.try_get::<Option<String>, _>("priority").ok().flatten(),
            };
            Ok(Some(conversation))
        } else {
            Ok(None)
        }
    }

    /// List conversations with pagination and optional filters
    pub async fn list_conversations(
        &self,
        limit: i64,
        offset: i64,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<Vec<Conversation>> {
        let mut query = String::from(
            "SELECT id, reference_number, status, inbox_id, contact_id, subject,
                    resolved_at, snoozed_until, created_at, updated_at, version
             FROM conversations
             WHERE 1=1"
        );

        // Add filters
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if inbox_id.is_some() {
            query.push_str(" AND inbox_id = ?");
        }
        if contact_id.is_some() {
            query.push_str(" AND contact_id = ?");
        }

        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut sql_query = sqlx::query(&query);

        // Bind filter parameters
        if let Some(s) = status {
            sql_query = sql_query.bind(s.to_string());
        }
        if let Some(inbox) = inbox_id {
            sql_query = sql_query.bind(inbox);
        }
        if let Some(contact) = contact_id {
            sql_query = sql_query.bind(contact);
        }

        // Bind pagination parameters
        sql_query = sql_query.bind(limit).bind(offset);

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut conversations = Vec::new();
        for row in rows {
            use sqlx::Row;
            let status_str: String = row.try_get("status")?;
            let conversation = Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: None,
            };
            conversations.push(conversation);
        }

        Ok(conversations)
    }

    /// Count total conversations with optional filters
    pub async fn count_conversations(
        &self,
        status: Option<ConversationStatus>,
        inbox_id: Option<String>,
        contact_id: Option<String>,
    ) -> ApiResult<i64> {
        let mut query = String::from("SELECT COUNT(*) as count FROM conversations WHERE 1=1");

        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if inbox_id.is_some() {
            query.push_str(" AND inbox_id = ?");
        }
        if contact_id.is_some() {
            query.push_str(" AND contact_id = ?");
        }

        let mut sql_query = sqlx::query(&query);

        if let Some(s) = status {
            sql_query = sql_query.bind(s.to_string());
        }
        if let Some(inbox) = inbox_id {
            sql_query = sql_query.bind(inbox);
        }
        if let Some(contact) = contact_id {
            sql_query = sql_query.bind(contact);
        }

        let row = sql_query.fetch_one(&self.pool).await?;
        use sqlx::Row;
        let count: i64 = row.try_get("count")?;

        Ok(count)
    }

    /// Set conversation priority (for automation rules)
    pub async fn set_conversation_priority(
        &self,
        conversation_id: &str,
        priority: &str,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE conversations
             SET priority = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(priority)
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error setting conversation priority: {:?}", e);
            ApiError::Internal(format!("Database error: {}", e))
        })?;

        tracing::info!(
            "Set priority to '{}' for conversation {}",
            priority,
            conversation_id
        );

        Ok(())
    }

    /// Update conversation status (for automation rules - bypasses state machine)
    pub async fn update_conversation_status(
        &self,
        conversation_id: &str,
        status: ConversationStatus,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        // Set resolved_at if transitioning to Resolved
        let resolved_at = if status == ConversationStatus::Resolved {
            Some(now.clone())
        } else {
            None
        };

        // Clear resolved_at if not resolved
        if status != ConversationStatus::Resolved {
            sqlx::query(
                "UPDATE conversations
                 SET status = ?, resolved_at = NULL, updated_at = ?
                 WHERE id = ?"
            )
            .bind(status.to_string())
            .bind(&now)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error updating conversation status: {:?}", e);
                ApiError::Internal(format!("Database error: {}", e))
            })?;
        } else {
            sqlx::query(
                "UPDATE conversations
                 SET status = ?, resolved_at = ?, updated_at = ?
                 WHERE id = ?"
            )
            .bind(status.to_string())
            .bind(&resolved_at)
            .bind(&now)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                eprintln!("Database error updating conversation status: {:?}", e);
                ApiError::Internal(format!("Database error: {}", e))
            })?;
        }

        tracing::info!(
            "Updated status to {:?} for conversation {}",
            status,
            conversation_id
        );

        Ok(())
    }

    // Legacy high-level update (deprecated by conversation_service, but keeping preventing break if used elsewhere? 
    // Actually, I should remove it or delegate to service, but database calling service is bad.
    // I will comment out update_conversation_status to avoid confusion/duplication if it's unused.)
    // Legacy high-level update (deprecated by conversation_service)
    /*
    pub async fn update_conversation_status(
        &self,
        conversation_id: &str,
        update_request: UpdateStatusRequest,
        agent_id: Option<String>,
        event_bus: Option<&crate::events::EventBus>,
    ) -> ApiResult<Conversation> {
        use crate::services::state_machine::{execute_transition, TransitionContext};

        // Get current conversation
        let current = self
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                crate::api::middleware::ApiError::NotFound("Conversation not found".to_string())
            })?;

        // Create transition context
        let context = TransitionContext {
            conversation_id: conversation_id.to_string(),
            from_status: current.status,
            to_status: update_request.status,
            agent_id,
            snooze_duration: update_request.snooze_duration.clone(),
        };

        // Execute transition (validates and publishes events)
        let _result = execute_transition(context, event_bus).map_err(|e| {
            crate::api::middleware::ApiError::BadRequest(format!("Invalid transition: {}", e))
        })?;

        tracing::info!(
            "Updating conversation {} status from {:?} to {:?}",
            conversation_id,
            current.status,
            update_request.status
        );

        // Update status in database
        let new_status_str = update_request.status.to_string();
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        // Set resolved_at if transitioning to Resolved
        let resolved_at = if update_request.status == ConversationStatus::Resolved {
            Some(now.clone())
        } else if update_request.status == ConversationStatus::Open {
            // Clear resolved_at when reopening
            None
        } else {
            current.resolved_at
        };

        // Set snoozed_until if snooze_duration provided
        let snoozed_until = if update_request.snooze_duration.is_some() {
            update_request.snooze_duration
        } else if update_request.status != ConversationStatus::Snoozed {
            // Clear snoozed_until if not snoozing
            None
        } else {
            current.snoozed_until
        };

        sqlx::query(
            "UPDATE conversations
             SET status = ?, resolved_at = ?, snoozed_until = ?, updated_at = ?, version = version + 1
             WHERE id = ?",
        )
        .bind(&new_status_str)
        .bind(resolved_at.as_ref())
        .bind(snoozed_until.as_ref())
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        // Fetch and return updated conversation
        let updated = self
            .get_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| {
                crate::api::middleware::ApiError::Internal("Conversation disappeared".to_string())
            })?;

        tracing::info!(
            "Conversation {} status updated successfully to {:?}",
            conversation_id,
            updated.status
        );

        Ok(updated)
    }
    */

    // Role operations
    pub async fn list_roles(&self) -> ApiResult<Vec<Role>> {
        let rows = sqlx::query(
            "SELECT id, name, description, permissions, CAST(is_protected AS INTEGER) as is_protected, created_at, updated_at
             FROM roles
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut roles = Vec::new();
        for row in rows {
            let permissions_json: String = row.try_get("permissions")?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_json)
                .unwrap_or_else(|_| Vec::new());

            roles.push(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                permissions,
                is_protected: row.try_get::<i32, _>("is_protected")? != 0,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(roles)
    }

    pub async fn get_role_by_id(&self, id: &str) -> ApiResult<Option<Role>> {
        let row = sqlx::query(
            "SELECT id, name, description, permissions, CAST(is_protected AS INTEGER) as is_protected, created_at, updated_at
             FROM roles
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let permissions_json: String = row.try_get("permissions")?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_json)
                .unwrap_or_else(|_| Vec::new());

            Ok(Some(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                permissions,
                is_protected: row.try_get::<i32, _>("is_protected")? != 0,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_role(&self, role: &Role) -> ApiResult<()> {
        let description_value: Option<&str> = role.description.as_deref();
        let permissions_json = serde_json::to_string(&role.permissions)
            .unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO roles (id, name, description, permissions, is_protected, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&role.id)
        .bind(&role.name)
        .bind(description_value)
        .bind(&permissions_json)
        .bind(if role.is_protected { 1 } else { 0 })
        .bind(&role.created_at)
        .bind(&role.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_role(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        permissions: Option<&Vec<String>>,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        // Build dynamic UPDATE query based on which fields are provided
        if let Some(perms) = permissions {
            let permissions_json = serde_json::to_string(perms)
                .unwrap_or_else(|_| "[]".to_string());

            if let Some(n) = name {
                sqlx::query(
                    "UPDATE roles
                     SET name = ?, description = ?, permissions = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(n)
                .bind(description)
                .bind(&permissions_json)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE roles
                     SET description = ?, permissions = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(description)
                .bind(&permissions_json)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
        } else if let Some(n) = name {
            sqlx::query(
                "UPDATE roles
                 SET name = ?, description = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(n)
            .bind(description)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn delete_role(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn count_users_with_role(&self, role_id: &str) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM user_roles
             WHERE role_id = ?",
        )
        .bind(role_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }

    // Permission operations
    pub async fn list_permissions(&self) -> ApiResult<Vec<Permission>> {
        let rows = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
             FROM permissions
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut permissions = Vec::new();
        for row in rows {
            permissions.push(Permission {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(permissions)
    }

    pub async fn get_role_permissions(&self, role_id: &str) -> ApiResult<Vec<Permission>> {
        let rows = sqlx::query(
            "SELECT p.id, p.name, p.description, p.created_at, p.updated_at
             FROM permissions p
             INNER JOIN role_permissions rp ON rp.permission_id = p.id
             WHERE rp.role_id = ?
             ORDER BY p.name",
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;

        let mut permissions = Vec::new();
        for row in rows {
            permissions.push(Permission {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(permissions)
    }

    pub async fn assign_permission_to_role(&self, role_permission: &RolePermission) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO role_permissions (role_id, permission_id, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(&role_permission.role_id)
        .bind(&role_permission.permission_id)
        .bind(&role_permission.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Agent update operations
    pub async fn update_agent(&self, agent_id: &str, first_name: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE agents
             SET first_name = ?
             WHERE id = ?",
        )
        .bind(first_name)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_agent_password(&self, agent_id: &str, password_hash: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE agents
             SET password_hash = ?
             WHERE id = ?",
        )
        .bind(password_hash)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_user_roles(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM user_roles WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Contact update operations
    pub async fn update_contact(&self, contact_id: &str, first_name: &Option<String>) -> ApiResult<()> {
        let first_name_value: Option<&str> = first_name.as_deref();

        sqlx::query(
            "UPDATE contacts
             SET first_name = ?
             WHERE id = ?",
        )
        .bind(first_name_value)
        .bind(contact_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_contact(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_user(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Generic user operations
    pub async fn list_users(
        &self,
        limit: i64,
        offset: i64,
        user_type_filter: Option<UserType>,
    ) -> ApiResult<(Vec<User>, i64)> {
        let (query, count_query) = if let Some(user_type) = user_type_filter {
            let type_str = match user_type {
                UserType::Agent => "agent",
                UserType::Contact => "contact",
            };

            (
                format!(
                    "SELECT id, email, user_type, created_at, updated_at
                     FROM users
                     WHERE user_type = '{}'
                     ORDER BY created_at DESC
                     LIMIT ? OFFSET ?",
                    type_str
                ),
                format!(
                    "SELECT COUNT(*) as count
                     FROM users
                     WHERE user_type = '{}'",
                    type_str
                ),
            )
        } else {
            (
                "SELECT id, email, user_type, created_at, updated_at
                 FROM users
                 ORDER BY created_at DESC
                 LIMIT ? OFFSET ?"
                    .to_string(),
                "SELECT COUNT(*) as count FROM users".to_string(),
            )
        };

        // Get total count
        let count_row = sqlx::query(&count_query)
            .fetch_one(&self.pool)
            .await?;
        let total_count: i64 = count_row.try_get("count")?;

        // Get users
        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            let user_type = match user_type_str.as_str() {
                "agent" => UserType::Agent,
                "contact" => UserType::Contact,
                _ => continue,
            };

            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok((users, total_count))
    }

    // Message operations
    pub async fn create_message(&self, message: &Message) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, sent_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(message.message_type.as_str())
        .bind(message.status.as_str())
        .bind(&message.content)
        .bind(&message.author_id)
        .bind(message.is_immutable)
        .bind(message.retry_count)
        .bind(&message.created_at)
        .bind(&message.sent_at)
        .bind(&message.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_message_by_id(&self, id: &str) -> ApiResult<Option<Message>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, sent_at, updated_at
             FROM messages
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let message_type_str: String = row.try_get("type")?;
            let status_str: String = row.try_get("status")?;

            Ok(Some(Message {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                message_type: MessageType::from(message_type_str),
                status: MessageStatus::from(status_str),
                content: row.try_get("content")?,
                author_id: row.try_get("author_id")?,
                is_immutable: row.try_get("is_immutable")?,
                retry_count: row.try_get("retry_count")?,
                created_at: row.try_get("created_at")?,
                sent_at: row.try_get("sent_at").ok(),
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_messages(
        &self,
        conversation_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Message>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM messages WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;
        let total_count: i64 = count_row.try_get("count")?;

        // Get messages
        let rows = sqlx::query(
            "SELECT id, conversation_id, type, status, content, author_id, is_immutable, retry_count, created_at, sent_at, updated_at
             FROM messages
             WHERE conversation_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(conversation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            let message_type_str: String = row.try_get("type")?;
            let status_str: String = row.try_get("status")?;

            messages.push(Message {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                message_type: MessageType::from(message_type_str),
                status: MessageStatus::from(status_str),
                content: row.try_get("content")?,
                author_id: row.try_get("author_id")?,
                is_immutable: row.try_get("is_immutable")?,
                retry_count: row.try_get("retry_count")?,
                created_at: row.try_get("created_at")?,
                sent_at: row.try_get("sent_at").ok(),
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok((messages, total_count))
    }

    pub async fn update_message_status(
        &self,
        message_id: &str,
        status: MessageStatus,
        sent_at: Option<&str>,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        if let Some(sent_at_value) = sent_at {
            sqlx::query(
                "UPDATE messages
                 SET status = ?, sent_at = ?, updated_at = ?, is_immutable = ?
                 WHERE id = ?",
            )
            .bind(status.as_str())
            .bind(sent_at_value)
            .bind(&now)
            .bind(status.is_immutable())
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE messages
                 SET status = ?, updated_at = ?, is_immutable = ?
                 WHERE id = ?",
            )
            .bind(status.as_str())
            .bind(&now)
            .bind(status.is_immutable())
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn update_conversation_message_timestamps(
        &self,
        conversation_id: &str,
        message_id: &str,
        last_message_at: &str,
        last_reply_at: Option<&str>,
    ) -> ApiResult<()> {
        if let Some(reply_at) = last_reply_at {
            sqlx::query(
                "UPDATE conversations
                 SET last_message_id = ?, last_message_at = ?, last_reply_at = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(message_id)
            .bind(last_message_at)
            .bind(reply_at)
            .bind(last_message_at)
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE conversations
                 SET last_message_id = ?, last_message_at = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(message_id)
            .bind(last_message_at)
            .bind(last_message_at)
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn count_messages(&self, conversation_id: &str) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM messages WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    // ========== Team Operations (T021-T023) ==========

    pub async fn create_team(&self, team: &Team) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO teams (id, name, description, sla_policy_id, business_hours, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&team.id)
        .bind(&team.name)
        .bind(&team.description)
        .bind(&team.sla_policy_id)
        .bind(&team.business_hours)
        .bind(&team.created_at)
        .bind(&team.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                ApiError::BadRequest(format!("Team with name '{}' already exists", team.name))
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

        tracing::info!("Team created: id={}, name={}", team.id, team.name);
        Ok(())
    }

    pub async fn get_team_by_id(&self, id: &str) -> ApiResult<Option<Team>> {
        let row = sqlx::query(
            "SELECT id, name, description, sla_policy_id, business_hours, created_at, updated_at
             FROM teams WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Team {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                sla_policy_id: row.try_get("sla_policy_id").ok(),
                business_hours: row.try_get("business_hours").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_teams(&self) -> ApiResult<Vec<Team>> {
        let rows = sqlx::query(
            "SELECT id, name, description, sla_policy_id, business_hours, created_at, updated_at
             FROM teams ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut teams = Vec::new();
        for row in rows {
            teams.push(Team {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                sla_policy_id: row.try_get("sla_policy_id").ok(),
                business_hours: row.try_get("business_hours").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(teams)
    }

    // ========== Team Membership Operations (T024-T028) ==========

    pub async fn add_team_member(
        &self,
        team_id: &str,
        user_id: &str,
        role: TeamMemberRole,
    ) -> ApiResult<()> {
        let membership = TeamMembership::new(team_id.to_string(), user_id.to_string(), role);

        sqlx::query(
            "INSERT INTO team_memberships (id, team_id, user_id, role, joined_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&membership.id)
        .bind(&membership.team_id)
        .bind(&membership.user_id)
        .bind(membership.role.to_string())
        .bind(&membership.joined_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                ApiError::BadRequest("User is already a member of this team".to_string())
            } else if e.to_string().contains("FOREIGN KEY") {
                ApiError::NotFound("Team or user not found".to_string())
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

        tracing::info!(
            "Team member added: team={}, user={}, role={}",
            team_id,
            user_id,
            role
        );
        Ok(())
    }

    pub async fn remove_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM team_memberships WHERE team_id = ? AND user_id = ?")
            .bind(team_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        tracing::info!("Team member removed: team={}, user={}", team_id, user_id);
        Ok(())
    }

    pub async fn get_team_members(&self, team_id: &str) -> ApiResult<Vec<User>> {
        let rows = sqlx::query(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at
             FROM users u
             INNER JOIN team_memberships tm ON u.id = tm.user_id
             WHERE tm.team_id = ?
             ORDER BY u.email ASC",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: match user_type_str.as_str() {
                    "agent" => UserType::Agent,
                    "contact" => UserType::Contact,
                    _ => UserType::Agent,
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(users)
    }

    pub async fn is_team_member(&self, team_id: &str, user_id: &str) -> ApiResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM team_memberships WHERE team_id = ? AND user_id = ?",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count > 0)
    }

    pub async fn get_user_teams(&self, user_id: &str) -> ApiResult<Vec<Team>> {
        let rows = sqlx::query(
            "SELECT t.id, t.name, t.description, t.sla_policy_id, t.business_hours, t.created_at, t.updated_at
             FROM teams t
             INNER JOIN team_memberships tm ON t.id = tm.team_id
             WHERE tm.user_id = ?
             ORDER BY t.name ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut teams = Vec::new();
        for row in rows {
            teams.push(Team {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                sla_policy_id: row.try_get("sla_policy_id").ok(),
                business_hours: row.try_get("business_hours").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(teams)
    }

    /// Update team's SLA policy
    pub async fn update_team_sla_policy(&self, team_id: &str, sla_policy_id: Option<&str>) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE teams SET sla_policy_id = ?, updated_at = ? WHERE id = ?"
        )
        .bind(sla_policy_id)
        .bind(now)
        .bind(team_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== Conversation Assignment Operations (T029-T032) ==========

    pub async fn assign_conversation_to_user(
        &self,
        conversation_id: &str,
        user_id: &str,
        assigned_by: &str,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE conversations
             SET assigned_user_id = ?, assigned_at = ?, assigned_by = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(user_id)
        .bind(&now)
        .bind(assigned_by)
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "Conversation {} assigned to user {} by {}",
            conversation_id,
            user_id,
            assigned_by
        );
        Ok(())
    }

    pub async fn assign_conversation_to_team(
        &self,
        conversation_id: &str,
        team_id: &str,
        assigned_by: &str,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE conversations
             SET assigned_team_id = ?, assigned_at = ?, assigned_by = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(team_id)
        .bind(&now)
        .bind(assigned_by)
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "Conversation {} assigned to team {} by {}",
            conversation_id,
            team_id,
            assigned_by
        );
        Ok(())
    }

    pub async fn unassign_conversation_user(&self, conversation_id: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE conversations
             SET assigned_user_id = NULL, updated_at = ?
             WHERE id = ?",
        )
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        tracing::info!("Conversation {} user assignment removed", conversation_id);
        Ok(())
    }

    pub async fn unassign_conversation_team(&self, conversation_id: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE conversations
             SET assigned_team_id = NULL, updated_at = ?
             WHERE id = ?",
        )
        .bind(&now)
        .bind(conversation_id)
        .execute(&self.pool)
        .await?;

        tracing::info!("Conversation {} team assignment removed", conversation_id);
        Ok(())
    }

    // ========== Conversation Participants (T033-T034) ==========

    pub async fn add_conversation_participant(
        &self,
        conversation_id: &str,
        user_id: &str,
        added_by: Option<&str>,
    ) -> ApiResult<()> {
        let participant = ConversationParticipant::new(
            conversation_id.to_string(),
            user_id.to_string(),
            added_by.map(String::from),
        );

        sqlx::query(
            "INSERT INTO conversation_participants (id, conversation_id, user_id, added_at, added_by)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&participant.id)
        .bind(&participant.conversation_id)
        .bind(&participant.user_id)
        .bind(&participant.added_at)
        .bind(&participant.added_by)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                // User is already a participant - this is OK, just log it
                tracing::debug!(
                    "User {} is already a participant in conversation {}",
                    user_id,
                    conversation_id
                );
                ApiError::BadRequest("User is already a participant in this conversation".to_string())
            } else if e.to_string().contains("FOREIGN KEY") {
                ApiError::NotFound("Conversation or user not found".to_string())
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

        tracing::info!(
            "Participant added: conversation={}, user={}",
            conversation_id,
            user_id
        );
        Ok(())
    }

    pub async fn get_conversation_participants(&self, conversation_id: &str) -> ApiResult<Vec<User>> {
        let rows = sqlx::query(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at
             FROM users u
             INNER JOIN conversation_participants cp ON u.id = cp.user_id
             WHERE cp.conversation_id = ?
             ORDER BY cp.added_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: match user_type_str.as_str() {
                    "agent" => UserType::Agent,
                    "contact" => UserType::Contact,
                    _ => UserType::Agent,
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(users)
    }

    // ========== Assignment History (T035-T036) ==========

    pub async fn record_assignment(&self, history: &AssignmentHistory) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO assignment_history (id, conversation_id, assigned_user_id, assigned_team_id, assigned_by, assigned_at, unassigned_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&history.id)
        .bind(&history.conversation_id)
        .bind(&history.assigned_user_id)
        .bind(&history.assigned_team_id)
        .bind(&history.assigned_by)
        .bind(&history.assigned_at)
        .bind(&history.unassigned_at)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Assignment history recorded: {}", history.id);
        Ok(())
    }

    pub async fn get_assignment_history(&self, conversation_id: &str) -> ApiResult<Vec<AssignmentHistory>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, assigned_user_id, assigned_team_id, assigned_by, assigned_at, unassigned_at
             FROM assignment_history
             WHERE conversation_id = ?
             ORDER BY assigned_at DESC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut history = Vec::new();
        for row in rows {
            history.push(AssignmentHistory {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_by: row.try_get("assigned_by")?,
                assigned_at: row.try_get("assigned_at")?,
                unassigned_at: row.try_get("unassigned_at").ok(),
            });
        }

        Ok(history)
    }

    // ========== Agent Availability (T037-T038) ==========

    pub async fn update_agent_availability(
        &self,
        user_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        sqlx::query("UPDATE agents SET availability_status = ? WHERE user_id = ?")
            .bind(status.to_string())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        tracing::info!("Agent {} availability updated to {}", user_id, status);
        Ok(())
    }

    pub async fn get_agent_availability(&self, user_id: &str) -> ApiResult<AgentAvailability> {
        let row = sqlx::query("SELECT availability_status FROM agents WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("availability_status")?;
            status_str
                .parse()
                .map_err(|e| ApiError::Internal(format!("Invalid availability status: {}", e)))
        } else {
            Err(ApiError::NotFound(format!("Agent not found for user {}", user_id)))
        }
    }

    // ========== Inbox Queries (T039) ==========

    pub async fn get_unassigned_conversations(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM conversations
             WHERE assigned_user_id IS NULL AND assigned_team_id IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get conversations
        let rows = sqlx::query(
            "SELECT * FROM conversations
             WHERE assigned_user_id IS NULL AND assigned_team_id IS NULL
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            let status_str: String = row.try_get("status")?;
            conversations.push(Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: None,
            });
        }

        Ok((conversations, total))
    }

    pub async fn get_team_conversations(
        &self,
        team_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM conversations WHERE assigned_team_id = ?",
        )
        .bind(team_id)
        .fetch_one(&self.pool)
        .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get conversations
        let rows = sqlx::query(
            "SELECT * FROM conversations
             WHERE assigned_team_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(team_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            let status_str: String = row.try_get("status")?;
            conversations.push(Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: None,
            });
        }

        Ok((conversations, total))
    }

    pub async fn get_user_assigned_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM conversations WHERE assigned_user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get conversations
        let rows = sqlx::query(
            "SELECT * FROM conversations
             WHERE assigned_user_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            let status_str: String = row.try_get("status")?;
            conversations.push(Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: None,
            });
        }

        Ok((conversations, total))
    }

    // ========== Batch Unassignment (T040) ==========

    pub async fn unassign_agent_open_conversations(&self, user_id: &str) -> ApiResult<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE conversations
             SET assigned_user_id = NULL, updated_at = ?
             WHERE assigned_user_id = ? AND status IN ('open', 'snoozed')",
        )
        .bind(&now)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        let count = result.rows_affected() as i64;
        tracing::info!(
            "Auto-unassigned {} open conversations for agent {}",
            count,
            user_id
        );
        Ok(count)
    }

    // ========== Get User Permissions ==========

    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        let rows = sqlx::query(
            "SELECT DISTINCT p.id, p.name, p.description, p.created_at, p.updated_at
             FROM permissions p
             INNER JOIN role_permissions rp ON p.id = rp.permission_id
             INNER JOIN user_roles ur ON rp.role_id = ur.role_id
             WHERE ur.user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut permissions = Vec::new();
        for row in rows {
            permissions.push(Permission {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(permissions)
    }

    // ========== Tag Operations (Feature 005) ==========

    /// Create a new tag
    pub async fn create_tag(&self, tag: &crate::models::Tag) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO tags (id, name, description, color, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&tag.id)
        .bind(&tag.name)
        .bind(&tag.description)
        .bind(&tag.color)
        .bind(&tag.created_at)
        .bind(&tag.updated_at)
        .execute(&self.pool)
        .await?;

        tracing::info!("Tag created: id={}, name={}", tag.id, tag.name);
        Ok(())
    }

    /// Get tag by ID
    pub async fn get_tag_by_id(&self, id: &str) -> ApiResult<Option<crate::models::Tag>> {
        let row = sqlx::query(
            "SELECT id, name, description, color, created_at, updated_at
             FROM tags
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get tag by name
    pub async fn get_tag_by_name(&self, name: &str) -> ApiResult<Option<crate::models::Tag>> {
        let row = sqlx::query(
            "SELECT id, name, description, color, created_at, updated_at
             FROM tags
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all tags with pagination
    pub async fn list_tags(&self, limit: i64, offset: i64) -> ApiResult<(Vec<crate::models::Tag>, i64)> {
        // Get total count
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM tags")
            .fetch_one(&self.pool)
            .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get tags
        let rows = sqlx::query(
            "SELECT id, name, description, color, created_at, updated_at
             FROM tags
             ORDER BY name ASC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(crate::models::Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok((tags, total))
    }

    /// Update tag properties (name is immutable)
    pub async fn update_tag(
        &self,
        id: &str,
        description: Option<String>,
        color: Option<String>,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE tags
             SET description = ?, color = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(description)
        .bind(color)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        tracing::info!("Tag updated: id={}", id);
        Ok(())
    }

    /// Delete tag (cascades to conversation_tags)
    pub async fn delete_tag(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        tracing::info!("Tag deleted: id={}", id);
        Ok(())
    }

    // ========== Conversation Tag Operations ==========

    /// Get all tags for a conversation
    pub async fn get_conversation_tags(&self, conversation_id: &str) -> ApiResult<Vec<crate::models::Tag>> {
        let rows = sqlx::query(
            "SELECT t.id, t.name, t.description, t.color, t.created_at, t.updated_at
             FROM tags t
             INNER JOIN conversation_tags ct ON t.id = ct.tag_id
             WHERE ct.conversation_id = ?
             ORDER BY ct.added_at DESC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(crate::models::Tag {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description").ok(),
                color: row.try_get("color").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(tags)
    }

    /// Add a tag to a conversation (idempotent)
    pub async fn add_conversation_tag(
        &self,
        conversation_id: &str,
        tag_id: &str,
        added_by: &str,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Use INSERT OR IGNORE for SQLite idempotency
        // This will silently ignore if the tag is already associated
        let result = sqlx::query(
            "INSERT OR IGNORE INTO conversation_tags (conversation_id, tag_id, added_by, added_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(tag_id)
        .bind(added_by)
        .bind(&now)
        .execute(&self.pool)
        .await;

        // For databases that don't support INSERT OR IGNORE, we can check if it exists first
        // But for now, we'll handle the error gracefully
        match result {
            Ok(_) => {
                tracing::debug!("Tag {} added to conversation {}", tag_id, conversation_id);
                Ok(())
            }
            Err(e) => {
                // If it's a unique constraint violation, treat as success (idempotent)
                if e.to_string().contains("UNIQUE") || e.to_string().contains("duplicate") {
                    tracing::debug!(
                        "Tag {} already associated with conversation {} (idempotent)",
                        tag_id,
                        conversation_id
                    );
                    Ok(())
                } else {
                    Err(ApiError::Internal(format!("Failed to add tag: {}", e)))
                }
            }
        }
    }

    /// Remove a tag from a conversation (idempotent)
    pub async fn remove_conversation_tag(&self, conversation_id: &str, tag_id: &str) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM conversation_tags
             WHERE conversation_id = ? AND tag_id = ?",
        )
        .bind(conversation_id)
        .bind(tag_id)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Tag {} removed from conversation {}", tag_id, conversation_id);
        Ok(())
    }

    /// Replace all conversation tags atomically
    pub async fn replace_conversation_tags(
        &self,
        conversation_id: &str,
        tag_ids: &[String],
        added_by: &str,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Delete all existing tags
        sqlx::query("DELETE FROM conversation_tags WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        // Insert new tags
        for tag_id in tag_ids {
            sqlx::query(
                "INSERT INTO conversation_tags (conversation_id, tag_id, added_by, added_at)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(conversation_id)
            .bind(tag_id)
            .bind(added_by)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        // Commit transaction
        tx.commit().await?;

        tracing::info!(
            "Replaced tags for conversation {}: {} tags",
            conversation_id,
            tag_ids.len()
        );
        Ok(())
    }

    /// Get conversations with a specific tag
    pub async fn get_conversations_by_tag(
        &self,
        tag_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        // Get total count
        let count_row = sqlx::query(
            "SELECT COUNT(DISTINCT c.id) as count
             FROM conversations c
             INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
             WHERE ct.tag_id = ?",
        )
        .bind(tag_id)
        .fetch_one(&self.pool)
        .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get conversations
        let rows = sqlx::query(
            "SELECT c.*
             FROM conversations c
             INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
             WHERE ct.tag_id = ?
             ORDER BY ct.added_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(tag_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for row in rows {
            let status_str: String = row.try_get("status")?;
            conversations.push(Conversation {
                id: row.try_get("id")?,
                reference_number: row.try_get("reference_number")?,
                status: ConversationStatus::from(status_str),
                inbox_id: row.try_get("inbox_id")?,
                contact_id: row.try_get("contact_id")?,
                subject: row.try_get("subject").ok(),
                resolved_at: row.try_get("resolved_at").ok(),
                snoozed_until: row.try_get("snoozed_until").ok(),
                assigned_user_id: row.try_get("assigned_user_id").ok(),
                assigned_team_id: row.try_get("assigned_team_id").ok(),
                assigned_at: row.try_get("assigned_at").ok(),
                assigned_by: row.try_get("assigned_by").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
                tags: None,
                priority: None,
            });
        }

        Ok((conversations, total))
    }

    /// Get conversations with multiple tags (AND or OR logic)
    pub async fn get_conversations_by_tags(
        &self,
        tag_ids: &[String],
        match_all: bool,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<Conversation>, i64)> {
        if tag_ids.is_empty() {
            return Ok((Vec::new(), 0));
        }

        if match_all {
            // AND logic: conversation must have ALL specified tags
            let placeholders = tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let tag_count = tag_ids.len() as i64;

            let count_query = format!(
                "SELECT COUNT(DISTINCT c.id) as count
                 FROM conversations c
                 WHERE (
                     SELECT COUNT(DISTINCT ct.tag_id)
                     FROM conversation_tags ct
                     WHERE ct.conversation_id = c.id
                     AND ct.tag_id IN ({})
                 ) = ?",
                placeholders
            );

            let mut count_query_builder = sqlx::query(&count_query);
            for tag_id in tag_ids {
                count_query_builder = count_query_builder.bind(tag_id);
            }
            count_query_builder = count_query_builder.bind(tag_count);

            let count_row = count_query_builder.fetch_one(&self.pool).await?;
            let total: i64 = count_row.try_get("count")?;

            let conversations_query = format!(
                "SELECT c.*
                 FROM conversations c
                 WHERE (
                     SELECT COUNT(DISTINCT ct.tag_id)
                     FROM conversation_tags ct
                     WHERE ct.conversation_id = c.id
                     AND ct.tag_id IN ({})
                 ) = ?
                 ORDER BY c.created_at DESC
                 LIMIT ? OFFSET ?",
                placeholders
            );

            let mut query_builder = sqlx::query(&conversations_query);
            for tag_id in tag_ids {
                query_builder = query_builder.bind(tag_id);
            }
            query_builder = query_builder.bind(tag_count).bind(limit).bind(offset);

            let rows = query_builder.fetch_all(&self.pool).await?;

            let mut conversations = Vec::new();
            for row in rows {
                let status_str: String = row.try_get("status")?;
                conversations.push(Conversation {
                    id: row.try_get("id")?,
                    reference_number: row.try_get("reference_number")?,
                    status: ConversationStatus::from(status_str),
                    inbox_id: row.try_get("inbox_id")?,
                    contact_id: row.try_get("contact_id")?,
                    subject: row.try_get("subject").ok(),
                    resolved_at: row.try_get("resolved_at").ok(),
                    snoozed_until: row.try_get("snoozed_until").ok(),
                    assigned_user_id: row.try_get("assigned_user_id").ok(),
                    assigned_team_id: row.try_get("assigned_team_id").ok(),
                    assigned_at: row.try_get("assigned_at").ok(),
                    assigned_by: row.try_get("assigned_by").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    version: row.try_get("version")?,
                    tags: None,
                    priority: None,
                });
            }

            Ok((conversations, total))
        } else {
            // OR logic: conversation has ANY of the specified tags
            let placeholders = tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

            let count_query = format!(
                "SELECT COUNT(DISTINCT c.id) as count
                 FROM conversations c
                 INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
                 WHERE ct.tag_id IN ({})",
                placeholders
            );

            let mut count_query_builder = sqlx::query(&count_query);
            for tag_id in tag_ids {
                count_query_builder = count_query_builder.bind(tag_id);
            }

            let count_row = count_query_builder.fetch_one(&self.pool).await?;
            let total: i64 = count_row.try_get("count")?;

            let conversations_query = format!(
                "SELECT DISTINCT c.*
                 FROM conversations c
                 INNER JOIN conversation_tags ct ON c.id = ct.conversation_id
                 WHERE ct.tag_id IN ({})
                 ORDER BY c.created_at DESC
                 LIMIT ? OFFSET ?",
                placeholders
            );

            let mut query_builder = sqlx::query(&conversations_query);
            for tag_id in tag_ids {
                query_builder = query_builder.bind(tag_id);
            }
            query_builder = query_builder.bind(limit).bind(offset);

            let rows = query_builder.fetch_all(&self.pool).await?;

            let mut conversations = Vec::new();
            for row in rows {
                let status_str: String = row.try_get("status")?;
                conversations.push(Conversation {
                    id: row.try_get("id")?,
                    reference_number: row.try_get("reference_number")?,
                    status: ConversationStatus::from(status_str),
                    inbox_id: row.try_get("inbox_id")?,
                    contact_id: row.try_get("contact_id")?,
                    subject: row.try_get("subject").ok(),
                    resolved_at: row.try_get("resolved_at").ok(),
                    snoozed_until: row.try_get("snoozed_until").ok(),
                    assigned_user_id: row.try_get("assigned_user_id").ok(),
                    assigned_team_id: row.try_get("assigned_team_id").ok(),
                    assigned_at: row.try_get("assigned_at").ok(),
                    assigned_by: row.try_get("assigned_by").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    version: row.try_get("version")?,
                    tags: None,
                    priority: None,
                });
            }

            Ok((conversations, total))
        }
    }

    // ========================================
    // Agent Availability Operations (Feature 006)
    // ========================================

    /// Update agent availability status with away_since logic
    pub async fn update_agent_availability_with_timestamp(
        &self,
        agent_id: &str,
        status: AgentAvailability,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Set away_since when transitioning to away/away_manual, clear otherwise
        let away_since = match status {
            AgentAvailability::Away | AgentAvailability::AwayManual => Some(now.clone()),
            _ => None,
        };

        sqlx::query(
            "UPDATE agents
             SET availability_status = ?,
                 away_since = ?,
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(status.to_string())
        .bind(away_since)
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update agent's last_activity_at timestamp
    pub async fn update_agent_activity(&self, agent_id: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE agents
             SET last_activity_at = ?
             WHERE id = ?",
        )
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update agent's last_login_at timestamp
    pub async fn update_agent_last_login(&self, agent_id: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE agents
             SET last_login_at = ?
             WHERE id = ?",
        )
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agents who are online but inactive beyond threshold
    pub async fn get_inactive_online_agents(
        &self,
        inactivity_threshold_seconds: i64,
    ) -> ApiResult<Vec<Agent>> {
        let threshold_time = chrono::Utc::now() - chrono::Duration::seconds(inactivity_threshold_seconds);
        let threshold_str = threshold_time.to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since
             FROM agents
             WHERE availability_status = ?
               AND last_activity_at IS NOT NULL
               AND last_activity_at < ?",
        )
        .bind("online")
        .bind(threshold_str)
        .fetch_all(&self.pool)
        .await?;

        let agents = rows
            .into_iter()
            .map(|row| {
                let status_str: String = row.try_get("availability_status").unwrap_or_else(|_| "offline".to_string());
                let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

                Ok(Agent {
                    id: row.try_get("id")?,
                    user_id: row.try_get("user_id")?,
                    first_name: row.try_get("first_name")?,
                    last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                    password_hash: row.try_get("password_hash")?,
                    availability_status: status,
                    last_login_at: row.try_get("last_login_at").ok(),
                    last_activity_at: row.try_get("last_activity_at").ok(),
                    away_since: row.try_get("away_since").ok(),
                    api_key: None,
                    api_secret_hash: None,
                    api_key_description: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    api_key_revoked_at: None,
                })
            })
            .collect::<ApiResult<Vec<Agent>>>()?;

        Ok(agents)
    }

    /// Get agents who are away/away_manual and idle beyond threshold
    pub async fn get_idle_away_agents(
        &self,
        max_idle_threshold_seconds: i64,
    ) -> ApiResult<Vec<Agent>> {
        let threshold_time = chrono::Utc::now() - chrono::Duration::seconds(max_idle_threshold_seconds);
        let threshold_str = threshold_time.to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since
             FROM agents
             WHERE availability_status IN (?, ?)
               AND away_since IS NOT NULL
               AND away_since < ?",
        )
        .bind("away")
        .bind("away_manual")
        .bind(threshold_str)
        .fetch_all(&self.pool)
        .await?;

        let agents = rows
            .into_iter()
            .map(|row| {
                let status_str: String = row.try_get("availability_status").unwrap_or_else(|_| "offline".to_string());
                let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

                Ok(Agent {
                    id: row.try_get("id")?,
                    user_id: row.try_get("user_id")?,
                    first_name: row.try_get("first_name")?,
                    last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                    password_hash: row.try_get("password_hash")?,
                    availability_status: status,
                    last_login_at: row.try_get("last_login_at").ok(),
                    last_activity_at: row.try_get("last_activity_at").ok(),
                    away_since: row.try_get("away_since").ok(),
                    api_key: None,
                    api_secret_hash: None,
                    api_key_description: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    api_key_revoked_at: None,
                })
            })
            .collect::<ApiResult<Vec<Agent>>>()?;

        Ok(agents)
    }

    // ========================================
    // Agent Activity Log Operations
    // ========================================

    /// Create activity log entry
    pub async fn create_activity_log(&self, log: &AgentActivityLog) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO agent_activity_logs
             (id, agent_id, event_type, old_status, new_status, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.agent_id)
        .bind(log.event_type.to_string())
        .bind(&log.old_status)
        .bind(&log.new_status)
        .bind(&log.metadata)
        .bind(&log.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agent activity logs (paginated)
    pub async fn get_agent_activity_logs(
        &self,
        agent_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<AgentActivityLog>, i64)> {
        // Get total count
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM agent_activity_logs WHERE agent_id = ?")
            .bind(agent_id)
            .fetch_one(&self.pool)
            .await?;
        let total: i64 = count_row.try_get("count")?;

        // Get logs
        let rows = sqlx::query(
            "SELECT id, agent_id, event_type, old_status, new_status, metadata, created_at
             FROM agent_activity_logs
             WHERE agent_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(agent_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let logs = rows
            .into_iter()
            .map(|row| {
                let event_type_str: String = row.try_get("event_type")?;
                let event_type = event_type_str.parse().unwrap_or(ActivityEventType::AvailabilityChanged);

                Ok(AgentActivityLog {
                    id: row.try_get("id")?,
                    agent_id: row.try_get("agent_id")?,
                    event_type,
                    old_status: row.try_get("old_status").ok(),
                    new_status: row.try_get("new_status").ok(),
                    metadata: row.try_get("metadata").ok(),
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect::<ApiResult<Vec<AgentActivityLog>>>()?;

        Ok((logs, total))
    }

    // ========================================
    // System Configuration Operations
    // ========================================

    /// Get configuration value by key
    pub async fn get_config_value(&self, key: &str) -> ApiResult<Option<String>> {
        let row = sqlx::query("SELECT value FROM system_config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(row.try_get("value")?))
        } else {
            Ok(None)
        }
    }

    /// Set configuration value
    pub async fn set_config_value(&self, key: &str, value: &str, description: Option<&str>) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO system_config (key, value, description, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET
                 value = excluded.value,
                 description = COALESCE(excluded.description, description),
                 updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(description)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================
    // SLA Policy Operations
    // ========================================

    /// Create a new SLA policy
    pub async fn create_sla_policy(&self, policy: &crate::models::SlaPolicy) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO sla_policies (id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&policy.id)
        .bind(&policy.name)
        .bind(&policy.description)
        .bind(&policy.first_response_time)
        .bind(&policy.resolution_time)
        .bind(&policy.next_response_time)
        .bind(&policy.created_at)
        .bind(&policy.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get SLA policy by ID
    pub async fn get_sla_policy(&self, id: &str) -> ApiResult<Option<crate::models::SlaPolicy>> {
        let row = sqlx::query(
            "SELECT id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at
             FROM sla_policies WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::SlaPolicy {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                first_response_time: row.try_get("first_response_time")?,
                resolution_time: row.try_get("resolution_time")?,
                next_response_time: row.try_get("next_response_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get SLA policy by name
    pub async fn get_sla_policy_by_name(&self, name: &str) -> ApiResult<Option<crate::models::SlaPolicy>> {
        let row = sqlx::query(
            "SELECT id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at
             FROM sla_policies WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::SlaPolicy {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                first_response_time: row.try_get("first_response_time")?,
                resolution_time: row.try_get("resolution_time")?,
                next_response_time: row.try_get("next_response_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all SLA policies with pagination
    pub async fn list_sla_policies(&self, limit: i64, offset: i64) -> ApiResult<(Vec<crate::models::SlaPolicy>, i64)> {
        let rows = sqlx::query(
            "SELECT id, name, description, first_response_time, resolution_time, next_response_time, created_at, updated_at
             FROM sla_policies
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let policies: Vec<crate::models::SlaPolicy> = rows.iter().map(|row| {
            Ok(crate::models::SlaPolicy {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                first_response_time: row.try_get("first_response_time")?,
                resolution_time: row.try_get("resolution_time")?,
                next_response_time: row.try_get("next_response_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }).collect::<ApiResult<Vec<_>>>()?;

        let count_row = sqlx::query("SELECT COUNT(*) as count FROM sla_policies")
            .fetch_one(&self.pool)
            .await?;
        let total: i64 = count_row.try_get("count")?;

        Ok((policies, total))
    }

    /// Update SLA policy
    pub async fn update_sla_policy(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
        first_response_time: Option<&str>,
        resolution_time: Option<&str>,
        next_response_time: Option<&str>,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        let mut query_parts = Vec::new();
        let mut bindings: Vec<String> = Vec::new();

        if let Some(name) = name {
            query_parts.push("name = ?");
            bindings.push(name.to_string());
        }

        if let Some(desc) = description {
            query_parts.push("description = ?");
            bindings.push(desc.map(|s| s.to_string()).unwrap_or_default());
        }

        if let Some(time) = first_response_time {
            query_parts.push("first_response_time = ?");
            bindings.push(time.to_string());
        }

        if let Some(time) = resolution_time {
            query_parts.push("resolution_time = ?");
            bindings.push(time.to_string());
        }

        if let Some(time) = next_response_time {
            query_parts.push("next_response_time = ?");
            bindings.push(time.to_string());
        }

        if query_parts.is_empty() {
            return Ok(());
        }

        query_parts.push("updated_at = ?");
        bindings.push(now.clone());

        let query_str = format!(
            "UPDATE sla_policies SET {} WHERE id = ?",
            query_parts.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for binding in bindings {
            query = query.bind(binding);
        }
        query = query.bind(id);

        query.execute(&self.pool).await?;

        Ok(())
    }

    /// Delete SLA policy
    pub async fn delete_sla_policy(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM sla_policies WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================
    // Applied SLA Operations
    // ========================================

    /// Create a new applied SLA
    pub async fn create_applied_sla(&self, applied_sla: &crate::models::AppliedSla) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO applied_slas (id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&applied_sla.id)
        .bind(&applied_sla.conversation_id)
        .bind(&applied_sla.sla_policy_id)
        .bind(applied_sla.status.to_string())
        .bind(&applied_sla.first_response_deadline_at)
        .bind(&applied_sla.resolution_deadline_at)
        .bind(&applied_sla.applied_at)
        .bind(&applied_sla.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get applied SLA by ID
    pub async fn get_applied_sla(&self, id: &str) -> ApiResult<Option<crate::models::AppliedSla>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
             FROM applied_slas WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::models::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get applied SLA by conversation ID
    pub async fn get_applied_sla_by_conversation(&self, conversation_id: &str) -> ApiResult<Option<crate::models::AppliedSla>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
             FROM applied_slas WHERE conversation_id = ?"
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::models::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get applied SLA by ID
    pub async fn get_applied_sla_by_id(&self, id: &str) -> ApiResult<Option<crate::models::AppliedSla>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
             FROM applied_slas WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::models::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List applied SLAs with optional filters
    pub async fn list_applied_slas(
        &self,
        status_filter: Option<crate::models::AppliedSlaStatus>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<(Vec<crate::models::AppliedSla>, i64)> {
        let (query_str, count_query_str) = if status_filter.is_some() {
            (
                "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
                 FROM applied_slas WHERE status = ? ORDER BY applied_at DESC LIMIT ? OFFSET ?",
                "SELECT COUNT(*) as count FROM applied_slas WHERE status = ?"
            )
        } else {
            (
                "SELECT id, conversation_id, sla_policy_id, status, first_response_deadline_at, resolution_deadline_at, applied_at, updated_at
                 FROM applied_slas ORDER BY applied_at DESC LIMIT ? OFFSET ?",
                "SELECT COUNT(*) as count FROM applied_slas"
            )
        };

        let rows = if let Some(status) = status_filter {
            sqlx::query(query_str)
                .bind(status.to_string())
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query_str)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };

        let applied_slas: Vec<crate::models::AppliedSla> = rows.iter().map(|row| {
            let status_str: String = row.try_get("status")?;
            Ok(crate::models::AppliedSla {
                id: row.try_get("id")?,
                conversation_id: row.try_get("conversation_id")?,
                sla_policy_id: row.try_get("sla_policy_id")?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                first_response_deadline_at: row.try_get("first_response_deadline_at")?,
                resolution_deadline_at: row.try_get("resolution_deadline_at")?,
                applied_at: row.try_get("applied_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }).collect::<ApiResult<Vec<_>>>()?;

        let count_row = if let Some(status) = status_filter {
            sqlx::query(count_query_str)
                .bind(status.to_string())
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query(count_query_str)
                .fetch_one(&self.pool)
                .await?
        };

        let total: i64 = count_row.try_get("count")?;

        Ok((applied_slas, total))
    }

    /// Update applied SLA status
    pub async fn update_applied_sla_status(&self, id: &str, status: crate::models::AppliedSlaStatus) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE applied_slas SET status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(status.to_string())
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete applied SLA
    pub async fn delete_applied_sla(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM applied_slas WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================
    // SLA Event Operations
    // ========================================

    /// Create a new SLA event
    pub async fn create_sla_event(&self, event: &crate::models::SlaEvent) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO sla_events (id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&event.id)
        .bind(&event.applied_sla_id)
        .bind(event.event_type.to_string())
        .bind(event.status.to_string())
        .bind(&event.deadline_at)
        .bind(&event.met_at)
        .bind(&event.breached_at)
        .bind(&event.created_at)
        .bind(&event.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get SLA event by ID
    pub async fn get_sla_event(&self, id: &str) -> ApiResult<Option<crate::models::SlaEvent>> {
        let row = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::models::SlaEvent {
                id: row.try_get("id")?,
                applied_sla_id: row.try_get("applied_sla_id")?,
                event_type: event_type_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                deadline_at: row.try_get("deadline_at")?,
                met_at: row.try_get("met_at")?,
                breached_at: row.try_get("breached_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all SLA events for an applied SLA
    pub async fn get_sla_events_by_applied_sla(&self, applied_sla_id: &str) -> ApiResult<Vec<crate::models::SlaEvent>> {
        let rows = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE applied_sla_id = ? ORDER BY created_at ASC"
        )
        .bind(applied_sla_id)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows.iter() {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;

            let event_type = event_type_str.parse().map_err(|e: String| {
                crate::api::middleware::ApiError::Internal(format!("Invalid event_type: {}", e))
            })?;

            let status = status_str.parse().map_err(|e: String| {
                crate::api::middleware::ApiError::Internal(format!("Invalid status: {}", e))
            })?;

            let id: String = row.try_get("id")?;
            let applied_sla_id: String = row.try_get("applied_sla_id")?;
            let deadline_at: String = row.try_get("deadline_at")?;
            // For nullable columns, try_get may fail with NULL values in sqlx Any driver
            // Use a workaround to handle this
            let met_at: Option<String> = row.try_get::<Option<String>, _>("met_at")
                .or_else(|_| Ok::<_, sqlx::Error>(None))?;
            let breached_at: Option<String> = row.try_get::<Option<String>, _>("breached_at")
                .or_else(|_| Ok::<_, sqlx::Error>(None))?;
            let created_at: String = row.try_get("created_at")?;
            let updated_at: String = row.try_get("updated_at")?;

            events.push(crate::models::SlaEvent {
                id,
                applied_sla_id,
                event_type,
                status,
                deadline_at,
                met_at,
                breached_at,
                created_at,
                updated_at,
            });
        }

        Ok(events)
    }

    /// Get pending SLA event by type for an applied SLA
    pub async fn get_pending_sla_event(
        &self,
        applied_sla_id: &str,
        event_type: crate::models::SlaEventType,
    ) -> ApiResult<Option<crate::models::SlaEvent>> {
        let row = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE applied_sla_id = ? AND event_type = ? AND status = 'pending'"
        )
        .bind(applied_sla_id)
        .bind(event_type.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;
            Ok(Some(crate::models::SlaEvent {
                id: row.try_get("id")?,
                applied_sla_id: row.try_get("applied_sla_id")?,
                event_type: event_type_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                status: status_str.parse().map_err(|e: String| {
                    sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                })?,
                deadline_at: row.try_get("deadline_at")?,
                // For nullable columns, try_get may fail with NULL values in sqlx Any driver
                met_at: row.try_get::<Option<String>, _>("met_at").or_else(|_| Ok::<_, sqlx::Error>(None))?,
                breached_at: row.try_get::<Option<String>, _>("breached_at").or_else(|_| Ok::<_, sqlx::Error>(None))?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all pending SLA events past their deadline
    pub async fn get_pending_events_past_deadline(&self) -> ApiResult<Vec<crate::models::SlaEvent>> {
        let now = chrono::Utc::now().to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, applied_sla_id, event_type, status, deadline_at, met_at, breached_at, created_at, updated_at
             FROM sla_events WHERE status = 'pending' AND deadline_at < ? ORDER BY deadline_at ASC"
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows.iter() {
            let event_type_str: String = row.try_get("event_type")?;
            let status_str: String = row.try_get("status")?;

            let event_type = event_type_str.parse().map_err(|e: String| {
                crate::api::middleware::ApiError::Internal(format!("Invalid event_type: {}", e))
            })?;

            let status = status_str.parse().map_err(|e: String| {
                crate::api::middleware::ApiError::Internal(format!("Invalid status: {}", e))
            })?;

            events.push(crate::models::SlaEvent {
                id: row.try_get("id")?,
                applied_sla_id: row.try_get("applied_sla_id")?,
                event_type,
                status,
                deadline_at: row.try_get("deadline_at")?,
                // For nullable columns, try_get may fail with NULL values in sqlx Any driver
                met_at: row.try_get::<Option<String>, _>("met_at").or_else(|_| Ok::<_, sqlx::Error>(None))?,
                breached_at: row.try_get::<Option<String>, _>("breached_at").or_else(|_| Ok::<_, sqlx::Error>(None))?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(events)
    }

    /// Mark SLA event as met
    pub async fn mark_sla_event_met(&self, event_id: &str, met_at: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE sla_events SET status = 'met', met_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind(met_at)
        .bind(now)
        .bind(event_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark SLA event as breached
    pub async fn mark_sla_event_breached(&self, event_id: &str, breached_at: &str) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE sla_events SET status = 'breached', breached_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind(breached_at)
        .bind(now)
        .bind(event_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete SLA event
    pub async fn delete_sla_event(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM sla_events WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Automation Rules CRUD operations

    /// Create automation rule
    pub async fn create_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()> {
        let event_subscription_json = serde_json::to_string(&rule.event_subscription)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize event_subscription: {}", e)))?;
        let condition_json = serde_json::to_string(&rule.condition)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize condition: {}", e)))?;
        let action_json = serde_json::to_string(&rule.action)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize action: {}", e)))?;

        sqlx::query(
            "INSERT INTO automation_rules (id, name, description, enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(rule.enabled)
        .bind(rule.rule_type.to_string())
        .bind(&event_subscription_json)
        .bind(&condition_json)
        .bind(&action_json)
        .bind(rule.priority)
        .bind(&rule.created_at)
        .bind(&rule.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get automation rule by ID
    pub async fn get_automation_rule_by_id(&self, id: &str) -> ApiResult<Option<AutomationRule>> {
        let row = sqlx::query(
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            eprintln!("Database error fetching automation rule: {:?}", e);
            ApiError::Internal(format!("Database error: {}", e))
        })?;

        if let Some(row) = row {
            let rule_type_str: String = row.try_get("rule_type")?;
            let rule_type = rule_type_str.parse::<RuleType>()
                .map_err(|e| ApiError::Internal(format!("Failed to parse rule_type '{}': {}", rule_type_str, e)))?;

            let event_subscription_str: String = row.try_get("event_subscription")?;
            let event_subscription: Vec<String> = serde_json::from_str(&event_subscription_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize event_subscription: {}", e)))?;

            let condition_str: String = row.try_get("condition")?;
            let condition: RuleCondition = serde_json::from_str(&condition_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize condition: {}", e)))?;

            let action_str: String = row.try_get("action")?;
            let action: RuleAction = serde_json::from_str(&action_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize action: {}", e)))?;

            Ok(Some(AutomationRule {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                enabled: {
                    let enabled_int: i32 = row.try_get("enabled")?;
                    enabled_int != 0
                },
                rule_type,
                event_subscription,
                condition,
                action,
                priority: row.try_get("priority")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get automation rule by name
    pub async fn get_automation_rule_by_name(&self, name: &str) -> ApiResult<Option<AutomationRule>> {
        let row = sqlx::query(
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let rule_type_str: String = row.try_get("rule_type")?;
            let rule_type = rule_type_str.parse::<RuleType>()
                .map_err(|e| ApiError::Internal(format!("Failed to parse rule_type '{}': {}", rule_type_str, e)))?;

            let event_subscription_str: String = row.try_get("event_subscription")?;
            let event_subscription: Vec<String> = serde_json::from_str(&event_subscription_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize event_subscription: {}", e)))?;

            let condition_str: String = row.try_get("condition")?;
            let condition: RuleCondition = serde_json::from_str(&condition_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize condition: {}", e)))?;

            let action_str: String = row.try_get("action")?;
            let action: RuleAction = serde_json::from_str(&action_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize action: {}", e)))?;

            Ok(Some(AutomationRule {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                enabled: {
                    let enabled_int: i32 = row.try_get("enabled")?;
                    enabled_int != 0
                },
                rule_type,
                event_subscription,
                condition,
                action,
                priority: row.try_get("priority")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all automation rules (with optional enabled filter)
    pub async fn get_automation_rules(&self, enabled_only: bool) -> ApiResult<Vec<AutomationRule>> {
        let query = if enabled_only {
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             WHERE enabled = TRUE
             ORDER BY priority ASC, created_at ASC"
        } else {
            "SELECT id, name, description, CAST(enabled AS INTEGER) as enabled, rule_type, event_subscription, condition, action, priority, created_at, updated_at
             FROM automation_rules
             ORDER BY priority ASC, created_at ASC"
        };

        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let mut rules = Vec::new();
        for row in rows {
            let rule_type_str: String = row.try_get("rule_type")?;
            let rule_type = rule_type_str.parse::<RuleType>()
                .map_err(|e| ApiError::Internal(format!("Failed to parse rule_type '{}': {}", rule_type_str, e)))?;

            let event_subscription_str: String = row.try_get("event_subscription")?;
            let event_subscription: Vec<String> = serde_json::from_str(&event_subscription_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize event_subscription: {}", e)))?;

            let condition_str: String = row.try_get("condition")?;
            let condition: RuleCondition = serde_json::from_str(&condition_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize condition: {}", e)))?;

            let action_str: String = row.try_get("action")?;
            let action: RuleAction = serde_json::from_str(&action_str)
                .map_err(|e| ApiError::Internal(format!("Failed to deserialize action: {}", e)))?;

            rules.push(AutomationRule {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                enabled: {
                    let enabled_int: i32 = row.try_get("enabled")?;
                    enabled_int != 0
                },
                rule_type,
                event_subscription,
                condition,
                action,
                priority: row.try_get("priority")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(rules)
    }

    /// Get enabled rules that subscribe to a specific event
    pub async fn get_enabled_rules_for_event(&self, event_type: &str) -> ApiResult<Vec<AutomationRule>> {
        // Get all enabled rules
        let all_rules = self.get_automation_rules(true).await?;

        // Filter by event subscription
        let matching_rules: Vec<AutomationRule> = all_rules
            .into_iter()
            .filter(|rule| rule.event_subscription.contains(&event_type.to_string()))
            .collect();

        Ok(matching_rules)
    }

    /// Update automation rule
    pub async fn update_automation_rule(&self, rule: &AutomationRule) -> ApiResult<()> {
        let event_subscription_json = serde_json::to_string(&rule.event_subscription)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize event_subscription: {}", e)))?;
        let condition_json = serde_json::to_string(&rule.condition)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize condition: {}", e)))?;
        let action_json = serde_json::to_string(&rule.action)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize action: {}", e)))?;

        sqlx::query(
            "UPDATE automation_rules
             SET name = ?, description = ?, enabled = ?, rule_type = ?, event_subscription = ?, condition = ?, action = ?, priority = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(rule.enabled)
        .bind(rule.rule_type.to_string())
        .bind(&event_subscription_json)
        .bind(&condition_json)
        .bind(&action_json)
        .bind(rule.priority)
        .bind(&rule.updated_at)
        .bind(&rule.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete automation rule
    pub async fn delete_automation_rule(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM automation_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Enable automation rule
    pub async fn enable_automation_rule(&self, id: &str) -> ApiResult<()> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE automation_rules SET enabled = TRUE, updated_at = ? WHERE id = ?",
        )
        .bind(&updated_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Disable automation rule
    pub async fn disable_automation_rule(&self, id: &str) -> ApiResult<()> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE automation_rules SET enabled = FALSE, updated_at = ? WHERE id = ?",
        )
        .bind(&updated_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Rule Evaluation Logs CRUD operations

    /// Create rule evaluation log
    pub async fn create_rule_evaluation_log(&self, log: &RuleEvaluationLog) -> ApiResult<()> {
        let condition_result_str = log.condition_result.as_ref().map(|r| r.to_string());
        let action_result_str = log.action_result.as_ref().map(|r| r.to_string());

        sqlx::query(
            "INSERT INTO rule_evaluation_logs (id, rule_id, rule_name, event_type, conversation_id, matched, condition_result, action_executed, action_result, error_message, evaluation_time_ms, evaluated_at, cascade_depth)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.rule_id)
        .bind(&log.rule_name)
        .bind(&log.event_type)
        .bind(&log.conversation_id)
        .bind(log.matched)
        .bind(&condition_result_str)
        .bind(log.action_executed)
        .bind(&action_result_str)
        .bind(&log.error_message)
        .bind(log.evaluation_time_ms)
        .bind(&log.evaluated_at)
        .bind(log.cascade_depth as i32)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get rule evaluation logs with optional filters
    pub async fn get_rule_evaluation_logs(
        &self,
        rule_id: Option<&str>,
        conversation_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        let mut query = String::from(
            "SELECT id, rule_id, rule_name, event_type, conversation_id, CAST(matched AS INTEGER) as matched, condition_result, CAST(action_executed AS INTEGER) as action_executed, action_result, error_message, evaluation_time_ms, evaluated_at, cascade_depth
             FROM rule_evaluation_logs
             WHERE 1=1"
        );

        let mut params: Vec<String> = Vec::new();

        if rule_id.is_some() {
            query.push_str(" AND rule_id = ?");
            params.push(rule_id.unwrap().to_string());
        }

        if conversation_id.is_some() {
            query.push_str(" AND conversation_id = ?");
            params.push(conversation_id.unwrap().to_string());
        }

        if event_type.is_some() {
            query.push_str(" AND event_type = ?");
            params.push(event_type.unwrap().to_string());
        }

        query.push_str(" ORDER BY evaluated_at DESC");

        if limit.is_some() {
            query.push_str(" LIMIT ?");
            params.push(limit.unwrap().to_string());
        }

        if offset.is_some() {
            query.push_str(" OFFSET ?");
            params.push(offset.unwrap().to_string());
        }

        let mut sql_query = sqlx::query(&query);
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut logs = Vec::new();
        for row in rows {
            let condition_result_str: Option<String> = row.try_get("condition_result")?;
            let condition_result = condition_result_str
                .and_then(|s| s.parse::<ConditionResult>().ok());

            let action_result_str: Option<String> = row.try_get("action_result")?;
            let action_result = action_result_str
                .and_then(|s| s.parse::<ActionResult>().ok());

            let cascade_depth: i32 = row.try_get("cascade_depth")?;

            // SQLite stores BOOLEAN as INTEGER
            let matched: i32 = row.try_get("matched")?;
            let matched = matched != 0;

            let action_executed: i32 = row.try_get("action_executed")?;
            let action_executed = action_executed != 0;

            logs.push(RuleEvaluationLog {
                id: row.try_get("id")?,
                rule_id: row.try_get("rule_id")?,
                rule_name: row.try_get("rule_name")?,
                event_type: row.try_get("event_type")?,
                conversation_id: row.try_get::<Option<String>, _>("conversation_id").ok().flatten(),
                matched,
                condition_result,
                action_executed,
                action_result,
                error_message: row.try_get::<Option<String>, _>("error_message").ok().flatten(),
                evaluation_time_ms: row.try_get("evaluation_time_ms")?,
                evaluated_at: row.try_get("evaluated_at")?,
                cascade_depth: cascade_depth as u32,
            });
        }

        Ok(logs)
    }

    /// Get evaluation logs for a specific rule
    pub async fn get_evaluation_logs_by_rule(&self, rule_id: &str) -> ApiResult<Vec<RuleEvaluationLog>> {
        self.get_rule_evaluation_logs(Some(rule_id), None, None, None, None).await
    }

    /// Get evaluation logs for a specific conversation
    pub async fn get_evaluation_logs_by_conversation(&self, conversation_id: &str) -> ApiResult<Vec<RuleEvaluationLog>> {
        self.get_rule_evaluation_logs(None, Some(conversation_id), None, None, None).await
    }

    /// Get evaluation logs for a specific rule
    pub async fn get_rule_evaluation_logs_by_rule(
        &self,
        rule_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<RuleEvaluationLog>> {
        self.get_rule_evaluation_logs(Some(rule_id), None, None, Some(limit), Some(offset))
            .await
    }

    // ===== Macro Operations =====

    pub async fn create_macro(&self, macro_obj: &Macro) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macros (id, name, message_content, created_by, created_at, updated_at, usage_count, access_control)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&macro_obj.id)
        .bind(&macro_obj.name)
        .bind(&macro_obj.message_content)
        .bind(&macro_obj.created_by)
        .bind(&macro_obj.created_at)
        .bind(&macro_obj.updated_at)
        .bind(macro_obj.usage_count)
        .bind(&macro_obj.access_control)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_by_id(&self, id: &str) -> ApiResult<Option<Macro>> {
        let row = sqlx::query(
            "SELECT id, name, message_content, created_by, created_at, updated_at, usage_count, access_control
             FROM macros
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Macro {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                message_content: row.try_get("message_content")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                usage_count: row.try_get("usage_count")?,
                access_control: row.try_get("access_control")?,
                actions: None, // Actions loaded separately
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_macro_by_name(&self, name: &str) -> ApiResult<Option<Macro>> {
        let row = sqlx::query(
            "SELECT id, name, message_content, created_by, created_at, updated_at, usage_count, access_control
             FROM macros
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Macro {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                message_content: row.try_get("message_content")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                usage_count: row.try_get("usage_count")?,
                access_control: row.try_get("access_control")?,
                actions: None, // Actions loaded separately
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_macros(&self) -> ApiResult<Vec<Macro>> {
        let rows = sqlx::query(
            "SELECT id, name, message_content, created_by, created_at, updated_at, usage_count, access_control
             FROM macros
             ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut macros = Vec::new();
        for row in rows {
            macros.push(Macro {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                message_content: row.try_get("message_content")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                usage_count: row.try_get("usage_count")?,
                access_control: row.try_get("access_control")?,
                actions: None, // Actions loaded separately
            });
        }

        Ok(macros)
    }

    pub async fn update_macro(&self, macro_obj: &Macro) -> ApiResult<()> {
        sqlx::query(
            "UPDATE macros
             SET name = ?, message_content = ?, updated_at = ?, access_control = ?
             WHERE id = ?",
        )
        .bind(&macro_obj.name)
        .bind(&macro_obj.message_content)
        .bind(&macro_obj.updated_at)
        .bind(&macro_obj.access_control)
        .bind(&macro_obj.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_macro(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM macros WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn increment_macro_usage(&self, id: &str) -> ApiResult<()> {
        sqlx::query("UPDATE macros SET usage_count = usage_count + 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Macro Action Operations =====

    pub async fn create_macro_action(&self, action: &MacroAction) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macro_actions (id, macro_id, action_type, action_value, action_order)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&action.id)
        .bind(&action.macro_id)
        .bind(&action.action_type)
        .bind(&action.action_value)
        .bind(action.action_order)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_actions(&self, macro_id: &str) -> ApiResult<Vec<MacroAction>> {
        let rows = sqlx::query(
            "SELECT id, macro_id, action_type, action_value, action_order
             FROM macro_actions
             WHERE macro_id = ?
             ORDER BY action_order ASC",
        )
        .bind(macro_id)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(MacroAction {
                id: row.try_get("id")?,
                macro_id: row.try_get("macro_id")?,
                action_type: row.try_get("action_type")?,
                action_value: row.try_get("action_value")?,
                action_order: row.try_get("action_order")?,
            });
        }

        Ok(actions)
    }

    pub async fn delete_macro_actions(&self, macro_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM macro_actions WHERE macro_id = ?")
            .bind(macro_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Macro Access Operations =====

    pub async fn create_macro_access(&self, access: &MacroAccess) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macro_access (id, macro_id, entity_type, entity_id, granted_at, granted_by)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&access.id)
        .bind(&access.macro_id)
        .bind(&access.entity_type)
        .bind(&access.entity_id)
        .bind(&access.granted_at)
        .bind(&access.granted_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_access(&self, macro_id: &str) -> ApiResult<Vec<MacroAccess>> {
        let rows = sqlx::query(
            "SELECT id, macro_id, entity_type, entity_id, granted_at, granted_by
             FROM macro_access
             WHERE macro_id = ?",
        )
        .bind(macro_id)
        .fetch_all(&self.pool)
        .await?;

        let mut accesses = Vec::new();
        for row in rows {
            accesses.push(MacroAccess {
                id: row.try_get("id")?,
                macro_id: row.try_get("macro_id")?,
                entity_type: row.try_get("entity_type")?,
                entity_id: row.try_get("entity_id")?,
                granted_at: row.try_get("granted_at")?,
                granted_by: row.try_get("granted_by")?,
            });
        }

        Ok(accesses)
    }

    pub async fn delete_macro_access(
        &self,
        macro_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM macro_access
             WHERE macro_id = ? AND entity_type = ? AND entity_id = ?",
        )
        .bind(macro_id)
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn user_has_macro_access(
        &self,
        macro_id: &str,
        user_id: &str,
    ) -> ApiResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM macro_access
             WHERE macro_id = ? AND entity_type = 'user' AND entity_id = ?",
        )
        .bind(macro_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i32 = row.try_get("count")?;
        Ok(count > 0)
    }

    pub async fn team_has_macro_access(
        &self,
        macro_id: &str,
        team_id: &str,
    ) -> ApiResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM macro_access
             WHERE macro_id = ? AND entity_type = 'team' AND entity_id = ?",
        )
        .bind(macro_id)
        .bind(team_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i32 = row.try_get("count")?;
        Ok(count > 0)
    }

    // ===== Macro Application Log Operations =====

    pub async fn create_macro_application_log(&self, log: &MacroApplicationLog) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macro_application_logs (id, macro_id, agent_id, conversation_id, applied_at, actions_queued, variables_replaced)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.macro_id)
        .bind(&log.agent_id)
        .bind(&log.conversation_id)
        .bind(&log.applied_at)
        .bind(&log.actions_queued)
        .bind(log.variables_replaced)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_application_logs(
        &self,
        macro_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<MacroApplicationLog>> {
        let rows = sqlx::query(
            "SELECT id, macro_id, agent_id, conversation_id, applied_at, actions_queued, variables_replaced
             FROM macro_application_logs
             WHERE macro_id = ?
             ORDER BY applied_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(macro_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(MacroApplicationLog {
                id: row.try_get("id")?,
                macro_id: row.try_get("macro_id")?,
                agent_id: row.try_get("agent_id")?,
                conversation_id: row.try_get("conversation_id")?,
                applied_at: row.try_get("applied_at")?,
                actions_queued: row.try_get("actions_queued")?,
                variables_replaced: row.try_get("variables_replaced")?,
            });
        }

        Ok(logs)
    }

    // ========== Notification Operations (T009-T012, T024-T027) ==========

    pub async fn create_notification(&self, notification: &UserNotification) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO user_notifications (id, user_id, type, created_at, is_read, conversation_id, message_id, actor_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&notification.id)
        .bind(&notification.user_id)
        .bind(notification.notification_type.as_str())
        .bind(&notification.created_at)
        .bind(if notification.is_read { 1 } else { 0 })
        .bind(&notification.conversation_id)
        .bind(&notification.message_id)
        .bind(&notification.actor_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_notification_by_id(&self, id: &str) -> ApiResult<Option<UserNotification>> {
        let row = sqlx::query(
            "SELECT id, user_id, type, created_at, is_read, conversation_id, message_id, actor_id
             FROM user_notifications
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let notification_type_str: String = row.try_get("type")?;
            let is_read_int: i32 = row.try_get("is_read")?;

            Ok(Some(UserNotification {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                notification_type: NotificationType::from(notification_type_str),
                created_at: row.try_get("created_at")?,
                is_read: is_read_int != 0,
                conversation_id: row.try_get("conversation_id").ok(),
                message_id: row.try_get("message_id").ok(),
                actor_id: row.try_get("actor_id").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_notifications(
        &self,
        user_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<UserNotification>> {
        let rows = sqlx::query(
            "SELECT id, user_id, type, created_at, is_read, conversation_id, message_id, actor_id
             FROM user_notifications
             WHERE user_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut notifications = Vec::new();
        for row in rows {
            let notification_type_str: String = row.try_get("type")?;
            let is_read_int: i32 = row.try_get("is_read")?;

            notifications.push(UserNotification {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                notification_type: NotificationType::from(notification_type_str),
                created_at: row.try_get("created_at")?,
                is_read: is_read_int != 0,
                conversation_id: row.try_get("conversation_id").ok(),
                message_id: row.try_get("message_id").ok(),
                actor_id: row.try_get("actor_id").ok(),
            });
        }

        Ok(notifications)
    }

    pub async fn mark_notification_as_read(&self, id: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE user_notifications
             SET is_read = 1
             WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn mark_all_notifications_as_read(&self, user_id: &str) -> ApiResult<i32> {
        let result = sqlx::query(
            "UPDATE user_notifications
             SET is_read = 1
             WHERE user_id = ? AND is_read = 0",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i32)
    }

    pub async fn get_unread_count(&self, user_id: &str) -> ApiResult<i32> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM user_notifications
             WHERE user_id = ? AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i32 = row.try_get("count")?;
        Ok(count)
    }

    pub async fn delete_old_notifications(&self, older_than_days: i32) -> ApiResult<i32> {
        // Calculate the cutoff timestamp
        let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(older_than_days as i64);
        let cutoff_str = cutoff
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query(
            "DELETE FROM user_notifications
             WHERE created_at < ?",
        )
        .bind(&cutoff_str)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i32)
    }

    pub async fn get_users_by_usernames(&self, usernames: &[String]) -> ApiResult<Vec<User>> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }

        // Build the placeholders for the IN clause
        let placeholders = usernames.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_str = format!(
            "SELECT u.id, u.email, u.user_type, u.created_at, u.updated_at
             FROM users u
             JOIN agents a ON u.id = a.user_id
             WHERE a.first_name IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for username in usernames {
            query = query.bind(username);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut users = Vec::new();
        for row in rows {
            let user_type_str: String = row.try_get("user_type")?;
            users.push(User {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                user_type: if user_type_str == "agent" {
                    UserType::Agent
                } else {
                    UserType::Contact
                },
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(users)
    }

    // ========================================================================
    // Webhook Operations
    // ========================================================================

    /// Create a new webhook
    pub async fn create_webhook(&self, webhook: &Webhook) -> ApiResult<()> {
        let subscribed_events_json = serde_json::to_string(&webhook.subscribed_events)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize events: {}", e)))?;

        sqlx::query(
            "INSERT INTO webhooks (id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&webhook.id)
        .bind(&webhook.name)
        .bind(&webhook.url)
        .bind(&subscribed_events_json)
        .bind(&webhook.secret)
        .bind(webhook.is_active)
        .bind(&webhook.created_at)
        .bind(&webhook.updated_at)
        .bind(&webhook.created_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a webhook by ID
    pub async fn get_webhook_by_id(&self, id: &str) -> ApiResult<Option<Webhook>> {
        let row = sqlx::query(
            "SELECT id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by
             FROM webhooks
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let subscribed_events_str: String = row.try_get("subscribed_events")?;
            let subscribed_events: Vec<String> = serde_json::from_str(&subscribed_events_str)
                .map_err(|e| ApiError::Internal(format!("Failed to parse events: {}", e)))?;

            Ok(Some(Webhook {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                url: row.try_get("url")?,
                subscribed_events,
                secret: row.try_get("secret")?,
                is_active: row.try_get("is_active")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                created_by: row.try_get("created_by")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all webhooks with pagination
    pub async fn list_webhooks(&self, limit: i64, offset: i64) -> ApiResult<Vec<Webhook>> {
        let rows = sqlx::query(
            "SELECT id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by
             FROM webhooks
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut webhooks = Vec::new();
        for row in rows {
            let subscribed_events_str: String = row.try_get("subscribed_events")?;
            let subscribed_events: Vec<String> = serde_json::from_str(&subscribed_events_str)
                .map_err(|e| ApiError::Internal(format!("Failed to parse events: {}", e)))?;

            webhooks.push(Webhook {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                url: row.try_get("url")?,
                subscribed_events,
                secret: row.try_get("secret")?,
                is_active: row.try_get("is_active")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                created_by: row.try_get("created_by")?,
            });
        }

        Ok(webhooks)
    }

    /// Get active webhooks that subscribe to a specific event type
    pub async fn get_active_webhooks_for_event(&self, event_type: &str) -> ApiResult<Vec<Webhook>> {
        let rows = sqlx::query(
            "SELECT id, name, url, subscribed_events, secret, is_active, created_at, updated_at, created_by
             FROM webhooks
             WHERE is_active = ?",
        )
        .bind(true)
        .fetch_all(&self.pool)
        .await?;

        let mut matching_webhooks = Vec::new();
        for row in rows {
            let subscribed_events_str: String = row.try_get("subscribed_events")?;
            let subscribed_events: Vec<String> = serde_json::from_str(&subscribed_events_str)
                .map_err(|e| ApiError::Internal(format!("Failed to parse events: {}", e)))?;

            // Filter webhooks that subscribe to this event
            if subscribed_events.contains(&event_type.to_string()) {
                matching_webhooks.push(Webhook {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    url: row.try_get("url")?,
                    subscribed_events,
                    secret: row.try_get("secret")?,
                    is_active: row.try_get("is_active")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    created_by: row.try_get("created_by")?,
                });
            }
        }

        Ok(matching_webhooks)
    }

    /// Update a webhook
    pub async fn update_webhook(&self, webhook: &Webhook) -> ApiResult<()> {
        let subscribed_events_json = serde_json::to_string(&webhook.subscribed_events)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize events: {}", e)))?;

        sqlx::query(
            "UPDATE webhooks
             SET name = ?, url = ?, subscribed_events = ?, secret = ?, is_active = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&webhook.name)
        .bind(&webhook.url)
        .bind(&subscribed_events_json)
        .bind(&webhook.secret)
        .bind(webhook.is_active)
        .bind(&webhook.updated_at)
        .bind(&webhook.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a webhook (cascades to deliveries)
    pub async fn delete_webhook(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM webhooks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Count total webhooks
    pub async fn count_webhooks(&self) -> ApiResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM webhooks")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("count")?)
    }

    // ========================================================================
    // Webhook Delivery Operations
    // ========================================================================

    /// Create a new webhook delivery record
    pub async fn create_webhook_delivery(&self, delivery: &WebhookDelivery) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO webhook_deliveries
             (id, webhook_id, event_type, payload, signature, status, http_status_code,
              retry_count, next_retry_at, attempted_at, completed_at, error_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&delivery.id)
        .bind(&delivery.webhook_id)
        .bind(&delivery.event_type)
        .bind(&delivery.payload)
        .bind(&delivery.signature)
        .bind(delivery.status.as_str())
        .bind(delivery.http_status_code)
        .bind(delivery.retry_count)
        .bind(&delivery.next_retry_at)
        .bind(&delivery.attempted_at)
        .bind(&delivery.completed_at)
        .bind(&delivery.error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update an existing webhook delivery record
    pub async fn update_webhook_delivery(&self, delivery: &WebhookDelivery) -> ApiResult<()> {
        sqlx::query(
            "UPDATE webhook_deliveries
             SET status = ?, http_status_code = ?, retry_count = ?,
                 next_retry_at = ?, attempted_at = ?, completed_at = ?, error_message = ?
             WHERE id = ?",
        )
        .bind(delivery.status.as_str())
        .bind(delivery.http_status_code)
        .bind(delivery.retry_count)
        .bind(&delivery.next_retry_at)
        .bind(&delivery.attempted_at)
        .bind(&delivery.completed_at)
        .bind(&delivery.error_message)
        .bind(&delivery.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending deliveries ready for processing
    pub async fn get_pending_deliveries(&self) -> ApiResult<Vec<WebhookDelivery>> {
        let now = chrono::Utc::now().to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, webhook_id, event_type, payload, signature, status,
                    http_status_code, retry_count, next_retry_at, attempted_at, completed_at, error_message
             FROM webhook_deliveries
             WHERE status = 'queued' AND (next_retry_at IS NULL OR next_retry_at <= ?)
             ORDER BY next_retry_at ASC, attempted_at ASC
             LIMIT 100",
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;

        let mut deliveries = Vec::new();
        for row in rows {
            deliveries.push(WebhookDelivery {
                id: row.try_get("id")?,
                webhook_id: row.try_get("webhook_id")?,
                event_type: row.try_get("event_type")?,
                payload: row.try_get("payload")?,
                signature: row.try_get("signature")?,
                status: DeliveryStatus::from(row.try_get::<String, _>("status")?),
                http_status_code: row.try_get("http_status_code")?,
                retry_count: row.try_get("retry_count")?,
                next_retry_at: row.try_get("next_retry_at")?,
                attempted_at: row.try_get("attempted_at")?,
                completed_at: row.try_get("completed_at")?,
                error_message: row.try_get("error_message")?,
            });
        }

        Ok(deliveries)
    }

    /// Get deliveries for a specific webhook with pagination
    pub async fn get_deliveries_for_webhook(
        &self,
        webhook_id: &str,
        limit: i64,
        offset: i64,
        status_filter: Option<&str>,
    ) -> ApiResult<Vec<WebhookDelivery>> {
        let query = if let Some(status) = status_filter {
            sqlx::query(
                "SELECT id, webhook_id, event_type, payload, signature, status,
                        http_status_code, retry_count, next_retry_at, attempted_at, completed_at, error_message
                 FROM webhook_deliveries
                 WHERE webhook_id = ? AND status = ?
                 ORDER BY attempted_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(webhook_id)
            .bind(status)
            .bind(limit)
            .bind(offset)
        } else {
            sqlx::query(
                "SELECT id, webhook_id, event_type, payload, signature, status,
                        http_status_code, retry_count, next_retry_at, attempted_at, completed_at, error_message
                 FROM webhook_deliveries
                 WHERE webhook_id = ?
                 ORDER BY attempted_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(webhook_id)
            .bind(limit)
            .bind(offset)
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut deliveries = Vec::new();
        for row in rows {
            deliveries.push(WebhookDelivery {
                id: row.try_get("id")?,
                webhook_id: row.try_get("webhook_id")?,
                event_type: row.try_get("event_type")?,
                payload: row.try_get("payload")?,
                signature: row.try_get("signature")?,
                status: DeliveryStatus::from(row.try_get::<String, _>("status")?),
                http_status_code: row.try_get("http_status_code")?,
                retry_count: row.try_get("retry_count")?,
                next_retry_at: row.try_get("next_retry_at")?,
                attempted_at: row.try_get("attempted_at")?,
                completed_at: row.try_get("completed_at")?,
                error_message: row.try_get("error_message")?,
            });
        }

        Ok(deliveries)
    }

    /// Count deliveries for a specific webhook
    pub async fn count_deliveries_for_webhook(
        &self,
        webhook_id: &str,
        status_filter: Option<&str>,
    ) -> ApiResult<i64> {
        let row = if let Some(status) = status_filter {
            sqlx::query(
                "SELECT COUNT(*) as count FROM webhook_deliveries WHERE webhook_id = ? AND status = ?",
            )
            .bind(webhook_id)
            .bind(status)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM webhook_deliveries WHERE webhook_id = ?")
                .bind(webhook_id)
                .fetch_one(&self.pool)
                .await?
        };

        Ok(row.try_get("count")?)
    }

    // ========================================
    // OIDC Provider Operations
    // ========================================

    /// Create a new OIDC provider
    pub async fn create_oidc_provider(&self, provider: &crate::models::OidcProvider) -> ApiResult<()> {
        let scopes_json = serde_json::to_string(&provider.scopes)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "INSERT INTO oidc_providers (id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.issuer_url)
        .bind(&provider.client_id)
        .bind(&provider.client_secret)
        .bind(&provider.redirect_uri)
        .bind(&scopes_json)
        .bind(provider.enabled)
        .bind(&provider.created_at)
        .bind(&provider.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get OIDC provider by name
    pub async fn get_oidc_provider_by_name(&self, name: &str) -> ApiResult<Option<crate::models::OidcProvider>> {
        let row = sqlx::query(
            "SELECT id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at
             FROM oidc_providers
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let scopes_json: String = row.try_get("scopes")?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_json)
                .map_err(|e| ApiError::Internal(format!("Failed to parse scopes: {}", e)))?;

            let enabled_val: i32 = row.try_get("enabled")?;
            let enabled = enabled_val != 0;

            Ok(Some(crate::models::OidcProvider {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                issuer_url: row.try_get("issuer_url")?,
                client_id: row.try_get("client_id")?,
                client_secret: row.try_get("client_secret")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scopes,
                enabled,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List OIDC providers with optional enabled filter
    pub async fn list_oidc_providers(&self, enabled_only: bool) -> ApiResult<Vec<crate::models::OidcProvider>> {
        let query = if enabled_only {
            sqlx::query(
                "SELECT id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at
                 FROM oidc_providers
                 WHERE enabled = 1
                 ORDER BY name",
            )
        } else {
            sqlx::query(
                "SELECT id, name, issuer_url, client_id, client_secret, redirect_uri, scopes, enabled, created_at, updated_at
                 FROM oidc_providers
                 ORDER BY name",
            )
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut providers = Vec::new();
        for row in rows {
            let scopes_json: String = row.try_get("scopes")?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_json)
                .map_err(|e| ApiError::Internal(format!("Failed to parse scopes: {}", e)))?;

            let enabled_val: i32 = row.try_get("enabled")?;
            let enabled = enabled_val != 0;

            providers.push(crate::models::OidcProvider {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                issuer_url: row.try_get("issuer_url")?,
                client_id: row.try_get("client_id")?,
                client_secret: row.try_get("client_secret")?,
                redirect_uri: row.try_get("redirect_uri")?,
                scopes,
                enabled,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(providers)
    }

    /// Update an existing OIDC provider
    pub async fn update_oidc_provider(&self, provider: &crate::models::OidcProvider) -> ApiResult<()> {
        let scopes_json = serde_json::to_string(&provider.scopes)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "UPDATE oidc_providers
             SET name = ?, issuer_url = ?, client_id = ?, client_secret = ?, redirect_uri = ?, scopes = ?, enabled = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&provider.name)
        .bind(&provider.issuer_url)
        .bind(&provider.client_id)
        .bind(&provider.client_secret)
        .bind(&provider.redirect_uri)
        .bind(&scopes_json)
        .bind(provider.enabled)
        .bind(&provider.updated_at)
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete an OIDC provider
    pub async fn delete_oidc_provider(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM oidc_providers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Toggle OIDC provider enabled status
    pub async fn toggle_oidc_provider(&self, id: &str) -> ApiResult<bool> {
        // First get current status
        let row = sqlx::query("SELECT enabled FROM oidc_providers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("OIDC provider {} not found", id)))?;

        let enabled_val: i32 = row.try_get("enabled")?;
        let current_enabled = enabled_val != 0;
        let new_enabled = !current_enabled;

        // Update to opposite
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query("UPDATE oidc_providers SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(new_enabled)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(new_enabled)
    }

    // ========================================
    // Auth Event Operations
    // ========================================

    /// Create a new authentication event
    pub async fn create_auth_event(&self, event: &crate::models::AuthEvent) -> ApiResult<()> {
        let event_type_str = event.event_type.to_string();
        let auth_method_str = match event.auth_method {
            crate::models::AuthMethod::Password => "password",
            crate::models::AuthMethod::Oidc => "oidc",
            crate::models::AuthMethod::ApiKey => "apikey",
        };

        sqlx::query(
            "INSERT INTO auth_events (id, event_type, user_id, email, auth_method, provider_name, ip_address, user_agent, error_reason, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(&event_type_str)
        .bind(&event.user_id)
        .bind(&event.email)
        .bind(auth_method_str)
        .bind(&event.provider_name)
        .bind(&event.ip_address)
        .bind(&event.user_agent)
        .bind(&event.error_reason)
        .bind(&event.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get auth events for a specific user with pagination
    pub async fn get_auth_events_by_user(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<crate::models::AuthEvent>> {
        let rows = sqlx::query(
            "SELECT id, event_type, user_id, email, auth_method, provider_name, ip_address, user_agent, error_reason, timestamp
             FROM auth_events
             WHERE user_id = ?
             ORDER BY timestamp DESC
             LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let event_type_str: String = row.try_get("event_type")?;
            let event_type = match event_type_str.as_str() {
                "login_success" => crate::models::AuthEventType::LoginSuccess,
                "login_failure" => crate::models::AuthEventType::LoginFailure,
                "logout" => crate::models::AuthEventType::Logout,
                "session_expired" => crate::models::AuthEventType::SessionExpired,
                "rate_limit_exceeded" => crate::models::AuthEventType::RateLimitExceeded,
                _ => crate::models::AuthEventType::LoginFailure,
            };

            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => crate::models::AuthMethod::Password,
                "oidc" => crate::models::AuthMethod::Oidc,
                _ => crate::models::AuthMethod::Password,
            };

            events.push(crate::models::AuthEvent {
                id: row.try_get("id")?,
                event_type,
                user_id: row.try_get("user_id")?,
                email: row.try_get("email")?,
                auth_method,
                provider_name: row.try_get("provider_name")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                error_reason: row.try_get("error_reason")?,
                timestamp: row.try_get("timestamp")?,
            });
        }

        Ok(events)
    }

    /// Get recent auth events (admin view) with pagination
    pub async fn get_recent_auth_events(
        &self,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<crate::models::AuthEvent>> {
        let rows = sqlx::query(
            "SELECT id, event_type, user_id, email, auth_method, provider_name, ip_address, user_agent, error_reason, timestamp
             FROM auth_events
             ORDER BY timestamp DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let event_type_str: String = row.try_get("event_type")?;
            let event_type = match event_type_str.as_str() {
                "login_success" => crate::models::AuthEventType::LoginSuccess,
                "login_failure" => crate::models::AuthEventType::LoginFailure,
                "logout" => crate::models::AuthEventType::Logout,
                "session_expired" => crate::models::AuthEventType::SessionExpired,
                "rate_limit_exceeded" => crate::models::AuthEventType::RateLimitExceeded,
                _ => crate::models::AuthEventType::LoginFailure,
            };

            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => crate::models::AuthMethod::Password,
                "oidc" => crate::models::AuthMethod::Oidc,
                _ => crate::models::AuthMethod::Password,
            };

            events.push(crate::models::AuthEvent {
                id: row.try_get("id")?,
                event_type,
                user_id: row.try_get("user_id")?,
                email: row.try_get("email")?,
                auth_method,
                provider_name: row.try_get("provider_name")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                error_reason: row.try_get("error_reason")?,
                timestamp: row.try_get("timestamp")?,
            });
        }

        Ok(events)
    }

    /// Update session last_accessed_at timestamp
    pub async fn update_session_last_accessed(&self, token: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query("UPDATE sessions SET last_accessed_at = ? WHERE token = ?")
            .bind(&now)
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store OIDC state for OAuth2 flow
    pub async fn create_oidc_state(&self, oidc_state: &crate::models::OidcState) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO oidc_states (state, provider_name, nonce, pkce_verifier, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&oidc_state.state)
        .bind(&oidc_state.provider_name)
        .bind(&oidc_state.nonce)
        .bind(&oidc_state.pkce_verifier)
        .bind(&oidc_state.created_at)
        .bind(&oidc_state.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieve and delete OIDC state (one-time use for security)
    pub async fn consume_oidc_state(&self, state: &str) -> ApiResult<Option<crate::models::OidcState>> {
        let oidc_state = self.get_oidc_state(state).await?;

        if oidc_state.is_some() {
            // Delete the state immediately to prevent replay attacks
            self.delete_oidc_state(state).await?;
        }

        Ok(oidc_state)
    }

    /// Get OIDC state by state parameter
    async fn get_oidc_state(&self, state: &str) -> ApiResult<Option<crate::models::OidcState>> {
        let row = sqlx::query(
            "SELECT state, provider_name, nonce, pkce_verifier, created_at, expires_at
             FROM oidc_states WHERE state = ?"
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| crate::models::OidcState {
            state: r.get("state"),
            provider_name: r.get("provider_name"),
            nonce: r.get("nonce"),
            pkce_verifier: r.get("pkce_verifier"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
        }))
    }

    /// Delete OIDC state
    async fn delete_oidc_state(&self, state: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM oidc_states WHERE state = ?")
            .bind(state)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clean up expired OIDC states
    pub async fn cleanup_expired_oidc_states(&self) -> ApiResult<u64> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query("DELETE FROM oidc_states WHERE expires_at < ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // ===== API Key Operations (Feature 015) =====

    /// Create API key for an agent
    pub async fn create_api_key(
        &self,
        agent_id: &str,
        api_key: &str,
        api_secret_hash: &str,
        description: &str,
    ) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE agents
             SET api_key = ?,
                 api_secret_hash = ?,
                 api_key_description = ?,
                 api_key_created_at = ?,
                 api_key_last_used_at = NULL,
                 api_key_revoked_at = NULL
             WHERE id = ?",
        )
        .bind(api_key)
        .bind(api_secret_hash)
        .bind(description)
        .bind(&now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agent by API key (for authentication)
    pub async fn get_agent_by_api_key(&self, api_key: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since,
                    api_key, api_secret_hash, api_key_description,
                    api_key_created_at, api_key_last_used_at, api_key_revoked_at
             FROM agents
             WHERE api_key = ? AND api_key IS NOT NULL",
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("availability_status").unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            Ok(Some(Agent {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: row.try_get("api_key").ok(),
                api_secret_hash: row.try_get("api_secret_hash").ok(),
                api_key_description: row.try_get("api_key_description").ok(),
                api_key_created_at: row.try_get("api_key_created_at").ok(),
                api_key_last_used_at: row.try_get("api_key_last_used_at").ok(),
                api_key_revoked_at: row.try_get("api_key_revoked_at").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update API key last used timestamp
    pub async fn update_api_key_last_used(&self, api_key: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        sqlx::query(
            "UPDATE agents
             SET api_key_last_used_at = ?
             WHERE api_key = ?",
        )
        .bind(&now)
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Revoke API key (soft delete with NULL fields)
    pub async fn revoke_api_key(&self, agent_id: &str) -> ApiResult<bool> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let result = sqlx::query(
            "UPDATE agents
             SET api_key = NULL,
                 api_secret_hash = NULL,
                 api_key_revoked_at = ?
             WHERE id = ? AND api_key IS NOT NULL",
        )
        .bind(&now)
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all active API keys with pagination and sorting
    pub async fn list_api_keys(
        &self,
        limit: i64,
        offset: i64,
        sort_by: &str,
        sort_order: &str,
    ) -> ApiResult<Vec<(String, String, String, String, Option<String>)>> {
        let order_clause = match sort_by {
            "last_used_at" => format!("a.api_key_last_used_at {}", sort_order),
            "description" => format!("a.api_key_description {}", sort_order),
            _ => format!("a.api_key_created_at {}", sort_order),
        };

        let query = format!(
            "SELECT a.id as agent_id, a.api_key, a.api_key_description,
                    a.api_key_created_at, a.api_key_last_used_at
             FROM agents a
             WHERE a.api_key IS NOT NULL
             ORDER BY {}
             LIMIT ? OFFSET ?",
            order_clause
        );

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            results.push((
                row.try_get("agent_id")?,
                row.try_get("api_key")?,
                row.try_get("api_key_description")?,
                row.try_get("api_key_created_at")?,
                row.try_get("api_key_last_used_at").ok(),
            ));
        }

        Ok(results)
    }

    /// Count active API keys
    pub async fn count_api_keys(&self) -> ApiResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM agents WHERE api_key IS NOT NULL")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get("count")?)
    }

    /// Get agent by ID (for API key operations)
    pub async fn get_agent_by_id(&self, agent_id: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, last_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since,
                    api_key, api_secret_hash, api_key_description,
                    api_key_created_at, api_key_last_used_at, api_key_revoked_at
             FROM agents
             WHERE id = ?",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let status_str: String = row.try_get("availability_status").unwrap_or_else(|_| "offline".to_string());
            let status = status_str.parse().unwrap_or(AgentAvailability::Offline);

            Ok(Some(Agent {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name").ok(), // Feature 016: Added last_name
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
                api_key: row.try_get("api_key").ok(),
                api_secret_hash: row.try_get("api_secret_hash").ok(),
                api_key_description: row.try_get("api_key_description").ok(),
                api_key_created_at: row.try_get("api_key_created_at").ok(),
                api_key_last_used_at: row.try_get("api_key_last_used_at").ok(),
                api_key_revoked_at: row.try_get("api_key_revoked_at").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    // ==================== Password Reset Operations (Feature 017) ====================

    /// Create a password reset token
    pub async fn create_password_reset_token(&self, token: &PasswordResetToken) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token, expires_at, used, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&token.id)
        .bind(&token.user_id)
        .bind(&token.token)
        .bind(&token.expires_at)
        .bind(if token.used { 1 } else { 0 })
        .bind(&token.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get password reset token by token value
    pub async fn get_password_reset_token(&self, token: &str) -> ApiResult<Option<PasswordResetToken>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, expires_at, used, created_at
             FROM password_reset_tokens
             WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(PasswordResetToken {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                expires_at: row.try_get("expires_at")?,
                used: row.try_get::<i32, _>("used")? == 1,
                created_at: row.try_get("created_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Count recent password reset requests for a user (for rate limiting)
    /// Returns count of ALL tokens created in the last hour (used or unused)
    /// Rate limiting counts all requests to prevent abuse, regardless of token status
    pub async fn count_recent_reset_requests(&self, user_id: &str, window_seconds: i64) -> ApiResult<i64> {
        let now = time::OffsetDateTime::now_utc();
        let window_start = now - time::Duration::seconds(window_seconds);
        let window_start_str = window_start
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM password_reset_tokens
             WHERE user_id = ? AND created_at > ?",
        )
        .bind(user_id)
        .bind(&window_start_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }

    /// Mark password reset token as used
    pub async fn mark_token_as_used(&self, token_id: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE password_reset_tokens
             SET used = 1
             WHERE id = ?",
        )
        .bind(token_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete password reset token (for lazy cleanup)
    pub async fn delete_password_reset_token(&self, token_id: &str) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM password_reset_tokens
             WHERE id = ?",
        )
        .bind(token_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Invalidate all unused password reset tokens for a user
    /// Used when generating a new token to invalidate previous tokens
    pub async fn invalidate_user_reset_tokens(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE password_reset_tokens
             SET used = 1
             WHERE user_id = ? AND used = 0",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update agent password hash by user_id (for password reset)
    pub async fn update_agent_password_by_user_id(&self, user_id: &str, password_hash: &str) -> ApiResult<()> {
        sqlx::query(
            "UPDATE agents
             SET password_hash = ?
             WHERE user_id = ?",
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete all sessions for a user (for session destruction on password reset)
    pub async fn delete_user_sessions(&self, user_id: &str) -> ApiResult<u64> {
        let result = sqlx::query(
            "DELETE FROM sessions
             WHERE user_id = ?",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Reset password with transaction (Feature 017: Password Reset)
    /// Performs all password reset operations atomically:
    /// 1. Update agent password
    /// 2. Mark token as used
    /// 3. Delete all user sessions
    ///
    /// If any step fails, the entire transaction is rolled back
    pub async fn reset_password_atomic(
        &self,
        user_id: &str,
        token_id: &str,
        password_hash: &str,
    ) -> ApiResult<u64> {
        let mut tx = self.pool.begin().await?;

        // 1. Update agent password
        sqlx::query(
            "UPDATE agents
             SET password_hash = ?
             WHERE user_id = ?",
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // 2. Mark token as used
        sqlx::query(
            "UPDATE password_reset_tokens
             SET used = 1
             WHERE id = ?",
        )
        .bind(token_id)
        .execute(&mut *tx)
        .await?;

        // 3. Delete all user sessions
        let result = sqlx::query(
            "DELETE FROM sessions
             WHERE user_id = ?",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Commit transaction - if this fails, all changes are rolled back
        tx.commit().await?;

        Ok(result.rows_affected())
    }

    /// Get all password reset tokens for a user (for testing)
    pub async fn get_all_password_reset_tokens_for_user(&self, user_id: &str) -> ApiResult<Vec<PasswordResetToken>> {
        let rows = sqlx::query(
            "SELECT id, user_id, token, expires_at, used, created_at
             FROM password_reset_tokens WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tokens = Vec::new();
        for row in rows {
            let used_int: i64 = row.try_get("used")?;
            tokens.push(PasswordResetToken {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                expires_at: row.try_get("expires_at")?,
                used: used_int != 0,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(tokens)
    }

    /// Get all sessions for a user (for testing)
    pub async fn get_user_sessions(&self, user_id: &str) -> ApiResult<Vec<Session>> {
        // Cast datetime columns to TEXT for compatibility with sqlx::any driver
        let rows = sqlx::query(
            "SELECT id, user_id, token, csrf_token,
                    CAST(expires_at AS TEXT) as expires_at,
                    CAST(created_at AS TEXT) as created_at,
                    CAST(last_accessed_at AS TEXT) as last_accessed_at,
                    auth_method, provider_name
             FROM sessions WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let auth_method_str: String = row.try_get("auth_method")?;
            let auth_method = match auth_method_str.as_str() {
                "password" => AuthMethod::Password,
                "oidc" => AuthMethod::Oidc,
                "apikey" => AuthMethod::ApiKey,
                _ => AuthMethod::Password,
            };

            // Handle NULL provider_name gracefully (NULL in DB becomes None)
            let provider_name: Option<String> = row.try_get("provider_name").ok();

            sessions.push(Session {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                csrf_token: row.try_get("csrf_token")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                last_accessed_at: row.try_get("last_accessed_at")?,
                auth_method,
                provider_name,
            });
        }

        Ok(sessions)
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
