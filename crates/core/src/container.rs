use service_builder::builder;
use std::sync::Arc;
use thiserror::Error;

/// Container for managing application services using service-builder
#[builder]
pub struct Container {
    #[builder(getter, setter)]
    pub config: Arc<crate::app_config::AppConfig>,
    
    #[builder(getter, setter)]
    pub database: Arc<dyn DatabaseConnection>,
}

/// Database connection trait  
pub trait DatabaseConnection: Send + Sync {
    fn is_connected(&self) -> bool;
    fn execute(&self, query: &str) -> Result<(), DatabaseError>;
}

/// Cache service trait
pub trait Cache: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: String) -> Result<(), CacheError>;
}

/// Logger service trait
pub trait Logger: Send + Sync {
    fn info(&self, message: &str);
    fn error(&self, message: &str);
    fn debug(&self, message: &str);
}

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Service not found: {service}")]
    ServiceNotFound { service: String },
    
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },
    
    #[error("Query failed: {reason}")]
    QueryFailed { reason: String },
}

#[derive(Error, Debug)]  
pub enum CacheError {
    #[error("Cache operation failed: {reason}")]
    OperationFailed { reason: String },
}

/// Optional services container for services not required at startup
pub struct OptionalServices {
    pub cache: Option<Arc<dyn Cache>>,
    pub logger: Option<Arc<dyn Logger>>,
}

impl OptionalServices {
    pub fn new() -> Self {
        Self {
            cache: None,
            logger: None,
        }
    }
    
    pub fn with_cache(mut self, cache: Arc<dyn Cache>) -> Self {
        self.cache = Some(cache);
        self
    }
    
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }
}

impl Container {
    /// Check if all required services are available (always true with required fields)
    pub fn validate(&self) -> Result<(), ContainerError> {
        // All required services are guaranteed by the builder pattern
        Ok(())
    }
    
    /// Get configuration service
    pub fn config(&self) -> Arc<crate::app_config::AppConfig> {
        self.config.clone()
    }
    
    /// Get database connection
    pub fn database(&self) -> Arc<dyn DatabaseConnection> {
        self.database.clone()
    }
}

// Default implementations for testing
pub mod test_implementations {
    use super::*;
    
    pub fn create_test_config() -> crate::app_config::AppConfig {
        crate::app_config::AppConfig {
            name: "test-app".to_string(),
            environment: crate::app_config::Environment::Testing,
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: Some("test-secret".to_string()),
            server: crate::app_config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 4,
            },
            logging: crate::app_config::LoggingConfig {
                level: "info".to_string(),
                format: "compact".to_string(),
            },
        }
    }
    
    pub struct TestDatabase {
        connected: bool,
    }
    
    impl TestDatabase {
        pub fn new() -> Self {
            Self { connected: true }
        }
    }
    
    impl DatabaseConnection for TestDatabase {
        fn is_connected(&self) -> bool {
            self.connected
        }
        
        fn execute(&self, _query: &str) -> Result<(), DatabaseError> {
            if self.connected {
                Ok(())
            } else {
                Err(DatabaseError::ConnectionFailed {
                    reason: "Database not connected".to_string(),
                })
            }
        }
    }
    
    pub struct TestLogger;
    
    impl Logger for TestLogger {
        fn info(&self, message: &str) {
            println!("[INFO] {}", message);
        }
        
        fn error(&self, message: &str) {
            eprintln!("[ERROR] {}", message);
        }
        
        fn debug(&self, message: &str) {
            println!("[DEBUG] {}", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_implementations::*;
    use std::sync::Arc;
    
    #[test]
    fn test_container_builder() {
        let config = Arc::new(test_implementations::create_test_config());
        let database = Arc::new(TestDatabase::new()) as Arc<dyn DatabaseConnection>;
        let logger = Arc::new(TestLogger) as Arc<dyn Logger>;
        
        let container = Container::builder()
            .config(config)
            .database(database)
            .build()
            .unwrap();
            
        assert!(container.validate().is_ok());
        
        let config = container.config();
        assert_eq!(config.name, "test-app");
        assert_eq!(config.environment, crate::app_config::Environment::Testing);
        
        let database = container.database();
        assert!(database.is_connected());
        
        // Test optional services
        let optional = OptionalServices::new().with_logger(logger);
        if let Some(logger) = optional.logger {
            logger.info("Container initialized successfully");
        }
    }
    
    #[test]
    fn test_container_validation_missing_services() {
        // With service-builder, missing required dependencies cause build() to fail
        // This test verifies that we can't create invalid containers
        let result = Container::builder().build();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_service_resolution() {
        let config = Arc::new(test_implementations::create_test_config());
        let database = Arc::new(TestDatabase::new()) as Arc<dyn DatabaseConnection>;
        
        let container = Container::builder()
            .config(config)
            .database(database)
            .build()
            .unwrap();
            
        // Test successful resolution
        let resolved_config = container.config();
        assert_eq!(resolved_config.name, "test-app");
        assert_eq!(resolved_config.environment, crate::app_config::Environment::Testing);
        
        let resolved_database = container.database();
        assert!(resolved_database.is_connected());
        
        // Test optional services
        let optional = OptionalServices::new();
        assert!(optional.cache.is_none());
        assert!(optional.logger.is_none());
    }
}