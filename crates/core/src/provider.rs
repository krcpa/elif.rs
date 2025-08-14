use crate::container::{Container, ContainerBuilder};
use std::collections::HashMap;
use thiserror::Error;

/// Service provider trait for registering services and managing lifecycle
pub trait ServiceProvider: Send + Sync {
    /// Provider name for identification and dependency resolution
    fn name(&self) -> &'static str;
    
    /// Register services in the container builder
    /// This is called during the registration phase
    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError>;
    
    /// Boot the provider after all services are registered
    /// This is called during the boot phase with access to the built container
    fn boot(&self, container: &Container) -> Result<(), ProviderError> {
        // Default implementation does nothing
        let _ = container; // Suppress unused parameter warning
        Ok(())
    }
    
    /// Provider dependencies (other providers that must be registered first)
    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }
    
    /// Defer boot phase (useful for providers that need other providers to be booted first)
    fn defer_boot(&self) -> bool {
        false
    }
}

/// Provider registry manages service providers and their lifecycle
pub struct ProviderRegistry {
    providers: Vec<Box<dyn ServiceProvider>>,
    registration_order: Vec<usize>,
    boot_order: Vec<usize>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            registration_order: Vec::new(),
            boot_order: Vec::new(),
        }
    }
    
    /// Register a service provider
    pub fn register<P: ServiceProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }
    
    /// Resolve provider dependencies and determine execution order
    pub fn resolve_dependencies(&mut self) -> Result<(), ProviderError> {
        
        // Create name to index mapping
        let name_to_index: HashMap<String, usize> = self.providers
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name().to_string(), i))
            .collect();
        
        // Resolve registration order (topological sort)
        self.registration_order = self.topological_sort(&name_to_index, false)?;
        
        // Resolve boot order (separate sort considering defer_boot)
        self.boot_order = self.topological_sort(&name_to_index, true)?;
        
        Ok(())
    }
    
    /// Perform topological sort considering dependencies
    fn topological_sort(&self, name_to_index: &HashMap<String, usize>, consider_defer: bool) -> Result<Vec<usize>, ProviderError> {
        let provider_count = self.providers.len();
        let mut visited = vec![false; provider_count];
        let mut temp_mark = vec![false; provider_count];
        let mut result = Vec::new();
        
        // Visit all providers
        for i in 0..provider_count {
            if !visited[i] {
                self.visit_provider(i, name_to_index, &mut visited, &mut temp_mark, &mut result, consider_defer)?;
            }
        }
        
        Ok(result)
    }
    
    /// Visit provider for dependency resolution
    fn visit_provider(
        &self,
        index: usize,
        name_to_index: &HashMap<String, usize>,
        visited: &mut Vec<bool>,
        temp_mark: &mut Vec<bool>,
        result: &mut Vec<usize>,
        consider_defer: bool,
    ) -> Result<(), ProviderError> {
        if temp_mark[index] {
            return Err(ProviderError::CircularDependency {
                provider: self.providers[index].name().to_string(),
            });
        }
        
        if visited[index] {
            return Ok(());
        }
        
        temp_mark[index] = true;
        
        // Visit dependencies first
        let dependencies = self.providers[index].dependencies();
        for dep_name in dependencies {
            if let Some(&dep_index) = name_to_index.get(dep_name) {
                self.visit_provider(dep_index, name_to_index, visited, temp_mark, result, consider_defer)?;
            } else {
                return Err(ProviderError::MissingDependency {
                    provider: self.providers[index].name().to_string(),
                    dependency: dep_name.to_string(),
                });
            }
        }
        
        // If considering defer_boot, non-deferred providers should come first
        if consider_defer && self.providers[index].defer_boot() {
            // Add deferred providers at the end
            // This is handled by processing all non-deferred first
        }
        
        temp_mark[index] = false;
        visited[index] = true;
        result.push(index);
        
        Ok(())
    }
    
    /// Register all providers with the container builder
    pub fn register_all(&self, mut builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError> {
        for &index in &self.registration_order {
            let provider = &self.providers[index];
            builder = provider.register(builder)
                .map_err(|e| ProviderError::RegistrationFailed {
                    provider: provider.name().to_string(),
                    error: Box::new(e),
                })?;
        }
        Ok(builder)
    }
    
    /// Boot all providers
    pub fn boot_all(&self, container: &Container) -> Result<(), ProviderError> {
        // Separate deferred and non-deferred providers
        let mut non_deferred = Vec::new();
        let mut deferred = Vec::new();
        
        for &index in &self.boot_order {
            if self.providers[index].defer_boot() {
                deferred.push(index);
            } else {
                non_deferred.push(index);
            }
        }
        
        // Boot non-deferred providers first
        for index in non_deferred {
            self.providers[index].boot(container)
                .map_err(|e| ProviderError::BootFailed {
                    provider: self.providers[index].name().to_string(),
                    error: Box::new(e),
                })?;
        }
        
        // Boot deferred providers
        for index in deferred {
            self.providers[index].boot(container)
                .map_err(|e| ProviderError::BootFailed {
                    provider: self.providers[index].name().to_string(),
                    error: Box::new(e),
                })?;
        }
        
        Ok(())
    }
    
    /// Get all registered provider names
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.name()).collect()
    }
    
    /// Get registration order
    pub fn registration_order(&self) -> &[usize] {
        &self.registration_order
    }
    
    /// Get boot order
    pub fn boot_order(&self) -> &[usize] {
        &self.boot_order
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Provider registration failed for '{provider}': {error}")]
    RegistrationFailed { 
        provider: String, 
        error: Box<dyn std::error::Error + Send + Sync> 
    },
    
    #[error("Provider boot failed for '{provider}': {error}")]
    BootFailed { 
        provider: String, 
        error: Box<dyn std::error::Error + Send + Sync> 
    },
    
    #[error("Circular dependency detected for provider '{provider}'")]
    CircularDependency { provider: String },
    
    #[error("Missing dependency '{dependency}' for provider '{provider}'")]
    MissingDependency { provider: String, dependency: String },
    
    #[error("Provider '{provider}' is already registered")]
    AlreadyRegistered { provider: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::{AppConfig, Environment};
    use crate::container::DatabaseConnection;
    use std::sync::{Arc, Mutex};
    
    // Test implementations
    fn create_test_config() -> AppConfig {
        AppConfig {
            name: "test-app".to_string(),
            environment: Environment::Testing,
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
    
    struct TestDatabase {
        connected: bool,
    }
    
    impl DatabaseConnection for TestDatabase {
        fn is_connected(&self) -> bool {
            self.connected
        }
        
        fn execute(&self, _query: &str) -> Result<(), crate::container::DatabaseError> {
            Ok(())
        }
    }
    
    // Test providers
    struct ConfigProvider;
    
    impl ServiceProvider for ConfigProvider {
        fn name(&self) -> &'static str {
            "config"
        }
        
        fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError> {
            let config = Arc::new(create_test_config());
            
            Ok(builder.config(config))
        }
    }
    
    struct DatabaseProvider;
    
    impl ServiceProvider for DatabaseProvider {
        fn name(&self) -> &'static str {
            "database"
        }
        
        fn dependencies(&self) -> Vec<&'static str> {
            vec!["config"]
        }
        
        fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError> {
            let database = Arc::new(TestDatabase { connected: true }) as Arc<dyn DatabaseConnection>;
            Ok(builder.database(database))
        }
        
        fn boot(&self, container: &Container) -> Result<(), ProviderError> {
            let database = container.database();
            if !database.is_connected() {
                return Err(ProviderError::BootFailed {
                    provider: "database".to_string(),
                    error: Box::new(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        "Database connection failed",
                    )),
                });
            }
            Ok(())
        }
    }
    
    // Boot tracking for testing
    lazy_static::lazy_static! {
        static ref BOOT_ORDER: Mutex<Vec<String>> = Mutex::new(Vec::new());
    }
    
    struct BootTrackingProvider {
        name: &'static str,
        defer: bool,
        provide_services: bool,
    }
    
    impl ServiceProvider for BootTrackingProvider {
        fn name(&self) -> &'static str {
            self.name
        }
        
        fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError> {
            if self.provide_services {
                let config = Arc::new(create_test_config());
                let database = Arc::new(TestDatabase { connected: true }) as Arc<dyn DatabaseConnection>;
                Ok(builder.config(config).database(database))
            } else {
                Ok(builder)
            }
        }
        
        fn boot(&self, _container: &Container) -> Result<(), ProviderError> {
            BOOT_ORDER.lock().unwrap().push(self.name.to_string());
            Ok(())
        }
        
        fn defer_boot(&self) -> bool {
            self.defer
        }
    }
    
    #[test]
    fn test_provider_registration_and_boot() {
        let mut registry = ProviderRegistry::new();
        registry.register(ConfigProvider);
        registry.register(DatabaseProvider);
        
        registry.resolve_dependencies().unwrap();
        
        let builder = Container::builder();
        let builder = registry.register_all(builder).unwrap();
        let container = builder.build().unwrap();
        
        registry.boot_all(&container).unwrap();
        
        // Verify services are available
        let config = container.config();
        assert_eq!(config.name, "test-app");
        
        let database = container.database();
        assert!(database.is_connected());
    }
    
    #[test]
    fn test_dependency_resolution() {
        let mut registry = ProviderRegistry::new();
        registry.register(DatabaseProvider); // Depends on config
        registry.register(ConfigProvider);   // No dependencies
        
        registry.resolve_dependencies().unwrap();
        
        let order = registry.registration_order();
        
        // Config should be registered before database
        let config_pos = order.iter().position(|&i| registry.providers[i].name() == "config").unwrap();
        let db_pos = order.iter().position(|&i| registry.providers[i].name() == "database").unwrap();
        
        assert!(config_pos < db_pos);
    }
    
    #[test]
    fn test_missing_dependency_error() {
        let mut registry = ProviderRegistry::new();
        registry.register(DatabaseProvider); // Depends on config, but config is not registered
        
        let result = registry.resolve_dependencies();
        assert!(matches!(result, Err(ProviderError::MissingDependency { .. })));
    }
    
    #[test]
    fn test_defer_boot_ordering() {
        BOOT_ORDER.lock().unwrap().clear();
        
        let mut registry = ProviderRegistry::new();
        registry.register(BootTrackingProvider { name: "normal", defer: false, provide_services: true });
        registry.register(BootTrackingProvider { name: "deferred", defer: true, provide_services: false });
        
        registry.resolve_dependencies().unwrap();
        
        let builder = Container::builder();
        let builder = registry.register_all(builder).unwrap();
        let container = builder.build().unwrap();
        
        registry.boot_all(&container).unwrap();
        
        let boot_order = BOOT_ORDER.lock().unwrap();
        assert_eq!(boot_order[0], "normal");
        assert_eq!(boot_order[1], "deferred");
    }
}