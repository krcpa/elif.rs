//! Configuration validation logic

use super::HttpConfig;
use elif_core::ConfigError;

impl HttpConfig {
    /// Validate the HTTP configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate timeout values
        if self.request_timeout_secs == 0 {
            return Err(ConfigError::validation_failed("Request timeout must be greater than 0"));
        }

        if self.keep_alive_timeout_secs == 0 {
            return Err(ConfigError::validation_failed("Keep-alive timeout must be greater than 0"));
        }

        if self.shutdown_timeout_secs == 0 {
            return Err(ConfigError::validation_failed("Shutdown timeout must be greater than 0"));
        }

        // Validate request size limits
        if self.max_request_size == 0 {
            return Err(ConfigError::validation_failed("Maximum request size must be greater than 0"));
        }

        // Validate health check path
        if self.health_check_path.is_empty() || !self.health_check_path.starts_with('/') {
            return Err(ConfigError::validation_failed("Health check path must be non-empty and start with '/'"));
        }

        Ok(())
    }
}