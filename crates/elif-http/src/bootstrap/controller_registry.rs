//! Controller Type Registry for runtime controller resolution
//!
//! This module implements a compile-time controller type registry that enables
//! runtime controller instantiation from string names, overcoming Rust's type
//! system limitations with trait object resolution.
//!
//! ## Architecture
//!
//! - `ControllerTypeRegistry`: Global registry of controller factory functions
//! - `ControllerFactory`: Type-erased factory function for controller creation
//! - `CONTROLLER_TYPE_REGISTRY`: Thread-safe global instance
//!
//! ## Usage
//!
//! Controllers are automatically registered via the `#[controller]` macro:
//!
//! ```rust
//! #[controller("/api/users")]
//! pub struct UserController;
//! 
//! // Macro generates registration call:
//! // CONTROLLER_TYPE_REGISTRY.register("UserController", || Box::new(UserController::new()));
//! ```
//!
//! The registry then enables runtime resolution:
//!
//! ```rust
//! let controller = CONTROLLER_TYPE_REGISTRY.create_controller("UserController")?;
//! ```

use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use crate::controller::ElifController;
use crate::bootstrap::BootstrapError;

/// Type-erased factory function for creating controller instances
pub type ControllerFactory = fn() -> Box<dyn ElifController>;

/// Thread-safe controller type registry for runtime resolution
#[derive(Debug)]
pub struct ControllerTypeRegistry {
    /// Map of controller type name to factory function
    factories: RwLock<HashMap<String, ControllerFactory>>,
}

impl ControllerTypeRegistry {
    /// Create a new empty controller type registry
    fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    /// Register a controller type with its factory function
    ///
    /// This is typically called by the `#[controller]` macro during compilation
    /// to register controller types for runtime resolution.
    ///
    /// # Arguments
    /// * `name` - Controller type name (e.g., "UserController")
    /// * `factory` - Factory function that creates controller instances
    ///
    /// # Panics
    /// Panics if a controller with the same name is already registered
    pub fn register(&self, name: &str, factory: ControllerFactory) {
        let mut factories = self.factories.write()
            .expect("Controller type registry lock poisoned");
        
        if factories.contains_key(name) {
            panic!(
                "Controller type '{}' is already registered. Each controller type must have a unique name.",
                name
            );
        }
        
        factories.insert(name.to_string(), factory);
        tracing::debug!("Registered controller type: {}", name);
    }

    /// Create a new controller instance by type name
    ///
    /// # Arguments
    /// * `name` - Controller type name to instantiate
    ///
    /// # Returns
    /// * `Ok(Box<dyn ElifController>)` - New controller instance
    /// * `Err(BootstrapError)` - If controller type is not registered
    pub fn create_controller(&self, name: &str) -> Result<Box<dyn ElifController>, BootstrapError> {
        let factories = self.factories.read()
            .expect("Controller type registry lock poisoned");
        
        let factory = factories.get(name)
            .ok_or_else(|| BootstrapError::ControllerNotFound {
                controller_name: name.to_string(),
                available_controllers: factories.keys().cloned().collect(),
            })?;
        
        let controller = factory();
        tracing::debug!("Created controller instance: {}", name);
        Ok(controller)
    }

    /// Check if a controller type is registered
    pub fn is_registered(&self, name: &str) -> bool {
        let factories = self.factories.read()
            .expect("Controller type registry lock poisoned");
        factories.contains_key(name)
    }

    /// Get all registered controller type names
    pub fn get_registered_types(&self) -> Vec<String> {
        let factories = self.factories.read()
            .expect("Controller type registry lock poisoned");
        factories.keys().cloned().collect()
    }

    /// Get the total number of registered controller types
    pub fn count(&self) -> usize {
        let factories = self.factories.read()
            .expect("Controller type registry lock poisoned");
        factories.len()
    }

    /// Clear all registered controller types
    ///
    /// This is primarily useful for testing purposes
    #[cfg(test)]
    pub fn clear(&self) {
        let mut factories = self.factories.write()
            .expect("Controller type registry lock poisoned");
        factories.clear();
        tracing::debug!("Cleared all registered controller types");
    }
}

/// Global controller type registry instance
///
/// This is automatically populated by the `#[controller]` macro during compilation
/// and used by the controller auto-registration system at runtime.
pub static CONTROLLER_TYPE_REGISTRY: Lazy<ControllerTypeRegistry> = Lazy::new(ControllerTypeRegistry::new);

/// Convenience function to register a controller type
///
/// This is used by the `#[controller]` macro to register controller types.
///
/// # Arguments
/// * `name` - Controller type name
/// * `factory` - Factory function that creates controller instances
pub fn register_controller_type(name: &str, factory: ControllerFactory) {
    CONTROLLER_TYPE_REGISTRY.register(name, factory);
}

/// Convenience function to create a controller instance
///
/// # Arguments
/// * `name` - Controller type name to instantiate
///
/// # Returns
/// * `Ok(Box<dyn ElifController>)` - New controller instance
/// * `Err(BootstrapError)` - If controller type is not registered
pub fn create_controller(name: &str) -> Result<Box<dyn ElifController>, BootstrapError> {
    CONTROLLER_TYPE_REGISTRY.create_controller(name)
}

/// Create a controller instance using Default trait
///
/// This function requires controllers to implement Default for auto-registration.
/// Controllers with dependencies should implement IocControllable instead.
pub fn create_controller_instance<T>() -> Box<dyn ElifController>
where
    T: ElifController + Default + 'static,
{
    Box::new(T::default()) as Box<dyn ElifController>
}

/// Create an IoC controller instance using IocControllable trait
///
/// This function attempts to create a controller with dependencies using
/// a minimal IoC container or Default implementations.
pub fn create_ioc_controller_instance<T>() -> Box<dyn ElifController>
where
    T: ElifController + crate::controller::factory::IocControllable + 'static,
{
    // Create a minimal IoC container for dependency resolution
    let container = elif_core::container::IocContainer::new();
    
    // Try to create the controller using the IocControllable trait
    match T::from_ioc_container(&container, None) {
        Ok(controller) => Box::new(controller) as Box<dyn ElifController>,
        Err(e) => {
            // Log the error and provide a helpful panic message
            tracing::error!("Failed to create IoC controller: {}", e);
            panic!(
                "Auto-registration failed for controller {}. Error: {}\n\
                Consider:\n\
                1. Implementing Default for all dependencies\n\
                2. Using router.controller_from_container::<T>() with proper IoC setup\n\
                3. Registering dependencies in IoC container before controller registration",
                std::any::type_name::<T>(),
                e
            );
        }
    }
}

/// Helper macro for auto-registering controllers
///
/// This macro is used by the #[controller] derive macro to automatically
/// register controller types at static initialization time using ctor.
/// It handles both simple controllers (with parameterless new()) and 
/// IoC-enabled controllers (implementing IocControllable).
#[macro_export]
macro_rules! __controller_auto_register {
    ($name:expr, $type:ty) => {
        // Use ctor to run registration at static initialization time
        // This ensures controllers are registered before main() runs
        #[::ctor::ctor]
        fn __register_controller() {
            $crate::bootstrap::controller_registry::register_controller_type(
                $name,
                || {
                    // Try to create the controller instance
                    // This requires the controller to implement Default
                    $crate::bootstrap::controller_registry::create_controller_instance::<$type>()
                }
            );
        }
    };
}

/// Helper macro for auto-registering IoC controllers
///
/// This macro is used for controllers that have dependencies but can be created
/// using Default implementations of those dependencies.
#[macro_export]
macro_rules! __controller_auto_register_ioc {
    ($name:expr, $type:ty) => {
        // Use ctor to run registration at static initialization time
        // This ensures controllers are registered before main() runs
        #[::ctor::ctor]
        fn __register_controller() {
            $crate::bootstrap::controller_registry::register_controller_type(
                $name,
                || {
                    // Try to create the controller instance using IoC container pattern
                    // This will use the IocControllable trait implementation
                    $crate::bootstrap::controller_registry::create_ioc_controller_instance::<$type>()
                }
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::{ElifController, ControllerRoute};
    use crate::{ElifRequest, ElifResponse, HttpResult};
    use crate::routing::HttpMethod;
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock controller for testing
    #[derive(Debug)]
    struct TestController {
        name: String,
    }

    impl TestController {
        fn new() -> Self {
            Self {
                name: "TestController".to_string(),
            }
        }
    }

    #[async_trait]
    impl ElifController for TestController {
        fn name(&self) -> &str {
            &self.name
        }

        fn base_path(&self) -> &str {
            "/test"
        }

        fn routes(&self) -> Vec<ControllerRoute> {
            vec![
                ControllerRoute {
                    method: HttpMethod::GET,
                    path: "".to_string(),
                    handler_name: "index".to_string(),
                    middleware: vec![],
                    params: vec![],
                }
            ]
        }

        fn dependencies(&self) -> Vec<String> {
            vec![]
        }

        async fn handle_request(
            self: Arc<Self>,
            _method_name: String,
            _request: ElifRequest,
        ) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok())
        }
    }

    #[test]
    fn test_controller_type_registry_creation() {
        let registry = ControllerTypeRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.get_registered_types().is_empty());
    }

    #[test]
    fn test_controller_registration() {
        let registry = ControllerTypeRegistry::new();
        
        // Register test controller
        registry.register("TestController", || Box::new(TestController::new()));
        
        assert_eq!(registry.count(), 1);
        assert!(registry.is_registered("TestController"));
        assert!(!registry.is_registered("NonExistentController"));
        
        let types = registry.get_registered_types();
        assert_eq!(types.len(), 1);
        assert!(types.contains(&"TestController".to_string()));
    }

    #[test]
    fn test_controller_creation() {
        let registry = ControllerTypeRegistry::new();
        
        // Register test controller
        registry.register("TestController", || Box::new(TestController::new()));
        
        // Create controller instance
        let result = registry.create_controller("TestController");
        assert!(result.is_ok());
        
        let controller = result.unwrap();
        assert_eq!(controller.name(), "TestController");
        assert_eq!(controller.base_path(), "/test");
        assert_eq!(controller.routes().len(), 1);
    }

    #[test]
    fn test_controller_not_found() {
        let registry = ControllerTypeRegistry::new();
        
        let result = registry.create_controller("NonExistentController");
        assert!(result.is_err());
        
        if let Err(BootstrapError::ControllerNotFound { controller_name, available_controllers }) = result {
            assert_eq!(controller_name, "NonExistentController");
            assert_eq!(available_controllers.len(), 0);
        } else {
            panic!("Expected ControllerNotFound error");
        }
    }

    #[test]
    #[should_panic(expected = "Controller type 'TestController' is already registered")]
    fn test_duplicate_registration() {
        let registry = ControllerTypeRegistry::new();
        
        // Register the same controller twice
        registry.register("TestController", || Box::new(TestController::new()));
        registry.register("TestController", || Box::new(TestController::new()));
    }

    #[test]
    fn test_global_registry_functions() {
        // Clear any existing registrations from other tests
        #[cfg(test)]
        CONTROLLER_TYPE_REGISTRY.clear();
        
        // Test registration via convenience function
        register_controller_type("GlobalTestController", || Box::new(TestController::new()));
        
        assert!(CONTROLLER_TYPE_REGISTRY.is_registered("GlobalTestController"));
        assert_eq!(CONTROLLER_TYPE_REGISTRY.count(), 1);
        
        // Test creation via convenience function
        let result = create_controller("GlobalTestController");
        assert!(result.is_ok());
        
        let controller = result.unwrap();
        assert_eq!(controller.name(), "TestController"); // TestController returns "TestController" as name
    }
}