/// Email Delivery Provider (Feature 021)
///
/// Implements MessageDeliveryProvider trait for sending agent replies via SMTP.
/// Formats emails with reference numbers and sends using lettre.
use crate::database::Database;
use crate::models::Message;
use crate::services::{EmailParserService, MessageDeliveryProvider};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials,
    Message as LettreMessage, SmtpTransport, Transport,
};

/// Email delivery provider for sending agent replies via SMTP
pub struct EmailDeliveryProvider {
    db: Database,
    parser: EmailParserService,
}

impl EmailDeliveryProvider {
    /// Create a new email delivery provider
    pub fn new(db: Database) -> Self {
        Self {
            db,
            parser: EmailParserService::new(),
        }
    }

    /// Render email body from message content with HTML template
    fn render_email_body(&self, content: &str, agent_name: Option<&str>) -> (String, bool) {
        // Try to load HTML template
        let template_path = "templates/agent_reply_email.html";
        match std::fs::read_to_string(template_path) {
            Ok(template) => {
                // Render HTML email
                let signature = if let Some(name) = agent_name {
                    format!("{}<br>Support Team", name)
                } else {
                    "Support Team".to_string()
                };

                let html_content = content.replace("\n", "<br>");
                let rendered = template
                    .replace("{{message_content}}", &html_content)
                    .replace("{{agent_signature}}", &signature);

                (rendered, true) // true = is HTML
            }
            Err(_) => {
                // Fallback to plain text if template not found
                tracing::debug!("HTML email template not found, using plain text fallback");
                let signature = if let Some(name) = agent_name {
                    format!("\n\n---\n{}\nSupport Team", name)
                } else {
                    "\n\n---\nSupport Team".to_string()
                };

                (format!("{}{}", content, signature), false) // false = is plain text
            }
        }
    }

    /// Format subject with reference number
    fn format_subject_with_reference(
        &self,
        original_subject: Option<&str>,
        reference_number: i64,
    ) -> String {
        let subject = original_subject.unwrap_or("Support Request");

        // Use the parser's existing method
        self.parser
            .format_subject_with_reference(subject, reference_number as i32)
    }
}

#[async_trait::async_trait]
impl MessageDeliveryProvider for EmailDeliveryProvider {
    async fn deliver(&self, message: &Message) -> Result<(), String> {
        // Load conversation to get reference number and subject
        let conversation = self
            .db
            .get_conversation_by_id(&message.conversation_id)
            .await
            .map_err(|e| format!("Failed to load conversation: {}", e))?
            .ok_or_else(|| format!("Conversation {} not found", message.conversation_id))?;

        // Get contact's email address from contact channels
        let contact_channels = self
            .db
            .get_contact_channels(&conversation.contact_id)
            .await
            .map_err(|e| format!("Failed to load contact channels: {}", e))?;

        let email_channel = contact_channels
            .into_iter()
            .find(|ch| !ch.email.is_empty())
            .ok_or_else(|| {
                format!(
                    "No email address found for contact {}",
                    conversation.contact_id
                )
            })?;

        // Get inbox email configuration
        let email_config = self
            .db
            .get_inbox_email_config(&conversation.inbox_id)
            .await
            .map_err(|e| format!("Failed to load inbox email config: {}", e))?
            .ok_or_else(|| {
                format!(
                    "No email configuration found for inbox {}",
                    conversation.inbox_id
                )
            })?;

        // Get agent name if message is from agent
        let agent_name = self
            .db
            .get_agent_by_id(&message.author_id)
            .await
            .ok()
            .flatten()
            .and_then(|agent| Some(agent.first_name));

        // Format subject with reference number
        let subject = self.format_subject_with_reference(
            conversation.subject.as_deref(),
            conversation.reference_number,
        );

        // Render email body
        let (body, is_html) = self.render_email_body(&message.content, agent_name.as_deref());

        // Build email message
        let from_address = format!(
            "{} <{}>",
            email_config.display_name, email_config.email_address
        );

        let content_type = if is_html {
            ContentType::TEXT_HTML
        } else {
            ContentType::TEXT_PLAIN
        };

        let email = LettreMessage::builder()
            .from(
                from_address
                    .parse()
                    .map_err(|e| format!("Invalid from address: {}", e))?,
            )
            .to(email_channel
                .email
                .parse()
                .map_err(|e| format!("Invalid to address: {}", e))?)
            .subject(&subject)
            .header(content_type)
            .body(body)
            .map_err(|e| format!("Failed to build email: {}", e))?;

        // Create SMTP transport
        let creds = Credentials::new(
            email_config.smtp_username.clone(),
            email_config.smtp_password.clone(),
        );

        let mailer = if email_config.smtp_use_tls {
            SmtpTransport::starttls_relay(&email_config.smtp_host)
                .map_err(|e| format!("Failed to create SMTP transport: {}", e))?
                .port(email_config.smtp_port as u16)
                .credentials(creds)
                .build()
        } else {
            SmtpTransport::builder_dangerous(&email_config.smtp_host)
                .port(email_config.smtp_port as u16)
                .credentials(creds)
                .build()
        };

        // Send email asynchronously
        tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("SMTP send error: {}", e))?;

        tracing::info!(
            "Email sent successfully to {} for conversation {} [#{}]",
            email_channel.email,
            conversation.id,
            conversation.reference_number
        );

        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "email"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_format_subject_with_reference() {
        let db = Database::new_mock();
        let provider = EmailDeliveryProvider::new(db);

        let subject = provider.format_subject_with_reference(Some("Support Request"), 123);
        assert!(subject.contains("[#123]"));
        assert!(subject.contains("Re: Support Request"));
    }

    #[tokio::test]
    async fn test_render_email_body() {
        let db = Database::new_mock();
        let provider = EmailDeliveryProvider::new(db);

        let (body, _is_html) =
            provider.render_email_body("Thanks for your inquiry!", Some("John Doe"));
        assert!(body.contains("Thanks for your inquiry!"));
        assert!(body.contains("John Doe"));
        assert!(body.contains("Support Team"));
    }

    #[tokio::test]
    async fn test_render_email_body_no_agent() {
        let db = Database::new_mock();
        let provider = EmailDeliveryProvider::new(db);

        let (body, _is_html) = provider.render_email_body("Thanks for your inquiry!", None);
        assert!(body.contains("Thanks for your inquiry!"));
        assert!(body.contains("Support Team"));
        assert!(!body.contains("undefined"));
    }
}
