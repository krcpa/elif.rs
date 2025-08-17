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
                let any_ref = instance.as_ref();
                let service_ref = any_ref.downcast_ref::<T>()?;
                Some(Arc::new(service_ref.clone()))
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