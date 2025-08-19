use crate::foundation::traits::Service;
use crate::container::registry::ServiceRegistry;
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;
use service_builder::builder;
use std::any::TypeId;
use std::sync::Arc;

/// Main dependency injection container
#[builder]
pub struct Container {
    #[builder(getter, setter)]
    registry: ServiceRegistry,
    
    #[builder(getter, setter)]
    scope: ServiceScope,
    
    #[builder(default)]
    initialized: bool,
}

impl Container {
    /// Create a new container with default registry and scope
    pub fn new() -> Self {
        ContainerBuilder::new()
            .registry(ServiceRegistry::new())
            .scope(ServiceScope::Singleton)
            .build_with_defaults().expect("Failed to build container")
    }
    
    /// Register a service in the container
    pub fn register<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + 'static,
    {
        self.registry.register_service(service)
    }
    
    /// Register a singleton service
    pub fn register_singleton<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + 'static,
    {
        self.registry.register_singleton(service)
    }
    
    /// Register a transient service
    pub fn register_transient<T>(&mut self, factory: Box<dyn Fn() -> T + Send + Sync>) -> Result<(), CoreError>
    where
        T: Service + 'static,
    {
        self.registry.register_transient(factory)
    }
    
    /// Resolve a service from the container
    pub fn resolve<T>(&self) -> Result<Arc<T>, CoreError>
    where
        T: Service + Clone + 'static,
    {
        self.registry.resolve::<T>()
    }
    
    /// Try to resolve a service, returning None if not found
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Service + Clone + 'static,
    {
        self.registry.try_resolve::<T>()
    }
    
    /// Check if a service is registered
    pub fn contains<T>(&self) -> bool
    where
        T: Service + 'static,
    {
        self.registry.contains::<T>()
    }
    
    /// Check if the container is properly configured
    pub fn validate(&self) -> Result<(), CoreError> {
        self.registry.validate()
    }
    
    /// Initialize the container and all its services
    pub async fn initialize(&mut self) -> Result<(), CoreError> {
        if self.initialized {
            return Ok(());
        }
        
        self.registry.initialize_all().await?;
        self.initialized = true;
        Ok(())
    }
    
    /// Check if the container is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.registry.service_count()
    }
    
    /// Get a list of all registered service types
    pub fn registered_services(&self) -> Vec<TypeId> {
        self.registry.registered_services()
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("service_count", &self.service_count())
            .field("initialized", &self.initialized)
            .field("scope", &self.scope)
            .finish()
    }
}