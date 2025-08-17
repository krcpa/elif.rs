use crate::EmailError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Email tracking event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackingEvent {
    /// Email was opened
    Open {
        /// When the email was opened
        opened_at: chrono::DateTime<chrono::Utc>,
        /// User agent string
        user_agent: Option<String>,
        /// IP address
        ip_address: Option<String>,
    },
    /// Link was clicked
    Click {
        /// When the link was clicked
        clicked_at: chrono::DateTime<chrono::Utc>,
        /// The clicked URL
        url: String,
        /// User agent string
        user_agent: Option<String>,
        /// IP address
        ip_address: Option<String>,
    },
    /// Email bounced
    Bounce {
        /// When the bounce occurred
        bounced_at: chrono::DateTime<chrono::Utc>,
        /// Bounce type (hard/soft)
        bounce_type: BounceType,
        /// Bounce reason
        reason: String,
    },
    /// Email was delivered
    Delivered {
        /// When the email was delivered
        delivered_at: chrono::DateTime<chrono::Utc>,
        /// Provider response
        response: Option<String>,
    },
    /// Email was marked as spam
    Spam {
        /// When marked as spam
        marked_at: chrono::DateTime<chrono::Utc>,
        /// Report details
        details: Option<String>,
    },
}

/// Email bounce types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BounceType {
    Hard,
    Soft,
}

/// Email tracking record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingRecord {
    /// Email ID being tracked
    pub email_id: Uuid,
    /// Tracking event
    pub event: TrackingEvent,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Email tracking service
pub struct TrackingService {
    /// Base URL for tracking endpoints
    pub base_url: String,
}

impl TrackingService {
    /// Create new tracking service
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    /// Generate tracking pixel URL
    pub fn generate_pixel_url(&self, email_id: Uuid) -> String {
        format!("{}/email/track/open?id={}&t={}", 
                self.base_url, email_id, chrono::Utc::now().timestamp())
    }

    /// Generate tracking link URL
    pub fn generate_link_url(&self, email_id: Uuid, target_url: &str) -> String {
        format!("{}/email/track/click?id={}&url={}", 
                self.base_url, email_id, urlencoding::encode(target_url))
    }

    /// Record tracking event
    pub async fn record_event(&self, record: TrackingRecord) -> Result<(), EmailError> {
        // In a real implementation, this would store to database or send to analytics service
        tracing::info!("Tracking event recorded: {:?}", record);
        Ok(())
    }

    /// Get tracking stats for an email
    pub async fn get_stats(&self, email_id: Uuid) -> Result<EmailStats, EmailError> {
        // In a real implementation, this would query the database
        Ok(EmailStats {
            email_id,
            opens: 0,
            clicks: 0,
            bounces: 0,
            delivered: false,
            first_opened_at: None,
            last_opened_at: None,
        })
    }
}

/// Email statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailStats {
    /// Email ID
    pub email_id: Uuid,
    /// Number of opens
    pub opens: u32,
    /// Number of clicks
    pub clicks: u32,
    /// Number of bounces
    pub bounces: u32,
    /// Whether delivered
    pub delivered: bool,
    /// First opened timestamp
    pub first_opened_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last opened timestamp
    pub last_opened_at: Option<chrono::DateTime<chrono::Utc>>,
}