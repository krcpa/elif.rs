use crate::container::{Container, ContainerBuilder};
use crate::provider::{ProviderRegistry, ServiceProvider};
use thiserror::Error;

/// Application module trait that integrates with service-builder
pub trait Module: Send + Sync {
    /// Module name for identification
    fn name(&self) -> &'static str;
    
    /// Configure services in the container builder
    fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError>;
    
    /// Boot the module after container is built
    fn boot(&self, _container: &Container) -> Result<(), ModuleError> {
        // Default implementation does nothing
        Ok(())
    }
    
    /// Module dependencies (other modules that must be loaded first)
    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }
}

/// Application that manages modules, providers, and container
pub struct Application {
    container: Container,
    modules: Vec<Box<dyn Module>>,
    providers: ProviderRegistry,
}

impl Application {
    /// Create a new application builder
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::new()
    }
    
    /// Get the service container
    pub fn container(&self) -> &Container {
        &self.container
    }
    
    /// Start the application by booting providers and modules
    pub async fn start(&mut self) -> Result<(), ApplicationError> {
        // Boot all providers first
        self.providers.boot_all(&self.container)
            .map_err(ApplicationError::ProviderBoot)?;
        
        // Then boot all modules
        for module in &self.modules {
            module.boot(&self.container)
                .map_err(|e| ApplicationError::ModuleBoot {
                    module: module.name().to_string(),
                    error: e,
                })?;
        }
        Ok(())
    }
}

/// Builder for constructing applications
pub struct ApplicationBuilder {
    modules: Vec<Box<dyn Module>>,
    providers: ProviderRegistry,
}

impl ApplicationBuilder {
    pub fn new() -> Self {
        Self {
            modules: vec![],
            providers: ProviderRegistry::new(),
        }
    }
    
    /// Add a module to the application
    pub fn module<M: Module + 'static>(mut self, module: M) -> Self {
        self.modules.push(Box::new(module));
        self
    }
    
    /// Add a service provider to the application
    pub fn provider<P: ServiceProvider + 'static>(mut self, provider: P) -> Self {
        self.providers.register(provider);
        self
    }
    
    /// Build the application by resolving dependencies and creating container
    pub fn build(mut self) -> Result<Application, ApplicationError> {
        // Resolve provider dependencies first
        self.providers.resolve_dependencies()
            .map_err(ApplicationError::ProviderDependency)?;
        
        // Create container with providers first
        let mut builder = Container::builder();
        
        // Register all providers
        builder = self.providers.register_all(builder)
            .map_err(ApplicationError::ProviderRegistration)?;
        
        // Sort modules by dependencies and configure them
        let sorted_modules = self.resolve_module_dependencies(builder)?;
        
        Ok(sorted_modules)
    }
    
    /// Resolve module dependencies and build container
    fn resolve_module_dependencies(self, mut builder: ContainerBuilder) -> Result<Application, ApplicationError> {
        let modules = self.modules;
        
        // For now, just configure modules in order (full dependency resolution can be added later)
        for module in &modules {
            builder = module.configure(builder)
                .map_err(|e| ApplicationError::ModuleConfiguration {
                    module: module.name().to_string(),
                    error: e,
                })?;
        }
        
        let container = builder.build()
            .map_err(|e| ApplicationError::ContainerBuild {
                error: e.to_string(),
            })?;
        
        Ok(Application {
            container,
            modules,
            providers: self.providers,
        })
    }
    
}

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Module configuration failed: {message}")]
    ConfigurationFailed { message: String },
    
    #[error("Module boot failed: {message}")]
    BootFailed { message: String },
    
    #[error("Service registration failed: {service}")]
    ServiceRegistrationFailed { service: String },
}

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Module configuration error in '{module}': {error}")]
    ModuleConfiguration { module: String, error: ModuleError },
    
    #[error("Module boot error in '{module}': {error}")]
    ModuleBoot { module: String, error: ModuleError },
    
    #[error("Container build failed: {error}")]
    ContainerBuild { error: String },
    
    #[error("Circular dependency detected for module '{module}'")]
    CircularDependency { module: String },
    
    #[error("Missing dependency '{dependency}' for module '{module}'")]
    MissingDependency { module: String, dependency: String },
    
    #[error("Provider dependency error: {0}")]
    ProviderDependency(#[from] crate::provider::ProviderError),
    
    #[error("Provider registration error: {0}")]
    ProviderRegistration(crate::provider::ProviderError),
    
    #[error("Provider boot error: {0}")]
    ProviderBoot(crate::provider::ProviderError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::{AppConfig, DatabaseConnection, Environment};
    use std::sync::Arc;
    
    // Test implementations
    struct TestConfig;
    impl AppConfig for TestConfig {
        fn get(&self, _key: &str) -> Option<String> { None }
        fn environment(&self) -> Environment { Environment::Testing }
    }
    
    struct TestDatabase;
    impl DatabaseConnection for TestDatabase {
        fn is_connected(&self) -> bool { true }
        fn execute(&self, _query: &str) -> Result<(), crate::container::DatabaseError> { Ok(()) }
    }
    
    // Test modules
    struct CoreModule;
    impl Module for CoreModule {
        fn name(&self) -> &'static str { "core" }
        
        fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> {
            let config = Arc::new(TestConfig) as Arc<dyn AppConfig>;
            let database = Arc::new(TestDatabase) as Arc<dyn DatabaseConnection>;
            
            Ok(builder.config(config).database(database))
        }
    }
    
    struct AuthModule;
    impl Module for AuthModule {
        fn name(&self) -> &'static str { "auth" }
        
        fn dependencies(&self) -> Vec<&'static str> {
            vec!["core"]
        }
        
        fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> {
            // Auth module doesn't add new services, just uses existing ones
            Ok(builder)
        }
        
        fn boot(&self, container: &Container) -> Result<(), ModuleError> {
            let _config = container.config();
            // Perform auth module initialization
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_application_with_modules_and_providers() {
        use crate::provider::ServiceProvider;
        
        // Create a provider that handles service registration
        struct TestProvider;
        impl ServiceProvider for TestProvider {
            fn name(&self) -> &'static str { "test" }
            
            fn register(&self, builder: crate::container::ContainerBuilder) -> Result<crate::container::ContainerBuilder, crate::provider::ProviderError> {
                let config = Arc::new(TestConfig) as Arc<dyn AppConfig>;
                let database = Arc::new(TestDatabase) as Arc<dyn DatabaseConnection>;
                Ok(builder.config(config).database(database))
            }
        }
        
        let mut app = Application::builder()
            .provider(TestProvider)
            .module(AuthModule)
            .build()
            .unwrap();
        
        // Verify container is properly configured
        let config = app.container().config();
        assert_eq!(config.environment() as u8, Environment::Testing as u8);
        
        // Start the application
        app.start().await.unwrap();
    }
    
    #[test]
    fn test_provider_integration() {
        use crate::provider::ServiceProvider;
        
        struct TestProvider;
        impl ServiceProvider for TestProvider {
            fn name(&self) -> &'static str { "test" }
            
            fn register(&self, builder: crate::container::ContainerBuilder) -> Result<crate::container::ContainerBuilder, crate::provider::ProviderError> {
                let config = Arc::new(TestConfig) as Arc<dyn AppConfig>;
                let database = Arc::new(TestDatabase) as Arc<dyn DatabaseConnection>;
                Ok(builder.config(config).database(database))
            }
        }
        
        let result = Application::builder()
            .provider(TestProvider)
            .build();
        
        assert!(result.is_ok());
    }
}