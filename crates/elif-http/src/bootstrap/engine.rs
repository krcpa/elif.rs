//! Bootstrap engine that orchestrates module discovery, dependency resolution,
//! container configuration, and server startup.

use crate::{
    bootstrap::{BootstrapError, BootstrapResult},
    config::HttpConfig,
    routing::ElifRouter,
    server::Server,
    Middleware,
};
use elif_core::{
    container::IocContainer,
    modules::{get_global_module_registry, CompileTimeModuleMetadata},
};
use std::{future::Future, net::SocketAddr, pin::Pin};

/// The main bootstrap orchestrator that handles the complete app startup process
///
/// This struct provides a fluent API for configuring and starting an elif.rs application:
/// - Discovers all modules automatically via the compile-time registry
/// - Resolves module dependencies 
/// - Configures the DI container with all providers
/// - Registers all controllers and their routes
/// - Sets up middleware pipeline
/// - Starts the HTTP server
#[derive(Debug)]
pub struct AppBootstrapper {
    /// Discovered module metadata from compile-time registry
    modules: Vec<CompileTimeModuleMetadata>,
    /// Resolved dependency order for module loading
    load_order: Vec<String>,
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
    /// 2. Resolves their dependency order
    /// 3. Sets up default configuration
    pub fn new() -> BootstrapResult<Self> {
        let registry = get_global_module_registry();
        
        // Get all modules and resolve dependency order
        let modules: Vec<CompileTimeModuleMetadata> = registry.all_modules()
            .into_iter()
            .cloned()
            .collect();
        
        if modules.is_empty() {
            return Err(BootstrapError::ModuleDiscoveryFailed {
                message: "No modules found. Make sure you have modules decorated with #[module]".to_string(),
            });
        }
        
        // Resolve dependency order
        let ordered_modules = registry.resolve_dependency_order()
            .map_err(|e| BootstrapError::CircularDependency { cycle: e })?;
        
        let load_order: Vec<String> = ordered_modules
            .into_iter()
            .map(|m| m.name.clone())
            .collect();
        
        tracing::info!("Bootstrap: Discovered {} modules", modules.len());
        tracing::info!("Bootstrap: Load order: {:?}", load_order);
        
        Ok(AppBootstrapper {
            modules,
            load_order,
            config: HttpConfig::default(),
            middleware: Vec::new(),
            container: None,
        })
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
    /// 1. Configures the DI container with all module providers
    /// 2. Creates and configures the router with all controller routes
    /// 3. Sets up the middleware pipeline
    /// 4. Starts the HTTP server
    pub fn listen(
        self,
        addr: impl Into<SocketAddr> + Send + 'static,
    ) -> Pin<Box<dyn Future<Output = BootstrapResult<()>> + Send>> {
        Box::pin(async move {
            let addr = addr.into();
            
            tracing::info!("Bootstrap: Starting server on {}", addr);
            
            // Step 1: Configure DI container
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
    
    /// Configure the DI container with all module providers
    async fn configure_container(&self) -> BootstrapResult<IocContainer> {
        if let Some(_container) = &self.container {
            // TODO: Clone IocContainer when it supports Clone
            // For now, return a new container
        }
        
        let mut container = IocContainer::new();
        
        // Configure container with providers from each module in dependency order
        for module_name in &self.load_order {
            let module = self.modules
                .iter()
                .find(|m| m.name == *module_name)
                .ok_or_else(|| BootstrapError::ContainerConfigurationFailed {
                    message: format!("Module '{}' not found in discovery results", module_name),
                })?;
            
            tracing::info!("Bootstrap: Configuring providers for module '{}'", module.name);
            
            // TODO: Register providers with container
            // For now, we'll register the module providers by name
            // This will need to be enhanced when we have actual provider instances
            for provider in &module.providers {
                tracing::debug!("Bootstrap: Registering provider '{}'", provider);
                // container.register_provider(provider)?;
            }
        }
        
        // Build the container
        container.build()
            .map_err(|e| BootstrapError::ContainerConfigurationFailed {
                message: format!("Failed to build container: {}", e),
            })?;
        
        tracing::info!("Bootstrap: Container configured with {} modules", self.modules.len());
        Ok(container)
    }
    
    /// Configure the router with all controller routes
    async fn configure_router(&self) -> BootstrapResult<ElifRouter> {
        let router = ElifRouter::new();
        
        // Register controllers from each module in dependency order
        for module_name in &self.load_order {
            let module = self.modules
                .iter()
                .find(|m| m.name == *module_name)
                .ok_or_else(|| BootstrapError::RouteRegistrationFailed {
                    message: format!("Module '{}' not found in discovery results", module_name),
                })?;
            
            tracing::info!("Bootstrap: Registering controllers for module '{}'", module.name);
            
            // TODO: Register controllers with router
            // For now, we'll log the controller names
            // This will need to be enhanced when we have actual controller instances
            for controller in &module.controllers {
                tracing::debug!("Bootstrap: Registering controller '{}'", controller);
                // router = router.controller(controller_instance)?;
            }
        }
        
        tracing::info!("Bootstrap: Router configured with controllers from {} modules", self.modules.len());
        Ok(router)
    }
    
    /// Get discovered modules (for debugging/introspection)
    pub fn modules(&self) -> &[CompileTimeModuleMetadata] {
        &self.modules
    }
    
    /// Get module load order (for debugging/introspection)
    pub fn load_order(&self) -> &[String] {
        &self.load_order
    }
}

impl Default for AppBootstrapper {
    fn default() -> Self {
        Self::new().expect("Failed to create default AppBootstrapper")
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
}