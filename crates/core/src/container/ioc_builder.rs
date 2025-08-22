use crate::container::binding::{ServiceBinder, ServiceBindings};
use crate::container::ioc_container::IocContainer;
use crate::container::autowiring::Injectable;
use crate::errors::CoreError;

/// Builder for IoC container with fluent API
#[derive(Debug)]
pub struct IocContainerBuilder {
    bindings: ServiceBindings,
}

impl IocContainerBuilder {
    /// Create a new IoC container builder
    pub fn new() -> Self {
        Self {
            bindings: ServiceBindings::new(),
        }
    }
    
    /// Build the IoC container
    pub fn build(self) -> Result<IocContainer, CoreError> {
        let mut container = IocContainer::from_bindings(self.bindings);
        container.build()?;
        Ok(container)
    }
}

impl ServiceBinder for IocContainerBuilder {
    fn bind<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        self.bindings.bind::<TInterface, TImpl>();
        self
    }
    
    fn bind_singleton<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        self.bindings.bind_singleton::<TInterface, TImpl>();
        self
    }
    
    fn bind_transient<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> &mut Self {
        self.bindings.bind_transient::<TInterface, TImpl>();
        self
    }
    
    fn bind_factory<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        self.bindings.bind_factory::<TInterface, _, _>(factory);
        self
    }
    
    fn bind_instance<TInterface: ?Sized + 'static, TImpl: Send + Sync + Clone + 'static>(&mut self, instance: TImpl) -> &mut Self {
        self.bindings.bind_instance::<TInterface, TImpl>(instance);
        self
    }
    
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: &str) -> &mut Self {
        self.bindings.bind_named::<TInterface, TImpl>(name);
        self
    }
    
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self {
        self.bindings.bind_injectable::<T>();
        self
    }
    
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self {
        self.bindings.bind_injectable_singleton::<T>();
        self
    }

    // Advanced binding methods implementation
    
    fn bind_with<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> crate::container::binding::AdvancedBindingBuilder<TInterface> {
        self.bindings.bind_with::<TInterface, TImpl>()
    }

    fn with_implementation<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, config: crate::container::binding::BindingConfig) -> &mut Self {
        self.bindings.with_implementation::<TInterface, TImpl>(config);
        self
    }

    fn bind_lazy<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        self.bindings.bind_lazy::<TInterface, F, T>(factory);
        self
    }

    fn bind_parameterized_factory<TInterface: ?Sized + 'static, P, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(P) -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        self.bindings.bind_parameterized_factory::<TInterface, P, F, T>(factory);
        self
    }

    fn bind_collection<TInterface: ?Sized + 'static, F>(&mut self, configure: F) -> &mut Self
    where
        F: FnOnce(&mut crate::container::binding::CollectionBindingBuilder<TInterface>),
    {
        self.bindings.bind_collection::<TInterface, F>(configure);
        self
    }

}

impl Default for IocContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait TestService: Send + Sync {
        fn get_value(&self) -> String;
    }

    #[derive(Default)]
    struct TestServiceImpl;
    
    unsafe impl Send for TestServiceImpl {}
    unsafe impl Sync for TestServiceImpl {}

    impl TestService for TestServiceImpl {
        fn get_value(&self) -> String {
            "test_value".to_string()
        }
    }

    #[test]
    fn test_ioc_container_builder() {
        let mut builder = IocContainerBuilder::new();
        builder.bind::<TestServiceImpl, TestServiceImpl>();
        let container = builder.build().unwrap();
        
        let service = container.resolve::<TestServiceImpl>().unwrap();
        assert_eq!(service.get_value(), "test_value");
    }

    #[test]
    fn test_builder_chaining() {
        let mut builder = IocContainerBuilder::new();
        
        builder
            .bind::<TestServiceImpl, TestServiceImpl>()
            .bind_singleton::<TestServiceImpl, TestServiceImpl>()
            .bind_transient::<TestServiceImpl, TestServiceImpl>();
        
        // Should have multiple bindings (will be deduplicated by service ID)
        assert!(builder.bindings.count() > 0);
    }
}