use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::persistence::Database;
use crate::domain::entities::{
    EmailProcessingLog, InboxEmailConfig, MessageAttachment, UpdateInboxEmailConfigRequest,
};
use sqlx::Row;
use time;

use crate::domain::ports::attachment_repository::AttachmentRepository;
use crate::domain::ports::email_repository::EmailRepository;

impl Database {
    // ========================================
    // Email Integration Operations
    // ========================================

    /// Decrypt password if encryption is enabled, otherwise return as-is
    fn decrypt_password_field(&self, encrypted: &str) -> String {
        use crate::shared::utils::encryption::{decrypt_password, is_encryption_enabled};

        if !is_encryption_enabled() {
            // Encryption not enabled, return plaintext
            return encrypted.to_string();
        }

        // Try to decrypt
        match decrypt_password(encrypted) {
            Ok(plaintext) => plaintext,
            Err(e) => {
                // If decryption fails, assume it's plaintext (for backward compatibility)
                tracing::warn!(
                    "Failed to decrypt password field: {}. Assuming plaintext.",
                    e
                );
                encrypted.to_string()
            }
        }
    }

    /// Encrypt password if encryption is enabled, otherwise return as-is
    fn encrypt_password_field(&self, plaintext: &str) -> ApiResult<String> {
        use crate::shared::utils::encryption::{encrypt_password, is_encryption_enabled};

        if !is_encryption_enabled() {
            // Encryption not enabled, store as plaintext
            tracing::warn!("ENCRYPTION_KEY not set - storing passwords in plaintext!");
            return Ok(plaintext.to_string());
        }

        // Encrypt
        encrypt_password(plaintext)
            .map_err(|e| ApiError::Internal(format!("Failed to encrypt password: {}", e)))
    }

    /// Get inbox email configuration by inbox_id
    pub async fn get_inbox_email_config(
        &self,
        inbox_id: &str,
    ) -> ApiResult<Option<InboxEmailConfig>> {
        let row = sqlx::query(
            "SELECT id, inbox_id, imap_host, imap_port, imap_username, imap_password, imap_use_tls, imap_folder,
                    smtp_host, smtp_port, smtp_username, smtp_password, smtp_use_tls,
                    email_address, display_name, poll_interval_seconds, enabled,
                    CAST(last_poll_at AS TEXT) as last_poll_at,
                    CAST(created_at AS TEXT) as created_at,
                    CAST(updated_at AS TEXT) as updated_at
             FROM inbox_email_configs WHERE inbox_id = ?"
        )
        .bind(inbox_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

        match row {
            Some(row) => {
                let imap_use_tls: i64 = row.try_get("imap_use_tls").map_err(|e| {
                    ApiError::Internal(format!("Failed to get imap_use_tls: {}", e))
                })?;
                let smtp_use_tls: i64 = row.try_get("smtp_use_tls").map_err(|e| {
                    ApiError::Internal(format!("Failed to get smtp_use_tls: {}", e))
                })?;
                let enabled: i64 = row
                    .try_get("enabled")
                    .map_err(|e| ApiError::Internal(format!("Failed to get enabled: {}", e)))?;

                // Decrypt passwords from database
                let imap_password_encrypted: String = row.try_get("imap_password")?;
                let smtp_password_encrypted: String = row.try_get("smtp_password")?;

                Ok(Some(InboxEmailConfig {
                    id: row.try_get("id")?,
                    inbox_id: row.try_get("inbox_id")?,
                    imap_host: row.try_get("imap_host")?,
                    imap_port: row.try_get("imap_port")?,
                    imap_username: row.try_get("imap_username")?,
                    imap_password: self.decrypt_password_field(&imap_password_encrypted),
                    imap_use_tls: imap_use_tls != 0,
                    imap_folder: row.try_get("imap_folder")?,
                    smtp_host: row.try_get("smtp_host")?,
                    smtp_port: row.try_get("smtp_port")?,
                    smtp_username: row.try_get("smtp_username")?,
                    smtp_password: self.decrypt_password_field(&smtp_password_encrypted),
                    smtp_use_tls: smtp_use_tls != 0,
                    email_address: row.try_get("email_address")?,
                    display_name: row.try_get("display_name")?,
                    poll_interval_seconds: row.try_get("poll_interval_seconds")?,
                    enabled: enabled != 0,
                    last_poll_at: row.try_get("last_poll_at").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all enabled inbox email configurations
    pub async fn get_enabled_email_configs(&self) -> ApiResult<Vec<InboxEmailConfig>> {
        let rows = sqlx::query(
            "SELECT c.id, c.inbox_id, c.imap_host, c.imap_port, c.imap_username, c.imap_password, c.imap_use_tls, c.imap_folder,
                    c.smtp_host, c.smtp_port, c.smtp_username, c.smtp_password, c.smtp_use_tls,
                    c.email_address, c.display_name, c.poll_interval_seconds, c.enabled,
                    CAST(c.last_poll_at AS TEXT) as last_poll_at,
                    CAST(c.created_at AS TEXT) as created_at,
                    CAST(c.updated_at AS TEXT) as updated_at
             FROM inbox_email_configs c
             INNER JOIN inboxes i ON c.inbox_id = i.id
             WHERE c.enabled = 1 AND i.deleted_at IS NULL
             ORDER BY c.created_at"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

        let mut configs = Vec::new();
        for row in rows {
            let imap_use_tls: i64 = row.try_get("imap_use_tls")?;
            let smtp_use_tls: i64 = row.try_get("smtp_use_tls")?;
            let enabled: i64 = row.try_get("enabled")?;

            configs.push(InboxEmailConfig {
                id: row.try_get("id")?,
                inbox_id: row.try_get("inbox_id")?,
                imap_host: row.try_get("imap_host")?,
                imap_port: row.try_get("imap_port")?,
                imap_username: row.try_get("imap_username")?,
                imap_password: row.try_get("imap_password")?,
                imap_use_tls: imap_use_tls != 0,
                imap_folder: row.try_get("imap_folder")?,
                smtp_host: row.try_get("smtp_host")?,
                smtp_port: row.try_get("smtp_port")?,
                smtp_username: row.try_get("smtp_username")?,
                smtp_password: row.try_get("smtp_password")?,
                smtp_use_tls: smtp_use_tls != 0,
                email_address: row.try_get("email_address")?,
                display_name: row.try_get("display_name")?,
                poll_interval_seconds: row.try_get("poll_interval_seconds")?,
                enabled: enabled != 0,
                last_poll_at: row.try_get("last_poll_at").ok(),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(configs)
    }

    /// Create inbox email configuration
    pub async fn create_inbox_email_config(
        &self,
        config: &InboxEmailConfig,
    ) -> ApiResult<InboxEmailConfig> {
        // Encrypt passwords before storing
        let imap_password_encrypted = self.encrypt_password_field(&config.imap_password)?;
        let smtp_password_encrypted = self.encrypt_password_field(&config.smtp_password)?;

        sqlx::query(
            "INSERT INTO inbox_email_configs (
                id, inbox_id, imap_host, imap_port, imap_username, imap_password, imap_use_tls, imap_folder,
                smtp_host, smtp_port, smtp_username, smtp_password, smtp_use_tls,
                email_address, display_name, poll_interval_seconds, enabled, last_poll_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&config.id)
        .bind(&config.inbox_id)
        .bind(&config.imap_host)
        .bind(config.imap_port)
        .bind(&config.imap_username)
        .bind(&imap_password_encrypted)
        .bind(if config.imap_use_tls { 1 } else { 0 })
        .bind(&config.imap_folder)
        .bind(&config.smtp_host)
        .bind(config.smtp_port)
        .bind(&config.smtp_username)
        .bind(&smtp_password_encrypted)
        .bind(if config.smtp_use_tls { 1 } else { 0 })
        .bind(&config.email_address)
        .bind(&config.display_name)
        .bind(config.poll_interval_seconds)
        .bind(if config.enabled { 1 } else { 0 })
        .bind(&config.last_poll_at)
        .bind(&config.created_at)
        .bind(&config.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create inbox email config: {}", e)))?;

        Ok(config.clone())
    }

    /// Update inbox email configuration
    pub async fn update_inbox_email_config(
        &self,
        id: &str,
        updates: &UpdateInboxEmailConfigRequest,
    ) -> ApiResult<InboxEmailConfig> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| ApiError::Internal(format!("Failed to format timestamp: {}", e)))?;

        // Get existing config
        let existing = self
            .get_inbox_email_config_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Inbox email config not found".to_string()))?;

        // Apply updates
        let updated = InboxEmailConfig {
            id: existing.id,
            inbox_id: existing.inbox_id,
            imap_host: updates.imap_host.clone().unwrap_or(existing.imap_host),
            imap_port: updates.imap_port.unwrap_or(existing.imap_port),
            imap_username: updates
                .imap_username
                .clone()
                .unwrap_or(existing.imap_username),
            imap_password: updates
                .imap_password
                .clone()
                .unwrap_or(existing.imap_password),
            imap_use_tls: updates.imap_use_tls.unwrap_or(existing.imap_use_tls),
            imap_folder: updates.imap_folder.clone().unwrap_or(existing.imap_folder),
            smtp_host: updates.smtp_host.clone().unwrap_or(existing.smtp_host),
            smtp_port: updates.smtp_port.unwrap_or(existing.smtp_port),
            smtp_username: updates
                .smtp_username
                .clone()
                .unwrap_or(existing.smtp_username),
            smtp_password: updates
                .smtp_password
                .clone()
                .unwrap_or(existing.smtp_password),
            smtp_use_tls: updates.smtp_use_tls.unwrap_or(existing.smtp_use_tls),
            email_address: updates
                .email_address
                .clone()
                .unwrap_or(existing.email_address),
            display_name: updates
                .display_name
                .clone()
                .unwrap_or(existing.display_name),
            poll_interval_seconds: updates
                .poll_interval_seconds
                .unwrap_or(existing.poll_interval_seconds),
            enabled: updates.enabled.unwrap_or(existing.enabled),
            last_poll_at: existing.last_poll_at,
            created_at: existing.created_at,
            updated_at: now.clone(),
        };

        // Encrypt passwords before storing
        let imap_password_encrypted = self.encrypt_password_field(&updated.imap_password)?;
        let smtp_password_encrypted = self.encrypt_password_field(&updated.smtp_password)?;

        sqlx::query(
            "UPDATE inbox_email_configs SET
                imap_host = ?, imap_port = ?, imap_username = ?, imap_password = ?, imap_use_tls = ?, imap_folder = ?,
                smtp_host = ?, smtp_port = ?, smtp_username = ?, smtp_password = ?, smtp_use_tls = ?,
                email_address = ?, display_name = ?, poll_interval_seconds = ?, enabled = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&updated.imap_host)
        .bind(updated.imap_port)
        .bind(&updated.imap_username)
        .bind(&imap_password_encrypted)
        .bind(if updated.imap_use_tls { 1 } else { 0 })
        .bind(&updated.imap_folder)
        .bind(&updated.smtp_host)
        .bind(updated.smtp_port)
        .bind(&updated.smtp_username)
        .bind(&smtp_password_encrypted)
        .bind(if updated.smtp_use_tls { 1 } else { 0 })
        .bind(&updated.email_address)
        .bind(&updated.display_name)
        .bind(updated.poll_interval_seconds)
        .bind(if updated.enabled { 1 } else { 0 })
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update inbox email config: {}", e)))?;

        Ok(updated)
    }

    /// Get inbox email configuration by id
    async fn get_inbox_email_config_by_id(&self, id: &str) -> ApiResult<Option<InboxEmailConfig>> {
        let row = sqlx::query(
            "SELECT id, inbox_id, imap_host, imap_port, imap_username, imap_password, imap_use_tls, imap_folder,
                    smtp_host, smtp_port, smtp_username, smtp_password, smtp_use_tls,
                    email_address, display_name, poll_interval_seconds, enabled,
                    CAST(last_poll_at AS TEXT) as last_poll_at,
                    CAST(created_at AS TEXT) as created_at,
                    CAST(updated_at AS TEXT) as updated_at
             FROM inbox_email_configs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

        match row {
            Some(row) => {
                let imap_use_tls: i64 = row.try_get("imap_use_tls")?;
                let smtp_use_tls: i64 = row.try_get("smtp_use_tls")?;
                let enabled: i64 = row.try_get("enabled")?;

                // Decrypt passwords from database
                let imap_password_encrypted: String = row.try_get("imap_password")?;
                let smtp_password_encrypted: String = row.try_get("smtp_password")?;

                Ok(Some(InboxEmailConfig {
                    id: row.try_get("id")?,
                    inbox_id: row.try_get("inbox_id")?,
                    imap_host: row.try_get("imap_host")?,
                    imap_port: row.try_get("imap_port")?,
                    imap_username: row.try_get("imap_username")?,
                    imap_password: self.decrypt_password_field(&imap_password_encrypted),
                    imap_use_tls: imap_use_tls != 0,
                    imap_folder: row.try_get("imap_folder")?,
                    smtp_host: row.try_get("smtp_host")?,
                    smtp_port: row.try_get("smtp_port")?,
                    smtp_username: row.try_get("smtp_username")?,
                    smtp_password: self.decrypt_password_field(&smtp_password_encrypted),
                    smtp_use_tls: smtp_use_tls != 0,
                    email_address: row.try_get("email_address")?,
                    display_name: row.try_get("display_name")?,
                    poll_interval_seconds: row.try_get("poll_interval_seconds")?,
                    enabled: enabled != 0,
                    last_poll_at: row.try_get("last_poll_at").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Update last poll time for an inbox
    pub async fn update_last_poll_time(&self, inbox_id: &str) -> ApiResult<()> {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| ApiError::Internal(format!("Failed to format timestamp: {}", e)))?;

        sqlx::query("UPDATE inbox_email_configs SET last_poll_at = ? WHERE inbox_id = ?")
            .bind(&now)
            .bind(inbox_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to update last poll time: {}", e)))?;

        Ok(())
    }

    /// Create message attachment
    pub async fn create_message_attachment(
        &self,
        attachment: &MessageAttachment,
    ) -> ApiResult<MessageAttachment> {
        sqlx::query(
            "INSERT INTO message_attachments (id, message_id, filename, content_type, file_size, file_path, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&attachment.id)
        .bind(&attachment.message_id)
        .bind(&attachment.filename)
        .bind(&attachment.content_type)
        .bind(attachment.file_size)
        .bind(&attachment.file_path)
        .bind(&attachment.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create message attachment: {}", e)))?;

        Ok(attachment.clone())
    }

    /// Get all attachments for a message
    pub async fn get_message_attachments(
        &self,
        message_id: &str,
    ) -> ApiResult<Vec<MessageAttachment>> {
        let rows = sqlx::query(
            "SELECT id, message_id, filename, content_type, file_size, file_path,
                    CAST(created_at AS TEXT) as created_at
             FROM message_attachments WHERE message_id = ? ORDER BY created_at",
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

        let mut attachments = Vec::new();
        for row in rows {
            attachments.push(MessageAttachment {
                id: row.try_get("id")?,
                message_id: row.try_get("message_id")?,
                filename: row.try_get("filename")?,
                content_type: row.try_get("content_type").ok(),
                file_size: row.try_get("file_size")?,
                file_path: row.try_get("file_path")?,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(attachments)
    }

    /// Log email processing result
    pub async fn log_email_processing(
        &self,
        log: &EmailProcessingLog,
    ) -> ApiResult<EmailProcessingLog> {
        sqlx::query(
            "INSERT INTO email_processing_log (
                id, inbox_id, email_message_id, email_uid, from_address, subject,
                processing_status, error_message, conversation_id, message_id, processed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.inbox_id)
        .bind(&log.email_message_id)
        .bind(&log.email_uid)
        .bind(&log.from_address)
        .bind(&log.subject)
        .bind(&log.processing_status)
        .bind(&log.error_message)
        .bind(&log.conversation_id)
        .bind(&log.message_id)
        .bind(&log.processed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to log email processing: {}", e)))?;

        Ok(log.clone())
    }

    /// Check if an email has already been processed
    pub async fn check_email_processed(
        &self,
        inbox_id: &str,
        email_message_id: &str,
    ) -> ApiResult<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM email_processing_log WHERE inbox_id = ? AND email_message_id = ?")
            .bind(inbox_id)
            .bind(email_message_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count > 0)
    }

    /// Delete inbox email configuration
    pub async fn delete_inbox_email_config(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM inbox_email_configs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to delete inbox email config: {}", e))
            })?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EmailRepository for Database {
    async fn get_inbox_email_config(&self, inbox_id: &str) -> ApiResult<Option<InboxEmailConfig>> {
        self.get_inbox_email_config(inbox_id).await
    }

    async fn get_enabled_email_configs(&self) -> ApiResult<Vec<InboxEmailConfig>> {
        self.get_enabled_email_configs().await
    }

    async fn create_inbox_email_config(
        &self,
        config: &InboxEmailConfig,
    ) -> ApiResult<InboxEmailConfig> {
        self.create_inbox_email_config(config).await
    }

    async fn update_inbox_email_config(
        &self,
        id: &str,
        updates: &UpdateInboxEmailConfigRequest,
    ) -> ApiResult<InboxEmailConfig> {
        self.update_inbox_email_config(id, updates).await
    }

    async fn delete_inbox_email_config(&self, id: &str) -> ApiResult<()> {
        self.delete_inbox_email_config(id).await
    }

    async fn update_last_poll_time(&self, inbox_id: &str) -> ApiResult<()> {
        self.update_last_poll_time(inbox_id).await
    }

    async fn log_email_processing(
        &self,
        log: &EmailProcessingLog,
    ) -> ApiResult<EmailProcessingLog> {
        self.log_email_processing(log).await
    }

    async fn check_email_processed(
        &self,
        inbox_id: &str,
        email_message_id: &str,
    ) -> ApiResult<bool> {
        self.check_email_processed(inbox_id, email_message_id).await
    }
}

#[async_trait::async_trait]
impl AttachmentRepository for Database {
    async fn create_message_attachment(
        &self,
        attachment: &MessageAttachment,
    ) -> ApiResult<MessageAttachment> {
        self.create_message_attachment(attachment).await
    }

    async fn get_message_attachments(&self, message_id: &str) -> ApiResult<Vec<MessageAttachment>> {
        self.get_message_attachments(message_id).await
    }
}
