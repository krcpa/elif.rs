//! HTTP server configuration
//! 
//! Provides configuration structures for HTTP server setup, integrating with
//! the elif-core configuration system.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use elif_core::app_config::{AppConfigTrait, ConfigError, ConfigSource};
use std::collections::HashMap;
use std::env;

/// HTTP server specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Keep alive timeout in seconds  
    pub keep_alive_timeout_secs: u64,
    /// Maximum request body size in bytes
    pub max_request_size: usize,
    /// Enable request tracing
    pub enable_tracing: bool,
    /// Health check endpoint path
    pub health_check_path: String,
    /// Server shutdown timeout in seconds
    pub shutdown_timeout_secs: u64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 30,
            keep_alive_timeout_secs: 75,
            max_request_size: 16 * 1024 * 1024, // 16MB
            enable_tracing: true,
            health_check_path: "/health".to_string(),
            shutdown_timeout_secs: 10,
        }
    }
}

impl AppConfigTrait for HttpConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let request_timeout_secs = get_env_or_default("HTTP_REQUEST_TIMEOUT", "30")?
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "request_timeout_secs".to_string(),
                value: env::var("HTTP_REQUEST_TIMEOUT").unwrap_or_default(),
                expected: "valid number of seconds".to_string(),
            })?;

        let keep_alive_timeout_secs = get_env_or_default("HTTP_KEEP_ALIVE_TIMEOUT", "75")?
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "keep_alive_timeout_secs".to_string(),
                value: env::var("HTTP_KEEP_ALIVE_TIMEOUT").unwrap_or_default(),
                expected: "valid number of seconds".to_string(),
            })?;

        let max_request_size = get_env_or_default("HTTP_MAX_REQUEST_SIZE", "16777216")?
            .parse::<usize>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "max_request_size".to_string(),
                value: env::var("HTTP_MAX_REQUEST_SIZE").unwrap_or_default(),
                expected: "valid number of bytes".to_string(),
            })?;

        let enable_tracing = get_env_or_default("HTTP_ENABLE_TRACING", "true")?
            .parse::<bool>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "enable_tracing".to_string(),
                value: env::var("HTTP_ENABLE_TRACING").unwrap_or_default(),
                expected: "true or false".to_string(),
            })?;

        let health_check_path = get_env_or_default("HTTP_HEALTH_CHECK_PATH", "/health")?;

        let shutdown_timeout_secs = get_env_or_default("HTTP_SHUTDOWN_TIMEOUT", "10")?
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "shutdown_timeout_secs".to_string(),
                value: env::var("HTTP_SHUTDOWN_TIMEOUT").unwrap_or_default(),
                expected: "valid number of seconds".to_string(),
            })?;

        Ok(HttpConfig {
            request_timeout_secs,
            keep_alive_timeout_secs,
            max_request_size,
            enable_tracing,
            health_check_path,
            shutdown_timeout_secs,
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Validate timeout values
        if self.request_timeout_secs == 0 {
            return Err(ConfigError::ValidationFailed {
                field: "request_timeout_secs".to_string(),
                reason: "Request timeout must be greater than 0".to_string(),
            });
        }

        if self.keep_alive_timeout_secs == 0 {
            return Err(ConfigError::ValidationFailed {
                field: "keep_alive_timeout_secs".to_string(),
                reason: "Keep-alive timeout must be greater than 0".to_string(),
            });
        }

        if self.shutdown_timeout_secs == 0 {
            return Err(ConfigError::ValidationFailed {
                field: "shutdown_timeout_secs".to_string(),
                reason: "Shutdown timeout must be greater than 0".to_string(),
            });
        }

        // Validate request size limits
        if self.max_request_size == 0 {
            return Err(ConfigError::ValidationFailed {
                field: "max_request_size".to_string(),
                reason: "Maximum request size must be greater than 0".to_string(),
            });
        }

        // Validate health check path
        if self.health_check_path.is_empty() || !self.health_check_path.starts_with('/') {
            return Err(ConfigError::ValidationFailed {
                field: "health_check_path".to_string(),
                reason: "Health check path must be non-empty and start with '/'".to_string(),
            });
        }

        Ok(())
    }

    fn config_sources(&self) -> HashMap<String, ConfigSource> {
        let mut sources = HashMap::new();
        sources.insert("request_timeout_secs".to_string(), 
            ConfigSource::EnvVar("HTTP_REQUEST_TIMEOUT".to_string()));
        sources.insert("keep_alive_timeout_secs".to_string(), 
            ConfigSource::EnvVar("HTTP_KEEP_ALIVE_TIMEOUT".to_string()));
        sources.insert("max_request_size".to_string(), 
            ConfigSource::EnvVar("HTTP_MAX_REQUEST_SIZE".to_string()));
        sources.insert("enable_tracing".to_string(), 
            ConfigSource::EnvVar("HTTP_ENABLE_TRACING".to_string()));
        sources.insert("health_check_path".to_string(), 
            ConfigSource::EnvVar("HTTP_HEALTH_CHECK_PATH".to_string()));
        sources.insert("shutdown_timeout_secs".to_string(), 
            ConfigSource::EnvVar("HTTP_SHUTDOWN_TIMEOUT".to_string()));
        sources
    }
}

impl HttpConfig {
    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Get keep-alive timeout as Duration
    pub fn keep_alive_timeout(&self) -> Duration {
        Duration::from_secs(self.keep_alive_timeout_secs)
    }

    /// Get shutdown timeout as Duration
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_secs)
    }
}

// Helper function for environment variable handling
fn get_env_or_default(key: &str, default: &str) -> Result<String, ConfigError> {
    Ok(env::var(key).unwrap_or_else(|_| default.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Global test lock to prevent concurrent environment modifications
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn set_test_env() {
        env::set_var("HTTP_REQUEST_TIMEOUT", "60");
        env::set_var("HTTP_KEEP_ALIVE_TIMEOUT", "120");
        env::set_var("HTTP_MAX_REQUEST_SIZE", "33554432"); // 32MB
        env::set_var("HTTP_ENABLE_TRACING", "false");
        env::set_var("HTTP_HEALTH_CHECK_PATH", "/api/health");
        env::set_var("HTTP_SHUTDOWN_TIMEOUT", "15");
    }

    fn clean_test_env() {
        env::remove_var("HTTP_REQUEST_TIMEOUT");
        env::remove_var("HTTP_KEEP_ALIVE_TIMEOUT");
        env::remove_var("HTTP_MAX_REQUEST_SIZE");
        env::remove_var("HTTP_ENABLE_TRACING");
        env::remove_var("HTTP_HEALTH_CHECK_PATH");
        env::remove_var("HTTP_SHUTDOWN_TIMEOUT");
    }

    #[test]
    fn test_http_config_defaults() {
        let config = HttpConfig::default();
        
        assert_eq!(config.request_timeout_secs, 30);
        assert_eq!(config.keep_alive_timeout_secs, 75);
        assert_eq!(config.max_request_size, 16 * 1024 * 1024);
        assert!(config.enable_tracing);
        assert_eq!(config.health_check_path, "/health");
        assert_eq!(config.shutdown_timeout_secs, 10);
    }

    #[test]
    fn test_http_config_from_env() {
        let _guard = TEST_MUTEX.lock().unwrap();
        set_test_env();

        let config = HttpConfig::from_env().unwrap();

        assert_eq!(config.request_timeout_secs, 60);
        assert_eq!(config.keep_alive_timeout_secs, 120);
        assert_eq!(config.max_request_size, 33554432);
        assert!(!config.enable_tracing);
        assert_eq!(config.health_check_path, "/api/health");
        assert_eq!(config.shutdown_timeout_secs, 15);

        clean_test_env();
    }

    #[test]
    fn test_http_config_validation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        
        let config = HttpConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid request timeout
        let mut invalid_config = config.clone();
        invalid_config.request_timeout_secs = 0;
        assert!(invalid_config.validate().is_err());

        // Test invalid health check path
        let mut invalid_config = config.clone();
        invalid_config.health_check_path = "no-slash".to_string();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_duration_helpers() {
        let config = HttpConfig::default();
        
        assert_eq!(config.request_timeout(), Duration::from_secs(30));
        assert_eq!(config.keep_alive_timeout(), Duration::from_secs(75));
        assert_eq!(config.shutdown_timeout(), Duration::from_secs(10));
    }
}