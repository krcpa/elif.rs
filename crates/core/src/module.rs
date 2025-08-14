use crate::container::{Container, ContainerBuilder};
use crate::provider::{ProviderRegistry, ServiceProvider};
use std::collections::HashMap;
use thiserror::Error;

/// HTTP method enumeration for route definitions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    OPTIONS,
    HEAD,
}

/// Route definition for module routing
#[derive(Debug, Clone)]
pub struct RouteDefinition {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String, // Handler function name or identifier
    pub middleware: Vec<String>, // Middleware names to apply
    pub description: Option<String>,
}

impl RouteDefinition {
    pub fn new(method: HttpMethod, path: impl Into<String>, handler: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            handler: handler.into(),
            middleware: Vec::new(),
            description: None,
        }
    }
    
    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
    
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Middleware definition for module middleware
#[derive(Debug, Clone)]
pub struct MiddlewareDefinition {
    pub name: String,
    pub priority: i32, // Lower numbers = higher priority (executed first)
    pub description: Option<String>,
}

impl MiddlewareDefinition {
    pub fn new(name: impl Into<String>, priority: i32) -> Self {
        Self {
            name: name.into(),
            priority,
            description: None,
        }
    }
    
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Application module trait that integrates with service-builder
pub trait Module: Send + Sync {
    /// Module name for identification
    fn name(&self) -> &'static str;
    
    /// Configure services in the container builder
    fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError>;
    
    /// Define routes for this module
    fn routes(&self) -> Vec<RouteDefinition> {
        vec![]
    }
    
    /// Define middleware for this module
    fn middleware(&self) -> Vec<MiddlewareDefinition> {
        vec![]
    }
    
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

/// Module registry for managing module lifecycle and dependencies
pub struct ModuleRegistry {
    modules: Vec<Box<dyn Module>>,
    loading_order: Vec<usize>,
    routes: Vec<RouteDefinition>,
    middleware: Vec<MiddlewareDefinition>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            loading_order: Vec::new(),
            routes: Vec::new(),
            middleware: Vec::new(),
        }
    }
    
    /// Register a module
    pub fn register<M: Module + 'static>(&mut self, module: M) {
        self.modules.push(Box::new(module));
    }
    
    /// Resolve module dependencies and determine loading order
    pub fn resolve_dependencies(&mut self) -> Result<(), ModuleError> {
        let module_count = self.modules.len();
        
        // Create name to index mapping
        let name_to_index: HashMap<String, usize> = self.modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name().to_string(), i))
            .collect();
        
        // Perform topological sort
        let mut visited = vec![false; module_count];
        let mut temp_mark = vec![false; module_count];
        let mut result = Vec::new();
        
        for i in 0..module_count {
            if !visited[i] {
                self.visit_module(i, &name_to_index, &mut visited, &mut temp_mark, &mut result)?;
            }
        }
        
        self.loading_order = result;
        Ok(())
    }
    
    /// Visit module for dependency resolution (topological sort)
    fn visit_module(
        &self,
        index: usize,
        name_to_index: &HashMap<String, usize>,
        visited: &mut Vec<bool>,
        temp_mark: &mut Vec<bool>,
        result: &mut Vec<usize>,
    ) -> Result<(), ModuleError> {
        if temp_mark[index] {
            return Err(ModuleError::CircularDependency {
                module: self.modules[index].name().to_string(),
            });
        }
        
        if visited[index] {
            return Ok(());
        }
        
        temp_mark[index] = true;
        
        // Visit all dependencies first
        let dependencies = self.modules[index].dependencies();
        for dep_name in dependencies {
            if let Some(&dep_index) = name_to_index.get(dep_name) {
                self.visit_module(dep_index, name_to_index, visited, temp_mark, result)?;
            } else {
                return Err(ModuleError::MissingDependency {
                    module: self.modules[index].name().to_string(),
                    dependency: dep_name.to_string(),
                });
            }
        }
        
        temp_mark[index] = false;
        visited[index] = true;
        result.push(index);
        
        Ok(())
    }
    
    /// Configure all modules with the container builder
    pub fn configure_all(&self, mut builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> {
        for &index in &self.loading_order {
            let module = &self.modules[index];
            builder = module.configure(builder)
                .map_err(|e| ModuleError::ConfigurationFailed {
                    module: module.name().to_string(),
                    error: e.to_string(),
                })?;
        }
        Ok(builder)
    }
    
    /// Boot all modules
    pub fn boot_all(&self, container: &Container) -> Result<(), ModuleError> {
        for &index in &self.loading_order {
            let module = &self.modules[index];
            module.boot(container)
                .map_err(|e| ModuleError::BootFailed {
                    module: module.name().to_string(),
                    error: e.to_string(),
                })?;
        }
        Ok(())
    }
    
    /// Collect all routes from modules
    pub fn collect_routes(&mut self) -> &[RouteDefinition] {
        self.routes.clear();
        for module in &self.modules {
            self.routes.extend(module.routes());
        }
        &self.routes
    }
    
    /// Collect all middleware from modules (sorted by priority)
    pub fn collect_middleware(&mut self) -> &[MiddlewareDefinition] {
        self.middleware.clear();
        for module in &self.modules {
            self.middleware.extend(module.middleware());
        }
        // Sort by priority (lower numbers first)
        self.middleware.sort_by_key(|m| m.priority);
        &self.middleware
    }
    
    /// Get all registered module names
    pub fn module_names(&self) -> Vec<&str> {
        self.modules.iter().map(|m| m.name()).collect()
    }
    
    /// Get loading order
    pub fn loading_order(&self) -> &[usize] {
        &self.loading_order
    }
}

/// Application that manages modules, providers, and container
pub struct Application {
    container: Container,
    modules: ModuleRegistry,
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
    
    /// Get module registry
    pub fn modules(&mut self) -> &mut ModuleRegistry {
        &mut self.modules
    }
    
    /// Get all routes from modules
    pub fn routes(&mut self) -> &[RouteDefinition] {
        self.modules.collect_routes()
    }
    
    /// Get all middleware from modules (sorted by priority)
    pub fn middleware(&mut self) -> &[MiddlewareDefinition] {
        self.modules.collect_middleware()
    }
    
    /// Start the application by booting providers and modules
    pub async fn start(&mut self) -> Result<(), ApplicationError> {
        // Boot all providers first
        self.providers.boot_all(&self.container)
            .map_err(ApplicationError::ProviderBoot)?;
        
        // Then boot all modules
        self.modules.boot_all(&self.container)
            .map_err(ApplicationError::ModuleBoot)?;
        
        Ok(())
    }
}

/// Builder for constructing applications
pub struct ApplicationBuilder {
    modules: ModuleRegistry,
    providers: ProviderRegistry,
}

impl ApplicationBuilder {
    pub fn new() -> Self {
        Self {
            modules: ModuleRegistry::new(),
            providers: ProviderRegistry::new(),
        }
    }
    
    /// Add a module to the application
    pub fn module<M: Module + 'static>(mut self, module: M) -> Self {
        self.modules.register(module);
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
            
        // Resolve module dependencies
        self.modules.resolve_dependencies()
            .map_err(ApplicationError::ModuleDependency)?;
        
        // Create container with providers first
        let mut builder = Container::builder();
        
        // Register all providers
        builder = self.providers.register_all(builder)
            .map_err(ApplicationError::ProviderRegistration)?;
            
        // Configure all modules in dependency order
        builder = self.modules.configure_all(builder)
            .map_err(ApplicationError::ModuleConfiguration)?;
        
        let container = builder.build()
            .map_err(|e| ApplicationError::ContainerBuild {
                error: e.to_string(),
            })?;
        
        Ok(Application {
            container,
            modules: self.modules,
            providers: self.providers,
        })
    }
    
}

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Module configuration failed for '{module}': {error}")]
    ConfigurationFailed { module: String, error: String },
    
    #[error("Module boot failed for '{module}': {error}")]
    BootFailed { module: String, error: String },
    
    #[error("Circular dependency detected for module '{module}'")]
    CircularDependency { module: String },
    
    #[error("Missing dependency '{dependency}' for module '{module}'")]
    MissingDependency { module: String, dependency: String },
    
    #[error("Service registration failed: {service}")]
    ServiceRegistrationFailed { service: String },
}

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Module configuration error: {0}")]
    ModuleConfiguration(#[from] ModuleError),
    
    #[error("Module boot error: {0}")]
    ModuleBoot(ModuleError),
    
    #[error("Module dependency error: {0}")]
    ModuleDependency(ModuleError),
    
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
    use crate::app_config::{AppConfig, Environment};
    use crate::container::DatabaseConnection;
    use std::sync::Arc;
    
    // Test implementations - use the shared function from container tests
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
            // Core module doesn't configure services directly
            Ok(builder)
        }
        
        fn routes(&self) -> Vec<RouteDefinition> {
            vec![
                RouteDefinition::new(HttpMethod::GET, "/", "CoreController::index")
                    .with_description("Core module home route"),
                RouteDefinition::new(HttpMethod::GET, "/health", "CoreController::health")
                    .with_middleware(vec!["logging".to_string()])
                    .with_description("Health check endpoint"),
            ]
        }
        
        fn middleware(&self) -> Vec<MiddlewareDefinition> {
            vec![
                MiddlewareDefinition::new("logging", 100)
                    .with_description("Request logging middleware"),
                MiddlewareDefinition::new("cors", 200)
                    .with_description("CORS handling middleware"),
            ]
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
        
        fn routes(&self) -> Vec<RouteDefinition> {
            vec![
                RouteDefinition::new(HttpMethod::POST, "/auth/login", "AuthController::login")
                    .with_middleware(vec!["rate_limit".to_string()])
                    .with_description("User login endpoint"),
                RouteDefinition::new(HttpMethod::POST, "/auth/logout", "AuthController::logout")
                    .with_middleware(vec!["auth".to_string()])
                    .with_description("User logout endpoint"),
            ]
        }
        
        fn middleware(&self) -> Vec<MiddlewareDefinition> {
            vec![
                MiddlewareDefinition::new("auth", 50)
                    .with_description("Authentication middleware"),
                MiddlewareDefinition::new("rate_limit", 150)
                    .with_description("Rate limiting middleware"),
            ]
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
                let config = Arc::new(create_test_config());
                let database = Arc::new(TestDatabase) as Arc<dyn DatabaseConnection>;
                Ok(builder.config(config).database(database))
            }
        }
        
        let mut app = Application::builder()
            .provider(TestProvider)
            .module(CoreModule)
            .module(AuthModule)
            .build()
            .unwrap();
        
        // Verify container is properly configured
        let config = app.container().config();
        assert_eq!(config.environment, Environment::Testing);
        
        // Start the application
        app.start().await.unwrap();
    }
    
    #[test]
    fn test_module_registry_dependency_resolution() {
        let mut registry = ModuleRegistry::new();
        registry.register(AuthModule); // Depends on core
        registry.register(CoreModule); // No dependencies
        
        registry.resolve_dependencies().unwrap();
        
        let loading_order = registry.loading_order();
        
        // Core should be loaded before auth
        let core_pos = loading_order.iter().position(|&i| registry.modules[i].name() == "core").unwrap();
        let auth_pos = loading_order.iter().position(|&i| registry.modules[i].name() == "auth").unwrap();
        
        assert!(core_pos < auth_pos);
    }
    
    #[test]
    fn test_module_routes_collection() {
        let mut registry = ModuleRegistry::new();
        registry.register(CoreModule);
        registry.register(AuthModule);
        
        let routes = registry.collect_routes();
        
        // Should have routes from both modules
        assert_eq!(routes.len(), 4);
        
        // Check specific routes exist
        assert!(routes.iter().any(|r| r.path == "/" && r.method == HttpMethod::GET));
        assert!(routes.iter().any(|r| r.path == "/health" && r.method == HttpMethod::GET));
        assert!(routes.iter().any(|r| r.path == "/auth/login" && r.method == HttpMethod::POST));
        assert!(routes.iter().any(|r| r.path == "/auth/logout" && r.method == HttpMethod::POST));
    }
    
    #[test]
    fn test_module_middleware_collection() {
        let mut registry = ModuleRegistry::new();
        registry.register(CoreModule);
        registry.register(AuthModule);
        
        let middleware = registry.collect_middleware();
        
        // Should have middleware from both modules, sorted by priority
        assert_eq!(middleware.len(), 4);
        
        // Check priority ordering (lower numbers first)
        assert_eq!(middleware[0].name, "auth"); // priority 50
        assert_eq!(middleware[1].name, "logging"); // priority 100
        assert_eq!(middleware[2].name, "rate_limit"); // priority 150
        assert_eq!(middleware[3].name, "cors"); // priority 200
    }
    
    #[test]
    fn test_module_missing_dependency() {
        let mut registry = ModuleRegistry::new();
        registry.register(AuthModule); // Depends on core, but core is not added
        
        let result = registry.resolve_dependencies();
        assert!(result.is_err());
        
        if let Err(ModuleError::MissingDependency { module, dependency }) = result {
            assert_eq!(module, "auth");
            assert_eq!(dependency, "core");
        } else {
            panic!("Expected MissingDependency error");
        }
    }
    
    #[test]
    fn test_module_circular_dependency() {
        struct ModuleA;
        impl Module for ModuleA {
            fn name(&self) -> &'static str { "a" }
            fn dependencies(&self) -> Vec<&'static str> { vec!["b"] }
            fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> { Ok(builder) }
        }
        
        struct ModuleB;
        impl Module for ModuleB {
            fn name(&self) -> &'static str { "b" }
            fn dependencies(&self) -> Vec<&'static str> { vec!["a"] }
            fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> { Ok(builder) }
        }
        
        let mut registry = ModuleRegistry::new();
        registry.register(ModuleA);
        registry.register(ModuleB);
        
        let result = registry.resolve_dependencies();
        assert!(result.is_err());
        assert!(matches!(result, Err(ModuleError::CircularDependency { .. })));
    }
    
    #[tokio::test]
    async fn test_full_application_with_modules() {
        use crate::provider::ServiceProvider;
        
        // Create a provider that handles service registration
        struct TestProvider;
        impl ServiceProvider for TestProvider {
            fn name(&self) -> &'static str { "test" }
            
            fn register(&self, builder: crate::container::ContainerBuilder) -> Result<crate::container::ContainerBuilder, crate::provider::ProviderError> {
                let config = Arc::new(create_test_config());
                let database = Arc::new(TestDatabase) as Arc<dyn DatabaseConnection>;
                Ok(builder.config(config).database(database))
            }
        }
        
        let mut app = Application::builder()
            .provider(TestProvider)
            .module(CoreModule)
            .module(AuthModule)
            .build()
            .unwrap();
        
        // Verify container is properly configured
        let config = app.container().config();
        assert_eq!(config.environment, Environment::Testing);
        
        // Check routes are collected
        let routes = app.routes();
        assert_eq!(routes.len(), 4);
        
        // Check middleware is collected and sorted
        let middleware = app.middleware();
        assert_eq!(middleware.len(), 4);
        assert_eq!(middleware[0].name, "auth"); // Lowest priority first
        
        // Start the application
        app.start().await.unwrap();
    }
}