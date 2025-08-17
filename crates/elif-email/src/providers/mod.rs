pub mod smtp;
pub mod sendgrid;
pub mod mailgun;

pub use smtp::*;
pub use sendgrid::*;
pub use mailgun::*;

use crate::{Email, EmailError, EmailResult, EmailProvider};
use std::sync::Arc;

/// Email provider manager that handles multiple providers
#[derive(Clone)]
pub struct EmailProviderManager {
    providers: std::collections::HashMap<String, Arc<dyn EmailProvider>>,
    default_provider: String,
}

impl EmailProviderManager {
    /// Create new provider manager
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
            default_provider: String::new(),
        }
    }

    /// Add provider
    pub fn add_provider(
        &mut self,
        name: impl Into<String>,
        provider: Arc<dyn EmailProvider>,
    ) -> &mut Self {
        self.providers.insert(name.into(), provider);
        self
    }

    /// Set default provider
    pub fn set_default(&mut self, name: impl Into<String>) -> &mut Self {
        self.default_provider = name.into();
        self
    }

    /// Get provider by name
    pub fn get_provider(&self, name: &str) -> Result<Arc<dyn EmailProvider>, EmailError> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| EmailError::configuration(format!("Provider '{}' not found", name)))
    }

    /// Get default provider
    pub fn get_default_provider(&self) -> Result<Arc<dyn EmailProvider>, EmailError> {
        if self.default_provider.is_empty() {
            return Err(EmailError::configuration("No default provider set"));
        }
        self.get_provider(&self.default_provider)
    }

    /// Send email using specific provider
    pub async fn send_with_provider(
        &self,
        email: &Email,
        provider_name: &str,
    ) -> Result<EmailResult, EmailError> {
        let provider = self.get_provider(provider_name)?;
        provider.send(email).await
    }

    /// Send email using default provider
    pub async fn send(&self, email: &Email) -> Result<EmailResult, EmailError> {
        let provider = self.get_default_provider()?;
        provider.send(email).await
    }

    /// Validate all providers
    pub async fn validate_all(&self) -> Result<(), EmailError> {
        for (name, provider) in &self.providers {
            if let Err(err) = provider.validate_config().await {
                return Err(EmailError::configuration(format!(
                    "Provider '{}' validation failed: {}",
                    name, err
                )));
            }
        }
        Ok(())
    }

    /// List available providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for EmailProviderManager {
    fn default() -> Self {
        Self::new()
    }
}