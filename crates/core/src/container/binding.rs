use crate::container::descriptor::{ServiceDescriptor, ServiceDescriptorFactoryBuilder, ServiceId};
use crate::container::scope::ServiceScope;
use crate::container::autowiring::Injectable;
use crate::errors::CoreError;

/// Conditional binding function type
pub type ConditionFn = Box<dyn Fn() -> bool + Send + Sync>;

/// Environment condition type
pub type EnvCondition = (&'static str, String);

/// Binding configuration for advanced features
pub struct BindingConfig {
    /// Named/tagged identifier
    pub name: Option<String>,
    /// Service lifetime
    pub lifetime: ServiceScope,
    /// Environment-based conditions
    pub env_conditions: Vec<EnvCondition>,
    /// Feature flag conditions
    pub feature_conditions: Vec<(String, bool)>,
    /// Custom condition functions
    pub conditions: Vec<ConditionFn>,
    /// Whether this is the default implementation
    pub is_default: bool,
    /// Profile-based conditions
    pub profile_conditions: Vec<String>,
}

impl BindingConfig {
    pub fn new() -> Self {
        Self {
            name: None,
            lifetime: ServiceScope::Transient,
            env_conditions: Vec::new(),
            feature_conditions: Vec::new(),
            conditions: Vec::new(),
            is_default: false,
            profile_conditions: Vec::new(),
        }
    }
    
    /// Check if all conditions are met
    pub fn evaluate_conditions(&self) -> bool {
        // Check environment conditions
        for (key, expected_value) in &self.env_conditions {
            if let Ok(actual_value) = std::env::var(key) {
                if actual_value != *expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check feature conditions
        for (feature, expected) in &self.feature_conditions {
            let feature_enabled = std::env::var(&format!("FEATURE_{}", feature.to_uppercase())).is_ok();
            if feature_enabled != *expected {
                return false;
            }
        }
        
        // Check profile conditions
        if !self.profile_conditions.is_empty() {
            let current_profile = std::env::var("PROFILE").unwrap_or_else(|_| "development".to_string());
            if !self.profile_conditions.contains(&current_profile) {
                return false;
            }
        }
        
        // Check custom conditions
        for condition in &self.conditions {
            if !condition() {
                return false;
            }
        }
        
        true
    }
}

/// Advanced binding builder for fluent configuration
pub struct AdvancedBindingBuilder<TInterface: ?Sized + 'static> {
    config: BindingConfig,
    _phantom: std::marker::PhantomData<*const TInterface>,
}

impl<TInterface: ?Sized + 'static> AdvancedBindingBuilder<TInterface> {
    pub fn new() -> Self {
        Self {
            config: BindingConfig::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set service name/tag
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.config.name = Some(name.into());
        self
    }
    
    /// Set service lifetime
    pub fn with_lifetime(mut self, lifetime: ServiceScope) -> Self {
        self.config.lifetime = lifetime;
        self
    }
    
    /// Add environment condition
    pub fn when_env(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.config.env_conditions.push((key, value.into()));
        self
    }
    
    /// Add feature flag condition
    pub fn when_feature(mut self, feature: impl Into<String>) -> Self {
        self.config.feature_conditions.push((feature.into(), true));
        self
    }
    
    /// Add inverse feature flag condition
    pub fn when_not_feature(mut self, feature: impl Into<String>) -> Self {
        self.config.feature_conditions.push((feature.into(), false));
        self
    }
    
    /// Add custom condition
    pub fn when<F>(mut self, condition: F) -> Self 
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.config.conditions.push(Box::new(condition));
        self
    }
    
    /// Mark as default implementation
    pub fn as_default(mut self) -> Self {
        self.config.is_default = true;
        self
    }
    
    /// Add profile condition
    pub fn in_profile(mut self, profile: impl Into<String>) -> Self {
        self.config.profile_conditions.push(profile.into());
        self
    }
    
    /// Get the configuration
    pub fn config(self) -> BindingConfig {
        self.config
    }
}

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
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: &str) -> &mut Self;
    
    /// Bind an Injectable service with auto-wiring
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self;
    
    /// Bind an Injectable service as singleton with auto-wiring  
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self;

    // Advanced binding methods
    
    /// Advanced bind with fluent configuration - returns builder for chaining
    fn bind_with<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> AdvancedBindingBuilder<TInterface>;
    
    /// Complete advanced binding with implementation
    fn with_implementation<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, config: BindingConfig) -> &mut Self;
    
    /// Bind a lazy service using factory that gets called only when needed
    fn bind_lazy<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        T: Send + Sync + 'static;
    
    /// Bind with parameterized factory
    fn bind_parameterized_factory<TInterface: ?Sized + 'static, P, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn(P) -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
        P: Send + Sync + 'static;
    
    /// Bind a collection of services 
    fn bind_collection<TInterface: ?Sized + 'static>(&mut self) -> CollectionBindingBuilder<TInterface>;
    
    /// Bind generic service
    fn bind_generic<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static, TGeneric>(&mut self) -> &mut Self
    where
        TGeneric: Send + Sync + 'static;
}

/// Builder for collection bindings
pub struct CollectionBindingBuilder<TInterface: ?Sized + 'static> {
    services: Vec<ServiceDescriptor>,
    _phantom: std::marker::PhantomData<*const TInterface>,
}

impl<TInterface: ?Sized + 'static> CollectionBindingBuilder<TInterface> {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Add a service to the collection
    pub fn add<TImpl: Send + Sync + Default + 'static>(mut self) -> Self {
        let descriptor = ServiceDescriptor::bind::<TInterface, TImpl>()
            .with_lifetime(ServiceScope::Transient)
            .build();
        self.services.push(descriptor);
        self
    }
    
    /// Add a named service to the collection
    pub fn add_named<TImpl: Send + Sync + Default + 'static>(mut self, name: impl Into<String>) -> Self {
        let descriptor = ServiceDescriptor::bind_named::<TInterface, TImpl>(name)
            .with_lifetime(ServiceScope::Transient)
            .build();
        self.services.push(descriptor);
        self
    }
    
    /// Get the collection of service descriptors
    pub fn services(self) -> Vec<ServiceDescriptor> {
        self.services
    }
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
    
    /// Get service descriptor by type and name without allocation
    pub fn get_descriptor_named<T: 'static + ?Sized>(&self, name: &str) -> Option<&ServiceDescriptor> {
        self.descriptors.iter().find(|d| d.service_id.matches_named::<T>(name))
    }
    
    /// Get all service IDs
    pub fn service_ids(&self) -> Vec<ServiceId> {
        self.descriptors.iter().map(|d| d.service_id.clone()).collect()
    }
    
    /// Check if a service is registered
    pub fn contains(&self, service_id: &ServiceId) -> bool {
        self.descriptors.iter().any(|d| d.service_id == *service_id)
    }
    
    /// Check if a named service is registered without allocation
    pub fn contains_named<T: 'static + ?Sized>(&self, name: &str) -> bool {
        self.descriptors.iter().any(|d| d.service_id.matches_named::<T>(name))
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
    
    fn bind_named<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, name: &str) -> &mut Self {
        let descriptor = ServiceDescriptor::bind_named::<TInterface, TImpl>(name)
            .with_lifetime(ServiceScope::Transient)
            .build();
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_injectable<T: Injectable>(&mut self) -> &mut Self {
        let dependencies = T::dependencies();
        let descriptor = ServiceDescriptor::autowired::<T>(dependencies);
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_injectable_singleton<T: Injectable>(&mut self) -> &mut Self {
        let dependencies = T::dependencies();
        let descriptor = ServiceDescriptor::autowired_singleton::<T>(dependencies);
        self.add_descriptor(descriptor);
        self
    }

    // Advanced binding methods implementation
    
    fn bind_with<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self) -> AdvancedBindingBuilder<TInterface> {
        AdvancedBindingBuilder::new()
    }
    
    fn with_implementation<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static>(&mut self, config: BindingConfig) -> &mut Self {
        // Only add binding if conditions are met
        if config.evaluate_conditions() {
            let mut builder = if let Some(name) = &config.name {
                ServiceDescriptor::bind_named::<TInterface, TImpl>(name.clone())
            } else {
                ServiceDescriptor::bind::<TInterface, TImpl>()
            };
            
            builder = builder.with_lifetime(config.lifetime);
            let descriptor = builder.build();
            self.add_descriptor(descriptor);
        }
        self
    }
    
    fn bind_lazy<TInterface: ?Sized + 'static, F, T>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let lazy_factory = move || -> Result<T, CoreError> {
            Ok(factory())
        };
        
        let descriptor = ServiceDescriptorFactoryBuilder::<TInterface>::new()
            .with_factory(lazy_factory)
            .build()
            .expect("Failed to build lazy factory descriptor");
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_parameterized_factory<TInterface: ?Sized + 'static, P, F, T>(&mut self, _factory: F) -> &mut Self
    where
        F: Fn(P) -> Result<T, CoreError> + Send + Sync + 'static,
        T: Send + Sync + 'static,
        P: Send + Sync + 'static,
    {
        // For now, parameterized factory stores the factory but requires parameter injection
        // This is a complex feature that would need parameter resolution at runtime
        let descriptor = ServiceDescriptorFactoryBuilder::<TInterface>::new()
            .with_factory(move || -> Result<T, CoreError> {
                // This would need to be resolved at runtime with proper parameter injection
                // For now, this is a placeholder implementation
                Err(CoreError::ServiceNotFound {
                    service_type: format!("Parameterized factory for {} requires runtime parameter resolution", 
                        std::any::type_name::<TInterface>()),
                })
            })
            .build()
            .expect("Failed to build parameterized factory descriptor");
        self.add_descriptor(descriptor);
        self
    }
    
    fn bind_collection<TInterface: ?Sized + 'static>(&mut self) -> CollectionBindingBuilder<TInterface> {
        CollectionBindingBuilder::new()
    }
    
    fn bind_generic<TInterface: ?Sized + 'static, TImpl: Send + Sync + Default + 'static, TGeneric>(&mut self) -> &mut Self
    where
        TGeneric: Send + Sync + 'static,
    {
        // Generic binding implementation - would need type parameter support in descriptors
        let descriptor = ServiceDescriptor::bind::<TInterface, TImpl>()
            .with_lifetime(ServiceScope::Transient)
            .build();
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

    #[test]
    fn test_advanced_binding_with_environment_conditions() {
        let mut bindings = ServiceBindings::new();
        
        // Set up environment for test
        std::env::set_var("CACHE_PROVIDER", "redis");
        
        let config = AdvancedBindingBuilder::<dyn TestRepository>::new()
            .named("redis")
            .when_env("CACHE_PROVIDER", "redis")
            .with_lifetime(ServiceScope::Singleton)
            .config();
        
        bindings.with_implementation::<dyn TestRepository, PostgresRepository>(config);
        
        assert_eq!(bindings.count(), 1);
        assert!(bindings.contains_named::<dyn TestRepository>("redis"));
        
        // Clean up environment
        std::env::remove_var("CACHE_PROVIDER");
    }

    #[test]
    fn test_conditional_binding_not_met() {
        let mut bindings = ServiceBindings::new();
        
        // Environment condition not met
        let config = AdvancedBindingBuilder::<dyn TestRepository>::new()
            .named("nonexistent")
            .when_env("NON_EXISTENT_VAR", "value")
            .config();
        
        bindings.with_implementation::<dyn TestRepository, PostgresRepository>(config);
        
        // Should not add binding since condition is not met
        assert_eq!(bindings.count(), 0);
    }

    #[test]
    fn test_feature_flag_conditions() {
        let mut bindings = ServiceBindings::new();
        
        // Set up feature flag
        std::env::set_var("FEATURE_ADVANCED_CACHE", "1");
        
        let config = AdvancedBindingBuilder::<dyn TestRepository>::new()
            .when_feature("advanced_cache")
            .config();
        
        bindings.with_implementation::<dyn TestRepository, PostgresRepository>(config);
        
        assert_eq!(bindings.count(), 1);
        
        // Clean up
        std::env::remove_var("FEATURE_ADVANCED_CACHE");
    }

    #[test] 
    fn test_profile_conditions() {
        let mut bindings = ServiceBindings::new();
        
        // Test with development profile
        std::env::set_var("PROFILE", "development");
        
        let config = AdvancedBindingBuilder::<dyn TestService>::new()
            .in_profile("development")
            .config();
        
        bindings.with_implementation::<dyn TestService, UserService>(config);
        
        assert_eq!(bindings.count(), 1);
        
        // Test with production profile (should not bind)
        std::env::set_var("PROFILE", "production");
        
        let config2 = AdvancedBindingBuilder::<dyn TestRepository>::new()
            .in_profile("development")
            .config();
        
        bindings.with_implementation::<dyn TestRepository, PostgresRepository>(config2);
        
        // Should still be 1, not 2
        assert_eq!(bindings.count(), 1);
        
        // Clean up
        std::env::remove_var("PROFILE");
    }

    #[test]
    fn test_custom_conditions() {
        let mut bindings = ServiceBindings::new();
        
        let config = AdvancedBindingBuilder::<dyn TestService>::new()
            .when(|| true) // Always true
            .config();
        
        bindings.with_implementation::<dyn TestService, UserService>(config);
        
        assert_eq!(bindings.count(), 1);
        
        let config2 = AdvancedBindingBuilder::<dyn TestRepository>::new()
            .when(|| false) // Always false
            .config();
        
        bindings.with_implementation::<dyn TestRepository, PostgresRepository>(config2);
        
        // Should still be 1, not 2
        assert_eq!(bindings.count(), 1);
    }

    #[test]
    fn test_lazy_binding() {
        let mut bindings = ServiceBindings::new();
        
        bindings.bind_lazy::<UserService, _, _>(|| {
            UserService::default()
        });
        
        assert_eq!(bindings.count(), 1);
        assert!(bindings.contains(&ServiceId::of::<UserService>()));
    }

    #[test]
    fn test_collection_binding() {
        let mut bindings = ServiceBindings::new();
        
        let collection = bindings.bind_collection::<dyn TestService>()
            .add::<UserService>()
            .add_named::<UserService>("named_user_service");
        
        let services = collection.services();
        assert_eq!(services.len(), 2);
        
        // Verify the services are correctly configured
        assert!(services.iter().any(|s| s.service_id == ServiceId::of::<dyn TestService>()));
        assert!(services.iter().any(|s| s.service_id == ServiceId::named::<dyn TestService>("named_user_service")));
    }

    #[test]
    fn test_multiple_conditions() {
        let mut bindings = ServiceBindings::new();
        
        // Set up multiple conditions
        std::env::set_var("ENV_VAR", "test_value");
        std::env::set_var("FEATURE_TEST", "1");
        std::env::set_var("PROFILE", "test");
        
        let config = AdvancedBindingBuilder::<dyn TestService>::new()
            .when_env("ENV_VAR", "test_value")
            .when_feature("test")
            .in_profile("test")
            .when(|| true)
            .named("complex_service")
            .with_lifetime(ServiceScope::Singleton)
            .config();
        
        bindings.with_implementation::<dyn TestService, UserService>(config);
        
        assert_eq!(bindings.count(), 1);
        assert!(bindings.contains_named::<dyn TestService>("complex_service"));
        
        // Clean up
        std::env::remove_var("ENV_VAR");
        std::env::remove_var("FEATURE_TEST");
        std::env::remove_var("PROFILE");
    }

    #[test]
    fn test_generic_binding() {
        let mut bindings = ServiceBindings::new();
        
        bindings.bind_generic::<dyn TestRepository, PostgresRepository, String>();
        
        assert_eq!(bindings.count(), 1);
        assert!(bindings.contains(&ServiceId::of::<dyn TestRepository>()));
    }
}