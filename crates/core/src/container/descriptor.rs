use std::any::{Any, TypeId};
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;

/// Service identifier combining type and optional name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceId {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub name: Option<String>,
}

impl ServiceId {
    /// Create a new service ID for a type
    pub fn of<T: 'static + ?Sized>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            name: None,
        }
    }
    
    /// Create a named service ID for a type
    pub fn named<T: 'static + ?Sized>(name: impl Into<String>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            name: Some(name.into()),
        }
    }
    
    /// Check if this ServiceId matches a type and name without allocating
    pub fn matches_named<T: 'static + ?Sized>(&self, name: &str) -> bool {
        self.type_id == TypeId::of::<T>() && 
        self.name.as_deref() == Some(name)
    }
    
    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
    
    /// Create a service ID directly from type IDs and names
    pub fn by_ids(type_id: TypeId, type_name: &'static str) -> Self {
        Self {
            type_id,
            type_name,
            name: None,
        }
    }
    
    /// Create a named service ID directly from type IDs and names
    pub fn named_by_ids(type_id: TypeId, type_name: &'static str, name: String) -> Self {
        Self {
            type_id,
            type_name,
            name: Some(name),
        }
    }
}

/// Factory function for creating service instances
/// We use Any here to avoid circular references with Container
pub type ServiceFactory = Box<dyn Fn() -> Result<Box<dyn Any + Send + Sync>, CoreError> + Send + Sync>;

/// Strategy for activating/creating service instances
pub enum ServiceActivationStrategy {
    /// Service created via factory function (traditional approach)
    Factory(ServiceFactory),
    /// Service created via auto-wiring (Injectable trait)
    AutoWired,
}

impl std::fmt::Debug for ServiceActivationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceActivationStrategy::Factory(_) => write!(f, "Factory(<factory_fn>)"),
            ServiceActivationStrategy::AutoWired => write!(f, "AutoWired"),
        }
    }
}

/// Service descriptor containing all metadata for a service
pub struct ServiceDescriptor {
    /// Service identifier (type + optional name)
    pub service_id: ServiceId,
    /// Implementation type ID
    pub implementation_id: TypeId,
    /// Service lifetime/scope
    pub lifetime: ServiceScope,
    /// Strategy for creating instances
    pub activation_strategy: ServiceActivationStrategy,
    /// Dependencies this service requires
    pub dependencies: Vec<ServiceId>,
}

impl std::fmt::Debug for ServiceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceDescriptor")
            .field("service_id", &self.service_id)
            .field("implementation_id", &self.implementation_id)
            .field("lifetime", &self.lifetime)
            .field("activation_strategy", &self.activation_strategy)
            .field("dependencies", &self.dependencies)
            .finish()
    }
}


impl ServiceDescriptor {
    /// Create a new service descriptor with type binding
    pub fn bind<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>() -> ServiceDescriptorBuilder<TInterface, TImpl> {
        ServiceDescriptorBuilder::new()
    }
    
    /// Create a named service descriptor
    pub fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(name: impl Into<String>) -> ServiceDescriptorBuilder<TInterface, TImpl> {
        ServiceDescriptorBuilder::new().with_name(name)
    }
    
    /// Create a singleton service descriptor
    pub fn singleton<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>() -> ServiceDescriptorBuilder<TInterface, TImpl> {
        ServiceDescriptorBuilder::new().with_lifetime(ServiceScope::Singleton)
    }
    
    /// Create a transient service descriptor
    pub fn transient<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>() -> ServiceDescriptorBuilder<TInterface, TImpl> {
        ServiceDescriptorBuilder::new().with_lifetime(ServiceScope::Transient)
    }
    
    /// Create an auto-wired service descriptor
    pub fn autowired<T: 'static>(dependencies: Vec<ServiceId>) -> ServiceDescriptor {
        ServiceDescriptor {
            service_id: ServiceId::of::<T>(),
            implementation_id: TypeId::of::<T>(),
            lifetime: ServiceScope::Transient,
            activation_strategy: ServiceActivationStrategy::AutoWired,
            dependencies,
        }
    }
    
    /// Create an auto-wired singleton service descriptor
    pub fn autowired_singleton<T: 'static>(dependencies: Vec<ServiceId>) -> ServiceDescriptor {
        ServiceDescriptor {
            service_id: ServiceId::of::<T>(),
            implementation_id: TypeId::of::<T>(),
            lifetime: ServiceScope::Singleton,
            activation_strategy: ServiceActivationStrategy::AutoWired,
            dependencies,
        }
    }
}

/// Builder for service descriptors
pub struct ServiceDescriptorBuilder<TInterface: ?Sized, TImpl> {
    name: Option<String>,
    lifetime: ServiceScope,
    dependencies: Vec<ServiceId>,
    _phantom: std::marker::PhantomData<(*const TInterface, TImpl)>,
}

impl<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static> ServiceDescriptorBuilder<TInterface, TImpl>
{
    /// Create a new service descriptor builder
    pub fn new() -> Self {
        Self {
            name: None,
            lifetime: ServiceScope::Transient,
            dependencies: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set the service name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Set the service lifetime
    pub fn with_lifetime(mut self, lifetime: ServiceScope) -> Self {
        self.lifetime = lifetime;
        self
    }
    
    /// Add a dependency
    pub fn depends_on<T: 'static>(mut self) -> Self {
        self.dependencies.push(ServiceId::of::<T>());
        self
    }
    
    /// Add a named dependency
    pub fn depends_on_named<T: 'static>(mut self, name: impl Into<String>) -> Self {
        self.dependencies.push(ServiceId::named::<T>(name));
        self
    }
    
    /// Build the service descriptor
    pub fn build(self) -> ServiceDescriptor {
        let service_id = if let Some(name) = self.name {
            ServiceId::named::<TInterface>(name)
        } else {
            ServiceId::of::<TInterface>()
        };
        
        let factory: ServiceFactory = Box::new(move || {
            let instance = TImpl::default();
            Ok(Box::new(instance) as Box<dyn Any + Send + Sync>)
        });
        
        ServiceDescriptor {
            service_id,
            implementation_id: TypeId::of::<TImpl>(),
            lifetime: self.lifetime,
            activation_strategy: ServiceActivationStrategy::Factory(factory),
            dependencies: self.dependencies,
        }
    }
}

/// Service descriptor builder with custom factory
pub struct ServiceDescriptorFactoryBuilder<TInterface: ?Sized> {
    name: Option<String>,
    lifetime: ServiceScope,
    dependencies: Vec<ServiceId>,
    factory: Option<ServiceFactory>,
    _phantom: std::marker::PhantomData<*const TInterface>,
}

impl<TInterface: ?Sized + 'static> ServiceDescriptorFactoryBuilder<TInterface> {
    /// Create a new factory-based service descriptor builder
    pub fn new() -> Self {
        Self {
            name: None,
            lifetime: ServiceScope::Transient,
            dependencies: Vec::new(),
            factory: None,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set the service name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Set the service lifetime
    pub fn with_lifetime(mut self, lifetime: ServiceScope) -> Self {
        self.lifetime = lifetime;
        self
    }
    
    /// Set the factory function
    pub fn with_factory<F, T>(mut self, factory: F) -> Self
    where
        F: Fn() -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let wrapped_factory: ServiceFactory = Box::new(move || {
            let instance = factory()?;
            Ok(Box::new(instance) as Box<dyn Any + Send + Sync>)
        });
        self.factory = Some(wrapped_factory);
        self
    }
    
    /// Build the service descriptor
    pub fn build(self) -> Result<ServiceDescriptor, CoreError> {
        let factory = self.factory.ok_or_else(|| CoreError::InvalidServiceDescriptor {
            message: "Factory function is required".to_string(),
        })?;
        
        let service_id = if let Some(name) = self.name {
            ServiceId::named::<TInterface>(name)
        } else {
            ServiceId::of::<TInterface>()
        };
        
        Ok(ServiceDescriptor {
            service_id,
            implementation_id: TypeId::of::<()>(), // Unknown for factory-based services
            lifetime: self.lifetime,
            activation_strategy: ServiceActivationStrategy::Factory(factory),
            dependencies: self.dependencies,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    trait TestTrait: Send + Sync {
        fn test_method(&self) -> String;
    }

    #[derive(Debug, Default)]
    struct TestImpl;
    
    unsafe impl Send for TestImpl {}
    unsafe impl Sync for TestImpl {}

    impl TestTrait for TestImpl {
        fn test_method(&self) -> String {
            "test".to_string()
        }
    }

    #[test]
    fn test_service_id_creation() {
        let id1 = ServiceId::of::<TestImpl>();
        let id2 = ServiceId::named::<TestImpl>("test");
        
        assert_eq!(id1.type_id, TypeId::of::<TestImpl>());
        assert_eq!(id1.name, None);
        
        assert_eq!(id2.type_id, TypeId::of::<TestImpl>());
        assert_eq!(id2.name, Some("test".to_string()));
        
        assert_ne!(id1, id2);
    }
    
    #[test]
    fn test_type_name_capture() {
        let id1 = ServiceId::of::<TestImpl>();
        let id2 = ServiceId::named::<TestImpl>("test");
        let id3 = ServiceId::of::<dyn TestTrait>();
        let id4 = ServiceId::of::<String>();
        
        // Verify that type names are actually captured, not "unknown"
        assert!(id1.type_name().contains("TestImpl"));
        assert!(id2.type_name().contains("TestImpl"));
        assert!(id3.type_name().contains("TestTrait"));
        assert_eq!(id4.type_name(), "alloc::string::String");
        
        // Verify type_name() method returns the stored value
        assert_eq!(id1.type_name(), id1.type_name);
        assert_eq!(id2.type_name(), id2.type_name);
    }

    #[test]
    fn test_service_descriptor_builder() {
        let descriptor = ServiceDescriptor::bind::<dyn TestTrait, TestImpl>()
            .with_lifetime(ServiceScope::Singleton)
            .depends_on::<String>()
            .build();
        
        assert_eq!(descriptor.lifetime, ServiceScope::Singleton);
        assert_eq!(descriptor.implementation_id, TypeId::of::<TestImpl>());
        assert_eq!(descriptor.dependencies.len(), 1);
        assert_eq!(descriptor.dependencies[0], ServiceId::of::<String>());
    }

    #[test]
    fn test_factory_service_descriptor() {
        let descriptor = ServiceDescriptorFactoryBuilder::<dyn TestTrait>::new()
            .with_lifetime(ServiceScope::Transient)
            .with_factory(|| -> Result<TestImpl, CoreError> {
                Ok(TestImpl)
            })
            .build()
            .unwrap();
        
        assert_eq!(descriptor.lifetime, ServiceScope::Transient);
    }
}