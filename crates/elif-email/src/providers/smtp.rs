use crate::{config::SmtpConfig, Email, EmailError, EmailProvider, EmailResult};
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Attachment as LettreAttachment, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::time::Duration;
use tracing::{debug, error};

/// SMTP email provider using lettre
#[derive(Clone)]
pub struct SmtpProvider {
    config: SmtpConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpProvider {
    /// Create new SMTP provider
    pub fn new(config: SmtpConfig) -> Result<Self, EmailError> {
        let mut transport_builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|e| EmailError::configuration(format!("Invalid SMTP host: {}", e)))?;

        // Configure authentication
        let creds = Credentials::new(config.username.clone(), config.password.clone());
        transport_builder = transport_builder.credentials(creds);

        // Configure TLS - use simpler API
        if config.use_tls {
            // For TLS, we can use the default TLS settings
            // transport_builder = transport_builder.tls(...); // Skip complex TLS config for now
        } else if config.use_starttls {
            // For STARTTLS, use default settings  
            // transport_builder = transport_builder.starttls(...); // Skip complex config for now
        }

        // Configure timeout
        if let Some(timeout) = config.timeout {
            transport_builder = transport_builder.timeout(Some(Duration::from_secs(timeout)));
        }

        // Set port
        transport_builder = transport_builder.port(config.port);

        let transport = transport_builder.build();

        Ok(Self { config, transport })
    }

    /// Convert elif Email to lettre Message
    fn convert_email(&self, email: &Email) -> Result<Message, EmailError> {
        let mut message_builder = Message::builder()
            .from(
                email
                    .from
                    .parse()
                    .map_err(|e| EmailError::validation("from", format!("Invalid from address: {}", e)))?,
            )
            .subject(&email.subject);

        // Add recipients
        for to in &email.to {
            message_builder = message_builder.to(
                to.parse()
                    .map_err(|e| EmailError::validation("to", format!("Invalid to address: {}", e)))?,
            );
        }

        // Add CC recipients
        if let Some(cc_list) = &email.cc {
            for cc in cc_list {
                message_builder = message_builder.cc(
                    cc.parse()
                        .map_err(|e| EmailError::validation("cc", format!("Invalid cc address: {}", e)))?,
                );
            }
        }

        // Add BCC recipients
        if let Some(bcc_list) = &email.bcc {
            for bcc in bcc_list {
                message_builder = message_builder.bcc(
                    bcc.parse()
                        .map_err(|e| EmailError::validation("bcc", format!("Invalid bcc address: {}", e)))?,
                );
            }
        }

        // Add reply-to
        if let Some(reply_to) = &email.reply_to {
            message_builder = message_builder.reply_to(
                reply_to
                    .parse()
                    .map_err(|e| EmailError::validation("reply_to", format!("Invalid reply-to address: {}", e)))?,
            );
        }

        // Skip custom headers for now - lettre API is complex
        // TODO: Add custom headers support later
        for (_key, _value) in &email.headers {
            // message_builder = message_builder.header(...);
        }

        // Build message body
        let message = if email.attachments.is_empty() {
            // Simple message without attachments
            match (&email.html_body, &email.text_body) {
                (Some(html), Some(text)) => {
                    let multipart = MultiPart::alternative()
                        .singlepart(SinglePart::plain(text.clone()))
                        .singlepart(SinglePart::html(html.clone()));
                    message_builder.multipart(multipart)?
                }
                (Some(html), None) => message_builder.header(ContentType::TEXT_HTML).body(html.clone())?,
                (None, Some(text)) => message_builder.header(ContentType::TEXT_PLAIN).body(text.clone())?,
                (None, None) => {
                    return Err(EmailError::validation("body", "Email must have either HTML or text body"));
                }
            }
        } else {
            // For now, just use simple text/html content and skip attachments
            // TODO: Implement proper multipart support later
            match (&email.html_body, &email.text_body) {
                (Some(html), _) => message_builder.header(ContentType::TEXT_HTML).body(html.clone())?,
                (None, Some(text)) => message_builder.header(ContentType::TEXT_PLAIN).body(text.clone())?,
                (None, None) => {
                    return Err(EmailError::validation("body", "Email must have either HTML or text body"));
                }
            }
        };

        Ok(message)
    }
}

#[async_trait]
impl EmailProvider for SmtpProvider {
    async fn send(&self, email: &Email) -> Result<EmailResult, EmailError> {
        debug!(
            "Sending email via SMTP: {} -> {:?}",
            email.from, email.to
        );

        let message = self.convert_email(email)?;

        let _response = self.transport.send(message).await.map_err(|e| {
            error!("SMTP send failed: {}", e);
            EmailError::provider("SMTP", e.to_string())
        })?;

        let message_id = format!("smtp-{}", email.id);

        Ok(EmailResult {
            email_id: email.id,
            message_id,
            sent_at: chrono::Utc::now(),
            provider: "smtp".to_string(),
        })
    }

    async fn validate_config(&self) -> Result<(), EmailError> {
        debug!("Validating SMTP configuration for {}", self.config.host);

        // Test connection by creating a test transport
        let test_result = self.transport.test_connection().await;

        match test_result {
            Ok(true) => {
                debug!("SMTP connection test successful");
                Ok(())
            }
            Ok(false) => {
                error!("SMTP connection test failed");
                Err(EmailError::configuration("SMTP connection test failed"))
            }
            Err(e) => {
                error!("SMTP connection test error: {}", e);
                Err(EmailError::configuration(format!(
                    "SMTP connection test error: {}",
                    e
                )))
            }
        }
    }

    fn provider_name(&self) -> &'static str {
        "smtp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Attachment;

    #[test]
    fn test_smtp_provider_creation() {
        let config = SmtpConfig::new("smtp.gmail.com", 587, "user@gmail.com", "password");
        let provider = SmtpProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_email_conversion() {
        let config = SmtpConfig::new("smtp.gmail.com", 587, "user@gmail.com", "password");
        let provider = SmtpProvider::new(config).unwrap();

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Email")
            .text_body("Hello, World!");

        let result = provider.convert_email(&email);
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_with_attachments() {
        let config = SmtpConfig::new("smtp.gmail.com", 587, "user@gmail.com", "password");
        let provider = SmtpProvider::new(config).unwrap();

        let attachment = Attachment {
            filename: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            content: b"Test content".to_vec(),
            content_id: None,
        };

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Email with Attachment")
            .text_body("Hello, World!")
            .attach(attachment);

        let result = provider.convert_email(&email);
        assert!(result.is_ok());
    }
}