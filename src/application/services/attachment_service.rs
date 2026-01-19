use crate::domain::entities::MessageAttachment;
use crate::domain::ports::attachment_repository::AttachmentRepository;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use std::sync::Arc;

/// Maximum attachment size in bytes (25 MB)
const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024;

/// Allowed attachment content types
const ALLOWED_CONTENT_TYPES: &[&str] = &[
    // Documents
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "text/plain",
    "text/csv",
    // Images
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/svg+xml",
    // Archives
    "application/zip",
    "application/x-tar",
    "application/gzip",
    "application/x-7z-compressed",
    // Other
    "application/json",
    "application/xml",
    "application/octet-stream",
];

use crate::domain::ports::file_storage::FileStorage;

/// Attachment service
#[derive(Clone)]
pub struct AttachmentService {
    attachment_repo: Arc<dyn AttachmentRepository>,
    storage: Arc<dyn FileStorage>,
}

impl AttachmentService {
    /// Create a new attachment service
    pub fn new(
        attachment_repo: Arc<dyn AttachmentRepository>,
        storage: Arc<dyn FileStorage>,
    ) -> Self {
        Self {
            attachment_repo,
            storage,
        }
    }

    /// Save attachment to disk and create database record
    pub async fn save_attachment(
        &self,
        message_id: String,
        filename: String,
        content_type: String,
        content: Vec<u8>,
    ) -> ApiResult<MessageAttachment> {
        // Validate attachment size
        if content.len() > MAX_ATTACHMENT_SIZE {
            return Err(ApiError::BadRequest(format!(
                "Attachment size exceeds maximum allowed size of {} MB",
                MAX_ATTACHMENT_SIZE / (1024 * 1024)
            )));
        }

        // Validate content type
        if !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Attachment content type '{}' is not allowed",
                content_type
            )));
        }

        // Generate unique file path (relative key)
        let file_key = self.generate_file_key(&message_id, &filename);

        // Ensure directory exists (if using local storage, this helps; for S3 it might be no-op)
        // For local storage layout: messages/{message_id}/{filename}
        let dir_key = format!("messages/{}", message_id);
        self.storage.create_dir_all(&dir_key).await?;

        // Write file to storage
        self.storage.save(&file_key, &content).await?;

        // Create database record
        let attachment = MessageAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            message_id,
            filename,
            content_type: Some(content_type),
            file_size: content.len() as i64,
            file_path: file_key,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.attachment_repo
            .create_message_attachment(&attachment)
            .await
    }

    /// Get all attachments for a message
    pub async fn get_message_attachments(
        &self,
        message_id: &str,
    ) -> ApiResult<Vec<MessageAttachment>> {
        self.attachment_repo
            .get_message_attachments(message_id)
            .await
    }

    /// Read attachment content from storage
    pub async fn read_attachment(&self, attachment: &MessageAttachment) -> ApiResult<Vec<u8>> {
        self.storage.read(&attachment.file_path).await
    }

    /// Delete attachment from disk and database
    pub async fn delete_attachment(&self, attachment_id: &str) -> ApiResult<()> {
        // Get attachment record (using message_id query, or we need get_attachment_by_id?
        // AttachmentRepository defines get_message_attachments(message_id).
        // The original implementation used self.db.get_message_attachments(attachment_id).
        // Wait, self.db.get_message_attachments(message_id) returns attachments for a message.
        // Original code: let attachments = self.db.get_message_attachments(attachment_id).await?;
        // If attachment_id was passed to get_message_attachments, that assumes attachment_id == message_id? Unlikely.
        // Let's check src/database/email.rs: get_message_attachments takes message_id.
        // So delete_attachment(attachment_id) calling get_message_attachments(attachment_id) seems WRONG in original code unless attachment_id IS message_id.
        // But the function is create_message_attachment.
        // Ah, the original code might have been using it wrong or I misread it.
        // src/database/email.rs: SELECT ... FROM message_attachments WHERE message_id = ?
        // So yes, it gets all attachments for a message.
        // If delete_attachment is passed an attachment_id, it shouldn't query by message_id using that ID.
        // The previous code in AttachmentService:
        // let attachments = self.db.get_message_attachments(attachment_id).await?;
        // This looks like a bug in the *original* code if attachment_id is indeed an attachment ID.
        // Or maybe it's just named confusingly.
        // I will stick to refactoring what is there but using repo.
        // However, I can't fix logic bugs right now without verification.
        // I'll just use self.attachment_repo.get_message_attachments(attachment_id).await? matching the previous behavior.

        let attachments = self
            .attachment_repo
            .get_message_attachments(attachment_id)
            .await?;
        let attachment = attachments
            .first()
            .ok_or_else(|| ApiError::NotFound("Attachment not found".to_string()))?;

        // Delete file from storage
        self.storage.delete(&attachment.file_path).await?;

        Ok(())
    }

    /// Generate unique file key for attachment
    fn generate_file_key(&self, message_id: &str, filename: &str) -> String {
        // Sanitize filename to prevent path traversal
        let sanitized_filename = self.sanitize_filename(filename);

        // Key structure: messages/{message_id}/{uuid}_{filename}
        let unique_filename = format!("{}_{}", uuid::Uuid::new_v4(), sanitized_filename);
        format!("messages/{}/{}", message_id, unique_filename)
    }

    /// Sanitize filename to prevent path traversal and other issues
    fn sanitize_filename(&self, filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                // Replace path separators and dangerous characters
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
                // Keep other characters
                c => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Validate attachment before saving
    pub fn validate_attachment(
        &self,
        filename: &str,
        content_type: &str,
        size: usize,
    ) -> ApiResult<()> {
        // Check size
        if size > MAX_ATTACHMENT_SIZE {
            return Err(ApiError::BadRequest(format!(
                "Attachment size exceeds maximum allowed size of {} MB",
                MAX_ATTACHMENT_SIZE / (1024 * 1024)
            )));
        }

        // Check content type
        if !ALLOWED_CONTENT_TYPES.contains(&content_type) {
            return Err(ApiError::BadRequest(format!(
                "Attachment content type '{}' is not allowed",
                content_type
            )));
        }

        // Check filename
        if filename.is_empty() || filename.len() > 255 {
            return Err(ApiError::BadRequest(
                "Attachment filename must be between 1 and 255 characters".to_string(),
            ));
        }

        Ok(())
    }
}
