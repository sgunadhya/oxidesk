/// Email service for sending password reset emails
/// Feature: 017-password-reset
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Failed to build email message: {0}")]
    MessageBuildError(String),

    #[error("Failed to send email: {0}")]
    SendError(String),

    #[error("SMTP configuration error: {0}")]
    ConfigError(String),
}

/// SMTP configuration loaded from environment variables
#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub reset_base_url: String,
}

impl SmtpConfig {
    /// Load SMTP configuration from environment variables
    pub fn from_env() -> Result<Self, EmailError> {
        let host = env::var("SMTP_HOST")
            .map_err(|_| EmailError::ConfigError("SMTP_HOST not set".to_string()))?;

        let port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .map_err(|_| EmailError::ConfigError("Invalid SMTP_PORT".to_string()))?;

        let username = env::var("SMTP_USERNAME")
            .map_err(|_| EmailError::ConfigError("SMTP_USERNAME not set".to_string()))?;

        let password = env::var("SMTP_PASSWORD")
            .map_err(|_| EmailError::ConfigError("SMTP_PASSWORD not set".to_string()))?;

        let from_email = env::var("SMTP_FROM_EMAIL")
            .map_err(|_| EmailError::ConfigError("SMTP_FROM_EMAIL not set".to_string()))?;

        let from_name =
            env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Oxidesk Support".to_string());

        let reset_base_url = env::var("RESET_PASSWORD_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        Ok(Self {
            host,
            port,
            username,
            password,
            from_email,
            from_name,
            reset_base_url,
        })
    }
}

/// Send password reset email with token
///
/// # Arguments
/// * `to_email` - Recipient email address
/// * `reset_token` - 32-character reset token
/// * `config` - SMTP configuration
///
/// # Returns
/// * `Ok(())` if email sent successfully
/// * `Err(EmailError)` if email failed to send
pub async fn send_password_reset_email(
    to_email: &str,
    reset_token: &str,
    config: &SmtpConfig,
) -> Result<(), EmailError> {
    let reset_link = format!(
        "{}/reset-password?token={}",
        config.reset_base_url, reset_token
    );

    // Try to load HTML template, fallback to plain text if not found
    let email_body = match std::fs::read_to_string("templates/password_reset_email.html") {
        Ok(template) => template.replace("{{reset_link}}", &reset_link),
        Err(_) => {
            // Fallback to plain text if template not found
            tracing::warn!("HTML email template not found, using plain text fallback");
            format!(
                "You requested a password reset for your Oxidesk account.\n\n\
                 Click the link below to reset your password:\n\
                 {}\n\n\
                 This link will expire in 1 hour.\n\n\
                 If you did not request a password reset, please ignore this email.",
                reset_link
            )
        }
    };

    let from_address = format!("{} <{}>", config.from_name, config.from_email);

    let content_type = if email_body.contains("<!DOCTYPE html") {
        ContentType::TEXT_HTML
    } else {
        ContentType::TEXT_PLAIN
    };

    let email =
        Message::builder()
            .from(from_address.parse().map_err(|e| {
                EmailError::MessageBuildError(format!("Invalid from address: {}", e))
            })?)
            .to(to_email
                .parse()
                .map_err(|e| EmailError::MessageBuildError(format!("Invalid to address: {}", e)))?)
            .subject("Password Reset Request")
            .header(content_type)
            .body(email_body)
            .map_err(|e| EmailError::MessageBuildError(e.to_string()))?;

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    let mailer = SmtpTransport::starttls_relay(&config.host)
        .map_err(|e| EmailError::SendError(format!("Failed to create SMTP transport: {}", e)))?
        .port(config.port)
        .credentials(creds)
        .build();

    // Send the email asynchronously
    tokio::task::spawn_blocking(move || mailer.send(&email))
        .await
        .map_err(|e| EmailError::SendError(format!("Task join error: {}", e)))?
        .map_err(|e| EmailError::SendError(format!("SMTP send error: {}", e)))?;

    tracing::info!("Password reset email sent successfully to {}", to_email);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_config_defaults() {
        // Test that missing optional env vars use defaults
        std::env::remove_var("SMTP_FROM_NAME");
        std::env::remove_var("RESET_PASSWORD_BASE_URL");

        // Set required vars
        std::env::set_var("SMTP_HOST", "smtp.example.com");
        std::env::set_var("SMTP_PORT", "587");
        std::env::set_var("SMTP_USERNAME", "test@example.com");
        std::env::set_var("SMTP_PASSWORD", "password");
        std::env::set_var("SMTP_FROM_EMAIL", "noreply@example.com");

        let config = SmtpConfig::from_env().unwrap();
        assert_eq!(config.from_name, "Oxidesk Support");
        assert_eq!(config.reset_base_url, "http://localhost:3000");
    }

    #[test]
    fn test_reset_link_format() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "test@example.com".to_string(),
            password: "password".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: "Oxidesk".to_string(),
            reset_base_url: "https://app.example.com".to_string(),
        };

        let token = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6";
        let link = format!("{}/reset-password?token={}", config.reset_base_url, token);

        assert_eq!(
            link,
            "https://app.example.com/reset-password?token=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6"
        );
    }
}
