use crate::container::ContainerBuilder;
use crate::modules::{MiddlewareDefinition, Module, ModuleError, ModuleMetadata, RouteDefinition};
use std::collections::HashMap;

/// Module registry for managing module lifecycle and dependencies
pub struct ModuleRegistry {
    modules: Vec<Box<dyn Module>>,
    loading_order: Vec<usize>,
    routes: Vec<RouteDefinition>,
    middleware: Vec<MiddlewareDefinition>,
    metadata_cache: HashMap<String, ModuleMetadata>,
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            loading_order: Vec::new(),
            routes: Vec::new(),
            middleware: Vec::new(),
            metadata_cache: HashMap::new(),
        }
    }

    /// Register a module
    pub fn register<M: Module + 'static>(&mut self, module: M) {
        let metadata = ModuleMetadata::from_module(&module);
        let name = metadata.name.clone();

        self.modules.push(Box::new(module));
        self.metadata_cache.insert(name, metadata);
    }

    /// Get the number of registered modules
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get module metadata by name
    pub fn get_metadata(&self, name: &str) -> Option<&ModuleMetadata> {
        self.metadata_cache.get(name)
    }

    /// Get all module metadata
    pub fn all_metadata(&self) -> Vec<&ModuleMetadata> {
        self.metadata_cache.values().collect()
    }

    /// Check if a module is registered
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.iter().any(|m| m.name() == name)
    }

    /// Resolve module dependencies and determine loading order
    pub fn resolve_dependencies(&mut self) -> Result<(), ModuleError> {
        let module_count = self.modules.len();

        // Create name to index mapping
        let name_to_index: HashMap<String, usize> = self
            .modules
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
    pub fn configure_all(
        &self,
        mut builder: ContainerBuilder,
    ) -> Result<ContainerBuilder, ModuleError> {
        for &index in &self.loading_order {
            let module = &self.modules[index];
            tracing::info!("Configuring module: {}", module.name());
            builder = module.configure(builder)?;
        }
        Ok(builder)
    }

    /// Boot all modules after container is built
    pub fn boot_all(&self, container: &crate::container::Container) -> Result<(), ModuleError> {
        for &index in &self.loading_order {
            let module = &self.modules[index];
            tracing::info!("Booting module: {}", module.name());
            module
                .boot(container)
                .map_err(|e| ModuleError::BootFailed {
                    message: format!("Failed to boot module '{}': {}", module.name(), e),
                })?;
        }
        Ok(())
    }

    /// Collect all routes from registered modules
    pub fn collect_routes(&mut self) -> Vec<RouteDefinition> {
        if self.routes.is_empty() {
            for module in &self.modules {
                self.routes.extend(module.routes());
            }
        }
        self.routes.clone()
    }

    /// Collect all middleware from registered modules
    pub fn collect_middleware(&mut self) -> Vec<MiddlewareDefinition> {
        if self.middleware.is_empty() {
            for module in &self.modules {
                self.middleware.extend(module.middleware());
            }
            // Sort middleware by priority
            self.middleware.sort();
        }
        self.middleware.clone()
    }

    /// Get modules in loading order
    pub fn modules_in_order(&self) -> Vec<&dyn Module> {
        self.loading_order
            .iter()
            .map(|&index| self.modules[index].as_ref())
            .collect()
    }

    /// Validate all modules
    pub fn validate(&self) -> Result<(), ModuleError> {
        // Check for duplicate module names
        let mut names = std::collections::HashSet::new();
        for module in &self.modules {
            let name = module.name();
            if names.contains(name) {
                return Err(ModuleError::ConfigurationFailed {
                    message: format!("Duplicate module name: {}", name),
                });
            }
            names.insert(name);
        }

        // Check dependencies exist
        for module in &self.modules {
            for dep in module.dependencies() {
                if !names.contains(dep) {
                    return Err(ModuleError::MissingDependency {
                        module: module.name().to_string(),
                        dependency: dep.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Clear all modules (useful for testing)
    pub fn clear(&mut self) {
        self.modules.clear();
        self.loading_order.clear();
        self.routes.clear();
        self.middleware.clear();
        self.metadata_cache.clear();
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRegistry")
            .field("module_count", &self.modules.len())
            .field("route_count", &self.routes.len())
            .field("middleware_count", &self.middleware.len())
            .field("loading_order", &self.loading_order)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::BaseModule;

    #[tokio::test]
    async fn test_module_registry() -> Result<(), ModuleError> {
        let mut registry = ModuleRegistry::new();

        // Register modules with dependencies
        let module_a = BaseModule::new("module_a");
        let module_b = BaseModule::new("module_b").with_dependencies(vec!["module_a"]);
        let module_c = BaseModule::new("module_c").with_dependencies(vec!["module_b", "module_a"]);

        registry.register(module_a);
        registry.register(module_b);
        registry.register(module_c);

        assert_eq!(registry.module_count(), 3);

        // Resolve dependencies
        registry.resolve_dependencies()?;

        // Check loading order
        let modules_in_order = registry.modules_in_order();
        let names: Vec<&str> = modules_in_order.iter().map(|m| m.name()).collect();

        // module_a should be first, module_c should be last
        assert_eq!(names[0], "module_a");
        assert_eq!(names[2], "module_c");

        Ok(())
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let mut registry = ModuleRegistry::new();

        let module_a = BaseModule::new("module_a").with_dependencies(vec!["module_b"]);
        let module_b = BaseModule::new("module_b").with_dependencies(vec!["module_a"]);

        registry.register(module_a);
        registry.register(module_b);

        let result = registry.resolve_dependencies();
        assert!(result.is_err());

        if let Err(ModuleError::CircularDependency { module }) = result {
            assert!(module == "module_a" || module == "module_b");
        } else {
            panic!("Expected circular dependency error");
        }
    }
}
