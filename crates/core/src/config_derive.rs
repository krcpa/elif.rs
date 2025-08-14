use crate::app_config::{AppConfigTrait, ConfigError, ConfigSource};
use std::collections::HashMap;

/// Attribute-based configuration builder for creating configuration structs
/// 
/// This provides a simplified approach to configuration without proc macros,
/// using builder pattern and attribute-like methods.
pub struct ConfigBuilder<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ConfigBuilder<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Configuration field descriptor for manual configuration building
pub struct ConfigField {
    pub name: String,
    pub env_var: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
    pub nested: bool,
    pub validation: Option<Box<dyn Fn(&str) -> Result<(), ConfigError> + Send + Sync>>,
}

impl std::fmt::Debug for ConfigField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigField")
            .field("name", &self.name)
            .field("env_var", &self.env_var)
            .field("default_value", &self.default_value)
            .field("required", &self.required)
            .field("nested", &self.nested)
            .field("validation", &self.validation.is_some())
            .finish()
    }
}

impl Clone for ConfigField {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            env_var: self.env_var.clone(),
            default_value: self.default_value.clone(),
            required: self.required,
            nested: self.nested,
            validation: None, // Can't clone function pointers
        }
    }
}

impl ConfigField {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            env_var: None,
            default_value: None,
            required: false,
            nested: false,
            validation: None,
        }
    }
    
    pub fn env(mut self, env_var: impl Into<String>) -> Self {
        self.env_var = Some(env_var.into());
        self
    }
    
    pub fn default(mut self, default_value: impl Into<String>) -> Self {
        self.default_value = Some(default_value.into());
        self
    }
    
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    
    pub fn nested(mut self) -> Self {
        self.nested = true;
        self
    }
    
    pub fn validate<F>(mut self, validator: F) -> Self 
    where
        F: Fn(&str) -> Result<(), ConfigError> + Send + Sync + 'static,
    {
        self.validation = Some(Box::new(validator));
        self
    }
}

/// Configuration schema for defining configuration structures
pub struct ConfigSchema {
    pub name: String,
    pub fields: Vec<ConfigField>,
}

impl ConfigSchema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }
    
    pub fn field(mut self, field: ConfigField) -> Self {
        self.fields.push(field);
        self
    }
    
    /// Load configuration values based on schema
    pub fn load_values(&self) -> Result<HashMap<String, String>, ConfigError> {
        let mut values = HashMap::new();
        
        for field in &self.fields {
            if field.nested {
                // Nested fields are handled separately
                continue;
            }
            
            let value = if let Some(env_var) = &field.env_var {
                match std::env::var(env_var) {
                    Ok(val) => val,
                    Err(_) if field.required => {
                        return Err(ConfigError::MissingEnvVar {
                            var: env_var.clone(),
                        });
                    }
                    Err(_) => {
                        if let Some(default) = &field.default_value {
                            default.clone()
                        } else {
                            continue;
                        }
                    }
                }
            } else if let Some(default) = &field.default_value {
                default.clone()
            } else if field.required {
                return Err(ConfigError::MissingEnvVar {
                    var: format!("{}_NOT_SPECIFIED", field.name.to_uppercase()),
                });
            } else {
                continue;
            };
            
            // Apply validation if present
            if let Some(validator) = &field.validation {
                validator(&value)?;
            }
            
            values.insert(field.name.clone(), value);
        }
        
        Ok(values)
    }
    
    /// Get configuration sources for debugging
    pub fn get_sources(&self) -> HashMap<String, ConfigSource> {
        let mut sources = HashMap::new();
        
        for field in &self.fields {
            let source = if field.nested {
                ConfigSource::Nested
            } else if let Some(env_var) = &field.env_var {
                ConfigSource::EnvVar(env_var.clone())
            } else if field.default_value.is_some() {
                ConfigSource::Default(field.name.clone())
            } else {
                ConfigSource::EnvVar(format!("{}_NOT_SPECIFIED", field.name.to_uppercase()))
            };
            
            sources.insert(field.name.clone(), source);
        }
        
        sources
    }
}

/// Example of a manually defined configuration using the schema system
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: Option<String>,
    pub pool_size: usize,
}

impl DatabaseConfig {
    /// Create configuration schema for DatabaseConfig
    pub fn schema() -> ConfigSchema {
        ConfigSchema::new("DatabaseConfig")
            .field(
                ConfigField::new("host")
                    .env("DB_HOST")
                    .default("localhost")
                    .validate(|val| {
                        if val.is_empty() {
                            Err(ConfigError::ValidationFailed {
                                field: "host".to_string(),
                                reason: "Host cannot be empty".to_string(),
                            })
                        } else {
                            Ok(())
                        }
                    })
            )
            .field(
                ConfigField::new("port")
                    .env("DB_PORT")
                    .default("5432")
                    .validate(|val| {
                        val.parse::<u16>().map_err(|_| ConfigError::InvalidValue {
                            field: "port".to_string(),
                            value: val.to_string(),
                            expected: "valid port number (0-65535)".to_string(),
                        })?;
                        Ok(())
                    })
            )
            .field(
                ConfigField::new("name")
                    .env("DB_NAME")
                    .required()
            )
            .field(
                ConfigField::new("username")
                    .env("DB_USERNAME")
                    .required()
            )
            .field(
                ConfigField::new("password")
                    .env("DB_PASSWORD")
            )
            .field(
                ConfigField::new("pool_size")
                    .env("DB_POOL_SIZE")
                    .default("10")
                    .validate(|val| {
                        let size: usize = val.parse().map_err(|_| ConfigError::InvalidValue {
                            field: "pool_size".to_string(),
                            value: val.to_string(),
                            expected: "valid number".to_string(),
                        })?;
                        
                        if size == 0 || size > 100 {
                            Err(ConfigError::ValidationFailed {
                                field: "pool_size".to_string(),
                                reason: "Pool size must be between 1 and 100".to_string(),
                            })
                        } else {
                            Ok(())
                        }
                    })
            )
    }
}

impl AppConfigTrait for DatabaseConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let schema = Self::schema();
        let values = schema.load_values()?;
        
        let host = values.get("host").unwrap_or(&"localhost".to_string()).clone();
        let port = values.get("port").unwrap_or(&"5432".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "port".to_string(),
                value: values.get("port").unwrap_or(&"5432".to_string()).clone(),
                expected: "valid port number".to_string(),
            })?;
        
        let name = values.get("name").ok_or_else(|| ConfigError::MissingEnvVar {
            var: "DB_NAME".to_string(),
        })?.clone();
        
        let username = values.get("username").ok_or_else(|| ConfigError::MissingEnvVar {
            var: "DB_USERNAME".to_string(),
        })?.clone();
        
        let password = values.get("password").cloned();
        
        let pool_size = values.get("pool_size").unwrap_or(&"10".to_string())
            .parse::<usize>()
            .map_err(|_| ConfigError::InvalidValue {
                field: "pool_size".to_string(),
                value: values.get("pool_size").unwrap_or(&"10".to_string()).clone(),
                expected: "valid number".to_string(),
            })?;
        
        Ok(DatabaseConfig {
            host,
            port,
            name,
            username,
            password,
            pool_size,
        })
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        if self.host.is_empty() {
            return Err(ConfigError::ValidationFailed {
                field: "host".to_string(),
                reason: "Host cannot be empty".to_string(),
            });
        }
        
        if self.name.is_empty() {
            return Err(ConfigError::ValidationFailed {
                field: "name".to_string(),
                reason: "Database name cannot be empty".to_string(),
            });
        }
        
        if self.username.is_empty() {
            return Err(ConfigError::ValidationFailed {
                field: "username".to_string(),
                reason: "Username cannot be empty".to_string(),
            });
        }
        
        if self.pool_size == 0 || self.pool_size > 100 {
            return Err(ConfigError::ValidationFailed {
                field: "pool_size".to_string(),
                reason: "Pool size must be between 1 and 100".to_string(),
            });
        }
        
        Ok(())
    }
    
    fn config_sources(&self) -> HashMap<String, ConfigSource> {
        Self::schema().get_sources()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    
    // Global test lock to prevent concurrent environment modifications
    static TEST_MUTEX: Mutex<()> = Mutex::new(());
    
    #[test]
    fn test_config_field_builder() {
        let field = ConfigField::new("test_field")
            .env("TEST_VAR")
            .default("default_value")
            .required();
        
        assert_eq!(field.name, "test_field");
        assert_eq!(field.env_var, Some("TEST_VAR".to_string()));
        assert_eq!(field.default_value, Some("default_value".to_string()));
        assert!(field.required);
    }
    
    #[test]
    fn test_config_schema() {
        let schema = ConfigSchema::new("TestConfig")
            .field(
                ConfigField::new("field1")
                    .env("TEST_FIELD1")
                    .default("default1")
            )
            .field(
                ConfigField::new("field2")
                    .env("TEST_FIELD2")
                    .required()
            );
        
        assert_eq!(schema.name, "TestConfig");
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.fields[0].name, "field1");
        assert_eq!(schema.fields[1].name, "field2");
    }
    
    #[test]
    fn test_database_config_from_env() {
        let _guard = TEST_MUTEX.lock().unwrap();
        // Set test environment
        env::set_var("DB_HOST", "test-host");
        env::set_var("DB_PORT", "3306");
        env::set_var("DB_NAME", "test_db");
        env::set_var("DB_USERNAME", "test_user");
        env::set_var("DB_PASSWORD", "test_pass");
        env::set_var("DB_POOL_SIZE", "5");
        
        let config = DatabaseConfig::from_env().unwrap();
        
        assert_eq!(config.host, "test-host");
        assert_eq!(config.port, 3306);
        assert_eq!(config.name, "test_db");
        assert_eq!(config.username, "test_user");
        assert_eq!(config.password, Some("test_pass".to_string()));
        assert_eq!(config.pool_size, 5);
        
        // Cleanup
        env::remove_var("DB_HOST");
        env::remove_var("DB_PORT");
        env::remove_var("DB_NAME");
        env::remove_var("DB_USERNAME");
        env::remove_var("DB_PASSWORD");
        env::remove_var("DB_POOL_SIZE");
    }
    
    #[test]
    fn test_database_config_defaults() {
        let _guard = TEST_MUTEX.lock().unwrap();
        // Clean environment
        env::remove_var("DB_HOST");
        env::remove_var("DB_PORT");
        env::remove_var("DB_POOL_SIZE");
        
        // Set required fields
        env::set_var("DB_NAME", "test_db");
        env::set_var("DB_USERNAME", "test_user");
        
        let config = DatabaseConfig::from_env().unwrap();
        
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.name, "test_db");
        assert_eq!(config.username, "test_user");
        assert_eq!(config.password, None);
        assert_eq!(config.pool_size, 10);
        
        // Cleanup
        env::remove_var("DB_NAME");
        env::remove_var("DB_USERNAME");
    }
    
    #[test]
    fn test_database_config_validation() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("DB_HOST", "valid-host");
        env::set_var("DB_NAME", "valid_db");
        env::set_var("DB_USERNAME", "valid_user");
        env::set_var("DB_POOL_SIZE", "5");
        
        let config = DatabaseConfig::from_env().unwrap();
        assert!(config.validate().is_ok());
        
        // Cleanup
        env::remove_var("DB_HOST");
        env::remove_var("DB_NAME");
        env::remove_var("DB_USERNAME");
        env::remove_var("DB_POOL_SIZE");
    }
    
    #[test]
    fn test_invalid_pool_size() {
        let _guard = TEST_MUTEX.lock().unwrap();
        env::set_var("DB_NAME", "test_db");
        env::set_var("DB_USERNAME", "test_user");
        env::set_var("DB_POOL_SIZE", "invalid");
        
        let result = DatabaseConfig::from_env();
        assert!(result.is_err());
        
        // Cleanup
        env::remove_var("DB_NAME");
        env::remove_var("DB_USERNAME");
        env::remove_var("DB_POOL_SIZE");
    }
}