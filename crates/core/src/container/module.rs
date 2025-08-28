use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::container::binding::{ServiceBinder, ServiceBindings};
use crate::container::ioc_container::IocContainer;
use crate::errors::CoreError;

/// Unique identifier for a service module
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    name: String,
    type_id: TypeId,
}

impl ModuleId {
    /// Create a module ID for a specific type
    pub fn of<T: ServiceModule + 'static>() -> Self {
        Self {
            name: std::any::type_name::<T>().to_string(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Create a named module ID
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_id: TypeId::of::<()>(), // Use unit type for named modules
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Service module trait for organizing related services
pub trait ServiceModule: Send + Sync
where
    Self: 'static,
{
    /// Get the unique identifier for this module
    fn id(&self) -> ModuleId
    where
        Self: Sized,
    {
        ModuleId::of::<Self>()
    }

    /// Get module name (defaults to type name)
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Get module description
    fn description(&self) -> Option<&str> {
        None
    }

    /// Get module version
    fn version(&self) -> Option<&str> {
        None
    }

    /// Configure services for this module using ServiceBindings
    fn configure(&self, services: &mut ServiceBindings) {
        // Default implementation does nothing
        // Modules should override this to register their services
        let _ = services;
    }

    /// Get module dependencies (other modules this module depends on)
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![]
    }

    /// Initialize module after all dependencies are loaded
    fn initialize(
        &self,
        container: &IocContainer,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CoreError>> + Send + '_>>
    {
        let _ = container; // Default implementation does nothing
        Box::pin(async move { Ok(()) })
    }

    /// Cleanup module resources
    fn shutdown(
        &self,
        container: &IocContainer,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CoreError>> + Send + '_>>
    {
        let _ = container; // Default implementation does nothing
        Box::pin(async move { Ok(()) })
    }

    /// Check if module is compatible with given version
    fn is_compatible_with(&self, other_version: &str) -> bool {
        let _ = other_version; // Default implementation is always compatible
        true
    }

    /// Get module metadata
    fn metadata(&self) -> ModuleMetadata
    where
        Self: Sized,
    {
        ModuleMetadata {
            id: self.id(),
            name: self.name().to_string(),
            description: self.description().map(|s| s.to_string()),
            version: self.version().map(|s| s.to_string()),
            dependencies: self.depends_on(),
        }
    }
}

/// Module metadata for introspection
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub id: ModuleId,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub dependencies: Vec<ModuleId>,
}

/// Module configuration options
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    /// Whether to automatically initialize the module
    pub auto_initialize: bool,
    /// Initialization timeout
    pub init_timeout: Option<std::time::Duration>,
    /// Whether to validate dependencies before loading
    pub validate_dependencies: bool,
    /// Additional configuration parameters
    pub parameters: HashMap<String, String>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            auto_initialize: true,
            init_timeout: Some(std::time::Duration::from_secs(30)),
            validate_dependencies: true,
            parameters: HashMap::new(),
        }
    }
}

/// Module loading state
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleState {
    /// Module is registered but not configured
    Registered,
    /// Module services are configured
    Configured,
    /// Module is initialized and ready
    Initialized,
    /// Module initialization failed
    Failed(String),
    /// Module is shut down
    Shutdown,
}

/// Information about a loaded module
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub metadata: ModuleMetadata,
    pub config: ModuleConfig,
    pub state: ModuleState,
    pub load_order: usize,
    pub init_duration: Option<std::time::Duration>,
}

/// Module registry for managing service modules
// Debug removed due to dyn ServiceModule incompatibility
pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Arc<dyn ServiceModule>>,
    loaded_modules: HashMap<ModuleId, LoadedModule>,
    dependency_graph: HashMap<ModuleId, Vec<ModuleId>>,
    load_order: Vec<ModuleId>,
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            loaded_modules: HashMap::new(),
            dependency_graph: HashMap::new(),
            load_order: Vec::new(),
        }
    }

    /// Register a module
    pub fn register_module<T: ServiceModule + 'static>(
        &mut self,
        module: T,
        config: Option<ModuleConfig>,
    ) -> Result<(), CoreError> {
        let module_arc = Arc::new(module);
        let module_id = module_arc.id();
        let metadata = module_arc.metadata();

        // Check for duplicate registration
        if self.modules.contains_key(&module_id) {
            return Err(CoreError::InvalidServiceDescriptor {
                message: format!("Module {} is already registered", metadata.name),
            });
        }

        // Store module
        self.modules.insert(module_id.clone(), module_arc);

        // Store dependency info
        self.dependency_graph
            .insert(module_id.clone(), metadata.dependencies.clone());

        // Create loaded module info
        let loaded_module = LoadedModule {
            metadata,
            config: config.unwrap_or_default(),
            state: ModuleState::Registered,
            load_order: 0, // Will be set during load ordering
            init_duration: None,
        };

        self.loaded_modules.insert(module_id, loaded_module);

        Ok(())
    }

    /// Register multiple modules
    pub fn register_modules<T: ServiceModule + 'static>(
        &mut self,
        modules: Vec<T>,
    ) -> Result<(), CoreError> {
        for module in modules {
            self.register_module(module, None)?;
        }
        Ok(())
    }

    /// Calculate load order based on dependencies
    pub fn calculate_load_order(&mut self) -> Result<Vec<ModuleId>, CoreError> {
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        let mut order = Vec::new();

        // Topological sort using DFS
        // Sort module IDs to ensure consistent ordering
        let mut module_ids: Vec<_> = self.modules.keys().cloned().collect();
        module_ids.sort_by(|a, b| a.name().cmp(b.name()));
        
        for module_id in module_ids {
            if !visited.contains(&module_id) {
                self.visit_module_for_ordering(
                    &module_id,
                    &mut visited,
                    &mut temp_visited,
                    &mut order,
                )?;
            }
        }

        // No reverse needed - DFS post-order already gives correct dependency order
        self.load_order = order.clone();

        // Update load order in module info
        for (index, module_id) in order.iter().enumerate() {
            if let Some(loaded_module) = self.loaded_modules.get_mut(module_id) {
                loaded_module.load_order = index;
            }
        }

        Ok(order)
    }

    /// Visit module for topological sorting (DFS)
    fn visit_module_for_ordering(
        &self,
        module_id: &ModuleId,
        visited: &mut HashSet<ModuleId>,
        temp_visited: &mut HashSet<ModuleId>,
        order: &mut Vec<ModuleId>,
    ) -> Result<(), CoreError> {
        if temp_visited.contains(module_id) {
            return Err(CoreError::InvalidServiceDescriptor {
                message: format!(
                    "Circular dependency detected involving module {}",
                    module_id.name()
                ),
            });
        }

        if visited.contains(module_id) {
            return Ok(());
        }

        temp_visited.insert(module_id.clone());

        // Visit dependencies first
        if let Some(dependencies) = self.dependency_graph.get(module_id) {
            for dep_id in dependencies {
                // Check if dependency is registered
                if !self.modules.contains_key(dep_id) {
                    return Err(CoreError::ServiceNotFound {
                        service_type: format!(
                            "Module dependency {} for module {}",
                            dep_id.name(),
                            module_id.name()
                        ),
                    });
                }
                self.visit_module_for_ordering(dep_id, visited, temp_visited, order)?;
            }
        }

        temp_visited.remove(module_id);
        visited.insert(module_id.clone());
        order.push(module_id.clone());

        Ok(())
    }

    /// Configure all registered modules
    pub fn configure_all<T: ServiceBinder>(&mut self, container: &mut T) -> Result<(), CoreError> {
        let order = if self.load_order.is_empty() {
            self.calculate_load_order()?
        } else {
            self.load_order.clone()
        };

        // Collect all service bindings from all modules first
        let mut bindings = ServiceBindings::new();

        for module_id in &order {
            let module = self
                .modules
                .get(module_id)
                .ok_or_else(|| CoreError::ServiceNotFound {
                    service_type: format!("Module {}", module_id.name()),
                })?
                .clone();

            // Let module configure its services into bindings
            module.configure(&mut bindings);

            // Update state
            if let Some(loaded_module) = self.loaded_modules.get_mut(module_id) {
                loaded_module.state = ModuleState::Configured;
            }
        }

        // Now register all collected bindings with the container using the new add_service_descriptor method
        for descriptor in bindings.into_descriptors() {
            container.add_service_descriptor(descriptor)?;
        }

        Ok(())
    }

    /// Initialize all modules in dependency order
    pub async fn initialize_all(&mut self, container: &IocContainer) -> Result<(), CoreError> {
        let order = self.load_order.clone();

        for module_id in order {
            let start_time = std::time::Instant::now();

            let module = self
                .modules
                .get(&module_id)
                .ok_or_else(|| CoreError::ServiceNotFound {
                    service_type: format!("Module {}", module_id.name()),
                })?
                .clone();

            // Get config for timeout
            let config = self
                .loaded_modules
                .get(&module_id)
                .map(|m| m.config.clone())
                .unwrap_or_default();

            // Initialize with timeout if specified
            let result = if let Some(timeout) = config.init_timeout {
                tokio::time::timeout(timeout, module.initialize(container))
                    .await
                    .map_err(|_| CoreError::InvalidServiceDescriptor {
                        message: format!("Module {} initialization timed out", module_id.name()),
                    })?
            } else {
                module.initialize(container).await
            };

            let duration = start_time.elapsed();

            // Update module state
            if let Some(loaded_module) = self.loaded_modules.get_mut(&module_id) {
                loaded_module.init_duration = Some(duration);
                loaded_module.state = match result {
                    Ok(()) => ModuleState::Initialized,
                    Err(ref e) => ModuleState::Failed(e.to_string()),
                };
            }

            result?;
        }

        Ok(())
    }

    /// Shutdown all modules in reverse dependency order
    pub async fn shutdown_all(&mut self, container: &IocContainer) -> Result<(), CoreError> {
        let mut order = self.load_order.clone();
        order.reverse(); // Shutdown in reverse order

        for module_id in order {
            let module = self
                .modules
                .get(&module_id)
                .ok_or_else(|| CoreError::ServiceNotFound {
                    service_type: format!("Module {}", module_id.name()),
                })?
                .clone();

            if let Err(e) = module.shutdown(container).await {
                eprintln!(
                    "Warning: Module {} shutdown failed: {}",
                    module_id.name(),
                    e
                );
                // Continue with other modules even if one fails
            }

            // Update state
            if let Some(loaded_module) = self.loaded_modules.get_mut(&module_id) {
                loaded_module.state = ModuleState::Shutdown;
            }
        }

        Ok(())
    }

    /// Get module by ID
    pub fn get_module(&self, module_id: &ModuleId) -> Option<&Arc<dyn ServiceModule>> {
        self.modules.get(module_id)
    }

    /// Get loaded module info
    pub fn get_loaded_module(&self, module_id: &ModuleId) -> Option<&LoadedModule> {
        self.loaded_modules.get(module_id)
    }

    /// Get all loaded modules
    pub fn get_all_loaded_modules(&self) -> Vec<&LoadedModule> {
        self.loaded_modules.values().collect()
    }

    /// Check if a module is loaded and initialized
    pub fn is_module_ready(&self, module_id: &ModuleId) -> bool {
        self.loaded_modules
            .get(module_id)
            .map(|m| m.state == ModuleState::Initialized)
            .unwrap_or(false)
    }

    /// Get module dependency graph for visualization
    pub fn get_dependency_graph(&self) -> &HashMap<ModuleId, Vec<ModuleId>> {
        &self.dependency_graph
    }

    /// Get load order
    pub fn get_load_order(&self) -> &[ModuleId] {
        &self.load_order
    }

    /// Validate all module dependencies
    pub fn validate_dependencies(&self) -> Result<(), Vec<CoreError>> {
        let mut errors = Vec::new();

        for (module_id, dependencies) in &self.dependency_graph {
            for dep_id in dependencies {
                if !self.modules.contains_key(dep_id) {
                    errors.push(CoreError::ServiceNotFound {
                        service_type: format!(
                            "Module dependency {} for module {}",
                            dep_id.name(),
                            module_id.name()
                        ),
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
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for modular container configuration
pub struct ModularContainerBuilder<T>
where
    T: ServiceBinder,
{
    registry: ModuleRegistry,
    container: T,
}

impl<T> ModularContainerBuilder<T>
where
    T: ServiceBinder,
{
    /// Create a new modular container builder
    pub fn new(container: T) -> Self {
        Self {
            registry: ModuleRegistry::new(),
            container,
        }
    }

    /// Add a module
    pub fn add_module<M: ServiceModule + 'static>(mut self, module: M) -> Result<Self, CoreError> {
        self.registry.register_module(module, None)?;
        Ok(self)
    }

    /// Add a module with configuration
    pub fn add_module_with_config<M: ServiceModule + 'static>(
        mut self,
        module: M,
        config: ModuleConfig,
    ) -> Result<Self, CoreError> {
        self.registry.register_module(module, Some(config))?;
        Ok(self)
    }

    /// Add multiple modules
    pub fn add_modules<M: ServiceModule + 'static>(
        mut self,
        modules: Vec<M>,
    ) -> Result<Self, CoreError> {
        self.registry.register_modules(modules)?;
        Ok(self)
    }

    /// Build the container with all modules configured
    pub fn build(mut self) -> Result<(T, ModuleRegistry), CoreError> {
        // Validate dependencies
        self.registry
            .validate_dependencies()
            .map_err(|errors| errors.into_iter().next().unwrap())?; // Return first error

        // Configure all modules
        self.registry.configure_all(&mut self.container)?;

        Ok((self.container, self.registry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ioc_container::IocContainer;

    // Test modules
    struct CoreModule;

    impl ServiceModule for CoreModule {
        fn name(&self) -> &str {
            "Core Module"
        }

        fn description(&self) -> Option<&str> {
            Some("Core application services")
        }

        fn configure(&self, _services: &mut ServiceBindings) {
            // No services to configure in test
        }
    }

    struct AuthModule;

    impl ServiceModule for AuthModule {
        fn name(&self) -> &str {
            "Auth Module"
        }

        fn depends_on(&self) -> Vec<ModuleId> {
            vec![ModuleId::of::<CoreModule>()]
        }

        fn configure(&self, _services: &mut ServiceBindings) {
            // No services to configure in test
        }
    }

    struct ApiModule;

    impl ServiceModule for ApiModule {
        fn name(&self) -> &str {
            "API Module"
        }

        fn depends_on(&self) -> Vec<ModuleId> {
            vec![ModuleId::of::<AuthModule>(), ModuleId::of::<CoreModule>()]
        }

        fn configure(&self, _services: &mut ServiceBindings) {
            // No services to configure in test
        }
    }

    #[test]
    fn test_module_registration() {
        let mut registry = ModuleRegistry::new();

        registry.register_module(CoreModule, None).unwrap();
        registry.register_module(AuthModule, None).unwrap();

        assert_eq!(registry.modules.len(), 2);
        assert_eq!(registry.loaded_modules.len(), 2);
    }

    #[test]
    fn test_dependency_ordering() {
        let mut registry = ModuleRegistry::new();

        registry.register_module(ApiModule, None).unwrap();
        registry.register_module(CoreModule, None).unwrap();
        registry.register_module(AuthModule, None).unwrap();

        let order = registry.calculate_load_order().unwrap();
        let order_names: Vec<String> = order.iter().map(|id| id.name().to_string()).collect();

        // CoreModule should be first (no dependencies)
        // AuthModule should be second (depends on Core)
        // ApiModule should be last (depends on both)
        assert_eq!(order_names[0], std::any::type_name::<CoreModule>());
        assert_eq!(order_names[1], std::any::type_name::<AuthModule>());
        assert_eq!(order_names[2], std::any::type_name::<ApiModule>());
    }

    #[test]
    fn test_circular_dependency_detection() {
        struct Module1;
        struct Module2;

        impl ServiceModule for Module1 {
            fn depends_on(&self) -> Vec<ModuleId> {
                vec![ModuleId::of::<Module2>()]
            }

            fn configure(&self, _services: &mut ServiceBindings) {}
        }

        impl ServiceModule for Module2 {
            fn depends_on(&self) -> Vec<ModuleId> {
                vec![ModuleId::of::<Module1>()] // Circular dependency
            }

            fn configure(&self, _services: &mut ServiceBindings) {}
        }

        let mut registry = ModuleRegistry::new();
        registry.register_module(Module1, None).unwrap();
        registry.register_module(Module2, None).unwrap();

        let result = registry.calculate_load_order();
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_dependency_validation() {
        struct ModuleWithMissingDep;

        impl ServiceModule for ModuleWithMissingDep {
            fn depends_on(&self) -> Vec<ModuleId> {
                vec![ModuleId::named("NonExistentModule")]
            }

            fn configure(&self, _services: &mut ServiceBindings) {}
        }

        let mut registry = ModuleRegistry::new();
        registry
            .register_module(ModuleWithMissingDep, None)
            .unwrap();

        let result = registry.calculate_load_order();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_modular_container_builder() {
        let container = IocContainer::new();
        let builder = ModularContainerBuilder::new(container);

        let result = builder
            .add_module(CoreModule)
            .unwrap()
            .add_module(AuthModule)
            .unwrap()
            .add_module(ApiModule)
            .unwrap()
            .build();

        assert!(result.is_ok());
        let (_container, registry) = result.unwrap();

        // Verify load order was calculated
        assert_eq!(registry.get_load_order().len(), 3);
    }

    #[test]
    fn test_module_service_configuration() {
        use crate::container::ioc_builder::IocContainerBuilder;

        // Test module that actually registers services
        struct TestModule;

        #[derive(Default)]
        struct TestService {
            #[allow(dead_code)]
            pub name: String,
        }

        impl ServiceModule for TestModule {
            fn configure(&self, services: &mut ServiceBindings) {
                services.bind::<TestService, TestService>();
            }
        }

        let mut registry = ModuleRegistry::new();
        registry.register_module(TestModule, None).unwrap();

        let mut container_builder = IocContainerBuilder::new();

        // Configure modules with the builder
        registry.configure_all(&mut container_builder).unwrap();

        // Build the container
        let container = container_builder.build().unwrap();

        // Verify the service was registered and can be resolved
        let service = container.resolve::<TestService>();
        assert!(service.is_ok());
    }
}
