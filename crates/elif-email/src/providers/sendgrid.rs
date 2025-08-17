use crate::{config::SendGridConfig, Email, EmailError, EmailProvider, EmailResult};
use async_trait::async_trait;
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

/// SendGrid email provider using reqwest HTTP client
#[derive(Clone)]
pub struct SendGridProvider {
    config: SendGridConfig,
    client: Client,
}

#[derive(Debug, Serialize)]
struct SendGridEmail {
    personalizations: Vec<Personalization>,
    from: EmailAddress,
    reply_to: Option<EmailAddress>,
    subject: String,
    content: Vec<Content>,
    attachments: Option<Vec<SendGridAttachment>>,
    headers: Option<std::collections::HashMap<String, String>>,
    custom_args: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct Personalization {
    to: Vec<EmailAddress>,
    cc: Option<Vec<EmailAddress>>,
    bcc: Option<Vec<EmailAddress>>,
    custom_args: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct EmailAddress {
    email: String,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct SendGridAttachment {
    content: String, // Base64 encoded
    #[serde(rename = "type")]
    content_type: String,
    filename: String,
    disposition: String,
    content_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendGridResponse {
    message_id: Option<String>,
    errors: Option<Vec<SendGridError>>,
}

#[derive(Debug, Deserialize)]
struct SendGridError {
    message: String,
    field: Option<String>,
    help: Option<String>,
}

impl SendGridProvider {
    /// Create new SendGrid provider
    pub fn new(config: SendGridConfig) -> Result<Self, EmailError> {
        let timeout = config.timeout.unwrap_or(30);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(|e| EmailError::configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Convert elif Email to SendGrid format
    fn convert_email(&self, email: &Email) -> Result<SendGridEmail, EmailError> {
        // Parse from address
        let from = self.parse_email_address(&email.from)?;

        // Parse reply-to
        let reply_to = if let Some(reply_to) = &email.reply_to {
            Some(self.parse_email_address(reply_to)?)
        } else {
            None
        };

        // Parse recipients
        let to: Result<Vec<EmailAddress>, EmailError> = email
            .to
            .iter()
            .map(|addr| self.parse_email_address(addr))
            .collect();
        let to = to?;

        let cc: Option<Vec<EmailAddress>> = if let Some(cc_list) = &email.cc {
            let cc_result: Result<Vec<EmailAddress>, EmailError> = cc_list
                .iter()
                .map(|addr| self.parse_email_address(addr))
                .collect();
            Some(cc_result?)
        } else {
            None
        };

        let bcc: Option<Vec<EmailAddress>> = if let Some(bcc_list) = &email.bcc {
            let bcc_result: Result<Vec<EmailAddress>, EmailError> = bcc_list
                .iter()
                .map(|addr| self.parse_email_address(addr))
                .collect();
            Some(bcc_result?)
        } else {
            None
        };

        // Build content
        let mut content = Vec::new();
        if let Some(text) = &email.text_body {
            content.push(Content {
                content_type: "text/plain".to_string(),
                value: text.clone(),
            });
        }
        if let Some(html) = &email.html_body {
            content.push(Content {
                content_type: "text/html".to_string(),
                value: html.clone(),
            });
        }

        if content.is_empty() {
            return Err(EmailError::validation("body", "Email must have either HTML or text body"));
        }

        // Build attachments
        let attachments = if email.attachments.is_empty() {
            None
        } else {
            let sendgrid_attachments: Vec<SendGridAttachment> = email
                .attachments
                .iter()
                .map(|att| SendGridAttachment {
                    content: base64::encode(&att.content),
                    content_type: att.content_type.clone(),
                    filename: att.filename.clone(),
                    disposition: if att.content_id.is_some() {
                        "inline".to_string()
                    } else {
                        "attachment".to_string()
                    },
                    content_id: att.content_id.clone(),
                })
                .collect();
            Some(sendgrid_attachments)
        };

        // Build custom args for tracking
        let mut custom_args = std::collections::HashMap::new();
        custom_args.insert("email_id".to_string(), email.id.to_string());
        custom_args.extend(email.tracking.custom_params.clone());

        let personalization = Personalization {
            to,
            cc,
            bcc,
            custom_args: Some(custom_args),
        };

        Ok(SendGridEmail {
            personalizations: vec![personalization],
            from,
            reply_to,
            subject: email.subject.clone(),
            content,
            attachments,
            headers: if email.headers.is_empty() {
                None
            } else {
                Some(email.headers.clone())
            },
            custom_args: Some(email.tracking.custom_params.clone()),
        })
    }

    /// Parse email address (simple implementation)
    fn parse_email_address(&self, addr: &str) -> Result<EmailAddress, EmailError> {
        // Simple parsing - in real implementation you might want more sophisticated parsing
        if addr.contains('<') && addr.contains('>') {
            // Format: "Name <email@domain.com>"
            let parts: Vec<&str> = addr.split('<').collect();
            if parts.len() != 2 {
                return Err(EmailError::validation("email", format!("Invalid email format: {}", addr)));
            }
            let name = parts[0].trim().trim_matches('"');
            let email = parts[1].trim().trim_end_matches('>');
            
            Ok(EmailAddress {
                email: email.to_string(),
                name: if name.is_empty() { None } else { Some(name.to_string()) },
            })
        } else {
            // Simple email address
            Ok(EmailAddress {
                email: addr.to_string(),
                name: None,
            })
        }
    }

    /// Get SendGrid API endpoint
    fn get_endpoint(&self) -> String {
        self.config
            .endpoint
            .clone()
            .unwrap_or_else(|| "https://api.sendgrid.com/v3/mail/send".to_string())
    }

    /// Build request headers
    fn build_headers(&self) -> Result<HeaderMap, EmailError> {
        let mut headers = HeaderMap::new();
        
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        let auth_header = format!("Bearer {}", self.config.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_header)
                .map_err(|e| EmailError::configuration(format!("Invalid API key format: {}", e)))?,
        );

        Ok(headers)
    }
}

#[async_trait]
impl EmailProvider for SendGridProvider {
    async fn send(&self, email: &Email) -> Result<EmailResult, EmailError> {
        debug!(
            "Sending email via SendGrid: {} -> {:?}",
            email.from, email.to
        );

        let sendgrid_email = self.convert_email(email)?;
        let headers = self.build_headers()?;
        let endpoint = self.get_endpoint();

        let response = self
            .client
            .post(&endpoint)
            .headers(headers)
            .json(&sendgrid_email)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if status.is_success() {
            // SendGrid returns message ID in the response headers for some endpoints
            // For v3/mail/send, we generate one based on the email ID
            let message_id = format!("sendgrid-{}", email.id);

            Ok(EmailResult {
                email_id: email.id,
                message_id,
                sent_at: chrono::Utc::now(),
                provider: "sendgrid".to_string(),
            })
        } else {
            // Try to parse error response
            let error_msg = if let Ok(sendgrid_response) = serde_json::from_str::<SendGridResponse>(&response_text) {
                if let Some(errors) = sendgrid_response.errors {
                    errors
                        .into_iter()
                        .map(|e| e.message)
                        .collect::<Vec<_>>()
                        .join("; ")
                } else {
                    format!("HTTP {}: {}", status, response_text)
                }
            } else {
                format!("HTTP {}: {}", status, response_text)
            };

            error!("SendGrid send failed: {}", error_msg);
            Err(EmailError::provider("SendGrid", error_msg))
        }
    }

    async fn validate_config(&self) -> Result<(), EmailError> {
        debug!("Validating SendGrid configuration");

        let headers = self.build_headers()?;
        
        // Test with a simple API call to validate the API key
        let test_endpoint = "https://api.sendgrid.com/v3/user/account";
        
        let response = self
            .client
            .get(test_endpoint)
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            debug!("SendGrid API key validation successful");
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("SendGrid API key validation failed: {} - {}", status, error_text);
            Err(EmailError::configuration(format!(
                "SendGrid API key validation failed: {} - {}",
                status, error_text
            )))
        }
    }

    fn provider_name(&self) -> &'static str {
        "sendgrid"
    }
}

// Add base64 encoding for attachments
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
    use crate::Attachment;

    #[test]
    fn test_sendgrid_provider_creation() {
        let config = SendGridConfig::new("test-api-key");
        let provider = SendGridProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_email_address_parsing() {
        let config = SendGridConfig::new("test-api-key");
        let provider = SendGridProvider::new(config).unwrap();

        // Simple email
        let result = provider.parse_email_address("user@example.com");
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.email, "user@example.com");
        assert_eq!(addr.name, None);

        // Email with name
        let result = provider.parse_email_address("\"John Doe\" <john@example.com>");
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.email, "john@example.com");
        assert_eq!(addr.name, Some("John Doe".to_string()));
    }

    #[test]
    fn test_email_conversion() {
        let config = SendGridConfig::new("test-api-key");
        let provider = SendGridProvider::new(config).unwrap();

        let email = Email::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test Email")
            .text_body("Hello, World!");

        let result = provider.convert_email(&email);
        assert!(result.is_ok());

        let sendgrid_email = result.unwrap();
        assert_eq!(sendgrid_email.from.email, "sender@example.com");
        assert_eq!(sendgrid_email.personalizations[0].to[0].email, "recipient@example.com");
        assert_eq!(sendgrid_email.subject, "Test Email");
    }
}