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

pub mod config;
pub mod error;
pub mod providers;
pub mod templates;
pub mod tracking;
pub mod mailable;
pub mod validation;

pub use config::*;
pub use error::*;
pub use mailable::*;
pub use providers::*;
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
}

/// Email tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingOptions {
    /// Enable open tracking
    pub track_opens: bool,
    /// Enable click tracking
    pub track_clicks: bool,
    /// Custom tracking parameters
    pub custom_params: HashMap<String, String>,
}

impl Default for TrackingOptions {
    fn default() -> Self {
        Self {
            track_opens: false,
            track_clicks: false,
            custom_params: HashMap::new(),
        }
    }
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
}

impl Default for Email {
    fn default() -> Self {
        Self::new()
    }
}