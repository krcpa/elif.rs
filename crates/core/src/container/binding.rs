use crate::container::descriptor::{ServiceDescriptor, ServiceDescriptorFactoryBuilder, ServiceId};
use crate::container::scope::ServiceScope;
use crate::container::autowiring::Injectable;
use crate::errors::CoreError;

/// Binding API for the IoC container
pub trait ServiceBinder {
    /// Bind an interface to an implementation
    fn bind<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self;
    
    /// Bind an interface to an implementation with singleton lifetime
    fn bind_singleton<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self;
    
    /// Bind an interface to an implementation with transient lifetime
    fn bind_transient<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self;
    
    /// Bind a service using a factory function
    fn bind_factory<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static;
    
    /// Bind a pre-created instance
    fn bind_instance<TInterface: ?Sized + 'static, TImpl: Send + Sync + Clone + 'static>(&mut self, instance: TImpl) -> &mut Self;
    
    /// Bind a named service
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: impl Into<String>) -> &mut Self;
    
    /// Bind an Injectable service with auto-wiring
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self;
    
    /// Bind an Injectable service as singleton with auto-wiring  
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self;
}

/// Collection of service bindings
#[derive(Debug)]
pub struct ServiceBindings {
    descriptors: Vec<ServiceDescriptor>,
}

impl ServiceBindings {
    /// Create a new service bindings collection
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
        }
    }
    
    /// Add a service descriptor
    pub fn add_descriptor(&mut self, descriptor: ServiceDescriptor) {
        self.descriptors.push(descriptor);
    }
    
    /// Get all service descriptors
    pub fn descriptors(&self) -> &[ServiceDescriptor] {
        &self.descriptors
    }
    
    /// Get service descriptors by service ID
    pub fn get_descriptor(&self, service_id: &ServiceId) -> Option<&ServiceDescriptor> {
        self.descriptors.iter().find(|d| d.service_id == *service_id)
    }
    
    /// Get all service IDs
    pub fn service_ids(&self) -> Vec<ServiceId> {
        self.descriptors.iter().map(|d| d.service_id.clone()).collect()
    }
    
    /// Check if a service is registered
    pub fn contains(&self, service_id: &ServiceId) -> bool {
        self.descriptors.iter().any(|d| d.service_id == *service_id)
    }
    
    /// Get the number of registered services
    pub fn count(&self) -> usize {
        self.descriptors.len()
    }
}

impl Default for ServiceBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceBinder for ServiceBindings {
    fn bind<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        let descriptor = ServiceDescriptor::bind::<TInterface, TImpl>()
            .with_lifetime(ServiceScope::Transient)
            .build();
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_singleton<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        let descriptor = ServiceDescriptor::bind::<TInterface, TImpl>()
            .with_lifetime(ServiceScope::Singleton)
            .build();
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_transient<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        let descriptor = ServiceDescriptor::bind::<TInterface, TImpl>()
            .with_lifetime(ServiceScope::Transient)
            .build();
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_factory<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let descriptor = ServiceDescriptorFactoryBuilder::<TInterface>::new()
            .with_factory(factory)
            .build()
            .expect("Failed to build factory descriptor");
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_instance<TInterface: ?Sized + 'static, TImpl: Send + Sync + Clone + 'static>(&mut self, instance: TImpl) -> &mut Self {
        let descriptor = ServiceDescriptorFactoryBuilder::<TInterface>::new()
            .with_lifetime(ServiceScope::Singleton)
            .with_factory({
                let instance = instance.clone();
                move || -> Result<TImpl, CoreError> {
                    Ok(instance.clone())
                }
            })
            .build()
            .expect("Failed to build instance descriptor");
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: impl Into<String>) -> &mut Self {
        let descriptor = ServiceDescriptor::bind_named::<TInterface, TImpl>(name)
            .with_lifetime(ServiceScope::Transient)
            .build();
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self {
        use std::any::Any;
        
        let service_id = ServiceId::of::<T>();
        let dependencies = T::dependencies();
        
        let factory = Box::new(move || -> Result<Box<dyn Any + Send + Sync>, CoreError> {
            // This is a placeholder - in a real implementation, we'd need access to the container
            Err(CoreError::InvalidServiceDescriptor {
                message: "Injectable services need special handling during resolution".to_string(),
            })
        });
        
        let descriptor = ServiceDescriptor {
            service_id,
            implementation_id: std::any::TypeId::of::<T>(),
            lifetime: ServiceScope::Transient,
            factory,
            dependencies,
        };
        
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self {
        use std::any::Any;
        
        let service_id = ServiceId::of::<T>();
        let dependencies = T::dependencies();
        
        let factory = Box::new(move || -> Result<Box<dyn Any + Send + Sync>, CoreError> {
            // This is a placeholder - in a real implementation, we'd need access to the container
            Err(CoreError::InvalidServiceDescriptor {
                message: "Injectable services need special handling during resolution".to_string(),
            })
        });
        
        let descriptor = ServiceDescriptor {
            service_id,
            implementation_id: std::any::TypeId::of::<T>(),
            lifetime: ServiceScope::Singleton,
            factory,
            dependencies,
        };
        
        self.add_descriptor(descriptor);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    trait TestRepository: Send + Sync {
        fn find(&self, id: u32) -> Option<String>;
    }

    #[derive(Default)]
    struct PostgresRepository;
    
    unsafe impl Send for PostgresRepository {}
    unsafe impl Sync for PostgresRepository {}

    impl TestRepository for PostgresRepository {
        fn find(&self, _id: u32) -> Option<String> {
            Some("postgres".to_string())
        }
    }

    #[allow(dead_code)]
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
    fn test_service_bindings() {
        let mut bindings = ServiceBindings::new();
        
        bindings
            .bind::<PostgresRepository, PostgresRepository>()
            .bind_singleton::<UserService, UserService>()
            .bind_named::<PostgresRepository, PostgresRepository>("postgres");
        
        assert_eq!(bindings.count(), 3);
        
        let service_ids = bindings.service_ids();
        assert_eq!(service_ids.len(), 3);
        
        // Check that we have the expected services
        assert!(bindings.contains(&ServiceId::of::<PostgresRepository>()));
        assert!(bindings.contains(&ServiceId::of::<UserService>()));
        assert!(bindings.contains(&ServiceId::named::<PostgresRepository>("postgres")));
    }

    #[test]
    fn test_factory_binding() {
        let mut bindings = ServiceBindings::new();
        
        bindings.bind_factory::<UserService, _, _>(|| {
            Ok(UserService::default())
        });
        
        assert_eq!(bindings.count(), 1);
        assert!(bindings.contains(&ServiceId::of::<UserService>()));
    }
}