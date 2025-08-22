//! Controller factory implementation for IoC container integration
//! 
//! Provides automatic controller instantiation with dependency injection
//! using the new IoC container from elif-core.

use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;

use crate::controller::base::ElifController;
use crate::request::ElifRequest;
use crate::errors::HttpError;
use elif_core::container::{IocContainer, ScopeId};

/// Trait for creating controller instances from the IoC container
#[async_trait]
pub trait ControllerFactory: Send + Sync {
    /// Create a controller instance with dependencies resolved from the container
    async fn create_controller(
        &self,
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<Arc<dyn ElifController>, HttpError>;
}

/// Generic controller factory that uses the from_ioc_container method
pub struct IocControllerFactory<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> IocControllerFactory<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Default for IocControllerFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> ControllerFactory for IocControllerFactory<T>
where
    T: ElifController + IocControllable + 'static,
{
    async fn create_controller(
        &self,
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<Arc<dyn ElifController>, HttpError> {
        let controller = T::from_ioc_container(container, scope)
            .map_err(|e| HttpError::InternalError {
                message: format!("Failed to create controller: {}", e),
            })?;
        
        Ok(Arc::new(controller))
    }
}

/// Trait for controllers that can be created from IoC container
pub trait IocControllable {
    /// Create controller instance with dependencies resolved from IoC container
    fn from_ioc_container(
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<Self, String>
    where
        Self: Sized;
}

/// Controller registry for managing controller factories
pub struct ControllerRegistry {
    factories: HashMap<String, Box<dyn ControllerFactory>>,
    container: Arc<IocContainer>,
}

impl ControllerRegistry {
    /// Create a new controller registry with IoC container
    pub fn new(container: Arc<IocContainer>) -> Self {
        Self {
            factories: HashMap::new(),
            container,
        }
    }

    /// Register a controller type with automatic factory creation
    pub fn register<T>(&mut self, name: &str) -> Result<(), HttpError>
    where
        T: ElifController + IocControllable + 'static,
    {
        let factory = Box::new(IocControllerFactory::<T>::new());
        self.factories.insert(name.to_string(), factory);
        Ok(())
    }

    /// Register a custom factory for a controller
    pub fn register_factory(
        &mut self,
        name: &str,
        factory: Box<dyn ControllerFactory>,
    ) {
        self.factories.insert(name.to_string(), factory);
    }

    /// Create a controller instance by name
    pub async fn create_controller(
        &self,
        name: &str,
        scope: Option<&ScopeId>,
    ) -> Result<Arc<dyn ElifController>, HttpError> {
        let factory = self.factories.get(name)
            .ok_or_else(|| HttpError::InternalError {
                message: format!("Controller '{}' not registered", name),
            })?;

        factory.create_controller(&self.container, scope).await
    }

    /// Create a scoped controller registry for request-specific controllers
    pub async fn create_scoped_registry(
        &self,
        request: &ElifRequest,
    ) -> Result<ScopedControllerRegistry<'_>, HttpError> {
        let scope_id = self.container.create_scope()
            .map_err(|e| HttpError::InternalError {
                message: format!("Failed to create request scope: {}", e),
            })?;

        Ok(ScopedControllerRegistry {
            registry: self,
            scope_id,
            request_context: RequestContext::from_request(request),
        })
    }

    /// Get list of registered controller names
    pub fn registered_controllers(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}

/// Scoped controller registry for request-specific dependency injection
pub struct ScopedControllerRegistry<'a> {
    registry: &'a ControllerRegistry,
    scope_id: ScopeId,
    #[allow(dead_code)]
    request_context: RequestContext,
}

impl<'a> ScopedControllerRegistry<'a> {
    /// Create a controller instance within this scope
    pub async fn create_controller(
        &self,
        name: &str,
    ) -> Result<Arc<dyn ElifController>, HttpError> {
        // Register request context in scope for injection
        // TODO: Add request context to scope once request-scoped services are implemented
        
        self.registry
            .create_controller(name, Some(&self.scope_id))
            .await
    }

    /// Get the scope ID for this registry
    pub fn scope_id(&self) -> &ScopeId {
        &self.scope_id
    }

    /// Dispose the scope when done
    pub async fn dispose(self) -> Result<(), HttpError> {
        self.registry.container
            .dispose_scope(&self.scope_id)
            .await
            .map_err(|e| HttpError::InternalError {
                message: format!("Failed to dispose scope: {}", e),
            })
    }
}

/// Request context for dependency injection
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
}

impl RequestContext {
    /// Create request context from ElifRequest
    pub fn from_request(request: &ElifRequest) -> Self {
        Self {
            method: request.method.to_string(),
            path: request.path().to_string(),
            query_params: HashMap::new(), // Simplified for now
            headers: HashMap::new(), // Simplified for now
        }
    }
}

/// Builder for controller registry
pub struct ControllerRegistryBuilder {
    container: Option<Arc<IocContainer>>,
    controllers: Vec<(String, Box<dyn ControllerFactory>)>,
}

impl ControllerRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            container: None,
            controllers: Vec::new(),
        }
    }

    /// Set the IoC container
    pub fn container(mut self, container: Arc<IocContainer>) -> Self {
        self.container = Some(container);
        self
    }

    /// Register a controller type
    pub fn register<T>(mut self, name: &str) -> Self
    where
        T: ElifController + IocControllable + 'static,
    {
        let factory = Box::new(IocControllerFactory::<T>::new());
        self.controllers.push((name.to_string(), factory));
        self
    }

    /// Register a custom factory
    pub fn register_factory(
        mut self,
        name: &str,
        factory: Box<dyn ControllerFactory>,
    ) -> Self {
        self.controllers.push((name.to_string(), factory));
        self
    }

    /// Build the controller registry
    pub fn build(self) -> Result<ControllerRegistry, HttpError> {
        let container = self.container
            .ok_or_else(|| HttpError::InternalError {
                message: "IoC container is required for controller registry".to_string(),
            })?;

        let mut registry = ControllerRegistry::new(container);
        
        for (name, factory) in self.controllers {
            registry.register_factory(&name, factory);
        }

        Ok(registry)
    }
}

impl Default for ControllerRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Auto-discovery helper for controllers
pub struct ControllerScanner;

impl ControllerScanner {
    /// Scan for controllers in a module and register them automatically
    /// This would typically be implemented as a proc macro in a full implementation
    pub async fn scan_and_register(
        _registry: &mut ControllerRegistry,
        _module_path: &str,
    ) -> Result<usize, HttpError> {
        // Placeholder implementation - in a real system this would use
        // reflection or proc macros to discover controller types
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::base::{ElifController, ControllerRoute};
    use crate::request::method::HttpMethod;
    use async_trait::async_trait;

    // Test controller for factory tests
    pub struct TestController {
        pub service: Arc<TestService>,
    }

    #[async_trait]
    impl ElifController for TestController {
        fn name(&self) -> &str {
            "TestController"
        }

        fn base_path(&self) -> &str {
            "/test"
        }

        fn routes(&self) -> Vec<ControllerRoute> {
            vec![]
        }

        async fn handle_request(
            self: Arc<Self>,
            _method_name: String,
            _request: ElifRequest,
        ) -> crate::errors::HttpResult<crate::response::ElifResponse> {
            Ok(crate::response::ElifResponse::ok().text("test"))
        }
    }

    impl IocControllable for TestController {
        fn from_ioc_container(
            container: &IocContainer,
            _scope: Option<&ScopeId>,
        ) -> Result<Self, String> {
            let service = container.resolve::<TestService>()
                .map_err(|e| format!("Failed to resolve TestService: {}", e))?;
            
            Ok(Self { service })
        }
    }

    // Test service
    #[derive(Default)]
    pub struct TestService {
        pub name: String,
    }

    unsafe impl Send for TestService {}
    unsafe impl Sync for TestService {}

    #[tokio::test]
    async fn test_controller_factory_creation() {
        let mut container = IocContainer::new();
        container.bind::<TestService, TestService>();
        container.build().expect("Container build failed");

        let container_arc = Arc::new(container);
        let mut registry = ControllerRegistry::new(container_arc);

        registry.register::<TestController>("test")
            .expect("Failed to register controller");

        let controller = registry.create_controller("test", None)
            .await
            .expect("Failed to create controller");

        assert_eq!(controller.name(), "TestController");
    }
}