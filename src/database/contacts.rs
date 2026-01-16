use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use crate::models::{Contact, ContactChannel, User, UserType};
use sqlx::Row;

use crate::domain::ports::contact_repository::ContactRepository;
use async_trait::async_trait;

#[async_trait]
impl ContactRepository for Database {
    // Contact operations
    async fn create_contact(&self, contact: &Contact) -> ApiResult<()> {
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

    async fn find_contact_by_user_id(&self, user_id: &str) -> ApiResult<Option<Contact>> {
        Database::find_contact_by_user_id(self, user_id).await
    }

    async fn create_contact_channel(&self, channel: &ContactChannel) -> ApiResult<()> {
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
    async fn get_contact_by_email(&self, email: &str) -> ApiResult<Option<Contact>> {
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
    async fn create_contact_from_message(
        &self,
        email: &str,
        full_name: Option<&str>,
        inbox_id: &str,
    ) -> ApiResult<String> {
        let mut tx = self.pool.begin().await?;

        // Create user
        let user = User::new(email.to_string(), UserType::Contact);
        self.create_user_internal(&mut *tx, &user).await?;

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
        let channel =
            ContactChannel::new(contact.id.clone(), inbox_id.to_string(), email.to_string());
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

    // Contact update operations
    async fn update_contact(&self, contact_id: &str, first_name: Option<String>) -> ApiResult<()> {
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

    async fn find_contact_channels(&self, contact_id: &str) -> ApiResult<Vec<ContactChannel>> {
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
    async fn list_contacts(&self, limit: i64, offset: i64) -> ApiResult<Vec<(User, Contact)>> {
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
                deleted_at: None,
                deleted_by: None,
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
    async fn count_contacts(&self) -> ApiResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM users
             WHERE user_type = 'contact'",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("count")?)
    }
    async fn delete_contact(&self, contact_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(contact_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

impl Database {
    pub async fn delete_user(&self, user_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_contact_name(&self, id: &str, first_name: &str) -> ApiResult<()> {
        sqlx::query("UPDATE contacts SET first_name = ? WHERE id = ?")
            .bind(first_name)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn find_contact_by_user_id(&self, user_id: &str) -> ApiResult<Option<Contact>> {
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
}
