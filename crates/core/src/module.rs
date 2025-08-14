use crate::container::{Container, ContainerBuilder};
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

/// Application that manages modules and container
pub struct Application {
    container: Container,
    modules: Vec<Box<dyn Module>>,
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
    
    /// Start the application by booting all modules
    pub async fn start(&self) -> Result<(), ApplicationError> {
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
}

impl ApplicationBuilder {
    pub fn new() -> Self {
        Self {
            modules: vec![],
        }
    }
    
    /// Add a module to the application
    pub fn module<M: Module + 'static>(mut self, module: M) -> Self {
        self.modules.push(Box::new(module));
        self
    }
    
    /// Build the application by resolving module dependencies and creating container
    pub fn build(self) -> Result<Application, ApplicationError> {
        // Sort modules by dependencies (topological sort)
        let sorted_modules = self.resolve_module_dependencies()?;
        
        // Create container by chaining all module configurations
        let mut builder = Container::builder();
        
        for module in &sorted_modules {
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
            modules: sorted_modules,
        })
    }
    
    /// Resolve module dependencies using topological sort
    fn resolve_module_dependencies(self) -> Result<Vec<Box<dyn Module>>, ApplicationError> {
        let mut modules = self.modules;
        let mut resolved: Vec<usize> = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();
        
        // Create a map of module names to indices for easy lookup
        let module_map: std::collections::HashMap<String, usize> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name().to_string(), i))
            .collect();
        
        fn visit_module(
            index: usize,
            modules: &[Box<dyn Module>],
            module_map: &std::collections::HashMap<String, usize>,
            visiting: &mut std::collections::HashSet<usize>,
            visited: &mut std::collections::HashSet<usize>,
            resolved: &mut Vec<usize>,
        ) -> Result<(), ApplicationError> {
            if visiting.contains(&index) {
                return Err(ApplicationError::CircularDependency {
                    module: modules[index].name().to_string(),
                });
            }
            
            if visited.contains(&index) {
                return Ok(());
            }
            
            visiting.insert(index);
            
            // Visit all dependencies first
            for dep_name in modules[index].dependencies() {
                if let Some(&dep_index) = module_map.get(dep_name) {
                    visit_module(dep_index, modules, module_map, visiting, visited, resolved)?;
                } else {
                    return Err(ApplicationError::MissingDependency {
                        module: modules[index].name().to_string(),
                        dependency: dep_name.to_string(),
                    });
                }
            }
            
            visiting.remove(&index);
            visited.insert(index);
            resolved.push(index);
            
            Ok(())
        }
        
        let mut resolved_indices = Vec::new();
        
        for i in 0..modules.len() {
            if !visited.contains(&i) {
                visit_module(i, &modules, &module_map, &mut visiting, &mut visited, &mut resolved_indices)?;
            }
        }
        
        // Reorder modules based on resolved dependencies
        // Sort indices in reverse order to avoid index shifting issues
        resolved_indices.sort_by(|a, b| b.cmp(a));
        
        let mut sorted_modules = Vec::new();
        for &index in &resolved_indices {
            sorted_modules.push(modules.swap_remove(index));
        }
        
        // Reverse to get correct dependency order
        sorted_modules.reverse();
        
        Ok(sorted_modules)
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
    async fn test_application_with_modules() {
        let app = Application::builder()
            .module(CoreModule)
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
    fn test_module_dependency_resolution() {
        let result = Application::builder()
            .module(AuthModule) // Depends on core
            .module(CoreModule) // Should be loaded first
            .build();
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_missing_dependency_error() {
        let result = Application::builder()
            .module(AuthModule) // Depends on core, but core is not added
            .build();
        
        assert!(result.is_err());
        if let Err(ApplicationError::MissingDependency { module, dependency }) = result {
            assert_eq!(module, "auth");
            assert_eq!(dependency, "core");
        }
    }
}