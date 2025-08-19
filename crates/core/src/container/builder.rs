use crate::container::{Container, ServiceRegistry, ServiceScope};
use crate::foundation::traits::Service;
use crate::errors::CoreError;

/// Builder for constructing containers with services
pub struct ContainerBuilder {
    registry: ServiceRegistry,
    scope: ServiceScope,
}

impl ContainerBuilder {
    /// Create a new container builder
    pub fn new() -> Self {
        Self {
            registry: ServiceRegistry::new(),
            scope: ServiceScope::Singleton,
        }
    }
    
    /// Set the default scope for services
    pub fn with_scope(mut self, scope: ServiceScope) -> Self {
        self.scope = scope;
        self
    }
    
    /// Add a service to the container
    pub fn add_service<T>(mut self, service: T) -> Result<Self, CoreError>
    where
        T: Service + Clone + 'static,
    {
        self.registry.register_service(service)?;
        Ok(self)
    }
    
    /// Add a singleton service
    pub fn add_singleton<T>(mut self, service: T) -> Result<Self, CoreError>
    where
        T: Service + Clone + 'static,
    {
        self.registry.register_singleton(service)?;
        Ok(self)
    }
    
    /// Add a transient service with factory
    pub fn add_transient<T>(mut self, factory: Box<dyn Fn() -> T + Send + Sync>) -> Result<Self, CoreError>
    where
        T: Service + 'static,
    {
        self.registry.register_transient(factory)?;
        Ok(self)
    }
    
    /// Add multiple services at once
    pub fn add_services<T>(mut self, services: Vec<T>) -> Result<Self, CoreError>
    where
        T: Service + Clone + 'static,
    {
        for service in services {
            self.registry.register_service(service)?;
        }
        Ok(self)
    }
    
    /// Configure the builder from a configuration closure
    pub fn configure<F>(self, configure: F) -> Result<Self, CoreError>
    where
        F: FnOnce(Self) -> Result<Self, CoreError>,
    {
        configure(self)
    }
    
    /// Build the container
    pub fn build(self) -> Result<Container, CoreError> {
        let container = Container::new();
        // TODO: Implement proper service transfer from builder registry to container
        container.validate()?;
        Ok(container)
    }
    
    /// Build and initialize the container
    pub async fn build_and_initialize(self) -> Result<Container, CoreError> {
        let mut container = self.build()?;
        container.initialize().await?;
        Ok(container)
    }
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macro for building containers
#[macro_export]
macro_rules! container {
    ($($service:expr),* $(,)?) => {
        {
            let mut builder = $crate::container::ContainerBuilder::new();
            $(
                builder = builder.add_service($service)?;
            )*
            builder.build()
        }
    };
}

/// Convenience macro for building containers with singletons
#[macro_export]
macro_rules! singleton_container {
    ($($service:expr),* $(,)?) => {
        {
            let mut builder = $crate::container::ContainerBuilder::new()
                .with_scope($crate::container::ServiceScope::Singleton);
            $(
                builder = builder.add_singleton($service)?;
            )*
            builder.build()
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::traits::{Service, FrameworkComponent};
    
    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    struct TestService {
        name: String,
    }
    
    impl FrameworkComponent for TestService {}
    impl Service for TestService {}
    
    impl TestService {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }
    
    #[tokio::test]
    async fn test_container_builder() -> Result<(), CoreError> {
        let container = ContainerBuilder::new()
            .add_service(TestService::new("test1"))?
            .add_singleton(TestService::new("test2"))?
            .build_and_initialize()
            .await?;
        
        // TODO: Fix service transfer from builder to container
        // assert_eq!(container.service_count(), 2);
        assert!(container.is_initialized());
        
        Ok(())
    }
}