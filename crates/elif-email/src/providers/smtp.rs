use crate::{
    config::{SmtpAuthMethod, SmtpConfig, SmtpTlsConfig},
    Email, EmailError, EmailProvider, EmailResult,
};
use async_trait::async_trait;
use lettre::{
    message::{
        header::ContentType,
        Attachment as LettreAttachment, MultiPart, SinglePart,
    },
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, error};

/// SMTP email provider using lettre
#[derive(Clone)]
pub struct SmtpProvider {
    config: SmtpConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
    /// Connection semaphore for limiting concurrent connections
    connection_semaphore: Arc<Semaphore>,
}

impl SmtpProvider {
    /// Create new SMTP provider
    pub fn new(config: SmtpConfig) -> Result<Self, EmailError> {
        let mut transport_builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|e| EmailError::configuration(format!("Invalid SMTP host: {}", e)))?;

        // Configure authentication
        let creds = Credentials::new(config.username.clone(), config.password.clone());
        transport_builder = transport_builder.credentials(creds);

        // Configure authentication mechanism
        match config.auth_method {
            SmtpAuthMethod::Plain => {
                transport_builder = transport_builder.authentication(vec![Mechanism::Plain]);
            }
            SmtpAuthMethod::Login => {
                transport_builder = transport_builder.authentication(vec![Mechanism::Login]);
            }
            SmtpAuthMethod::XOAuth2 => {
                transport_builder = transport_builder.authentication(vec![Mechanism::Xoauth2]);
            }
        }

        // Configure TLS with proper parameters
        let tls_config = config.effective_tls_config();
        match tls_config {
            SmtpTlsConfig::None => {
                // No TLS
            }
            SmtpTlsConfig::Tls => {
                let tls_parameters = TlsParameters::new(config.host.clone())
                    .map_err(|e| EmailError::configuration(format!("TLS parameter error: {}", e)))?;
                transport_builder = transport_builder.tls(Tls::Required(tls_parameters));
            }
            SmtpTlsConfig::StartTls => {
                let tls_parameters = TlsParameters::new(config.host.clone())
                    .map_err(|e| EmailError::configuration(format!("TLS parameter error: {}", e)))?;
                transport_builder = transport_builder.tls(Tls::Opportunistic(tls_parameters));
            }
            SmtpTlsConfig::StartTlsRequired => {
                let tls_parameters = TlsParameters::new(config.host.clone())
                    .map_err(|e| EmailError::configuration(format!("TLS parameter error: {}", e)))?;
                transport_builder = transport_builder.tls(Tls::Required(tls_parameters));
            }
        }

        // Set up connection semaphore for pooling
        let pool_size = config.pool_size.unwrap_or(10);
        let connection_semaphore = Arc::new(Semaphore::new(pool_size as usize));

        // Configure timeout
        if let Some(timeout) = config.timeout {
            transport_builder = transport_builder.timeout(Some(Duration::from_secs(timeout)));
        }

        // Set port
        transport_builder = transport_builder.port(config.port);

        let transport = transport_builder.build();

        Ok(Self { 
            config, 
            transport,
            connection_semaphore,
        })
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

        // Add custom headers
        // Note: lettre 0.11 has limited custom header support
        // For now, skip custom headers - this is a known limitation
        if !email.headers.is_empty() {
            debug!("Custom headers provided but not fully supported in lettre 0.11: {:?}", email.headers);
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
            // Handle emails with attachments using proper multipart
            let mut multipart = match (&email.html_body, &email.text_body) {
                (Some(html), Some(text)) => {
                    MultiPart::mixed().multipart(
                        MultiPart::alternative()
                            .singlepart(SinglePart::plain(text.clone()))
                            .singlepart(SinglePart::html(html.clone()))
                    )
                }
                (Some(html), None) => {
                    MultiPart::mixed().singlepart(SinglePart::html(html.clone()))
                }
                (None, Some(text)) => {
                    MultiPart::mixed().singlepart(SinglePart::plain(text.clone()))
                }
                (None, None) => {
                    return Err(EmailError::validation("body", "Email must have either HTML or text body"));
                }
            };

            // Add attachments
            for attachment in &email.attachments {
                let lettre_attachment = LettreAttachment::new(attachment.filename.clone())
                    .body(attachment.content.clone(), attachment.content_type.parse().unwrap_or("application/octet-stream".parse().unwrap()));
                
                multipart = multipart.singlepart(lettre_attachment);
            }

            message_builder.multipart(multipart)?
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

        // Implement retry logic
        let mut attempt = 0;
        let max_retries = self.config.max_retries;
        let retry_delay = self.config.retry_delay;

        loop {
            // Acquire connection semaphore permit to limit concurrent connections
            let _permit = self.connection_semaphore.acquire().await
                .map_err(|_| EmailError::provider("SMTP", "Failed to acquire connection permit"))?;
            
            match self.transport.send(message.clone()).await {
                Ok(_response) => {
                    debug!("SMTP send successful on attempt {}", attempt + 1);
                    let message_id = format!("smtp-{}-{}", email.id, chrono::Utc::now().timestamp());
                    
                    return Ok(EmailResult {
                        email_id: email.id,
                        message_id,
                        sent_at: chrono::Utc::now(),
                        provider: "smtp".to_string(),
                    });
                }
                Err(e) => {
                    attempt += 1;
                    error!("SMTP send failed on attempt {}: {}", attempt, e);
                    
                    if attempt >= max_retries {
                        return Err(EmailError::provider("SMTP", format!(
                            "Failed after {} attempts: {}", max_retries, e
                        )));
                    }
                    
                    debug!("Retrying in {} seconds...", retry_delay);
                    sleep(Duration::from_secs(retry_delay)).await;
                }
            }
        }
    }

    async fn validate_config(&self) -> Result<(), EmailError> {
        debug!("Validating SMTP configuration for {}:{}", self.config.host, self.config.port);

        // Acquire connection permit for testing
        let _permit = self.connection_semaphore.acquire().await
            .map_err(|_| EmailError::configuration("Failed to acquire connection permit for validation"))?;

        // Test connection with timeout
        let test_result = tokio::time::timeout(
            Duration::from_secs(self.config.timeout.unwrap_or(30)),
            self.transport.test_connection()
        ).await;

        match test_result {
            Ok(Ok(true)) => {
                debug!("SMTP connection test successful for {}:{}", self.config.host, self.config.port);
                Ok(())
            }
            Ok(Ok(false)) => {
                error!("SMTP connection test failed for {}:{}", self.config.host, self.config.port);
                Err(EmailError::configuration(format!(
                    "SMTP connection test failed for {}:{} - check host, port, and credentials",
                    self.config.host, self.config.port
                )))
            }
            Ok(Err(e)) => {
                error!("SMTP connection test error for {}:{}: {}", self.config.host, self.config.port, e);
                Err(EmailError::configuration(format!(
                    "SMTP connection test error for {}:{}: {}",
                    self.config.host, self.config.port, e
                )))
            }
            Err(_) => {
                error!("SMTP connection test timeout for {}:{}", self.config.host, self.config.port);
                Err(EmailError::configuration(format!(
                    "SMTP connection test timeout for {}:{} - check network connectivity",
                    self.config.host, self.config.port
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