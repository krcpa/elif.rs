use crate::{config::MailgunConfig, Email, EmailError, EmailProvider, EmailResult};
use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    multipart::{Form, Part},
    Client,
};
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, error};

/// Mailgun email provider using reqwest HTTP client
#[derive(Clone)]
pub struct MailgunProvider {
    config: MailgunConfig,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct MailgunResponse {
    id: Option<String>,
    message: String,
}

impl MailgunProvider {
    /// Create new Mailgun provider
    pub fn new(config: MailgunConfig) -> Result<Self, EmailError> {
        let timeout = config.timeout.unwrap_or(30);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| {
                EmailError::configuration(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { config, client })
    }

    /// Get Mailgun API endpoint
    fn get_endpoint(&self) -> String {
        let region = self.config.region.as_deref().unwrap_or("us");
        match region {
            "eu" => format!(
                "https://api.eu.mailgun.net/v3/{}/messages",
                self.config.domain
            ),
            _ => format!("https://api.mailgun.net/v3/{}/messages", self.config.domain),
        }
    }

    /// Build request headers
    fn build_headers(&self) -> Result<HeaderMap, EmailError> {
        let mut headers = HeaderMap::new();

        let auth_string = format!("api:{}", self.config.api_key);
        let auth_header = format!("Basic {}", base64::encode(auth_string.as_bytes()));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_header)
                .map_err(|e| EmailError::configuration(format!("Invalid API key format: {}", e)))?,
        );

        Ok(headers)
    }

    /// Convert elif Email to Mailgun multipart form
    fn convert_email(&self, email: &Email) -> Result<Form, EmailError> {
        let mut form = Form::new();

        // From address
        form = form.text("from", email.from.clone());

        // To addresses
        let to_list = email.to.join(",");
        form = form.text("to", to_list);

        // CC addresses
        if let Some(cc_list) = &email.cc {
            if !cc_list.is_empty() {
                form = form.text("cc", cc_list.join(","));
            }
        }

        // BCC addresses
        if let Some(bcc_list) = &email.bcc {
            if !bcc_list.is_empty() {
                form = form.text("bcc", bcc_list.join(","));
            }
        }

        // Reply-to
        if let Some(reply_to) = &email.reply_to {
            form = form.text("h:Reply-To", reply_to.clone());
        }

        // Subject
        form = form.text("subject", email.subject.clone());

        // Body content
        if let Some(text) = &email.text_body {
            form = form.text("text", text.clone());
        }
        if let Some(html) = &email.html_body {
            form = form.text("html", html.clone());
        }

        // Validate that we have at least one body
        if email.text_body.is_none() && email.html_body.is_none() {
            return Err(EmailError::validation(
                "body",
                "Email must have either HTML or text body",
            ));
        }

        // Custom headers
        for (key, value) in &email.headers {
            form = form.text(format!("h:{}", key), value.clone());
        }

        // Tracking options
        if email.tracking.track_opens {
            form = form.text("o:tracking-opens", "true");
        }
        if email.tracking.track_clicks {
            form = form.text("o:tracking-clicks", "true");
        }

        // Custom variables for tracking
        form = form.text("v:email_id", email.id.to_string());
        for (key, value) in &email.tracking.custom_params {
            form = form.text(format!("v:{}", key), value.clone());
        }

        // Attachments
        for attachment in &email.attachments {
            let part = Part::bytes(attachment.content.clone())
                .file_name(attachment.filename.clone())
                .mime_str(&attachment.content_type)
                .map_err(|e| {
                    EmailError::validation(
                        "attachment",
                        format!("Invalid content type '{}': {}", attachment.content_type, e),
                    )
                })?;

            match attachment.disposition {
                crate::AttachmentDisposition::Inline => {
                    form = form.part("inline", part);
                }
                crate::AttachmentDisposition::Attachment => {
                    form = form.part("attachment", part);
                }
            }
        }

        Ok(form)
    }
}

#[async_trait]
impl EmailProvider for MailgunProvider {
    async fn send(&self, email: &Email) -> Result<EmailResult, EmailError> {
        debug!(
            "Sending email via Mailgun: {} -> {:?}",
            email.from, email.to
        );

        let form = self.convert_email(email)?;
        let headers = self.build_headers()?;
        let endpoint = self.get_endpoint();

        let response = self
            .client
            .post(&endpoint)
            .headers(headers)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if status.is_success() {
            // Try to parse the response to get the message ID
            let message_id = if let Ok(mailgun_response) =
                serde_json::from_str::<MailgunResponse>(&response_text)
            {
                mailgun_response
                    .id
                    .unwrap_or_else(|| format!("mailgun-{}", email.id))
            } else {
                format!("mailgun-{}", email.id)
            };

            Ok(EmailResult {
                email_id: email.id,
                message_id,
                sent_at: chrono::Utc::now(),
                provider: "mailgun".to_string(),
            })
        } else {
            let error_msg = if let Ok(mailgun_response) =
                serde_json::from_str::<MailgunResponse>(&response_text)
            {
                mailgun_response.message
            } else {
                format!("HTTP {}: {}", status, response_text)
            };

            error!("Mailgun send failed: {}", error_msg);
            Err(EmailError::provider("Mailgun", error_msg))
        }
    }

    async fn validate_config(&self) -> Result<(), EmailError> {
        debug!(
            "Validating Mailgun configuration for domain: {}",
            self.config.domain
        );

        let headers = self.build_headers()?;

        // Test with domain info endpoint to validate the API key and domain
        let region = self.config.region.as_deref().unwrap_or("us");
        let test_endpoint = match region {
            "eu" => format!("https://api.eu.mailgun.net/v3/{}", self.config.domain),
            _ => format!("https://api.mailgun.net/v3/{}", self.config.domain),
        };

        let response = self
            .client
            .get(&test_endpoint)
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Mailgun configuration validation successful");
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(
                "Mailgun configuration validation failed: {} - {}",
                status, error_text
            );
            Err(EmailError::configuration(format!(
                "Mailgun configuration validation failed: {} - {}",
                status, error_text
            )))
        }
    }

    fn provider_name(&self) -> &'static str {
        "mailgun"
    }
}

// Re-use base64 encoding from SendGrid
mod base64 {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailgun_provider_creation() {
        let config = MailgunConfig::new("test-api-key", "mg.example.com");
        let provider = MailgunProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_endpoint_generation() {
        // US region (default)
        let config = MailgunConfig::new("test-key", "mg.example.com");
        let provider = MailgunProvider::new(config).unwrap();
        assert_eq!(
            provider.get_endpoint(),
            "https://api.mailgun.net/v3/mg.example.com/messages"
        );

        // EU region
        let mut config = MailgunConfig::new("test-key", "mg.example.com");
        config.region = Some("eu".to_string());
        let provider = MailgunProvider::new(config).unwrap();
        assert_eq!(
            provider.get_endpoint(),
            "https://api.eu.mailgun.net/v3/mg.example.com/messages"
        );
    }

    #[test]
    fn test_email_conversion() {
        let config = MailgunConfig::new("test-api-key", "mg.example.com");
        let provider = MailgunProvider::new(config).unwrap();

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Email")
            .text_body("Hello, World!");

        let result = provider.convert_email(&email);
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_with_tracking() {
        let config = MailgunConfig::new("test-api-key", "mg.example.com");
        let provider = MailgunProvider::new(config).unwrap();

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Email")
            .text_body("Hello, World!")
            .with_tracking(true, true);

        let result = provider.convert_email(&email);
        assert!(result.is_ok());
    }
}
