use std::env;
use std::str::FromStr;
use std::collections::HashMap;
use thiserror::Error;

/// Configuration trait for application configuration
pub trait AppConfigTrait: Sized {
    /// Load configuration from environment variables
    fn from_env() -> Result<Self, ConfigError>;
    
    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError>;
    
    /// Get configuration source information for debugging
    fn config_sources(&self) -> HashMap<String, ConfigSource>;
}

/// Configuration source information for debugging and hot-reload
#[derive(Debug, Clone)]
pub enum ConfigSource {
    EnvVar(String),
    Default(String),
    Nested,
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

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

/// Application configuration structure
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub name: String,
    pub environment: Environment,
    pub database_url: String,
    pub jwt_secret: Option<String>,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl AppConfigTrait for AppConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let name = get_env_or_default("APP_NAME", "elif-app")?;
        let environment = get_env_or_default("APP_ENV", "development")?;
        let environment = Environment::from_str(&environment)?;
        
        let database_url = get_env_required("DATABASE_URL")?;
        let jwt_secret = get_env_optional("JWT_SECRET");
        
        let server = ServerConfig::from_env()?;
        let logging = LoggingConfig::from_env()?;
        
        Ok(AppConfig {
            name,
            environment,
            database_url,
            jwt_secret,
            server,
            logging,
        })
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate app name
        if self.name.is_empty() {
            return Err(ConfigError::ValidationFailed {
                field: "name".to_string(),
                reason: "App name cannot be empty".to_string(),
            });
        }
        
        // Validate database URL
        if self.database_url.is_empty() {
            return Err(ConfigError::ValidationFailed {
                field: "database_url".to_string(),
                reason: "Database URL cannot be empty".to_string(),
            });
        }
        
        // Validate JWT secret in production
        if self.environment == Environment::Production && self.jwt_secret.is_none() {
            return Err(ConfigError::ValidationFailed {
                field: "jwt_secret".to_string(),
                reason: "JWT secret is required in production".to_string(),
            });
        }
        
        // Validate nested configurations
        self.server.validate()?;
        self.logging.validate()?;
        
        Ok(())
    }
    
    fn config_sources(&self) -> HashMap<String, ConfigSource> {
        let mut sources = HashMap::new();
        sources.insert("name".to_string(), 
            ConfigSource::EnvVar("APP_NAME".to_string()));
        sources.insert("environment".to_string(), 
            ConfigSource::EnvVar("APP_ENV".to_string()));
        sources.insert("database_url".to_string(), 
            ConfigSource::EnvVar("DATABASE_URL".to_string()));
        sources.insert("jwt_secret".to_string(), 
            ConfigSource::EnvVar("JWT_SECRET".to_string()));
        sources.insert("server".to_string(), 
            ConfigSource::Nested);
        sources.insert("logging".to_string(), 
            ConfigSource::Nested);
        sources
    }
}

impl AppConfigTrait for ServerConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let host = get_env_or_default("SERVER_HOST", "0.0.0.0")?;
        let port = get_env_or_default("SERVER_PORT", "3000")?;
        let port = port.parse::<u16>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "port".to_string(),
                value: port,
                expected: "valid port number (0-65535)".to_string(),
            })?;
            
        let workers = get_env_or_default("SERVER_WORKERS", "0")?;
        let workers = workers.parse::<usize>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "workers".to_string(),
                value: workers,
                expected: "valid number".to_string(),
            })?;
        
        // Auto-detect workers if 0
        let workers = if workers == 0 {
            num_cpus::get()
        } else {
            workers
        };
        
        Ok(ServerConfig {
            host,
            port,
            workers,
        })
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate host
        if self.host.is_empty() {
            return Err(ConfigError::ValidationFailed {
                field: "host".to_string(),
                reason: "Host cannot be empty".to_string(),
            });
        }
        
        // Validate port range
        if self.port == 0 {
            return Err(ConfigError::ValidationFailed {
                field: "port".to_string(),
                reason: "Port cannot be 0".to_string(),
            });
        }
        
        // Validate workers
        if self.workers == 0 {
            return Err(ConfigError::ValidationFailed {
                field: "workers".to_string(),
                reason: "Workers cannot be 0 after auto-detection".to_string(),
            });
        }
        
        Ok(())
    }
    
    fn config_sources(&self) -> HashMap<String, ConfigSource> {
        let mut sources = HashMap::new();
        sources.insert("host".to_string(), 
            ConfigSource::EnvVar("SERVER_HOST".to_string()));
        sources.insert("port".to_string(), 
            ConfigSource::EnvVar("SERVER_PORT".to_string()));
        sources.insert("workers".to_string(), 
            ConfigSource::EnvVar("SERVER_WORKERS".to_string()));
        sources
    }
}

impl AppConfigTrait for LoggingConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let level = get_env_or_default("LOG_LEVEL", "info")?;
        let format = get_env_or_default("LOG_FORMAT", "compact")?;
        
        Ok(LoggingConfig {
            level,
            format,
        })
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.to_lowercase().as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "level".to_string(),
                value: self.level.clone(),
                expected: "trace, debug, info, warn, or error".to_string(),
            });
        }
        
        // Validate log format
        let valid_formats = ["compact", "pretty", "json"];
        if !valid_formats.contains(&self.format.to_lowercase().as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "format".to_string(),
                value: self.format.clone(),
                expected: "compact, pretty, or json".to_string(),
            });
        }
        
        Ok(())
    }
    
    fn config_sources(&self) -> HashMap<String, ConfigSource> {
        let mut sources = HashMap::new();
        sources.insert("level".to_string(), 
            ConfigSource::EnvVar("LOG_LEVEL".to_string()));
        sources.insert("format".to_string(), 
            ConfigSource::EnvVar("LOG_FORMAT".to_string()));
        sources
    }
}

/// Configuration hot-reload system for development
pub struct ConfigWatcher {
    config: AppConfig,
    last_check: std::time::Instant,
    check_interval: std::time::Duration,
}

impl ConfigWatcher {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            last_check: std::time::Instant::now(),
            check_interval: std::time::Duration::from_secs(1),
        }
    }
    
    /// Check for configuration changes (for development hot-reload)
    pub fn check_for_changes(&mut self) -> Result<Option<AppConfig>, ConfigError> {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_check) < self.check_interval {
            return Ok(None);
        }
        
        self.last_check = now;
        
        // In development mode, reload from environment
        if self.config.environment == Environment::Development {
            let new_config = AppConfig::from_env()?;
            new_config.validate()?;
            
            // Simple comparison - in a real implementation, you'd want more sophisticated change detection
            let changed = format!("{:?}", new_config) != format!("{:?}", self.config);
            
            if changed {
                self.config = new_config.clone();
                return Ok(Some(new_config));
            }
        }
        
        Ok(None)
    }
    
    /// Get current configuration
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
}

// Helper functions for environment variable handling
fn get_env_required(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingEnvVar {
        var: key.to_string(),
    })
}

fn get_env_optional(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn get_env_or_default(key: &str, default: &str) -> Result<String, ConfigError> {
    Ok(env::var(key).unwrap_or_else(|_| default.to_string()))
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {var}")]
    MissingEnvVar { var: String },
    
    #[error("Invalid value for {field}: '{value}', expected {expected}")]
    InvalidValue { field: String, value: String, expected: String },
    
    #[error("Validation failed for {field}: {reason}")]
    ValidationFailed { field: String, reason: String },
    
    #[error("Configuration parsing error: {message}")]
    ParseError { message: String },
    
    #[error("Configuration reload error: {message}")]
    ReloadError { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    // Helper function to set test environment variables
    fn set_test_env() {
        env::set_var("APP_NAME", "test-app");
        env::set_var("APP_ENV", "testing");
        env::set_var("DATABASE_URL", "sqlite::memory:");
        env::set_var("JWT_SECRET", "test-secret-key");
        env::set_var("SERVER_HOST", "127.0.0.1");
        env::set_var("SERVER_PORT", "8080");
        env::set_var("SERVER_WORKERS", "4");
        env::set_var("LOG_LEVEL", "debug");
        env::set_var("LOG_FORMAT", "pretty");
    }
    
    fn clean_test_env() {
        env::remove_var("APP_NAME");
        env::remove_var("APP_ENV");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        env::remove_var("SERVER_HOST");
        env::remove_var("SERVER_PORT");
        env::remove_var("SERVER_WORKERS");
        env::remove_var("LOG_LEVEL");
        env::remove_var("LOG_FORMAT");
    }
    
    #[test]
    fn test_app_config_from_env() {
        set_test_env();
        
        let config = AppConfig::from_env().unwrap();
        
        assert_eq!(config.name, "test-app");
        assert_eq!(config.environment, Environment::Testing);
        assert_eq!(config.database_url, "sqlite::memory:");
        assert_eq!(config.jwt_secret, Some("test-secret-key".to_string()));
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.workers, 4);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.format, "pretty");
        
        clean_test_env();
    }
    
    #[test]
    fn test_app_config_defaults() {
        clean_test_env();
        env::set_var("DATABASE_URL", "sqlite::memory:");
        
        let config = AppConfig::from_env().unwrap();
        
        assert_eq!(config.name, "elif-app");
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.format, "compact");
        
        clean_test_env();
    }
    
    #[test]
    fn test_missing_required_env_var() {
        clean_test_env();
        // Don't set DATABASE_URL
        
        let result = AppConfig::from_env();
        assert!(result.is_err());
        
        if let Err(ConfigError::MissingEnvVar { var }) = result {
            assert_eq!(var, "DATABASE_URL");
        } else {
            panic!("Expected MissingEnvVar error");
        }
    }
    
    #[test]
    fn test_config_validation() {
        set_test_env();
        
        let config = AppConfig::from_env().unwrap();
        assert!(config.validate().is_ok());
        
        clean_test_env();
    }
    
    #[test]
    fn test_production_jwt_secret_validation() {
        set_test_env();
        env::set_var("APP_ENV", "production");
        env::remove_var("JWT_SECRET");
        
        let config = AppConfig::from_env().unwrap();
        let result = config.validate();
        
        assert!(result.is_err());
        if let Err(ConfigError::ValidationFailed { field, .. }) = result {
            assert_eq!(field, "jwt_secret");
        } else {
            panic!("Expected ValidationFailed error for jwt_secret");
        }
        
        clean_test_env();
    }
    
    #[test]
    fn test_invalid_port() {
        set_test_env();
        env::set_var("SERVER_PORT", "invalid");
        
        let result = AppConfig::from_env();
        assert!(result.is_err());
        
        if let Err(ConfigError::InvalidValue { field, .. }) = result {
            assert_eq!(field, "port");
        } else {
            panic!("Expected InvalidValue error for port");
        }
        
        clean_test_env();
    }
    
    #[test]
    fn test_invalid_log_level() {
        set_test_env();
        env::set_var("LOG_LEVEL", "invalid");
        
        let config = AppConfig::from_env().unwrap();
        let result = config.validate();
        
        assert!(result.is_err());
        if let Err(ConfigError::InvalidValue { field, .. }) = result {
            assert_eq!(field, "level");
        } else {
            panic!("Expected InvalidValue error for log level");
        }
        
        clean_test_env();
    }
    
    #[test]
    fn test_environment_parsing() {
        assert_eq!(Environment::from_str("development").unwrap(), Environment::Development);
        assert_eq!(Environment::from_str("dev").unwrap(), Environment::Development);
        assert_eq!(Environment::from_str("testing").unwrap(), Environment::Testing);
        assert_eq!(Environment::from_str("test").unwrap(), Environment::Testing);
        assert_eq!(Environment::from_str("production").unwrap(), Environment::Production);
        assert_eq!(Environment::from_str("prod").unwrap(), Environment::Production);
        
        assert!(Environment::from_str("invalid").is_err());
    }
    
    #[test]
    fn test_config_sources() {
        set_test_env();
        
        let config = AppConfig::from_env().unwrap();
        let sources = config.config_sources();
        
        assert!(matches!(sources.get("name"), Some(ConfigSource::EnvVar(_))));
        assert!(matches!(sources.get("server"), Some(ConfigSource::Nested)));
        
        clean_test_env();
    }
    
    #[test]
    fn test_config_watcher() {
        set_test_env();
        
        let config = AppConfig::from_env().unwrap();
        let mut watcher = ConfigWatcher::new(config);
        
        // Check that no changes are detected immediately
        let result = watcher.check_for_changes().unwrap();
        assert!(result.is_none());
        
        clean_test_env();
    }
}