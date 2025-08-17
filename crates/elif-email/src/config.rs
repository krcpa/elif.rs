use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Email system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Default sender email
    pub default_from: String,
    /// Default email provider
    pub default_provider: String,
    /// Provider configurations
    pub providers: HashMap<String, ProviderConfig>,
    /// Template configuration
    pub templates: TemplateConfig,
    /// Tracking configuration
    pub tracking: GlobalTrackingConfig,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    #[serde(rename = "smtp")]
    Smtp(SmtpConfig),
    #[serde(rename = "sendgrid")]
    SendGrid(SendGridConfig),
    #[serde(rename = "mailgun")]
    Mailgun(MailgunConfig),
}

/// SMTP authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmtpAuthMethod {
    #[serde(rename = "plain")]
    Plain,
    #[serde(rename = "login")]
    Login,
    #[serde(rename = "xoauth2")]
    XOAuth2,
}

/// SMTP TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmtpTlsConfig {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "tls")]
    Tls,
    #[serde(rename = "starttls")]
    StartTls,
    #[serde(rename = "starttls_required")]
    StartTlsRequired,
}

/// SMTP provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP server hostname
    pub host: String,
    /// SMTP server port
    pub port: u16,
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
    /// TLS configuration
    pub tls: SmtpTlsConfig,
    /// Authentication method
    pub auth_method: SmtpAuthMethod,
    /// Connection timeout in seconds
    pub timeout: Option<u64>,
    /// Connection pool size
    pub pool_size: Option<u32>,
    /// Enable connection keepalive
    pub keepalive: bool,
    /// Max retry attempts
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay: u64,
    
    /// Legacy fields for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_starttls: Option<bool>,
}

/// SendGrid provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendGridConfig {
    /// SendGrid API key
    pub api_key: String,
    /// API endpoint (usually v3/mail/send)
    pub endpoint: Option<String>,
    /// Request timeout in seconds
    pub timeout: Option<u64>,
}

/// Mailgun provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailgunConfig {
    /// Mailgun API key
    pub api_key: String,
    /// Mailgun domain
    pub domain: String,
    /// API endpoint region (us/eu)
    pub region: Option<String>,
    /// Request timeout in seconds
    pub timeout: Option<u64>,
}

/// Template system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Templates directory path
    pub templates_dir: String,
    /// Layouts directory path
    pub layouts_dir: String,
    /// Partials directory path
    pub partials_dir: String,
    /// Enable template caching
    pub enable_cache: bool,
    /// Template file extension
    pub template_extension: String,
}

/// Email queue configuration (placeholder for future queue integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Enable background email queuing
    pub enabled: bool,
    /// Queue name for emails
    pub queue_name: String,
    /// Max retry attempts
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay: u64,
    /// Batch size for processing
    pub batch_size: usize,
}

/// Global tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTrackingConfig {
    /// Enable tracking by default
    pub enabled: bool,
    /// Tracking base URL
    pub base_url: Option<String>,
    /// Tracking pixel endpoint
    pub pixel_endpoint: String,
    /// Link redirect endpoint
    pub link_endpoint: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            default_from: "noreply@example.com".to_string(),
            default_provider: "smtp".to_string(),
            providers: HashMap::new(),
            templates: TemplateConfig::default(),
            tracking: GlobalTrackingConfig::default(),
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            templates_dir: "templates/emails".to_string(),
            layouts_dir: "templates/emails/layouts".to_string(),
            partials_dir: "templates/emails/partials".to_string(),
            enable_cache: true,
            template_extension: ".hbs".to_string(),
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            queue_name: "emails".to_string(),
            max_retries: 3,
            retry_delay: 60,
            batch_size: 10,
        }
    }
}

impl Default for GlobalTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: None,
            pixel_endpoint: "/email/track/open".to_string(),
            link_endpoint: "/email/track/click".to_string(),
        }
    }
}

impl Default for SmtpAuthMethod {
    fn default() -> Self {
        SmtpAuthMethod::Plain
    }
}

impl Default for SmtpTlsConfig {
    fn default() -> Self {
        SmtpTlsConfig::StartTls
    }
}

impl SmtpConfig {
    /// Create new SMTP configuration
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            tls: SmtpTlsConfig::StartTls,
            auth_method: SmtpAuthMethod::Plain,
            timeout: Some(30),
            pool_size: Some(10),
            keepalive: true,
            max_retries: 3,
            retry_delay: 5,
            use_tls: None,
            use_starttls: None,
        }
    }

    /// Get effective TLS configuration, handling legacy settings
    pub fn effective_tls_config(&self) -> SmtpTlsConfig {
        // Handle legacy configuration - if either field is present, use legacy logic
        if self.use_tls.is_some() || self.use_starttls.is_some() {
            let use_tls = self.use_tls.unwrap_or(false);
            let use_starttls = self.use_starttls.unwrap_or(false);
            match (use_tls, use_starttls) {
                (true, false) => SmtpTlsConfig::Tls,
                (false, true) => SmtpTlsConfig::StartTls,
                (false, false) => SmtpTlsConfig::None,
                (true, true) => SmtpTlsConfig::StartTls, // Prefer STARTTLS when both are set
            }
        } else {
            self.tls.clone()
        }
    }
}

impl SendGridConfig {
    /// Create new SendGrid configuration
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint: None,
            timeout: Some(30),
        }
    }
}

impl MailgunConfig {
    /// Create new Mailgun configuration
    pub fn new(api_key: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            domain: domain.into(),
            region: None,
            timeout: Some(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_tls_config_new_field() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.tls = SmtpTlsConfig::Tls;
        
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::Tls));
    }

    #[test]
    fn test_effective_tls_config_legacy_both() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.use_tls = Some(true);
        config.use_starttls = Some(false);
        
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::Tls));
    }

    #[test]
    fn test_effective_tls_config_legacy_only_tls() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.use_tls = Some(true);
        // use_starttls remains None, should default to false
        
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::Tls));
    }

    #[test]
    fn test_effective_tls_config_legacy_only_starttls() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.use_starttls = Some(true);
        // use_tls remains None, should default to false
        
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::StartTls));
    }

    #[test]
    fn test_effective_tls_config_legacy_both_false() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.use_tls = Some(false);
        config.use_starttls = Some(false);
        
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::None));
    }

    #[test]
    fn test_effective_tls_config_legacy_both_true() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.use_tls = Some(true);
        config.use_starttls = Some(true);
        
        // Should prefer STARTTLS when both are true
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::StartTls));
    }

    #[test]
    fn test_effective_tls_config_legacy_false_none() {
        let mut config = SmtpConfig::new("smtp.gmail.com", 587, "user", "pass");
        config.use_tls = Some(false);
        // use_starttls remains None, should default to false
        
        assert!(matches!(config.effective_tls_config(), SmtpTlsConfig::None));
    }
}