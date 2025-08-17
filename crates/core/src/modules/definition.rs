use crate::container::{Container, ContainerBuilder};
use crate::errors::CoreError;
use crate::modules::routing::{RouteDefinition, MiddlewareDefinition};

/// Core module error type
#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("Circular dependency detected in module: {module}")]
    CircularDependency { module: String },
    
    #[error("Missing dependency '{dependency}' for module '{module}'")]
    MissingDependency { module: String, dependency: String },
    
    #[error("Module configuration failed: {message}")]
    ConfigurationFailed { message: String },
    
    #[error("Module boot failed: {message}")]
    BootFailed { message: String },
    
    #[error("Container error: {0}")]
    Container(#[from] CoreError),
}

/// Application module trait that integrates with the framework
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
    
    /// Module version for compatibility checking
    fn version(&self) -> Option<&'static str> {
        None
    }
    
    /// Module description
    fn description(&self) -> Option<&'static str> {
        None
    }
    
    /// Check if this module can be disabled
    fn is_optional(&self) -> bool {
        true
    }
}

/// Module metadata for introspection
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub dependencies: Vec<String>,
    pub is_optional: bool,
    pub route_count: usize,
    pub middleware_count: usize,
}

impl ModuleMetadata {
    /// Create metadata from a module
    pub fn from_module<M: Module + ?Sized>(module: &M) -> Self {
        let routes = module.routes();
        let middleware = module.middleware();
        
        Self {
            name: module.name().to_string(),
            version: module.version().map(|v| v.to_string()),
            description: module.description().map(|d| d.to_string()),
            dependencies: module.dependencies().iter().map(|d| d.to_string()).collect(),
            is_optional: module.is_optional(),
            route_count: routes.len(),
            middleware_count: middleware.len(),
        }
    }
}

/// Base module implementation for common functionality
#[derive(Debug)]
pub struct BaseModule {
    name: &'static str,
    version: Option<&'static str>,
    description: Option<&'static str>,
    dependencies: Vec<&'static str>,
    is_optional: bool,
}

impl BaseModule {
    /// Create a new base module
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            version: None,
            description: None,
            dependencies: Vec::new(),
            is_optional: true,
        }
    }
    
    /// Set module version
    pub fn with_version(mut self, version: &'static str) -> Self {
        self.version = Some(version);
        self
    }
    
    /// Set module description
    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Set module dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<&'static str>) -> Self {
        self.dependencies = dependencies;
        self
    }
    
    /// Set if module is optional
    pub fn with_optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }
}

impl Module for BaseModule {
    fn name(&self) -> &'static str {
        self.name
    }
    
    fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> {
        // Base module doesn't configure anything by default
        Ok(builder)
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        self.dependencies.clone()
    }
    
    fn version(&self) -> Option<&'static str> {
        self.version
    }
    
    fn description(&self) -> Option<&'static str> {
        self.description
    }
    
    fn is_optional(&self) -> bool {
        self.is_optional
    }
}

/// Macro to simplify module creation
#[macro_export]
macro_rules! module {
    (
        name: $name:expr,
        $(version: $version:expr,)?
        $(description: $description:expr,)?
        $(dependencies: [$($dep:expr),* $(,)?],)?
        $(optional: $optional:expr,)?
        configure: |$builder:ident| $config:block
        $(, boot: |$container:ident| $boot:block)?
        $(, routes: $routes:expr)?
        $(, middleware: $middleware:expr)?
    ) => {
        {
            struct CustomModule;
            
            impl $crate::modules::Module for CustomModule {
                fn name(&self) -> &'static str {
                    $name
                }
                
                $(fn version(&self) -> Option<&'static str> {
                    Some($version)
                })?
                
                $(fn description(&self) -> Option<&'static str> {
                    Some($description)
                })?
                
                $(fn dependencies(&self) -> Vec<&'static str> {
                    vec![$($dep),*]
                })?
                
                $(fn is_optional(&self) -> bool {
                    $optional
                })?
                
                fn configure(&self, $builder: $crate::container::ContainerBuilder) 
                    -> Result<$crate::container::ContainerBuilder, $crate::modules::ModuleError> 
                {
                    $config
                }
                
                $(fn boot(&self, $container: &$crate::container::Container) 
                    -> Result<(), $crate::modules::ModuleError> 
                {
                    $boot
                })?
                
                $(fn routes(&self) -> Vec<$crate::modules::RouteDefinition> {
                    $routes
                })?
                
                $(fn middleware(&self) -> Vec<$crate::modules::MiddlewareDefinition> {
                    $middleware
                })?
            }
            
            CustomModule
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_metadata() {
        let base_module = BaseModule::new("test_module")
            .with_version("1.0.0")
            .with_description("A test module")
            .with_dependencies(vec!["dependency1", "dependency2"])
            .with_optional(false);
        
        let metadata = ModuleMetadata::from_module(&base_module);
        
        assert_eq!(metadata.name, "test_module");
        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert_eq!(metadata.description, Some("A test module".to_string()));
        assert_eq!(metadata.dependencies, vec!["dependency1", "dependency2"]);
        assert!(!metadata.is_optional);
    }
}