use crate::foundation::traits::{Service, FrameworkComponent, Initializable};
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Service entry in the registry
pub enum ServiceEntry {
    /// Single instance (singleton)
    Instance(Arc<dyn Any + Send + Sync>),
    /// Factory function for creating instances
    Factory(Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>),
}

impl std::fmt::Debug for ServiceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceEntry::Instance(_) => f.debug_tuple("Instance").field(&"<instance>").finish(),
            ServiceEntry::Factory(_) => f.debug_tuple("Factory").field(&"<factory>").finish(),
        }
    }
}

/// Registry for managing service instances and factories
#[derive(Debug)]
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<TypeId, ServiceEntry>>>,
    scopes: Arc<RwLock<HashMap<TypeId, ServiceScope>>>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            scopes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a service instance
    pub fn register_service<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + 'static,
    {
        let type_id = TypeId::of::<T>();
        let arc_service = Arc::new(service);
        
        let mut services = self.services.write().map_err(|_| CoreError::LockError {
            resource: "service_registry".to_string(),
        })?;
        
        let mut scopes = self.scopes.write().map_err(|_| CoreError::LockError {
            resource: "service_scopes".to_string(),
        })?;
        
        services.insert(type_id, ServiceEntry::Instance(arc_service));
        scopes.insert(type_id, ServiceScope::Singleton);
        
        Ok(())
    }
    
    /// Register a singleton service
    pub fn register_singleton<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + 'static,
    {
        let type_id = TypeId::of::<T>();
        let arc_service = Arc::new(service);
        
        let mut services = self.services.write().map_err(|_| CoreError::LockError {
            resource: "service_registry".to_string(),
        })?;
        
        let mut scopes = self.scopes.write().map_err(|_| CoreError::LockError {
            resource: "service_scopes".to_string(),
        })?;
        
        services.insert(type_id, ServiceEntry::Instance(arc_service));
        scopes.insert(type_id, ServiceScope::Singleton);
        
        Ok(())
    }
    
    /// Register a transient service factory
    pub fn register_transient<T>(&mut self, factory: Box<dyn Fn() -> T + Send + Sync>) -> Result<(), CoreError>
    where
        T: Service + 'static,
    {
        let type_id = TypeId::of::<T>();
        
        let wrapped_factory: Box<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync> = 
            Box::new(move || -> Box<dyn Any + Send + Sync> {
                Box::new(factory())
            });
        
        let mut services = self.services.write().map_err(|_| CoreError::LockError {
            resource: "service_registry".to_string(),
        })?;
        
        let mut scopes = self.scopes.write().map_err(|_| CoreError::LockError {
            resource: "service_scopes".to_string(),
        })?;
        
        services.insert(type_id, ServiceEntry::Factory(wrapped_factory));
        scopes.insert(type_id, ServiceScope::Transient);
        
        Ok(())
    }
    
    /// Resolve a service instance
    pub fn resolve<T>(&self) -> Result<Arc<T>, CoreError>
    where
        T: Service + Clone + 'static,
    {
        self.try_resolve::<T>()
            .ok_or_else(|| CoreError::ServiceNotFound {
                service_type: std::any::type_name::<T>().to_string(),
            })
    }
    
    /// Try to resolve a service instance
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Service + Clone + 'static,
    {
        let type_id = TypeId::of::<T>();
        let services = self.services.read().ok()?;
        
        match services.get(&type_id)? {
            ServiceEntry::Instance(instance) => {
                // Clone the Arc itself, not the service - this maintains singleton behavior
                instance.clone().downcast::<T>().ok()
            }
            ServiceEntry::Factory(factory) => {
                let instance = factory();
                let boxed = instance.downcast::<T>().ok()?;
                Some(Arc::new(*boxed))
            }
        }
    }
    
    /// Check if a service type is registered
    pub fn contains<T>(&self) -> bool
    where
        T: Service + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.services.read()
            .map(|services| services.contains_key(&type_id))
            .unwrap_or(false)
    }
    
    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.services.read()
            .map(|services| services.len())
            .unwrap_or(0)
    }
    
    /// Get all registered service type IDs
    pub fn registered_services(&self) -> Vec<TypeId> {
        self.services.read()
            .map(|services| services.keys().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Validate all registered services
    pub fn validate(&self) -> Result<(), CoreError> {
        // For now, just check that we can acquire read locks
        let _services = self.services.read().map_err(|_| CoreError::LockError {
            resource: "service_registry".to_string(),
        })?;
        
        let _scopes = self.scopes.read().map_err(|_| CoreError::LockError {
            resource: "service_scopes".to_string(),
        })?;
        
        Ok(())
    }
    
    /// Initialize all services that implement Initializable
    pub async fn initialize_all(&self) -> Result<(), CoreError> {
        // This is a simplified version - in a real implementation,
        // we would need to handle service dependencies and initialization order
        self.validate()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::traits::Service;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[derive(Debug, Clone)]
    struct TestService {
        id: usize,
        counter: Arc<AtomicUsize>,
    }
    
    impl TestService {
        fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            Self {
                id: COUNTER.fetch_add(1, Ordering::SeqCst),
                counter: Arc::new(AtomicUsize::new(0)),
            }
        }
        
        fn increment(&self) -> usize {
            self.counter.fetch_add(1, Ordering::SeqCst) + 1
        }
        
        fn get_count(&self) -> usize {
            self.counter.load(Ordering::SeqCst)
        }
    }
    
    impl crate::foundation::traits::FrameworkComponent for TestService {}
    
    impl Service for TestService {}
    
    #[test]
    fn test_singleton_behavior() {
        let mut registry = ServiceRegistry::new();
        let service = TestService::new();
        let original_id = service.id;
        
        // Register as singleton
        registry.register_singleton(service).unwrap();
        
        // Resolve multiple times
        let instance1 = registry.resolve::<TestService>().unwrap();
        let instance2 = registry.resolve::<TestService>().unwrap();
        let instance3 = registry.resolve::<TestService>().unwrap();
        
        // All instances should have the same ID (same original service)
        assert_eq!(instance1.id, original_id);
        assert_eq!(instance2.id, original_id);
        assert_eq!(instance3.id, original_id);
        
        // Increment counter on one instance
        let count1 = instance1.increment();
        assert_eq!(count1, 1);
        
        // Other instances should see the same count (shared state)
        assert_eq!(instance2.get_count(), 1);
        assert_eq!(instance3.get_count(), 1);
        
        // Increment on another instance
        let count2 = instance2.increment();
        assert_eq!(count2, 2);
        
        // All should see the updated count
        assert_eq!(instance1.get_count(), 2);
        assert_eq!(instance3.get_count(), 2);
    }
    
    #[test]
    fn test_singleton_arc_sharing() {
        let mut registry = ServiceRegistry::new();
        let service = TestService::new();
        
        registry.register_singleton(service).unwrap();
        
        // Resolve multiple times
        let instance1 = registry.resolve::<TestService>().unwrap();
        let instance2 = registry.resolve::<TestService>().unwrap();
        
        // The Arc pointers should be the same (true singleton)
        assert!(Arc::ptr_eq(&instance1, &instance2));
    }
    
    #[test]
    fn test_transient_behavior() {
        let mut registry = ServiceRegistry::new();
        
        // Register transient factory
        registry.register_transient::<TestService>(Box::new(|| TestService::new())).unwrap();
        
        // Resolve multiple times
        let instance1 = registry.resolve::<TestService>().unwrap();
        let instance2 = registry.resolve::<TestService>().unwrap();
        
        // Each instance should have different IDs (new instances)
        assert_ne!(instance1.id, instance2.id);
        
        // The Arc pointers should be different (different instances)
        assert!(!Arc::ptr_eq(&instance1, &instance2));
        
        // Increment on one should not affect the other
        instance1.increment();
        assert_eq!(instance1.get_count(), 1);
        assert_eq!(instance2.get_count(), 0);
    }
    
    #[test]
    fn test_service_registry_operations() {
        let mut registry = ServiceRegistry::new();
        let service = TestService::new();
        
        // Initially empty
        assert!(!registry.contains::<TestService>());
        assert_eq!(registry.service_count(), 0);
        
        // Register service
        registry.register_singleton(service).unwrap();
        
        // Should now contain the service
        assert!(registry.contains::<TestService>());
        assert_eq!(registry.service_count(), 1);
        
        // Should be able to resolve
        let resolved = registry.resolve::<TestService>().unwrap();
        assert_eq!(resolved.service_id(), "elif_core::container::registry::tests::TestService");
    }
}