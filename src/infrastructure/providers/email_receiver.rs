use crate::application::services::AttachmentService;
use crate::domain::entities::{
    ConversationStatus, CreateConversation, EmailProcessingLog, InboxEmailConfig, Message,
};
/// Email Receiver Service (Feature 021)
///
/// Handles receiving and processing incoming emails via IMAP.
/// Creates conversations, messages, contacts, and attachments from emails.
use crate::domain::ports::attachment_repository::AttachmentRepository;
use crate::domain::ports::conversation_repository::ConversationRepository;
use crate::domain::ports::email_repository::EmailRepository;
use crate::domain::ports::message_repository::MessageRepository;
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use crate::infrastructure::providers::{EmailParserService, ParsedEmail};
use async_imap::Session;
use async_native_tls::{TlsConnector, TlsStream};
use futures::StreamExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

/// Email receiver service
pub struct EmailReceiverService {
    email_repo: Arc<dyn EmailRepository>,
    conversation_repo: Arc<dyn ConversationRepository>,
    message_repo: Arc<dyn MessageRepository>,
    contact_service: crate::application::services::ContactService,
    parser: EmailParserService,
    attachment_service: AttachmentService,
}

impl EmailReceiverService {
    /// Create a new email receiver service
    pub fn new(
        email_repo: Arc<dyn EmailRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        message_repo: Arc<dyn MessageRepository>,
        contact_service: crate::application::services::ContactService,
        attachment_service: AttachmentService,
    ) -> Self {
        Self {
            email_repo,
            conversation_repo,
            message_repo,
            contact_service,
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
                .map_err(|e| {
                    ApiError::Internal(format!("Failed to establish TLS connection: {}", e))
                })?
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
        let unseen_uids = session.uid_search("UNSEEN").await.map_err(|e| {
            ApiError::Internal(format!("Failed to search for unseen messages: {:?}", e))
        })?;

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
            .get_or_create_contact(
                inbox_id,
                &parsed_email.from_address,
                parsed_email.from_name.as_deref(),
            )
            .await?;

        // Create conversation
        let create_conv = CreateConversation {
            inbox_id: inbox_id.to_string(),
            contact_id: contact_id.clone(),
            subject: parsed_email.subject.clone(),
        };
        let conversation = self
            .conversation_repo
            .create_conversation(&create_conv)
            .await?;

        // Create incoming message
        let content = parsed_email
            .text_body
            .clone()
            .or_else(|| parsed_email.html_body.clone())
            .unwrap_or_default();
        let message = Message::new_incoming(conversation.id.clone(), content, contact_id.clone());
        let message_id = message.id.clone();
        self.message_repo.create_message(&message).await?;

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
        if let Some(contact) = self
            .contact_service
            .get_contact_by_email(email_address)
            .await?
        {
            return Ok(contact.id);
        }

        // Create new contact using the service method
        let contact_id = self
            .contact_service
            .create_contact_from_message(email_address, name, inbox_id)
            .await?;

        Ok(contact_id)
    }

    /// Process inbox - fetch and process all new emails
    pub async fn process_inbox(&self, inbox_id: &str) -> ApiResult<u32> {
        // Get inbox email configuration
        let config = self
            .email_repo
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
                .email_repo
                .check_email_processed(inbox_id, &parsed_email.message_id)
                .await?
            {
                tracing::info!(
                    "Email {} already processed, skipping",
                    parsed_email.message_id
                );
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
                    tracing::error!(
                        "Failed to process email {}: {:?}",
                        parsed_email.message_id,
                        e
                    );
                    log.mark_failed(e.to_string())
                }
            };

            // Log processing result
            self.email_repo.log_email_processing(&log).await?;
        }

        // Logout from IMAP
        let _ = session.logout().await;

        // Update last poll time
        self.email_repo.update_last_poll_time(inbox_id).await?;

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
                .conversation_repo
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
                let message =
                    Message::new_incoming(conversation.id.clone(), content, contact_id.clone());
                let message_id = message.id.clone();
                self.message_repo.create_message(&message).await?;

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
                    self.conversation_repo
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
        self.process_new_email(inbox_id, email_uid, parsed_email)
            .await
    }
}

use crate::domain::ports::time_service::TimeService;

/// Background email polling worker
pub struct EmailPollingWorker<F>
where
    F: Fn() -> crate::application::services::ContactService + Send + Sync + 'static,
{
    email_repo: Arc<dyn EmailRepository>,
    conversation_repo: Arc<dyn ConversationRepository>,
    message_repo: Arc<dyn MessageRepository>,
    attachment_repo: Arc<dyn AttachmentRepository>,
    contact_service_factory: F,
    file_storage: Arc<dyn crate::domain::ports::file_storage::FileStorage>,
    distributed_lock: Arc<dyn crate::domain::ports::distributed_lock::DistributedLock>,
    time_service: Arc<dyn TimeService>,
}

impl<F> EmailPollingWorker<F>
where
    F: Fn() -> crate::application::services::ContactService + Send + Sync + 'static,
{
    pub fn new(
        email_repo: Arc<dyn EmailRepository>,
        conversation_repo: Arc<dyn ConversationRepository>,
        message_repo: Arc<dyn MessageRepository>,
        attachment_repo: Arc<dyn AttachmentRepository>,
        contact_service_factory: F,
        file_storage: Arc<dyn crate::domain::ports::file_storage::FileStorage>,
        distributed_lock: Arc<dyn crate::domain::ports::distributed_lock::DistributedLock>,
        time_service: Arc<dyn TimeService>,
    ) -> Self {
        Self {
            email_repo,
            conversation_repo,
            message_repo,
            attachment_repo,
            contact_service_factory,
            file_storage,
            distributed_lock,
            time_service,
        }
    }

    pub async fn run(&self) {
        tracing::info!("Email polling worker started");

        loop {
            // Get all enabled email configurations
            match self.email_repo.get_enabled_email_configs().await {
                Ok(configs) => {
                    tracing::info!(
                        "Found {} enabled email configurations to process",
                        configs.len()
                    );

                    // Process each inbox concurrently using select_all or join_all
                    // Since we want to wait for all, join_all is appropriate
                    let mut futures = Vec::new();

                    for config in configs {
                        let contact_service = (self.contact_service_factory)();
                        let receiver = EmailReceiverService::new(
                            self.email_repo.clone(),
                            self.conversation_repo.clone(),
                            self.message_repo.clone(),
                            contact_service,
                            AttachmentService::new(
                                self.attachment_repo.clone(),
                                self.file_storage.clone(),
                            ),
                        );
                        let inbox_id = config.inbox_id.clone();
                        let distributed_lock = self.distributed_lock.clone();

                        futures.push(async move {
                            // Try to acquire distributed lock for this inbox
                            let lock_key = format!("email_poll:{}", inbox_id);
                            // We use a random UUID as owner to identify this worker instance
                            // Ideally this should be a unique worker ID (e.g. hostname + pid or uuid generated on startup)
                            // For now let's just use a fresh UUID inside the closure, but that means if we crash we can't release?
                            // No, if we crash the TTL cleans it up.
                            // But cleaner is to have a consistent worker ID on the struct.
                            // I will use a transient UUID for now as I can't change struct easily without more edits. A new UUID per attempt is technically fine but "owner" matching for release relies on it.
                            let owner = uuid::Uuid::new_v4().to_string();

                            match distributed_lock.acquire(&lock_key, &owner, 50).await {
                                // 50s TTL (slightly less than 60s poll interval)
                                Ok(true) => {
                                    // Got lock, process
                                    match receiver.process_inbox(&inbox_id).await {
                                        Ok(count) => {
                                            if count > 0 {
                                                tracing::info!(
                                                    "Processed {} emails for inbox {}",
                                                    count,
                                                    inbox_id
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to process inbox {}: {:?}",
                                                inbox_id,
                                                e
                                            );
                                        }
                                    }

                                    // Release lock
                                    if let Err(e) =
                                        distributed_lock.release(&lock_key, &owner).await
                                    {
                                        tracing::warn!(
                                            "Failed to release lock for inbox {}: {}",
                                            inbox_id,
                                            e
                                        );
                                    }
                                }
                                Ok(false) => {
                                    tracing::debug!(
                                        "Could not acquire lock for inbox {}, skipping",
                                        inbox_id
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to check lock for inbox {}: {}",
                                        inbox_id,
                                        e
                                    );
                                }
                            }
                        });
                    }

                    // Wait for all inbox processing to complete
                    futures::future::join_all(futures).await;
                }
                Err(e) => {
                    tracing::error!("Failed to get enabled email configs: {:?}", e);
                }
            }

            // Wait 60 seconds before next poll
            self.time_service
                .sleep(std::time::Duration::from_secs(60))
                .await;
        }
    }
}
