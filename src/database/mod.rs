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
            "INSERT INTO agents (id, user_id, first_name, password_hash)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&agent.id)
        .bind(&agent.user_id)
        .bind(&agent.first_name)
        .bind(&agent.password_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_agent_by_user_id(&self, user_id: &str) -> ApiResult<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id, user_id, first_name, password_hash, availability_status,
                    last_login_at, last_activity_at, away_since
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
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    // Role operations
    pub async fn get_role_by_name(&self, name: &str) -> ApiResult<Option<Role>> {
        let row = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
             FROM roles
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
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
            "SELECT r.id, r.name, r.description, r.created_at, r.updated_at
             FROM roles r
             INNER JOIN user_roles ur ON r.id = ur.role_id
             WHERE ur.user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut roles = Vec::new();
        for row in rows {
            roles.push(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(roles)
    }

    // Session operations
    pub async fn create_session(&self, session: &Session) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, expires_at, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.token)
        .bind(&session.expires_at)
        .bind(&session.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_session_by_token(&self, token: &str) -> ApiResult<Option<Session>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, expires_at, created_at
             FROM sessions
             WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Session {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                token: row.try_get("token")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
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
                    a.id as agent_id, a.user_id as agent_user_id, a.first_name, a.password_hash,
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
                password_hash: row.try_get("password_hash")?,
                availability_status: status,
                last_login_at: row.try_get("last_login_at").ok(),
                last_activity_at: row.try_get("last_activity_at").ok(),
                away_since: row.try_get("away_since").ok(),
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
            "SELECT id, name, description, created_at, updated_at
             FROM roles
             ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut roles = Vec::new();
        for row in rows {
            roles.push(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(roles)
    }

    pub async fn get_role_by_id(&self, id: &str) -> ApiResult<Option<Role>> {
        let row = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
             FROM roles
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Role {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get::<Option<String>, _>("description").ok().flatten(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_role(&self, role: &Role) -> ApiResult<()> {
        let description_value: Option<&str> = role.description.as_deref();

        sqlx::query(
            "INSERT INTO roles (id, name, description, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&role.id)
        .bind(&role.name)
        .bind(description_value)
        .bind(&role.created_at)
        .bind(&role.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_role(&self, id: &str, name: &str, description: &Option<String>) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();

        let description_value: Option<&str> = description.as_deref();

        sqlx::query(
            "UPDATE roles
             SET name = ?, description = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(name)
        .bind(description_value)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

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
            "SELECT id, user_id, first_name, password_hash, availability_status,
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
                    password_hash: row.try_get("password_hash")?,
                    availability_status: status,
                    last_login_at: row.try_get("last_login_at").ok(),
                    last_activity_at: row.try_get("last_activity_at").ok(),
                    away_since: row.try_get("away_since").ok(),
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
            "SELECT id, user_id, first_name, password_hash, availability_status,
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
                    password_hash: row.try_get("password_hash")?,
                    availability_status: status,
                    last_login_at: row.try_get("last_login_at").ok(),
                    last_activity_at: row.try_get("last_activity_at").ok(),
                    away_since: row.try_get("away_since").ok(),
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
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }

}
