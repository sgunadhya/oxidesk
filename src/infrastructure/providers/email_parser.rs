/// Email Parser Service (Feature 021)
///
/// Handles parsing of incoming emails using mail-parser crate.
/// Extracts headers, body content, and attachments.
use crate::infrastructure::http::middleware::error::ApiResult;
use mail_parser::{MessageParser, MimeHeaders};

/// Parsed email data structure
#[derive(Debug, Clone)]
pub struct ParsedEmail {
    /// RFC 5322 Message-ID header
    pub message_id: String,

    /// Email sender address
    pub from_address: String,

    /// Email sender display name
    pub from_name: Option<String>,

    /// Email subject
    pub subject: Option<String>,

    /// Plain text body content
    pub text_body: Option<String>,

    /// HTML body content
    pub html_body: Option<String>,

    /// Email references (for threading)
    pub references: Vec<String>,

    /// In-Reply-To header (for threading)
    pub in_reply_to: Option<String>,

    /// Parsed attachments
    pub attachments: Vec<EmailAttachment>,
}

/// Email attachment data
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    /// Attachment filename
    pub filename: String,

    /// MIME content type
    pub content_type: String,

    /// Binary content
    pub content: Vec<u8>,

    /// Content size in bytes
    pub size: usize,
}

/// Email parser service
pub struct EmailParserService;

impl EmailParserService {
    /// Create a new email parser service
    pub fn new() -> Self {
        Self
    }

    /// Parse raw email bytes into structured data
    pub fn parse_email(&self, raw_email: &[u8]) -> ApiResult<ParsedEmail> {
        let message = MessageParser::default().parse(raw_email).ok_or_else(|| {
            crate::infrastructure::http::middleware::error::ApiError::BadRequest("Failed to parse email".to_string())
        })?;

        // Extract Message-ID (required)
        let message_id = message
            .message_id()
            .ok_or_else(|| {
                crate::infrastructure::http::middleware::error::ApiError::BadRequest("Email missing Message-ID header".to_string())
            })?
            .to_string();

        // Extract From address (required)
        let from = message
            .from()
            .and_then(|addrs| addrs.first())
            .ok_or_else(|| {
                crate::infrastructure::http::middleware::error::ApiError::BadRequest("Email missing From header".to_string())
            })?;

        let from_address = from
            .address()
            .ok_or_else(|| crate::infrastructure::http::middleware::error::ApiError::BadRequest("Invalid From address".to_string()))?
            .to_string();

        let from_name = from.name().map(|s| s.to_string());

        // Extract subject (optional)
        let subject = message.subject().map(|s| s.to_string());

        // Extract body content
        let text_body = message.body_text(0).map(|s| s.to_string());
        let html_body = message.body_html(0).map(|s| s.to_string());

        // Extract threading headers
        let references = if let Some(list) = message.references().as_text_list() {
            list.iter().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        let in_reply_to = message.in_reply_to().as_text().map(|s| s.to_string());

        // Extract attachments
        let mut attachments = Vec::new();
        for attachment in message.attachments() {
            let body = attachment.contents();

            let filename = attachment
                .attachment_name()
                .unwrap_or("unnamed_attachment")
                .to_string();

            let content_type = attachment
                .content_type()
                .map(|ct| ct.c_type.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            attachments.push(EmailAttachment {
                filename,
                content_type,
                size: body.len(),
                content: body.to_vec(),
            });
        }

        Ok(ParsedEmail {
            message_id,
            from_address,
            from_name,
            subject,
            text_body,
            html_body,
            references,
            in_reply_to,
            attachments,
        })
    }

    /// Extract reference number from email subject
    /// Looks for pattern: [#123] or [REF#123]
    pub fn extract_reference_number(&self, subject: &str) -> Option<i32> {
        // Match patterns like [#123], [REF#123], [ref#123], etc.
        let re = regex::Regex::new(r"(?i)\[(?:ref\s*)?#(\d+)\]").ok()?;

        re.captures(subject)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<i32>().ok())
    }

    /// Format subject with reference number
    /// Example: "Original Subject" -> "Re: Original Subject [#123]"
    pub fn format_subject_with_reference(
        &self,
        original_subject: &str,
        reference_number: i32,
    ) -> String {
        let subject = original_subject.trim();

        // Remove existing "Re:" prefix if present
        let subject = if subject.to_lowercase().starts_with("re:") {
            subject[3..].trim()
        } else {
            subject
        };

        // Remove existing reference number if present
        let re = regex::Regex::new(r"\s*\[(?:ref\s*)?#\d+\]\s*").unwrap();
        let subject = re.replace_all(subject, " ").trim().to_string();

        // Add "Re:" prefix and reference number
        format!("Re: {} [#{}]", subject, reference_number)
    }
}

impl Default for EmailParserService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_reference_number() {
        let parser = EmailParserService::new();

        assert_eq!(
            parser.extract_reference_number("Support Request [#123]"),
            Some(123)
        );
        assert_eq!(
            parser.extract_reference_number("Re: Bug Report [REF#456]"),
            Some(456)
        );
        assert_eq!(
            parser.extract_reference_number("Re: Question [ref#789]"),
            Some(789)
        );
        assert_eq!(parser.extract_reference_number("No reference here"), None);
        assert_eq!(parser.extract_reference_number("Invalid [#abc]"), None);
    }

    #[test]
    fn test_format_subject_with_reference() {
        let parser = EmailParserService::new();

        assert_eq!(
            parser.format_subject_with_reference("Bug Report", 123),
            "Re: Bug Report [#123]"
        );

        assert_eq!(
            parser.format_subject_with_reference("Re: Bug Report", 123),
            "Re: Bug Report [#123]"
        );

        assert_eq!(
            parser.format_subject_with_reference("Bug Report [#456]", 123),
            "Re: Bug Report [#123]"
        );
    }
}
