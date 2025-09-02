//! Module metadata structures for compile-time module discovery
//!
//! This module provides the data structures needed to support compile-time
//! module scanning and automatic discovery of #[module] decorated types.

use std::collections::HashMap;

/// Metadata about a single module discovered at compile time
#[derive(Debug, Clone, PartialEq)]
pub struct CompileTimeModuleMetadata {
    /// Name of the module type
    pub name: String,
    /// List of controller types in this module
    pub controllers: Vec<String>,
    /// List of provider types in this module  
    pub providers: Vec<String>,
    /// List of imported modules
    pub imports: Vec<String>,
    /// List of exported providers
    pub exports: Vec<String>,
}

impl CompileTimeModuleMetadata {
    /// Create a new module metadata instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            controllers: Vec::new(),
            providers: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Add a controller to this module
    pub fn with_controller(mut self, controller: String) -> Self {
        self.controllers.push(controller);
        self
    }

    /// Add multiple controllers to this module
    pub fn with_controllers(mut self, controllers: Vec<String>) -> Self {
        self.controllers.extend(controllers);
        self
    }

    /// Add a provider to this module
    pub fn with_provider(mut self, provider: String) -> Self {
        self.providers.push(provider);
        self
    }

    /// Add multiple providers to this module
    pub fn with_providers(mut self, providers: Vec<String>) -> Self {
        self.providers.extend(providers);
        self
    }

    /// Add an import to this module
    pub fn with_import(mut self, import: String) -> Self {
        self.imports.push(import);
        self
    }

    /// Add multiple imports to this module
    pub fn with_imports(mut self, imports: Vec<String>) -> Self {
        self.imports.extend(imports);
        self
    }

    /// Add an export to this module
    pub fn with_export(mut self, export: String) -> Self {
        self.exports.push(export);
        self
    }

    /// Add multiple exports to this module
    pub fn with_exports(mut self, exports: Vec<String>) -> Self {
        self.exports.extend(exports);
        self
    }
}

/// Global registry of module metadata discovered at compile time
#[derive(Debug, Clone)]
pub struct CompileTimeModuleRegistry {
    /// Map of module name to metadata
    modules: HashMap<String, CompileTimeModuleMetadata>,
    /// Dependency graph: module name -> list of dependency names
    dependency_graph: HashMap<String, Vec<String>>,
}

impl CompileTimeModuleRegistry {
    /// Create a new empty module registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependency_graph: HashMap::new(),
        }
    }

    /// Register a module in the registry
    pub fn register_module(&mut self, metadata: CompileTimeModuleMetadata) {
        let module_name = metadata.name.clone();
        let dependencies = metadata.imports.clone();
        
        self.modules.insert(module_name.clone(), metadata);
        self.dependency_graph.insert(module_name, dependencies);
    }

    /// Get all registered modules
    pub fn all_modules(&self) -> Vec<&CompileTimeModuleMetadata> {
        self.modules.values().collect()
    }

    /// Find a module by name
    pub fn find_module(&self, name: &str) -> Option<&CompileTimeModuleMetadata> {
        self.modules.get(name)
    }

    /// Get all controllers from all modules
    pub fn all_controllers(&self) -> Vec<String> {
        self.modules
            .values()
            .flat_map(|module| module.controllers.iter().cloned())
            .collect()
    }

    /// Get all providers from all modules
    pub fn all_providers(&self) -> Vec<String> {
        self.modules
            .values()
            .flat_map(|module| module.providers.iter().cloned())
            .collect()
    }

    /// Get modules that have controllers
    pub fn modules_with_controllers(&self) -> Vec<&CompileTimeModuleMetadata> {
        self.modules
            .values()
            .filter(|module| !module.controllers.is_empty())
            .collect()
    }

    /// Get modules that have providers
    pub fn modules_with_providers(&self) -> Vec<&CompileTimeModuleMetadata> {
        self.modules
            .values()
            .filter(|module| !module.providers.is_empty())
            .collect()
    }

    /// Get the dependency graph
    pub fn dependency_graph(&self) -> &HashMap<String, Vec<String>> {
        &self.dependency_graph
    }

    /// Resolve module dependencies and return modules in dependency order
    pub fn resolve_dependency_order(&self) -> Result<Vec<&CompileTimeModuleMetadata>, String> {
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        let mut result = Vec::new();

        // Sort module names for deterministic ordering
        let mut module_names: Vec<_> = self.modules.keys().collect();
        module_names.sort();

        for module_name in module_names {
            if !visited.contains(module_name) {
                self.visit_for_topological_sort(
                    module_name,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    /// Visit a module during topological sorting
    fn visit_for_topological_sort<'a>(
        &'a self,
        module_name: &str,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<&'a CompileTimeModuleMetadata>,
    ) -> Result<(), String> {
        if temp_visited.contains(module_name) {
            return Err(format!("Circular dependency detected involving module '{}'", module_name));
        }

        if visited.contains(module_name) {
            return Ok(());
        }

        temp_visited.insert(module_name.to_string());

        // Visit dependencies first
        if let Some(dependencies) = self.dependency_graph.get(module_name) {
            for dep_name in dependencies {
                if !self.modules.contains_key(dep_name) {
                    return Err(format!(
                        "Module '{}' depends on '{}' which is not registered",
                        module_name, dep_name
                    ));
                }
                self.visit_for_topological_sort(dep_name, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(module_name);
        visited.insert(module_name.to_string());

        if let Some(metadata) = self.modules.get(module_name) {
            result.push(metadata);
        }

        Ok(())
    }

    /// Get the number of registered modules
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Default for CompileTimeModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe global module registry
use std::sync::{Mutex, OnceLock};

static GLOBAL_MODULE_REGISTRY: OnceLock<Mutex<CompileTimeModuleRegistry>> = OnceLock::new();

/// Register a module in the global registry
/// 
/// # Panics
/// Panics if the global registry lock is poisoned, which indicates a serious
/// inconsistency in the module system that should not be silently ignored.
pub fn register_module_globally(metadata: CompileTimeModuleMetadata) {
    let registry_mutex = GLOBAL_MODULE_REGISTRY.get_or_init(|| Mutex::new(CompileTimeModuleRegistry::new()));
    registry_mutex
        .lock()
        .expect("Global module registry is poisoned")
        .register_module(metadata);
}

/// Get a copy of the global module registry
/// 
/// # Panics
/// Panics if the global registry lock is poisoned, which indicates a serious
/// inconsistency in the module system that should not be silently ignored.
pub fn get_global_module_registry() -> CompileTimeModuleRegistry {
    let registry_mutex = GLOBAL_MODULE_REGISTRY.get_or_init(|| Mutex::new(CompileTimeModuleRegistry::new()));
    registry_mutex
        .lock()
        .expect("Global module registry is poisoned")
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_metadata_creation() {
        let metadata = CompileTimeModuleMetadata::new("UserModule".to_string())
            .with_controller("UserController".to_string())
            .with_provider("UserService".to_string())
            .with_import("AuthModule".to_string())
            .with_export("UserService".to_string());

        assert_eq!(metadata.name, "UserModule");
        assert_eq!(metadata.controllers, vec!["UserController"]);
        assert_eq!(metadata.providers, vec!["UserService"]);
        assert_eq!(metadata.imports, vec!["AuthModule"]);
        assert_eq!(metadata.exports, vec!["UserService"]);
    }

    #[test]
    fn test_module_registry_basic_operations() {
        let mut registry = CompileTimeModuleRegistry::new();

        let user_module = CompileTimeModuleMetadata::new("UserModule".to_string())
            .with_controller("UserController".to_string())
            .with_provider("UserService".to_string());

        let auth_module = CompileTimeModuleMetadata::new("AuthModule".to_string())
            .with_provider("AuthService".to_string());

        registry.register_module(user_module);
        registry.register_module(auth_module);

        assert_eq!(registry.module_count(), 2);
        assert!(registry.find_module("UserModule").is_some());
        assert!(registry.find_module("AuthModule").is_some());
        assert!(registry.find_module("NonExistentModule").is_none());

        let controllers = registry.all_controllers();
        assert_eq!(controllers.len(), 1);
        assert!(controllers.contains(&"UserController".to_string()));

        let providers = registry.all_providers();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"UserService".to_string()));
        assert!(providers.contains(&"AuthService".to_string()));
    }

    #[test]
    fn test_dependency_resolution() {
        let mut registry = CompileTimeModuleRegistry::new();

        // Create modules with dependencies: UserModule depends on AuthModule
        let auth_module = CompileTimeModuleMetadata::new("AuthModule".to_string())
            .with_provider("AuthService".to_string());

        let user_module = CompileTimeModuleMetadata::new("UserModule".to_string())
            .with_controller("UserController".to_string())
            .with_import("AuthModule".to_string()); // UserModule depends on AuthModule

        registry.register_module(user_module);
        registry.register_module(auth_module);

        let resolved_order = registry.resolve_dependency_order().unwrap();
        let module_names: Vec<_> = resolved_order.iter().map(|m| &m.name).collect();

        // AuthModule should come before UserModule
        assert_eq!(module_names.len(), 2);
        let auth_index = module_names.iter().position(|&name| name == "AuthModule").unwrap();
        let user_index = module_names.iter().position(|&name| name == "UserModule").unwrap();
        assert!(auth_index < user_index);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut registry = CompileTimeModuleRegistry::new();

        // Create circular dependency: A depends on B, B depends on A
        let module_a = CompileTimeModuleMetadata::new("ModuleA".to_string())
            .with_import("ModuleB".to_string());

        let module_b = CompileTimeModuleMetadata::new("ModuleB".to_string())
            .with_import("ModuleA".to_string());

        registry.register_module(module_a);
        registry.register_module(module_b);

        let result = registry.resolve_dependency_order();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency"));
    }

    #[test]
    fn test_missing_dependency_detection() {
        let mut registry = CompileTimeModuleRegistry::new();

        let user_module = CompileTimeModuleMetadata::new("UserModule".to_string())
            .with_import("NonExistentModule".to_string()); // Missing dependency

        registry.register_module(user_module);

        let result = registry.resolve_dependency_order();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not registered"));
    }

    #[test]
    fn test_global_registry() {
        // Test global registry functions
        let metadata = CompileTimeModuleMetadata::new("TestModule".to_string())
            .with_controller("TestController".to_string());

        register_module_globally(metadata);

        let global_registry = get_global_module_registry();
        assert!(global_registry.find_module("TestModule").is_some());
    }
}