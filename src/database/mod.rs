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
            "SELECT id, user_id, first_name, password_hash
             FROM agents
             WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Agent {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                first_name: row.try_get("first_name")?,
                password_hash: row.try_get("password_hash")?,
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
                description: row.try_get("description")?,
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
                description: row.try_get("description")?,
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
                    a.id as agent_id, a.user_id as agent_user_id, a.first_name, a.password_hash
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

            let agent = Agent {
                id: row.try_get("agent_id")?,
                user_id: row.try_get("agent_user_id")?,
                first_name: row.try_get("first_name")?,
                password_hash: row.try_get("password_hash")?,
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
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            version: row.try_get("version")?,
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
                    resolved_at, snoozed_until, created_at, updated_at, version
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
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
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
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
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
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
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
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                version: row.try_get("version")?,
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
                description: row.try_get("description")?,
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
                description: row.try_get("description")?,
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
                description: row.try_get("description")?,
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
                description: row.try_get("description")?,
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
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }

}
