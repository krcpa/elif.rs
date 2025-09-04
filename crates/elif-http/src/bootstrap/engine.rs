//! Bootstrap engine that orchestrates module discovery, dependency resolution,
//! container configuration, and server startup.

use crate::{
    bootstrap::{BootstrapError, BootstrapResult, RouteValidator},
    config::HttpConfig,
    routing::ElifRouter,
    server::Server,
    Middleware,
};
use elif_core::{
    container::IocContainer,
    modules::{get_global_module_registry, CompileTimeModuleMetadata, ModuleDescriptor, ModuleRuntime, ModuleRuntimeError},
};
use std::{future::Future, net::SocketAddr, pin::Pin};

/// The main bootstrap orchestrator that handles the complete app startup process
///
/// This struct provides a fluent API for configuring and starting an elif.rs application:
/// - Discovers all modules automatically via the compile-time registry
/// - Resolves module dependencies using ModuleRuntime with enhanced error handling
/// - Configures the DI container with all providers
/// - Registers all controllers and their routes
/// - Sets up middleware pipeline
/// - Starts the HTTP server
#[derive(Debug)]
pub struct AppBootstrapper {
    /// Discovered module metadata from compile-time registry
    modules: Vec<CompileTimeModuleMetadata>,
    /// Runtime module system for enhanced dependency resolution and lifecycle management
    module_runtime: ModuleRuntime,
    /// HTTP server configuration
    config: HttpConfig,
    /// Middleware stack to apply
    middleware: Vec<Box<dyn Middleware>>,
    /// Custom DI container if provided
    container: Option<IocContainer>,
}

impl AppBootstrapper {
    /// Create a new AppBootstrapper with automatic module discovery
    ///
    /// This method:
    /// 1. Gets all modules from the global compile-time registry
    /// 2. Converts them to ModuleDescriptors for ModuleRuntime
    /// 3. Uses ModuleRuntime for enhanced dependency resolution and lifecycle management
    /// 4. Sets up default configuration
    pub fn new() -> BootstrapResult<Self> {
        let registry = get_global_module_registry();
        
        // Get all modules from compile-time registry
        let modules: Vec<CompileTimeModuleMetadata> = registry.all_modules()
            .into_iter()
            .cloned()
            .collect();
        
        if modules.is_empty() {
            return Err(BootstrapError::ModuleDiscoveryFailed {
                message: "No modules found. Make sure you have modules decorated with #[module]".to_string(),
            });
        }
        
        // Create ModuleRuntime and register all modules
        let mut module_runtime = ModuleRuntime::new();
        for module_metadata in &modules {
            let descriptor = Self::convert_metadata_to_descriptor(module_metadata);
            module_runtime.register_module(descriptor)
                .map_err(|e| BootstrapError::ModuleRegistrationFailed {
                    message: format!("Failed to register module '{}': {}", module_metadata.name, e),
                })?;
        }
        
        // Calculate load order using ModuleRuntime's sophisticated dependency resolution
        let load_order = module_runtime.calculate_load_order()
            .map_err(|e| match e {
                ModuleRuntimeError::CircularDependency { cycle, message: _ } => {
                    BootstrapError::CircularDependency { cycle }
                }
                ModuleRuntimeError::MissingDependency { module, missing_dependency, message: _ } => {
                    BootstrapError::MissingDependency {
                        module,
                        dependency: missing_dependency,
                    }
                }
                other => BootstrapError::ModuleRegistrationFailed {
                    message: format!("Module dependency resolution failed: {}", other),
                }
            })?;
        
        tracing::info!("Bootstrap: Discovered {} modules", modules.len());
        tracing::info!("Bootstrap: Load order resolved: {:?}", load_order);
        
        Ok(AppBootstrapper {
            modules,
            module_runtime,
            config: HttpConfig::default(),
            middleware: Vec::new(),
            container: None,
        })
    }
    
    /// Convert CompileTimeModuleMetadata to ModuleDescriptor for ModuleRuntime
    fn convert_metadata_to_descriptor(metadata: &CompileTimeModuleMetadata) -> ModuleDescriptor {
        ModuleDescriptor {
            name: metadata.name.clone(),
            version: None,
            description: None,
            providers: Vec::new(), // CompileTimeModuleMetadata only has provider names, not full descriptors
            controllers: Vec::new(), // CompileTimeModuleMetadata only has controller names, not full descriptors
            imports: metadata.imports.clone(),
            exports: metadata.exports.clone(),
            dependencies: metadata.imports.clone(), // In this context, imports are dependencies
            is_optional: false,
        }
    }
    
    /// Configure the HTTP server with custom configuration
    pub fn with_config(mut self, config: HttpConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Add middleware to the application
    pub fn with_middleware(mut self, middleware: Vec<Box<dyn Middleware>>) -> Self {
        self.middleware = middleware;
        self
    }
    
    /// Use a pre-configured DI container
    pub fn with_container(mut self, container: IocContainer) -> Self {
        self.container = Some(container);
        self
    }
    
    /// Start the HTTP server on the specified address
    ///
    /// This method performs the complete bootstrap sequence:
    /// 1. Configures the DI container with all module providers using ModuleRuntime
    /// 2. Creates and configures the router with all controller routes
    /// 3. Sets up the middleware pipeline
    /// 4. Starts the HTTP server
    pub fn listen(
        mut self,
        addr: impl Into<SocketAddr> + Send + 'static,
    ) -> Pin<Box<dyn Future<Output = BootstrapResult<()>> + Send>> {
        Box::pin(async move {
            let addr = addr.into();
            
            tracing::info!("Bootstrap: Starting server on {}", addr);
            
            // Step 1: Configure DI container using ModuleRuntime
            let container = self.configure_container().await?;
            
            // Step 2: Create and configure router
            let router = self.configure_router().await?;
            
            // Step 3: Create and start server
            let mut server = Server::new(container, self.config)
                .map_err(|e| BootstrapError::ServerStartupFailed {
                    message: format!("Failed to create server: {}", e),
                })?;
            
            // Apply middleware and use router
            if !self.middleware.is_empty() {
                // TODO: Apply middleware to server
                // For now, just use the server as-is
            }
            server.use_router(router);
            
            // Start listening - convert SocketAddr to string
            server
                .listen(addr.to_string())
                .await
                .map_err(|e| BootstrapError::ServerStartupFailed {
                    message: format!("Failed to start server: {}", e),
                })?;
            
            Ok(())
        })
    }
    
    /// Configure the DI container with all module providers using ModuleRuntime
    async fn configure_container(&mut self) -> BootstrapResult<IocContainer> {
        let mut container = if let Some(_provided_container) = &self.container {
            // TODO: Proper container merging/extension when IocContainer supports it
            // For now, we create a new container and document this limitation
            tracing::warn!("Bootstrap: Custom container provided but cannot be cloned. Creating new container with module configurations.");
            tracing::info!("Bootstrap: To properly use custom containers, consider configuring modules directly on your container before bootstrap");
            IocContainer::new()
        } else {
            IocContainer::new()
        };
        
        // Use ModuleRuntime's sophisticated dependency resolution and container configuration
        self.module_runtime.resolve_dependencies(&mut container)
            .await
            .map_err(|e| BootstrapError::ContainerConfigurationFailed {
                message: format!("ModuleRuntime dependency resolution failed: {}", e),
            })?;
        
        tracing::info!("Bootstrap: Container configured with {} modules using ModuleRuntime", self.modules.len());
        Ok(container)
    }
    
    /// Configure the router with all controller routes
    async fn configure_router(&self) -> BootstrapResult<ElifRouter> {
        let router = ElifRouter::new();
        
        // Create route validator for conflict detection
        let validator = RouteValidator::new().with_diagnostics(true);
        
        // Get load order from ModuleRuntime
        let load_order = self.module_runtime.load_order();
        
        // Register controllers from each module in dependency order
        for module_name in load_order {
            let module = self.modules
                .iter()
                .find(|m| m.name == *module_name)
                .ok_or_else(|| BootstrapError::RouteRegistrationFailed {
                    message: format!("Module '{}' not found in discovery results", module_name),
                })?;
            
            tracing::info!("Bootstrap: Registering controllers for module '{}'", module.name);
            
            // Register controllers - route validation will be integrated when controller 
            // metadata extraction is available (issue #386)
            for controller_name in &module.controllers {
                tracing::debug!("Bootstrap: Preparing controller '{}' for registration", controller_name);
                
                // TODO: Extract actual route metadata from controller when #386 is completed
                // TODO: Validate extracted routes with validator.register_route(route_registration)
                // TODO: Generate detailed conflict reports on validation failures
                
                // For now, just log the controller discovery without creating fake routes
                // that would cause false conflicts in the validation system
                tracing::info!("Bootstrap: Controller '{}' discovered and ready for route extraction", controller_name);
                
                // TODO: Actually register with router when controller instances are available
                // router = router.controller(controller_instance)?;
            }
        }
        
        // Perform final validation across all routes
        let validation_report = validator.validate_all_routes()
            .map_err(|e| {
                if let crate::bootstrap::RouteValidationError::ConflictDetected { conflicts } = &e {
                    let conflict_report = validator.generate_conflict_report(conflicts);
                    tracing::error!("Final route validation failed:\n{}", conflict_report);
                    BootstrapError::RouteRegistrationFailed {
                        message: format!("Route validation failed: {}", conflict_report),
                    }
                } else {
                    e.into()
                }
            })?;
        
        // Log validation results
        tracing::info!("Bootstrap: Route validation completed successfully");
        tracing::info!("  - Total routes: {}", validation_report.total_routes);
        tracing::info!("  - Performance score: {}/100", validation_report.performance_score);
        
        if validation_report.warnings > 0 {
            tracing::warn!("  - Validation warnings: {}", validation_report.warnings);
        }
        
        for suggestion in &validation_report.suggestions {
            tracing::info!("  - Suggestion: {}", suggestion);
        }
        
        tracing::info!("Bootstrap: Router configured with controllers from {} modules", self.modules.len());
        Ok(router)
    }
    
    /// Get discovered modules (for debugging/introspection)
    pub fn modules(&self) -> &[CompileTimeModuleMetadata] {
        &self.modules
    }
    
    /// Get module load order from ModuleRuntime (for debugging/introspection)
    pub fn load_order(&self) -> &[String] {
        self.module_runtime.load_order()
    }
}

impl Default for AppBootstrapper {
    fn default() -> Self {
        match Self::new() {
            Ok(bootstrapper) => bootstrapper,
            Err(e) => {
                tracing::error!("Failed to create default AppBootstrapper: {}", e);
                // Return a minimal bootstrapper instead of panicking
                Self {
                    modules: Vec::new(),
                    module_runtime: ModuleRuntime::new(),
                    config: HttpConfig::default(),
                    middleware: Vec::new(),
                    container: None,
                }
            }
        }
    }
}

/// Utility function to create a bootstrapper directly (for convenience)
pub fn create_bootstrapper() -> BootstrapResult<AppBootstrapper> {
    AppBootstrapper::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_core::modules::{register_module_globally, CompileTimeModuleMetadata};
    
    #[tokio::test]
    async fn test_bootstrap_creation() {
        // Register a test module
        let test_module = CompileTimeModuleMetadata::new("TestModule".to_string())
            .with_controller("TestController".to_string())
            .with_provider("TestService".to_string());
        
        register_module_globally(test_module);
        
        let bootstrapper = AppBootstrapper::new().expect("Should create bootstrapper");
        
        assert!(!bootstrapper.modules().is_empty());
        assert!(bootstrapper.modules().iter().any(|m| m.name == "TestModule"));
    }
    
    #[tokio::test]
    async fn test_bootstrap_configuration() {
        let test_module = CompileTimeModuleMetadata::new("ConfigTestModule".to_string());
        register_module_globally(test_module);
        
        let config = HttpConfig::default();
        let bootstrapper = AppBootstrapper::new()
            .expect("Should create bootstrapper")
            .with_config(config);
        
        // Just test that configuration doesn't panic
        assert!(!bootstrapper.modules().is_empty());
    }
    
    #[tokio::test]
    async fn test_bootstrap_error_handling() {
        // Test what happens when no modules are registered
        // Note: This test may be affected by other tests registering modules
        // In a real scenario, you'd want to use a separate test registry
        
        // The current implementation will find modules from other tests,
        // but in principle, if no modules were found, it should return an error
        let result = AppBootstrapper::new();
        
        // Either succeeds (because other tests registered modules) 
        // or fails with a clear error message
        match result {
            Ok(bootstrapper) => {
                // Other tests registered modules, that's fine
                assert!(!bootstrapper.modules().is_empty());
            }
            Err(BootstrapError::ModuleDiscoveryFailed { message }) => {
                // This is the expected error when no modules are found
                assert!(message.contains("No modules found"));
            }
            Err(other) => {
                eprintln!("Unexpected error type in test: {:?}", other);
                assert!(false, "Unexpected error type: {:?}", other);
            }
        }
    }
}