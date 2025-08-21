use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::container::binding::{ServiceBinder, ServiceBindings};
use crate::container::descriptor::ServiceId;
use crate::container::resolver::DependencyResolver as GraphDependencyResolver;
use crate::container::autowiring::{DependencyResolver, Injectable};
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;

/// Service instance storage
#[derive(Debug)]
enum ServiceInstance {
    /// Singleton instance
    Singleton(Arc<dyn Any + Send + Sync>),
}

/// Modern IoC container with proper dependency injection
#[derive(Debug)]
pub struct IocContainer {
    /// Service bindings and descriptors
    bindings: ServiceBindings,
    /// Dependency resolver
    resolver: Option<GraphDependencyResolver>,
    /// Instantiated services
    instances: Arc<RwLock<HashMap<ServiceId, ServiceInstance>>>,
    /// Whether the container is built and ready
    is_built: bool,
}

impl IocContainer {
    /// Create a new IoC container
    pub fn new() -> Self {
        Self {
            bindings: ServiceBindings::new(),
            resolver: None,
            instances: Arc::new(RwLock::new(HashMap::new())),
            is_built: false,
        }
    }
    
    /// Create IoC container from existing bindings
    pub fn from_bindings(bindings: ServiceBindings) -> Self {
        Self {
            bindings,
            resolver: None,
            instances: Arc::new(RwLock::new(HashMap::new())),
            is_built: false,
        }
    }
    
    /// Build the container and prepare for service resolution
    pub fn build(&mut self) -> Result<(), CoreError> {
        if self.is_built {
            return Ok(());
        }
        
        // Build dependency resolver
        let resolver = GraphDependencyResolver::new(self.bindings.descriptors())?;
        self.resolver = Some(resolver);
        
        // Validate dependencies
        let service_ids = self.bindings.service_ids().into_iter().collect();
        if let Some(resolver) = &self.resolver {
            resolver.validate_dependencies(&service_ids)?;
        }
        
        self.is_built = true;
        Ok(())
    }
    
    /// Resolve a service by type
    pub fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        let service_id = ServiceId::of::<T>();
        self.resolve_by_id(&service_id)
    }
    
    /// Resolve a named service
    pub fn resolve_named<T: Send + Sync + 'static>(&self, name: impl Into<String>) -> Result<Arc<T>, CoreError> {
        let service_id = ServiceId::named::<T>(name);
        self.resolve_by_id(&service_id)
    }
    
    /// Resolve a service by service ID
    fn resolve_by_id<T: Send + Sync + 'static>(&self, service_id: &ServiceId) -> Result<Arc<T>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        // Check if we have a cached instance
        {
            let instances = self.instances.read().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            
            if let Some(ServiceInstance::Singleton(instance)) = instances.get(service_id) {
                return instance.clone().downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            service_id.name.as_deref().unwrap_or("default")
                        ),
                    });
            }
        }
        
        // Get service descriptor
        let descriptor = self.bindings.get_descriptor(service_id)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: format!("{}({})", 
                    std::any::type_name::<T>(),
                    service_id.name.as_deref().unwrap_or("default")
                ),
            })?;
        
        // Resolve dependencies first
        self.resolve_dependencies(&descriptor.dependencies)?;
        
        // Create the service instance based on activation strategy
        let arc_instance = match &descriptor.activation_strategy {
            crate::container::descriptor::ServiceActivationStrategy::Factory(factory) => {
                let instance = factory()?;
                let typed_instance = instance.downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: format!("{}({})", 
                            std::any::type_name::<T>(),
                            service_id.name.as_deref().unwrap_or("default")
                        ),
                    })?;
                Arc::new(*typed_instance)
            },
            crate::container::descriptor::ServiceActivationStrategy::AutoWired => {
                return Err(CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Service {} is marked as auto-wired but resolve_by_id was called instead of resolve_injectable. Use resolve_injectable() for auto-wired services.",
                        std::any::type_name::<T>()
                    ),
                });
            }
        };
        
        // Cache if singleton
        if descriptor.lifetime == ServiceScope::Singleton {
            let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            instances.insert(service_id.clone(), ServiceInstance::Singleton(arc_instance.clone()));
        }
        
        Ok(arc_instance)
    }
    
    /// Resolve all dependencies for a service
    fn resolve_dependencies(&self, dependencies: &[ServiceId]) -> Result<(), CoreError> {
        for dep_id in dependencies {
            // For now, we'll just validate that the dependency exists
            if !self.bindings.contains(dep_id) {
                return Err(CoreError::ServiceNotFound {
                    service_type: format!("{}({})", 
                        dep_id.type_name(),
                        dep_id.name.as_deref().unwrap_or("default")
                    ),
                });
            }
        }
        Ok(())
    }
    
    /// Try to resolve a service, returning None if not found
    pub fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.resolve::<T>().ok()
    }
    
    /// Try to resolve a named service, returning None if not found
    pub fn try_resolve_named<T: Send + Sync + 'static>(&self, name: impl Into<String>) -> Option<Arc<T>> {
        self.resolve_named::<T>(name).ok()
    }
    
    /// Resolve a service using the Injectable trait (auto-wiring)
    pub fn resolve_injectable<T: Injectable>(&self) -> Result<Arc<T>, CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before resolving services".to_string(),
            });
        }
        
        let service_id = ServiceId::of::<T>();
        
        // Check if we have a cached instance
        {
            let instances = self.instances.read().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            
            if let Some(ServiceInstance::Singleton(instance)) = instances.get(&service_id) {
                return instance.clone().downcast::<T>()
                    .map_err(|_| CoreError::ServiceNotFound {
                        service_type: std::any::type_name::<T>().to_string(),
                    });
            }
        }
        
        // Verify the service is configured for auto-wiring
        let descriptor = self.bindings.get_descriptor(&service_id)
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: std::any::type_name::<T>().to_string(),
            })?;
            
        let arc_instance = match &descriptor.activation_strategy {
            crate::container::descriptor::ServiceActivationStrategy::AutoWired => {
                // Create the service using Injectable
                let service_instance = T::create(self)?;
                Arc::new(service_instance)
            },
            crate::container::descriptor::ServiceActivationStrategy::Factory(_) => {
                return Err(CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Service {} is configured with a factory but resolve_injectable was called. Use resolve() for factory-based services.",
                        std::any::type_name::<T>()
                    ),
                });
            }
        };
        
        // Cache if singleton
        if descriptor.lifetime == ServiceScope::Singleton {
            let mut instances = self.instances.write().map_err(|_| CoreError::LockError {
                resource: "service_instances".to_string(),
            })?;
            instances.insert(service_id, ServiceInstance::Singleton(arc_instance.clone()));
        }
        
        Ok(arc_instance)
    }
    
    /// Resolve a trait object by downcasting from a concrete implementation
    pub fn resolve_trait<T: ?Sized + Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        // For trait objects, we need special handling
        // This is a placeholder - in a real implementation, we'd need metadata about
        // which concrete type implements which trait
        Err(CoreError::ServiceNotFound {
            service_type: std::any::type_name::<T>().to_string(),
        })
    }
    
    /// Check if a service is registered
    pub fn contains<T: 'static>(&self) -> bool {
        let service_id = ServiceId::of::<T>();
        self.bindings.contains(&service_id)
    }
    
    /// Check if a named service is registered
    pub fn contains_named<T: 'static>(&self, name: impl Into<String>) -> bool {
        let service_id = ServiceId::named::<T>(name);
        self.bindings.contains(&service_id)
    }
    
    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.bindings.count()
    }
    
    /// Get all registered service IDs
    pub fn registered_services(&self) -> Vec<ServiceId> {
        self.bindings.service_ids()
    }
    
    /// Validate the container configuration
    pub fn validate(&self) -> Result<(), CoreError> {
        if !self.is_built {
            return Err(CoreError::InvalidServiceDescriptor {
                message: "Container must be built before validation".to_string(),
            });
        }
        
        // Validate dependency resolution
        if let Some(resolver) = &self.resolver {
            let service_ids = self.bindings.service_ids().into_iter().collect();
            resolver.validate_dependencies(&service_ids)?;
        }
        
        Ok(())
    }
}

impl ServiceBinder for IocContainer {
    fn bind<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind::<TInterface, TImpl>();
        self
    }
    
    fn bind_singleton<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_singleton::<TInterface, TImpl>();
        self
    }
    
    fn bind_transient<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_transient::<TInterface, TImpl>();
        self
    }
    
    fn bind_factory<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_factory::<TInterface, _, _>(factory);
        self
    }
    
    fn bind_instance<TInterface: ?Sized + 'static, TImpl: Send + Sync + Clone + 'static>(&mut self, instance: TImpl) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_instance::<TInterface, TImpl>(instance);
        self
    }
    
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: impl Into<String>) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_named::<TInterface, TImpl>(name);
        self
    }
    
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_injectable::<T>();
        self
    }
    
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self {
        if self.is_built {
            panic!("Cannot add bindings after container is built");
        }
        self.bindings.bind_injectable_singleton::<T>();
        self
    }
}

impl Default for IocContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver for IocContainer {
    fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
        self.resolve::<T>()
    }
    
    fn resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Result<Arc<T>, CoreError> {
        self.resolve_named::<T>(name.to_string())
    }
    
    fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.try_resolve::<T>()
    }
    
    fn try_resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
        self.try_resolve_named::<T>(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait TestRepository: Send + Sync {
        fn find(&self, id: u32) -> Option<String>;
    }

    #[derive(Default)]
    struct PostgresRepository;
    
    unsafe impl Send for PostgresRepository {}
    unsafe impl Sync for PostgresRepository {}

    impl TestRepository for PostgresRepository {
        fn find(&self, _id: u32) -> Option<String> {
            Some("postgres_data".to_string())
        }
    }

    trait TestService: Send + Sync {
        fn get_data(&self) -> String;
    }

    #[derive(Default)]
    struct UserService;
    
    unsafe impl Send for UserService {}
    unsafe impl Sync for UserService {}

    impl TestService for UserService {
        fn get_data(&self) -> String {
            "user_data".to_string()
        }
    }

    #[test]
    fn test_basic_binding_and_resolution() {
        let mut container = IocContainer::new();
        
        container
            .bind::<PostgresRepository, PostgresRepository>()
            .bind_singleton::<UserService, UserService>();
        
        container.build().unwrap();
        
        let repo = container.resolve::<PostgresRepository>().unwrap();
        assert_eq!(repo.find(1), Some("postgres_data".to_string()));
        
        let service = container.resolve::<UserService>().unwrap();
        assert_eq!(service.get_data(), "user_data");
    }

    #[test]
    fn test_named_services() {
        let mut container = IocContainer::new();
        
        container
            .bind_named::<PostgresRepository, PostgresRepository>("postgres")
            .bind_named::<PostgresRepository, PostgresRepository>("backup");
        
        container.build().unwrap();
        
        let postgres_repo = container.resolve_named::<PostgresRepository>("postgres").unwrap();
        let backup_repo = container.resolve_named::<PostgresRepository>("backup").unwrap();
        
        assert_eq!(postgres_repo.find(1), Some("postgres_data".to_string()));
        assert_eq!(backup_repo.find(1), Some("postgres_data".to_string()));
    }

    #[test]
    fn test_singleton_behavior() {
        let mut container = IocContainer::new();
        
        container.bind_singleton::<UserService, UserService>();
        container.build().unwrap();
        
        let service1 = container.resolve::<UserService>().unwrap();
        let service2 = container.resolve::<UserService>().unwrap();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    fn test_transient_behavior() {
        let mut container = IocContainer::new();
        
        container.bind_transient::<UserService, UserService>();
        container.build().unwrap();
        
        let service1 = container.resolve::<UserService>().unwrap();
        let service2 = container.resolve::<UserService>().unwrap();
        
        // Should be different instances
        assert!(!Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    #[should_panic(expected = "Cannot add bindings after container is built")]
    fn test_cannot_bind_after_build() {
        let mut container = IocContainer::new();
        container.build().unwrap();
        
        // This should panic
        container.bind::<UserService, UserService>();
    }

    #[test]
    fn test_service_not_found() {
        let mut container = IocContainer::new();
        container.build().unwrap();
        
        let result = container.resolve::<UserService>();
        assert!(result.is_err());
        
        if let Err(CoreError::ServiceNotFound { service_type }) = result {
            assert!(service_type.contains("UserService"));
        }
    }
}