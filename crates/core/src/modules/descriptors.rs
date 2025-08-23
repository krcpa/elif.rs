//! Module descriptor system for Epic 3 - Module Descriptor Generation
//!
//! Provides comprehensive descriptors that capture module structure, dependencies,
//! and auto-configuration capabilities for runtime composition and dependency resolution.
//!
//! ## Features
//! - **ModuleDescriptor**: Complete module structure with providers, controllers, imports, exports
//! - **ServiceDescriptor**: Service metadata including lifecycle and trait mappings
//! - **ControllerDescriptor**: Controller metadata for routing integration
//! - **Auto-configuration**: Generated `__auto_configure()` functions for IoC integration

use std::any::TypeId;
use crate::container::{IocContainer, ContainerBuilder};
use crate::modules::ModuleError;

/// Lifecycle management for services
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceLifecycle {
    /// Single instance per container
    Singleton,
    /// Single instance per scope
    Scoped, 
    /// New instance per request
    Transient,
}

impl Default for ServiceLifecycle {
    fn default() -> Self {
        Self::Singleton
    }
}

/// Service provider definition with complete metadata
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    /// Service type name for debugging
    pub service_name: String,
    /// Service type ID for resolution
    pub service_type: TypeId,
    /// Implementation type ID (if different from service)
    pub implementation_type: Option<TypeId>,
    /// Named binding identifier
    pub name: Option<String>,
    /// Service lifecycle management
    pub lifecycle: ServiceLifecycle,
    /// Whether this service implements a trait
    pub is_trait_service: bool,
    /// Dependencies that must be resolved first
    pub dependencies: Vec<TypeId>,
}

impl ServiceDescriptor {
    /// Create a new service descriptor
    pub fn new<S: 'static>(
        service_name: impl Into<String>,
        lifecycle: ServiceLifecycle,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            service_type: TypeId::of::<S>(),
            implementation_type: None,
            name: None,
            lifecycle,
            is_trait_service: false,
            dependencies: Vec::new(),
        }
    }
    
    /// Create a service descriptor for trait mapping
    pub fn trait_mapping<S: 'static, I: 'static>(
        service_name: impl Into<String>,
        implementation_name: impl Into<String>,
        lifecycle: ServiceLifecycle,
    ) -> Self {
        Self {
            service_name: format!("{} => {}", service_name.into(), implementation_name.into()),
            service_type: TypeId::of::<S>(),
            implementation_type: Some(TypeId::of::<I>()),
            name: None,
            lifecycle,
            is_trait_service: true,
            dependencies: Vec::new(),
        }
    }
    
    /// Set named binding
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Set service dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<TypeId>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

/// Controller definition with metadata
#[derive(Debug, Clone)]
pub struct ControllerDescriptor {
    /// Controller type name for debugging
    pub controller_name: String,
    /// Controller type ID
    pub controller_type: TypeId,
    /// Base path for controller routes
    pub base_path: Option<String>,
    /// Middleware applied to controller
    pub middleware: Vec<String>,
    /// Dependencies that must be injected
    pub dependencies: Vec<TypeId>,
}

impl ControllerDescriptor {
    /// Create a new controller descriptor
    pub fn new<C: 'static>(controller_name: impl Into<String>) -> Self {
        Self {
            controller_name: controller_name.into(),
            controller_type: TypeId::of::<C>(),
            base_path: None,
            middleware: Vec::new(),
            dependencies: Vec::new(),
        }
    }
    
    /// Set controller base path
    pub fn with_base_path(mut self, path: impl Into<String>) -> Self {
        self.base_path = Some(path.into());
        self
    }
    
    /// Set controller middleware
    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
    
    /// Set controller dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<TypeId>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

/// Complete module descriptor with all metadata and auto-configuration
#[derive(Debug, Clone)]
pub struct ModuleDescriptor {
    /// Module name for identification
    pub name: String,
    /// Module version for compatibility
    pub version: Option<String>,
    /// Module description
    pub description: Option<String>,
    /// Service providers defined in this module
    pub providers: Vec<ServiceDescriptor>,
    /// Controllers defined in this module
    pub controllers: Vec<ControllerDescriptor>,
    /// Other modules that this module imports
    pub imports: Vec<String>,
    /// Services that this module exports to other modules
    pub exports: Vec<String>,
    /// Dependencies that must be loaded first
    pub dependencies: Vec<String>,
    /// Whether this module can be disabled
    pub is_optional: bool,
}

impl ModuleDescriptor {
    /// Create a new module descriptor
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            description: None,
            providers: Vec::new(),
            controllers: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            is_optional: true,
        }
    }
    
    /// Set module version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
    
    /// Set module description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add a service provider
    pub fn with_provider(mut self, provider: ServiceDescriptor) -> Self {
        self.providers.push(provider);
        self
    }
    
    /// Add multiple service providers
    pub fn with_providers(mut self, providers: Vec<ServiceDescriptor>) -> Self {
        self.providers.extend(providers);
        self
    }
    
    /// Add a controller
    pub fn with_controller(mut self, controller: ControllerDescriptor) -> Self {
        self.controllers.push(controller);
        self
    }
    
    /// Add multiple controllers
    pub fn with_controllers(mut self, controllers: Vec<ControllerDescriptor>) -> Self {
        self.controllers.extend(controllers);
        self
    }
    
    /// Set module imports
    pub fn with_imports(mut self, imports: Vec<String>) -> Self {
        self.imports = imports;
        self
    }
    
    /// Set module exports
    pub fn with_exports(mut self, exports: Vec<String>) -> Self {
        self.exports = exports;
        self
    }
    
    /// Set module dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
    
    /// Set if module is optional
    pub fn with_optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }
    
    /// Get total service count
    pub fn service_count(&self) -> usize {
        self.providers.len()
    }
    
    /// Get total controller count
    pub fn controller_count(&self) -> usize {
        self.controllers.len()
    }
    
    /// Check if module has exports
    pub fn has_exports(&self) -> bool {
        !self.exports.is_empty()
    }
    
    /// Check if module has imports
    pub fn has_imports(&self) -> bool {
        !self.imports.is_empty()
    }
}

/// Auto-configuration trait for modules to implement IoC integration
pub trait ModuleAutoConfiguration {
    /// Generate the module descriptor
    fn module_descriptor() -> ModuleDescriptor;
    
    /// Auto-configure the IoC container with this module's services
    fn auto_configure(container: &mut IocContainer) -> Result<(), ModuleError>;
    
    /// Configure the container builder (for compatibility with existing Module trait)
    fn configure_builder(builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> {
        Ok(builder)
    }
}

/// Module composition result for module! macro
#[derive(Debug)]
pub struct ModuleComposition {
    /// Modules included in the composition
    pub modules: Vec<ModuleDescriptor>,
    /// Overrides applied to the composition
    pub overrides: Vec<ServiceDescriptor>,
    /// Final merged configuration
    pub merged_descriptor: ModuleDescriptor,
}

impl ModuleComposition {
    /// Create a new module composition
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            overrides: Vec::new(),
            merged_descriptor: ModuleDescriptor::new("Composed"),
        }
    }
    
    /// Add a module to the composition
    pub fn with_module(mut self, descriptor: ModuleDescriptor) -> Self {
        self.modules.push(descriptor);
        self
    }
    
    /// Add overrides to the composition
    pub fn with_overrides(mut self, overrides: Vec<ServiceDescriptor>) -> Self {
        self.overrides = overrides;
        self
    }
    
    /// Apply composition and resolve conflicts
    pub fn compose(mut self) -> Result<ModuleDescriptor, ModuleError> {
        // First validate modules for circular imports and missing exports
        let validator = ModuleDependencyValidator::new(&self.modules);
        if let Err(validation_errors) = validator.validate() {
            return Err(ModuleError::ConfigurationFailed {
                message: format!(
                    "Module validation failed: {}",
                    validation_errors.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            });
        }
        
        // Get topological sort to ensure proper loading order
        let loading_order = validator.topological_sort()
            .map_err(|e| ModuleError::ConfigurationFailed { 
                message: format!("Failed to determine module loading order: {}", e) 
            })?;
        
        // Merge all modules into final descriptor in dependency order
        let mut final_descriptor = ModuleDescriptor::new("ComposedApplication");
        
        // Create a HashMap for O(1) module lookups instead of O(N) linear search
        let module_map: std::collections::HashMap<_, _> = self.modules.iter()
            .map(|m| (&m.name, m))
            .collect();
        
        // Process modules in topological order - now O(N) instead of O(N^2)
        for module_name in &loading_order {
            if let Some(module) = module_map.get(module_name) {
                final_descriptor.providers.extend(module.providers.clone());
                final_descriptor.controllers.extend(module.controllers.clone());
                final_descriptor.imports.extend(module.imports.clone());
                final_descriptor.exports.extend(module.exports.clone());
            }
        }
        
        // Apply overrides (replace matching services) - O(M+N) instead of O(M*N)
        // Overrides match on (service_type, name) - same TypeId and same named binding
        use std::collections::HashMap;
        let override_map: HashMap<_, _> = self.overrides.iter()
            .map(|s| ((s.service_type, s.name.clone()), s.clone()))
            .collect();

        // Remove original providers that have overrides
        final_descriptor.providers.retain(|p| {
            let key = (p.service_type, p.name.clone());
            !override_map.contains_key(&key)
        });
        
        // Add all overrides (both replacements and new services)
        final_descriptor.providers.extend(override_map.into_values());
        
        self.merged_descriptor = final_descriptor.clone();
        Ok(final_descriptor)
    }
    
    /// Auto-configure all modules in the composition
    pub fn auto_configure_all(
        &self, 
        _container: &mut IocContainer
    ) -> Result<(), ModuleError> {
        // Apply merged configuration to container
        for _provider in &self.merged_descriptor.providers {
            // This is a placeholder - actual IoC integration will depend on
            // how we adapt the existing binding system
            // For now, we'll implement basic registration patterns
        }
        
        Ok(())
    }
}

impl Default for ModuleComposition {
    fn default() -> Self {
        Self::new()
    }
}

/// Module validation errors
#[derive(Debug, Clone)]
pub enum ModuleValidationError {
    /// Circular import detected
    CircularImport {
        module: String,
        cycle: Vec<String>,
    },
    /// Missing export - module tries to import something that isn't exported
    MissingExport {
        importing_module: String,
        target_module: String,
        missing_service: String,
    },
    /// Self-import detected
    SelfImport {
        module: String,
    },
    /// Duplicate service export
    DuplicateExport {
        module: String,
        service: String,
    },
}

impl std::fmt::Display for ModuleValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleValidationError::CircularImport { module, cycle } => {
                write!(f, "Circular import detected in module '{}': {}", module, cycle.join(" -> "))
            }
            ModuleValidationError::MissingExport { importing_module, target_module, missing_service } => {
                write!(
                    f,
                    "Module '{}' tries to import '{}' from '{}', but '{}' doesn't export it",
                    importing_module, missing_service, target_module, target_module
                )
            }
            ModuleValidationError::SelfImport { module } => {
                write!(f, "Module '{}' cannot import itself", module)
            }
            ModuleValidationError::DuplicateExport { module, service } => {
                write!(f, "Module '{}' exports '{}' multiple times", module, service)
            }
        }
    }
}

impl std::error::Error for ModuleValidationError {}

/// Module dependency validator for detecting circular imports and missing exports
#[derive(Debug)]
pub struct ModuleDependencyValidator<'a> {
    /// Modules being validated
    modules: &'a [ModuleDescriptor],
}

impl<'a> ModuleDependencyValidator<'a> {
    /// Create a new validator
    pub fn new(modules: &'a [ModuleDescriptor]) -> Self {
        Self { modules }
    }
    
    /// Validate all modules for structural issues
    /// Note: Circular import detection is handled by topological_sort() for efficiency
    pub fn validate(&self) -> Result<(), Vec<ModuleValidationError>> {
        let mut errors = Vec::new();
        
        // Check for missing exports
        if let Err(export_errors) = self.validate_missing_exports() {
            errors.extend(export_errors);
        }
        
        // Check for self-imports
        if let Err(self_import_errors) = self.validate_self_imports() {
            errors.extend(self_import_errors);
        }
        
        // Check for duplicate exports within modules
        if let Err(duplicate_errors) = self.validate_duplicate_exports() {
            errors.extend(duplicate_errors);
        }
        
        // Check for circular imports via topological sort (efficient single-pass)
        if let Err(circular_error) = self.topological_sort() {
            errors.push(circular_error);
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    
    /// Validate that all imports have corresponding exports
    fn validate_missing_exports(&self) -> Result<(), Vec<ModuleValidationError>> {
        let mut errors = Vec::new();
        
        // Build export map: module_name -> exported_services
        let mut export_map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        
        for module in self.modules {
            export_map.insert(
                module.name.clone(),
                module.exports.iter().cloned().collect(),
            );
        }
        
        // Check each module's imports
        for module in self.modules {
            for import_module in &module.imports {
                // Check if the imported module exists
                if let Some(exported_services) = export_map.get(import_module.as_str()) {
                    // For now, we assume modules import all exported services
                    // In a more sophisticated system, we could track specific service imports
                    if exported_services.is_empty() {
                        // Importing from a module that exports nothing might be intentional
                        // (e.g., for side effects), so we'll allow it
                        continue;
                    }
                } else {
                    // The imported module doesn't exist in our module set
                    // This could be an external module, so we'll create a generic error
                    errors.push(ModuleValidationError::MissingExport {
                        importing_module: module.name.clone(),
                        target_module: import_module.clone(),
                        missing_service: "*unknown*".to_string(),
                    });
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate that modules don't import themselves
    fn validate_self_imports(&self) -> Result<(), Vec<ModuleValidationError>> {
        let mut errors = Vec::new();
        
        for module in self.modules {
            if module.imports.contains(&module.name) {
                errors.push(ModuleValidationError::SelfImport {
                    module: module.name.clone(),
                });
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate that modules don't export the same service multiple times
    fn validate_duplicate_exports(&self) -> Result<(), Vec<ModuleValidationError>> {
        let mut errors = Vec::new();
        
        for module in self.modules {
            let mut seen_exports = std::collections::HashSet::new();
            
            for export in &module.exports {
                if !seen_exports.insert(export.clone()) {
                    errors.push(ModuleValidationError::DuplicateExport {
                        module: module.name.clone(),
                        service: export.clone(),
                    });
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Get topological order of modules (dependencies first)
    pub fn topological_sort(&self) -> Result<Vec<String>, ModuleValidationError> {
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        
        for module in self.modules {
            if !visited.contains(&module.name) {
                if let Err(cycle) = self.topological_visit(
                    &module.name,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                ) {
                    return Err(ModuleValidationError::CircularImport {
                        module: module.name.clone(),
                        cycle,
                    });
                }
            }
        }
        
        // Result is already in correct dependency order (dependencies first)
        Ok(result)
    }
    
    /// Topological sort helper using DFS
    fn topological_visit(
        &self,
        module_name: &str,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        if temp_visited.contains(module_name) {
            return Err(vec![module_name.to_string()]);
        }
        
        if visited.contains(module_name) {
            return Ok(());
        }
        
        temp_visited.insert(module_name.to_string());
        
        if let Some(module) = self.modules.iter().find(|m| m.name == module_name) {
            for import in &module.imports {
                if let Err(mut cycle) = self.topological_visit(import, visited, temp_visited, result) {
                    cycle.insert(0, module_name.to_string());
                    return Err(cycle);
                }
            }
        }
        
        temp_visited.remove(module_name);
        visited.insert(module_name.to_string());
        result.push(module_name.to_string());
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    
    #[test]
    fn test_service_descriptor_creation() {
        let descriptor = ServiceDescriptor::new::<String>("TestService", ServiceLifecycle::Singleton);
        
        assert_eq!(descriptor.service_name, "TestService");
        assert_eq!(descriptor.service_type, TypeId::of::<String>());
        assert_eq!(descriptor.lifecycle, ServiceLifecycle::Singleton);
        assert!(!descriptor.is_trait_service);
    }
    
    #[test]
    fn test_trait_service_descriptor() {
        let descriptor = ServiceDescriptor::trait_mapping::<String, Vec<u8>>(
            "TraitService",
            "Implementation", 
            ServiceLifecycle::Scoped
        );
        
        assert!(descriptor.service_name.contains(" => "));
        assert_eq!(descriptor.service_type, TypeId::of::<String>());
        assert_eq!(descriptor.implementation_type, Some(TypeId::of::<Vec<u8>>()));
        assert!(descriptor.is_trait_service);
        assert_eq!(descriptor.lifecycle, ServiceLifecycle::Scoped);
    }
    
    #[test]
    fn test_controller_descriptor_creation() {
        let descriptor = ControllerDescriptor::new::<String>("TestController")
            .with_base_path("/api")
            .with_middleware(vec!["auth".to_string(), "cors".to_string()]);
        
        assert_eq!(descriptor.controller_name, "TestController");
        assert_eq!(descriptor.base_path, Some("/api".to_string()));
        assert_eq!(descriptor.middleware, vec!["auth", "cors"]);
    }
    
    #[test]
    fn test_module_descriptor_builder() {
        let provider = ServiceDescriptor::new::<String>("TestService", ServiceLifecycle::Singleton);
        let controller = ControllerDescriptor::new::<Vec<u8>>("TestController");
        
        let descriptor = ModuleDescriptor::new("TestModule")
            .with_version("1.0.0")
            .with_description("Test module for Epic 3")
            .with_provider(provider)
            .with_controller(controller)
            .with_imports(vec!["DatabaseModule".to_string()])
            .with_exports(vec!["TestService".to_string()])
            .with_optional(false);
        
        assert_eq!(descriptor.name, "TestModule");
        assert_eq!(descriptor.version, Some("1.0.0".to_string()));
        assert_eq!(descriptor.service_count(), 1);
        assert_eq!(descriptor.controller_count(), 1);
        assert!(descriptor.has_imports());
        assert!(descriptor.has_exports());
        assert!(!descriptor.is_optional);
    }
    
    #[test]
    fn test_module_composition() {
        let module1 = ModuleDescriptor::new("Module1")
            .with_provider(ServiceDescriptor::new::<String>("Service1", ServiceLifecycle::Singleton));
            
        let module2 = ModuleDescriptor::new("Module2") 
            .with_provider(ServiceDescriptor::new::<Vec<u8>>("Service2", ServiceLifecycle::Scoped));
        
        let composition = ModuleComposition::new()
            .with_module(module1)
            .with_module(module2);
        
        let result = composition.compose().unwrap();
        
        assert_eq!(result.name, "ComposedApplication");
        assert_eq!(result.service_count(), 2);
    }
    
    #[test]
    fn test_module_composition_with_overrides() {
        let module = ModuleDescriptor::new("TestModule")
            .with_provider(ServiceDescriptor::new::<String>("OriginalService", ServiceLifecycle::Singleton));
        
        let override_service = ServiceDescriptor::new::<String>("OverrideService", ServiceLifecycle::Transient);
        
        let composition = ModuleComposition::new()
            .with_module(module)
            .with_overrides(vec![override_service]);
            
        let result = composition.compose().unwrap();
        
        // Should have the override service instead of original
        assert_eq!(result.service_count(), 1);
        assert_eq!(result.providers[0].service_name, "OverrideService");
        assert_eq!(result.providers[0].lifecycle, ServiceLifecycle::Transient);
    }
    
    #[test]
    fn test_module_validation_circular_imports() {
        let module_a = ModuleDescriptor::new("ModuleA")
            .with_imports(vec!["ModuleB".to_string()]);
            
        let module_b = ModuleDescriptor::new("ModuleB")
            .with_imports(vec!["ModuleC".to_string()]);
            
        let module_c = ModuleDescriptor::new("ModuleC")
            .with_imports(vec!["ModuleA".to_string()]); // Creates cycle A -> B -> C -> A
        
        let modules = vec![module_a, module_b, module_c];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        
        match &errors[0] {
            ModuleValidationError::CircularImport { module, cycle } => {
                assert!(module == "ModuleA" || module == "ModuleB" || module == "ModuleC");
                assert!(cycle.len() >= 3);
            }
            _ => panic!("Expected CircularImport error"),
        }
    }
    
    #[test]
    fn test_module_validation_self_import() {
        let module = ModuleDescriptor::new("SelfModule")
            .with_imports(vec!["SelfModule".to_string()]);
        
        let modules = vec![module];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        
        // Should have at least one error (self-import)
        assert!(!errors.is_empty());
        
        // Check that one of the errors is a self-import error
        let has_self_import = errors.iter().any(|e| matches!(e, 
            ModuleValidationError::SelfImport { module } if module == "SelfModule"
        ));
        assert!(has_self_import, "Should have SelfImport error");
    }
    
    #[test]
    fn test_module_validation_missing_exports() {
        let module_a = ModuleDescriptor::new("ModuleA")
            .with_imports(vec!["NonExistentModule".to_string()]);
        
        let modules = vec![module_a];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        
        match &errors[0] {
            ModuleValidationError::MissingExport { importing_module, target_module, .. } => {
                assert_eq!(importing_module, "ModuleA");
                assert_eq!(target_module, "NonExistentModule");
            }
            _ => panic!("Expected MissingExport error"),
        }
    }
    
    #[test]
    fn test_module_validation_duplicate_exports() {
        let module = ModuleDescriptor::new("TestModule")
            .with_exports(vec!["Service1".to_string(), "Service1".to_string()]);
        
        let modules = vec![module];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        
        match &errors[0] {
            ModuleValidationError::DuplicateExport { module, service } => {
                assert_eq!(module, "TestModule");
                assert_eq!(service, "Service1");
            }
            _ => panic!("Expected DuplicateExport error"),
        }
    }
    
    #[test]
    fn test_module_validation_success() {
        let module_a = ModuleDescriptor::new("ModuleA")
            .with_exports(vec!["ServiceA".to_string()]);
            
        let module_b = ModuleDescriptor::new("ModuleB")
            .with_imports(vec!["ModuleA".to_string()])
            .with_exports(vec!["ServiceB".to_string()]);
        
        let modules = vec![module_a, module_b];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_module_topological_sort() {
        let module_a = ModuleDescriptor::new("ModuleA"); // No dependencies
        
        let module_b = ModuleDescriptor::new("ModuleB")
            .with_imports(vec!["ModuleA".to_string()]); // Depends on A
            
        let module_c = ModuleDescriptor::new("ModuleC")
            .with_imports(vec!["ModuleB".to_string()]); // Depends on B
        
        let modules = vec![module_c, module_a, module_b];
        let validator = ModuleDependencyValidator::new(&modules);
        let sorted = validator.topological_sort().unwrap();
        
        // Should be sorted in dependency order: A, B, C
        // (dependencies first, then modules that depend on them)
        let a_pos = sorted.iter().position(|m| m == "ModuleA").unwrap();
        let b_pos = sorted.iter().position(|m| m == "ModuleB").unwrap();
        let c_pos = sorted.iter().position(|m| m == "ModuleC").unwrap();
        
        assert!(a_pos < b_pos, "ModuleA should come before ModuleB");
        assert!(b_pos < c_pos, "ModuleB should come before ModuleC");
    }
    
    #[test]
    fn test_module_validation_error_display() {
        let error = ModuleValidationError::CircularImport {
            module: "TestModule".to_string(),
            cycle: vec!["A".to_string(), "B".to_string(), "C".to_string(), "A".to_string()],
        };
        
        let error_string = format!("{}", error);
        assert!(error_string.contains("Circular import detected in module 'TestModule'"));
        assert!(error_string.contains("A -> B -> C -> A"));
    }
}