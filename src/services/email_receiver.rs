/// Email Receiver Service (Feature 021)
///
/// Handles receiving and processing incoming emails via IMAP.
/// Creates conversations, messages, contacts, and attachments from emails.

use crate::database::Database;
use crate::error::{ApiError, ApiResult};
use crate::models::{
    ConversationStatus, CreateConversation, EmailProcessingLog, InboxEmailConfig, Message,
};
use crate::services::{AttachmentService, EmailParserService, ParsedEmail};
use async_imap::Session;
use async_native_tls::{TlsConnector, TlsStream};
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

/// Email receiver service
pub struct EmailReceiverService {
    db: Database,
    parser: EmailParserService,
    attachment_service: AttachmentService,
}

impl EmailReceiverService {
    /// Create a new email receiver service
    pub fn new(db: Database, attachment_service: AttachmentService) -> Self {
        Self {
            db,
            parser: EmailParserService::new(),
            attachment_service,
        }
    }

    /// Connect to IMAP server
    async fn connect_imap(
        &self,
        config: &InboxEmailConfig,
    ) -> ApiResult<Session<TlsStream<Compat<TcpStream>>>> {
        // Connect to IMAP server
        let addr = format!("{}:{}", config.imap_host, config.imap_port);
        let tcp_stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to connect to IMAP server: {}", e)))?;

        // Convert tokio stream to futures compat
        let tcp_stream_compat = tcp_stream.compat();

        // Establish TLS connection if enabled
        let tls_stream = if config.imap_use_tls {
            let connector = TlsConnector::new();
            connector
                .connect(&config.imap_host, tcp_stream_compat)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to establish TLS connection: {}", e)))?
        } else {
            return Err(ApiError::Internal(
                "Non-TLS IMAP connections are not supported".to_string(),
            ));
        };

        // Create IMAP client
        let client = async_imap::Client::new(tls_stream);

        // Login
        let session = client
            .login(&config.imap_username, &config.imap_password)
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to authenticate with IMAP server: {:?}", e))
            })?;

        Ok(session)
    }

    /// Fetch new (UNSEEN) emails from inbox
    async fn fetch_new_emails(
        &self,
        session: &mut Session<TlsStream<Compat<TcpStream>>>,
        folder: &str,
    ) -> ApiResult<Vec<(u32, Vec<u8>)>> {
        // Select mailbox
        session
            .select(folder)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to select mailbox: {:?}", e)))?;

        // Search for UNSEEN messages
        let unseen_uids = session
            .uid_search("UNSEEN")
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to search for unseen messages: {:?}", e)))?;

        if unseen_uids.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch email bodies
        let mut emails = Vec::new();
        for uid in unseen_uids {
            // Fetch full RFC822 message
            let mut messages = session
                .uid_fetch(uid.to_string(), "RFC822")
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to fetch email: {:?}", e)))?;

            // Collect the stream
            while let Some(fetch_result) = messages.next().await {
                match fetch_result {
                    Ok(message) => {
                        if let Some(body) = message.body() {
                            emails.push((uid, body.to_vec()));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch email UID {}: {:?}", uid, e);
                    }
                }
            }
        }

        Ok(emails)
    }

    /// Process a new incoming email and create conversation
    async fn process_new_email(
        &self,
        inbox_id: &str,
        _email_uid: u32,
        parsed_email: &ParsedEmail,
    ) -> ApiResult<(String, String)> {
        // Get or create contact from email sender
        let contact_id = self
            .get_or_create_contact(inbox_id, &parsed_email.from_address, parsed_email.from_name.as_deref())
            .await?;

        // Create conversation
        let create_conv = CreateConversation {
            inbox_id: inbox_id.to_string(),
            contact_id: contact_id.clone(),
            subject: parsed_email.subject.clone(),
        };
        let conversation = self.db.create_conversation(&create_conv).await?;

        // Create incoming message
        let content = parsed_email
            .text_body
            .clone()
            .or_else(|| parsed_email.html_body.clone())
            .unwrap_or_default();
        let message = Message::new_incoming(conversation.id.clone(), content, contact_id.clone());
        let message_id = message.id.clone();
        self.db.create_message(&message).await?;

        // Store attachments
        for attachment in &parsed_email.attachments {
            self.attachment_service
                .save_attachment(
                    message_id.clone(),
                    attachment.filename.clone(),
                    attachment.content_type.clone(),
                    attachment.content.clone(),
                )
                .await?;
        }

        Ok((conversation.id, message_id))
    }

    /// Get or create contact from email address
    async fn get_or_create_contact(
        &self,
        inbox_id: &str,
        email_address: &str,
        name: Option<&str>,
    ) -> ApiResult<String> {
        // Try to find existing contact by email
        if let Some(contact) = self.db.get_contact_by_email(email_address).await? {
            return Ok(contact.id);
        }

        // Create new contact using the database method
        let contact_id = self
            .db
            .create_contact_from_message(email_address, name, inbox_id)
            .await?;

        Ok(contact_id)
    }

    /// Process inbox - fetch and process all new emails
    pub async fn process_inbox(&self, inbox_id: &str) -> ApiResult<u32> {
        // Get inbox email configuration
        let config = self
            .db
            .get_inbox_email_config(inbox_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Inbox email configuration not found".to_string()))?;

        if !config.enabled {
            return Ok(0);
        }

        // Connect to IMAP
        let mut session = self.connect_imap(&config).await?;

        // Fetch new emails
        let emails = self
            .fetch_new_emails(&mut session, &config.imap_folder)
            .await?;

        let mut processed_count = 0;

        for (uid, raw_email) in emails {
            // Parse email
            let parsed_email = match self.parser.parse_email(&raw_email) {
                Ok(email) => email,
                Err(e) => {
                    tracing::warn!("Failed to parse email UID {}: {:?}", uid, e);
                    continue;
                }
            };

            // Check for duplicates
            if self
                .db
                .check_email_processed(inbox_id, &parsed_email.message_id)
                .await?
            {
                tracing::info!("Email {} already processed, skipping", parsed_email.message_id);
                continue;
            }

            // Process email (with reply matching support)
            let log = EmailProcessingLog::new(
                inbox_id.to_string(),
                parsed_email.message_id.clone(),
                parsed_email.from_address.clone(),
                parsed_email.subject.clone(),
            );

            let log = match self.process_reply_email(inbox_id, uid, &parsed_email).await {
                Ok((conversation_id, message_id)) => {
                    processed_count += 1;

                    // Mark as SEEN
                    let _ = session
                        .uid_store(format!("{}", uid), "+FLAGS (\\Seen)")
                        .await;

                    log.mark_success(conversation_id, message_id)
                }
                Err(e) => {
                    tracing::error!("Failed to process email {}: {:?}", parsed_email.message_id, e);
                    log.mark_failed(e.to_string())
                }
            };

            // Log processing result
            self.db.log_email_processing(&log).await?;
        }

        // Logout from IMAP
        let _ = session.logout().await;

        // Update last poll time
        self.db.update_last_poll_time(inbox_id).await?;

        Ok(processed_count)
    }

    /// Process email reply (with reference number matching)
    async fn process_reply_email(
        &self,
        inbox_id: &str,
        email_uid: u32,
        parsed_email: &ParsedEmail,
    ) -> ApiResult<(String, String)> {
        // Try to extract reference number from subject
        if let Some(ref_number) = parsed_email
            .subject
            .as_ref()
            .and_then(|s| self.parser.extract_reference_number(s))
        {
            // Try to find existing conversation
            if let Some(conversation) = self
                .db
                .get_conversation_by_reference_number(ref_number as i64)
                .await?
            {
                tracing::info!(
                    "Matched email to conversation {} via reference number #{}",
                    conversation.id,
                    ref_number
                );

                // Get or create contact
                let contact_id = self
                    .get_or_create_contact(
                        inbox_id,
                        &parsed_email.from_address,
                        parsed_email.from_name.as_deref(),
                    )
                    .await?;

                // Create incoming message on existing conversation
                let content = parsed_email
                    .text_body
                    .clone()
                    .or_else(|| parsed_email.html_body.clone())
                    .unwrap_or_default();
                let message = Message::new_incoming(conversation.id.clone(), content, contact_id.clone());
                let message_id = message.id.clone();
                self.db.create_message(&message).await?;

                // Store attachments
                for attachment in &parsed_email.attachments {
                    self.attachment_service
                        .save_attachment(
                            message_id.clone(),
                            attachment.filename.clone(),
                            attachment.content_type.clone(),
                            attachment.content.clone(),
                        )
                        .await?;
                }

                // Reopen conversation if it was closed
                if conversation.status != ConversationStatus::Open {
                    self.db
                        .update_conversation_status(&conversation.id, ConversationStatus::Open)
                        .await?;
                }

                return Ok((conversation.id, message_id));
            } else {
                tracing::warn!(
                    "Reference number #{} found in subject but conversation not found, creating new",
                    ref_number
                );
            }
        }

        // Fallback: create new conversation
        self.process_new_email(inbox_id, email_uid, parsed_email).await
    }
}

/// Spawn background email polling worker
/// Polls all enabled email inboxes every 60 seconds
pub fn spawn_email_polling_worker(db: Database, attachment_storage_path: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Email polling worker started");

        let attachment_storage_path = attachment_storage_path.clone();

        loop {
            // Get all enabled email configurations
            match db.get_enabled_email_configs().await {
                Ok(configs) => {
                    tracing::info!("Found {} enabled email configurations to process", configs.len());

                    // Process each inbox concurrently
                    let mut handles = Vec::new();
                    for config in configs {
                        let receiver = EmailReceiverService::new(
                            db.clone(),
                            AttachmentService::new(db.clone(), attachment_storage_path.clone()),
                        );
                        let inbox_id = config.inbox_id.clone();

                        let handle = tokio::spawn(async move {
                            match receiver.process_inbox(&inbox_id).await {
                                Ok(count) => {
                                    if count > 0 {
                                        tracing::info!("Processed {} emails for inbox {}", count, inbox_id);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to process inbox {}: {:?}", inbox_id, e);
                                }
                            }
                        });

                        handles.push(handle);
                    }

                    // Wait for all inbox processing to complete
                    for handle in handles {
                        let _ = handle.await;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get enabled email configs: {:?}", e);
                }
            }

            // Wait 60 seconds before next poll
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    })
}
