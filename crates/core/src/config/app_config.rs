use crate::config::{ConfigError, ConfigSource};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

/// Configuration trait for application configuration
pub trait AppConfigTrait: Sized {
    /// Load configuration from environment variables
    fn from_env() -> Result<Self, ConfigError>;

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError>;

    /// Get configuration source information for debugging
    fn config_sources(&self) -> HashMap<String, ConfigSource>;
}

/// Environment enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl FromStr for Environment {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "testing" | "test" => Ok(Environment::Testing),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(ConfigError::InvalidValue {
                field: "environment".to_string(),
                value: s.to_string(),
                expected: "development, testing, or production".to_string(),
            }),
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env_str = match self {
            Environment::Development => "development",
            Environment::Testing => "testing",
            Environment::Production => "production",
        };
        write!(f, "{}", env_str)
    }
}

impl Environment {
    /// Check if environment is development
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }

    /// Check if environment is testing
    pub fn is_testing(&self) -> bool {
        matches!(self, Environment::Testing)
    }

    /// Check if environment is production
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }

    /// Get debug mode status based on environment
    pub fn debug_mode(&self) -> bool {
        !self.is_production()
    }
}

/// Default application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub environment: Environment,
    pub debug: bool,
    pub port: u16,
    pub host: String,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
    pub log_level: String,
    pub secret_key: Option<String>,
}

impl AppConfig {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self {
            environment: Environment::Development,
            debug: true,
            port: 3000,
            host: "127.0.0.1".to_string(),
            database_url: None,
            redis_url: None,
            log_level: "info".to_string(),
            secret_key: None,
        }
    }

    /// Create configuration for development
    pub fn development() -> Self {
        Self {
            environment: Environment::Development,
            debug: true,
            port: 3000,
            host: "127.0.0.1".to_string(),
            database_url: Some("postgres://localhost/elif_dev".to_string()),
            redis_url: Some("redis://localhost:6379".to_string()),
            log_level: "debug".to_string(),
            secret_key: Some("dev_secret_key".to_string()),
        }
    }

    /// Create configuration for testing
    pub fn testing() -> Self {
        Self {
            environment: Environment::Testing,
            debug: true,
            port: 0, // Random port for tests
            host: "127.0.0.1".to_string(),
            database_url: Some("postgres://localhost/elif_test".to_string()),
            redis_url: Some("redis://localhost:6379/1".to_string()),
            log_level: "warn".to_string(),
            secret_key: Some("test_secret_key".to_string()),
        }
    }

    /// Create configuration for production
    pub fn production() -> Self {
        Self {
            environment: Environment::Production,
            debug: false,
            port: 8080,
            host: "0.0.0.0".to_string(),
            database_url: None, // Must be provided via env
            redis_url: None,    // Must be provided via env
            log_level: "info".to_string(),
            secret_key: None, // Must be provided via env
        }
    }

    /// Get the bind address
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if database is configured
    pub fn has_database(&self) -> bool {
        self.database_url.is_some()
    }

    /// Check if Redis is configured
    pub fn has_redis(&self) -> bool {
        self.redis_url.is_some()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AppConfigTrait for AppConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::new();

        // Environment
        if let Ok(env_str) = env::var("ENVIRONMENT") {
            config.environment = env_str.parse()?;
        }

        // Debug mode (defaults based on environment if not set)
        if let Ok(debug_str) = env::var("DEBUG") {
            config.debug = debug_str.parse().unwrap_or(config.environment.debug_mode());
        } else {
            config.debug = config.environment.debug_mode();
        }

        // Port
        if let Ok(port_str) = env::var("PORT") {
            config.port = port_str.parse().map_err(|_| ConfigError::InvalidValue {
                field: "port".to_string(),
                value: port_str,
                expected: "valid port number (0-65535)".to_string(),
            })?;
        }

        // Host
        if let Ok(host) = env::var("HOST") {
            config.host = host;
        }

        // Database URL
        config.database_url = env::var("DATABASE_URL").ok();

        // Redis URL
        config.redis_url = env::var("REDIS_URL").ok();

        // Log level
        if let Ok(log_level) = env::var("LOG_LEVEL") {
            config.log_level = log_level;
        }

        // Secret key
        config.secret_key = env::var("SECRET_KEY").ok();

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Port is u16, so it's automatically within valid range (0-65535)
        // Only validate that it's not 0 if needed, except in testing environment
        if !self.environment.is_testing() && self.port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "port".to_string(),
                value: self.port.to_string(),
                expected: "port between 1 and 65535".to_string(),
            });
        }

        // Validate log level
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "log_level".to_string(),
                value: self.log_level.clone(),
                expected: format!("one of: {}", valid_levels.join(", ")),
            });
        }

        // Production environment validations
        if self.environment.is_production() {
            if self.secret_key.is_none() {
                return Err(ConfigError::MissingRequired {
                    field: "secret_key".to_string(),
                    hint: "SECRET_KEY environment variable is required in production".to_string(),
                });
            }

            if self.debug {
                return Err(ConfigError::InvalidValue {
                    field: "debug".to_string(),
                    value: "true".to_string(),
                    expected: "false in production environment".to_string(),
                });
            }
        }

        Ok(())
    }

    fn config_sources(&self) -> HashMap<String, ConfigSource> {
        let mut sources = HashMap::new();

        sources.insert(
            "environment".to_string(),
            if env::var("ENVIRONMENT").is_ok() {
                ConfigSource::EnvVar("ENVIRONMENT".to_string())
            } else {
                ConfigSource::Default("development".to_string())
            },
        );

        sources.insert(
            "debug".to_string(),
            if env::var("DEBUG").is_ok() {
                ConfigSource::EnvVar("DEBUG".to_string())
            } else {
                ConfigSource::Default("based on environment".to_string())
            },
        );

        sources.insert(
            "port".to_string(),
            if env::var("PORT").is_ok() {
                ConfigSource::EnvVar("PORT".to_string())
            } else {
                ConfigSource::Default("3000".to_string())
            },
        );

        // Add other fields...

        sources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_validation_in_testing_environment() {
        // Port 0 should be allowed in testing environment
        let mut config = AppConfig::testing();
        config.port = 0;
        assert!(
            config.validate().is_ok(),
            "Port 0 should be allowed in testing environment"
        );
    }

    #[test]
    fn test_port_validation_in_non_testing_environment() {
        // Port 0 should not be allowed in development environment
        let mut config = AppConfig::development();
        config.port = 0;
        assert!(
            config.validate().is_err(),
            "Port 0 should not be allowed in development environment"
        );

        // Port 0 should not be allowed in production environment
        let mut config = AppConfig::production();
        config.port = 0;
        assert!(
            config.validate().is_err(),
            "Port 0 should not be allowed in production environment"
        );
    }

    #[test]
    fn test_valid_port_numbers() {
        // Valid ports should work in all environments
        let mut config = AppConfig::development();
        config.port = 3000;
        assert!(config.validate().is_ok());

        config = AppConfig::testing();
        config.port = 8080;
        assert!(config.validate().is_ok());

        config = AppConfig::production();
        config.port = 443;
        // Note: Production config may fail validation due to other requirements (secret key, etc.)
        // We only care about the port validation here
        let result = config.validate();
        // Extract port-specific errors
        if let Err(ConfigError::InvalidValue { field, .. }) = result {
            assert_ne!(
                field, "port",
                "Port validation should not fail for valid port numbers"
            );
        }
    }
}
