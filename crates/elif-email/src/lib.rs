//! # elif-email
//!
//! Email system for elif.rs with multiple providers, templating, and background queuing.
//!
//! ## Features
//!
//! - Multiple email providers (SMTP, SendGrid, Mailgun)
//! - Handlebars template system with layouts
//! - Background email queuing
//! - Email tracking and analytics
//! - Type-safe email composition with Mailable trait

pub mod compression;
pub mod config;
pub mod error;
pub mod mailable;
pub mod providers;
pub mod queue;
pub mod templates;
pub mod tracking;
pub mod validation;

#[cfg(feature = "integration-examples")]
pub mod integration_example;

pub use compression::*;
pub use config::*;
pub use error::*;
pub use mailable::*;
pub use providers::*;
#[cfg(test)]
pub use providers::{MockEmailProvider, PanickingEmailProvider};
pub use queue::*;
pub use templates::*;
pub use tracking::*;
pub use validation::*;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core email message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    /// Unique identifier for tracking
    pub id: Uuid,
    /// Sender email address
    pub from: String,
    /// Recipient email addresses
    pub to: Vec<String>,
    /// CC recipients
    pub cc: Option<Vec<String>>,
    /// BCC recipients
    pub bcc: Option<Vec<String>>,
    /// Reply-to address
    pub reply_to: Option<String>,
    /// Email subject
    pub subject: String,
    /// HTML body content
    pub html_body: Option<String>,
    /// Plain text body content
    pub text_body: Option<String>,
    /// Email attachments
    pub attachments: Vec<Attachment>,
    /// Email headers
    pub headers: HashMap<String, String>,
    /// Tracking metadata
    pub tracking: TrackingOptions,
}

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Filename
    pub filename: String,
    /// MIME content type
    pub content_type: String,
    /// Binary content
    pub content: Vec<u8>,
    /// Inline attachment ID for embedding in HTML
    pub content_id: Option<String>,
    /// Attachment disposition (attachment or inline)
    pub disposition: AttachmentDisposition,
    /// Attachment size in bytes
    pub size: usize,
    /// Whether content is compressed
    pub compressed: bool,
}

/// Attachment disposition type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AttachmentDisposition {
    /// Regular file attachment
    #[default]
    Attachment,
    /// Inline attachment (e.g., embedded image)
    Inline,
}

/// Email tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackingOptions {
    /// Enable open tracking
    pub track_opens: bool,
    /// Enable click tracking
    pub track_clicks: bool,
    /// Custom tracking parameters
    pub custom_params: HashMap<String, String>,
}

/// Email provider abstraction
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email immediately
    async fn send(&self, email: &Email) -> Result<EmailResult, EmailError>;

    /// Validate configuration
    async fn validate_config(&self) -> Result<(), EmailError>;

    /// Get provider name
    fn provider_name(&self) -> &'static str;
}

/// Email sending result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailResult {
    /// Email ID
    pub email_id: Uuid,
    /// Provider-specific message ID
    pub message_id: String,
    /// Send timestamp
    pub sent_at: chrono::DateTime<chrono::Utc>,
    /// Provider name
    pub provider: String,
}

impl Email {
    /// Create a new email
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            from: String::new(),
            to: Vec::new(),
            cc: None,
            bcc: None,
            reply_to: None,
            subject: String::new(),
            html_body: None,
            text_body: None,
            attachments: Vec::new(),
            headers: HashMap::new(),
            tracking: TrackingOptions::default(),
        }
    }

    /// Set sender
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }

    /// Add recipient
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to.push(to.into());
        self
    }

    /// Set subject
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set HTML body
    pub fn html_body(mut self, html: impl Into<String>) -> Self {
        self.html_body = Some(html.into());
        self
    }

    /// Set text body
    pub fn text_body(mut self, text: impl Into<String>) -> Self {
        self.text_body = Some(text.into());
        self
    }

    /// Add attachment
    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Enable tracking
    pub fn with_tracking(mut self, track_opens: bool, track_clicks: bool) -> Self {
        self.tracking.track_opens = track_opens;
        self.tracking.track_clicks = track_clicks;
        self
    }

    /// Add an inline attachment (for embedding in HTML emails)
    pub fn attach_inline(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Validate all attachments against configuration
    pub fn validate_attachments(
        &self,
        config: &crate::config::AttachmentConfig,
    ) -> Result<(), crate::EmailError> {
        crate::compression::validate_attachments(&self.attachments, config)
    }

    /// Get all inline attachments (for HTML embedding)
    pub fn inline_attachments(&self) -> Vec<&Attachment> {
        self.attachments
            .iter()
            .filter(|a| matches!(a.disposition, AttachmentDisposition::Inline))
            .collect()
    }

    /// Get all regular attachments
    pub fn regular_attachments(&self) -> Vec<&Attachment> {
        self.attachments
            .iter()
            .filter(|a| matches!(a.disposition, AttachmentDisposition::Attachment))
            .collect()
    }

    /// Apply compression to all attachments if configured
    pub fn compress_attachments(
        &mut self,
        config: &crate::config::AttachmentConfig,
    ) -> Result<(), crate::EmailError> {
        for attachment in &mut self.attachments {
            crate::compression::AttachmentCompressor::compress_if_beneficial(attachment, config)?;
        }
        Ok(())
    }

    /// Get estimated size after compression
    pub fn estimated_compressed_size(&self) -> usize {
        self.attachments
            .iter()
            .map(|a| {
                if a.compressed {
                    a.size
                } else {
                    let ratio =
                        crate::compression::AttachmentCompressor::estimate_compression_ratio(a);
                    (a.size as f64 * ratio) as usize
                }
            })
            .sum()
    }
}

impl Default for Email {
    fn default() -> Self {
        Self::new()
    }
}

impl Attachment {
    /// Create a new attachment with automatic MIME type detection
    pub fn new(filename: impl Into<String>, content: Vec<u8>) -> Self {
        let filename = filename.into();
        let content_type = mime_guess::from_path(&filename)
            .first()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let size = content.len();

        Self {
            filename,
            content_type,
            content,
            content_id: None,
            disposition: AttachmentDisposition::Attachment,
            size,
            compressed: false,
        }
    }

    /// Create a new inline attachment (for embedding in HTML)
    pub fn inline(
        filename: impl Into<String>,
        content: Vec<u8>,
        content_id: impl Into<String>,
    ) -> Self {
        let mut attachment = Self::new(filename, content);
        attachment.disposition = AttachmentDisposition::Inline;
        attachment.content_id = Some(content_id.into());
        attachment
    }

    /// Set custom content type
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    /// Set as inline attachment
    pub fn as_inline(mut self, content_id: impl Into<String>) -> Self {
        self.disposition = AttachmentDisposition::Inline;
        self.content_id = Some(content_id.into());
        self
    }

    /// Check if attachment is an image
    pub fn is_image(&self) -> bool {
        self.content_type.starts_with("image/")
    }

    /// Check if attachment can be compressed
    pub fn can_compress(&self) -> bool {
        matches!(
            self.content_type.as_str(),
            "image/jpeg"
                | "image/png"
                | "image/webp"
                | "text/plain"
                | "text/html"
                | "text/css"
                | "text/javascript"
                | "application/json"
                | "application/xml"
        )
    }

    /// Validate attachment against configuration
    pub fn validate(
        &self,
        config: &crate::config::AttachmentConfig,
    ) -> Result<(), crate::EmailError> {
        // Check size limits
        if self.size > config.max_size {
            return Err(crate::EmailError::validation(
                "attachment_size",
                format!(
                    "Attachment '{}' is too large: {} bytes (max: {} bytes)",
                    self.filename, self.size, config.max_size
                ),
            ));
        }

        // Check allowed types
        if !config.allowed_types.is_empty() && !config.allowed_types.contains(&self.content_type) {
            return Err(crate::EmailError::validation(
                "attachment_type",
                format!("Attachment type '{}' is not allowed", self.content_type),
            ));
        }

        // Check blocked types
        if config.blocked_types.contains(&self.content_type) {
            return Err(crate::EmailError::validation(
                "attachment_type",
                format!("Attachment type '{}' is blocked", self.content_type),
            ));
        }

        Ok(())
    }
}
