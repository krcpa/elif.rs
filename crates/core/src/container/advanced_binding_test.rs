#[cfg(test)]
mod advanced_binding_tests {
    use super::super::{
        IocContainer, ServiceBinder, AdvancedBindingBuilder,
        ServiceScope
    };

    // Test concrete implementations - avoiding trait objects for now
    #[derive(Default, Clone)]
    struct RedisCache {
        name: String,
    }

    impl RedisCache {
        fn new_with_name(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
            }
        }
        
        fn get_name(&self) -> &str {
            &self.name
        }
    }

    #[derive(Default, Clone)]
    struct MemoryCache {
        name: String,
    }

    impl MemoryCache {
        fn new_with_name(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
            }
        }
        
        fn get_name(&self) -> &str {
            &self.name
        }
    }

    #[derive(Default)]
    struct LocalStorage;

    #[derive(Default)]
    struct S3Storage;

    #[derive(Default)]
    struct SmtpEmailService;

    #[derive(Default)]
    struct SendGridEmailService;

    #[test]
    fn test_named_service_binding_and_resolution() {
        let mut container = IocContainer::new();
        
        // Test with concrete types instead of trait objects
        let redis_config = AdvancedBindingBuilder::<RedisCache>::new()
            .named("redis")
            .with_lifetime(ServiceScope::Singleton)
            .config();
            
        container.with_implementation::<RedisCache, RedisCache>(redis_config);
        
        let memory_config = AdvancedBindingBuilder::<MemoryCache>::new()
            .named("memory")
            .with_lifetime(ServiceScope::Transient)
            .config();
            
        container.with_implementation::<MemoryCache, MemoryCache>(memory_config);
        
        container.build().expect("Failed to build container");
        
        // Test named resolution with concrete types
        assert!(container.resolve_named::<RedisCache>("redis").is_ok());
        assert!(container.resolve_named::<MemoryCache>("memory").is_ok());
        assert!(container.resolve_named::<RedisCache>("nonexistent").is_err());
    }

    #[test]
    fn test_environment_conditional_binding() {
        let mut container = IocContainer::new();
        
        // Set up environment for test
        std::env::set_var("CACHE_PROVIDER", "redis");
        
        let redis_config = AdvancedBindingBuilder::<RedisCache>::new()
            .named("cache")
            .when_env("CACHE_PROVIDER", "redis")
            .config();
            
        container.with_implementation::<RedisCache, RedisCache>(redis_config);
        
        let memory_config = AdvancedBindingBuilder::<MemoryCache>::new()
            .named("cache")
            .when_env("CACHE_PROVIDER", "memory")
            .config();
            
        container.with_implementation::<MemoryCache, MemoryCache>(memory_config);
        
        container.build().expect("Failed to build container");
        
        // Should resolve to Redis since environment is set to "redis"
        let cache = container.resolve_named::<RedisCache>("cache");
        assert!(cache.is_ok());
        
        // Clean up
        std::env::remove_var("CACHE_PROVIDER");
    }

    #[test]
    fn test_feature_flag_conditional_binding() {
        let mut container = IocContainer::new();
        
        // Enable feature flag
        std::env::set_var("FEATURE_CLOUD_STORAGE", "1");
        
        let s3_config = AdvancedBindingBuilder::<S3Storage>::new()
            .when_feature("cloud_storage")
            .config();
            
        container.with_implementation::<S3Storage, S3Storage>(s3_config);
        
        let local_config = AdvancedBindingBuilder::<LocalStorage>::new()
            .when_not_feature("cloud_storage")
            .config();
            
        container.with_implementation::<LocalStorage, LocalStorage>(local_config);
        
        container.build().expect("Failed to build container");
        
        // Should resolve to S3 since feature is enabled
        let storage = container.resolve::<S3Storage>();
        assert!(storage.is_ok());
        
        // Clean up
        std::env::remove_var("FEATURE_CLOUD_STORAGE");
    }

    #[test]
    fn test_profile_based_conditional_binding() {
        let mut container = IocContainer::new();
        
        // Set profile to production
        std::env::set_var("PROFILE", "production");
        
        let prod_config = AdvancedBindingBuilder::<SmtpEmailService>::new()
            .named("main_email")
            .in_profile("production")
            .config();
            
        container.with_implementation::<SmtpEmailService, SmtpEmailService>(prod_config);
        
        let dev_config = AdvancedBindingBuilder::<SendGridEmailService>::new()
            .named("main_email")
            .in_profile("development")
            .config();
            
        container.with_implementation::<SendGridEmailService, SendGridEmailService>(dev_config);
        
        container.build().expect("Failed to build container");
        
        // Should resolve to SMTP for production
        let email = container.resolve_named::<SmtpEmailService>("main_email");
        assert!(email.is_ok());
        
        // Clean up
        std::env::remove_var("PROFILE");
    }

    #[test]
    fn test_custom_condition_binding() {
        let mut container = IocContainer::new();
        
        let always_true_config = AdvancedBindingBuilder::<MemoryCache>::new()
            .named("always_available")
            .when(|| true)
            .config();
            
        container.with_implementation::<MemoryCache, MemoryCache>(always_true_config);
        
        let never_config = AdvancedBindingBuilder::<RedisCache>::new()
            .named("never_available")
            .when(|| false)
            .config();
            
        container.with_implementation::<RedisCache, RedisCache>(never_config);
        
        container.build().expect("Failed to build container");
        
        // Should only resolve the "always_available" service
        assert!(container.resolve_named::<MemoryCache>("always_available").is_ok());
        assert!(container.resolve_named::<RedisCache>("never_available").is_err());
    }

    #[test]
    fn test_lazy_factory_binding() {
        let mut container = IocContainer::new();
        
        container.bind_lazy::<MemoryCache, _, _>(|| {
            println!("Creating lazy MemoryCache instance");
            MemoryCache::default()
        });
        
        container.build().expect("Failed to build container");
        
        // Should successfully create instance via lazy factory
        let cache = container.resolve::<MemoryCache>();
        assert!(cache.is_ok());
    }

    #[test]
    fn test_multiple_conditions_binding() {
        let mut container = IocContainer::new();
        
        // Set up multiple environment conditions
        std::env::set_var("ENVIRONMENT", "test");
        std::env::set_var("FEATURE_ADVANCED", "1");
        std::env::set_var("PROFILE", "integration");
        
        let complex_config = AdvancedBindingBuilder::<RedisCache>::new()
            .named("complex")
            .when_env("ENVIRONMENT", "test")
            .when_feature("advanced")
            .in_profile("integration")
            .when(|| std::env::var("USER").is_ok()) // Most systems have USER env var
            .with_lifetime(ServiceScope::Singleton)
            .config();
            
        container.with_implementation::<RedisCache, RedisCache>(complex_config);
        
        container.build().expect("Failed to build container");
        
        // Should resolve since all conditions are met
        let cache = container.resolve_named::<RedisCache>("complex");
        assert!(cache.is_ok());
        
        // Clean up
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("FEATURE_ADVANCED");
        std::env::remove_var("PROFILE");
    }

    #[test]
    fn test_resolve_concrete_types() {
        let mut container = IocContainer::new();
        
        // Bind multiple concrete types
        container.bind::<RedisCache, RedisCache>();
        container.bind_named::<MemoryCache, MemoryCache>("memory");
        container.bind_singleton::<S3Storage, S3Storage>();
        
        container.build().expect("Failed to build container");
        
        // Resolve concrete implementations
        assert!(container.resolve::<RedisCache>().is_ok());
        assert!(container.resolve_named::<MemoryCache>("memory").is_ok());
        assert!(container.resolve::<S3Storage>().is_ok());
    }

    #[test]
    fn test_service_statistics() {
        let mut container = IocContainer::new();
        
        container.bind::<MemoryCache, MemoryCache>();
        container.bind_singleton::<RedisCache, RedisCache>();
        container.bind_named::<S3Storage, S3Storage>("s3");
        
        container.build().expect("Failed to build container");
        
        let stats = container.get_statistics();
        assert_eq!(stats.total_services, 3);
        assert_eq!(stats.singleton_services, 1);
        assert_eq!(stats.transient_services, 2);
    }

    #[test]
    fn test_service_validation() {
        let mut container = IocContainer::new();
        
        container.bind::<MemoryCache, MemoryCache>();
        container.bind_singleton::<RedisCache, RedisCache>();
        
        container.build().expect("Failed to build container");
        
        // Validate all services can be resolved
        let validation_result = container.validate_all_services();
        assert!(validation_result.is_ok());
    }

    #[test]
    fn test_service_registration_queries() {
        let mut container = IocContainer::new();
        
        container.bind::<MemoryCache, MemoryCache>();
        container.bind_named::<RedisCache, RedisCache>("redis");
        
        // Test service existence queries
        assert!(container.contains::<MemoryCache>());
        assert!(container.contains_named::<RedisCache>("redis"));
        assert!(!container.contains_named::<RedisCache>("nonexistent"));
        
        container.build().expect("Failed to build container");
        
        let registered = container.get_registered_services();
        assert!(registered.len() >= 2);
    }

    #[test]
    fn test_condition_evaluation_edge_cases() {
        // Test when environment variable doesn't exist
        let config = AdvancedBindingBuilder::<RedisCache>::new()
            .when_env("NON_EXISTENT_VAR", "any_value")
            .config();
        
        assert!(!config.evaluate_conditions());
        
        // Test when environment variable exists but value doesn't match
        std::env::set_var("TEST_VAR", "wrong_value");
        let config2 = AdvancedBindingBuilder::<MemoryCache>::new()
            .when_env("TEST_VAR", "expected_value")
            .config();
            
        assert!(!config2.evaluate_conditions());
        
        // Clean up
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_collection_binding() {
        let mut container = IocContainer::new();
        
        // Use the fixed closure-based API that actually registers services
        container.bind_collection::<RedisCache, _>(|collection| {
            collection
                .add::<RedisCache>()
                .add_named::<RedisCache>("special_redis");
        });
        
        container.build().expect("Failed to build container");
        
        // Verify that the services were actually registered and can be resolved
        assert!(container.resolve::<RedisCache>().is_ok());
        assert!(container.resolve_named::<RedisCache>("special_redis").is_ok());
        
        // Check statistics show the right number of services
        let stats = container.get_statistics();
        assert_eq!(stats.total_services, 2); // RedisCache and named RedisCache
        assert_eq!(stats.transient_services, 2); // All are transient by default
    }

    #[test]
    fn test_factory_binding() {
        let mut container = IocContainer::new();
        
        container.bind_factory::<MemoryCache, _, _>(|| {
            Ok(MemoryCache::new_with_name("factory-created"))
        });
        
        container.build().expect("Failed to build container");
        
        let cache = container.resolve::<MemoryCache>();
        assert!(cache.is_ok());
        assert_eq!(cache.unwrap().get_name(), "factory-created");
    }

    #[test]
    fn test_instance_binding() {
        let mut container = IocContainer::new();
        
        let instance = RedisCache::new_with_name("pre-created");
        container.bind_instance::<RedisCache, RedisCache>(instance);
        
        container.build().expect("Failed to build container");
        
        let resolved = container.resolve::<RedisCache>();
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap().get_name(), "pre-created");
    }

    #[test]
    fn test_mixed_lifetimes() {
        let mut container = IocContainer::new();
        
        // Mix different lifetimes
        container.bind::<MemoryCache, MemoryCache>(); // Transient
        container.bind_singleton::<RedisCache, RedisCache>(); // Singleton
        container.bind_named::<S3Storage, S3Storage>("storage"); // Named transient
        
        container.build().expect("Failed to build container");
        
        // Resolve multiple times to verify singleton behavior
        let redis1 = container.resolve::<RedisCache>().unwrap();
        let redis2 = container.resolve::<RedisCache>().unwrap();
        
        // For singletons, we get the same Arc
        assert!(std::ptr::eq(redis1.as_ref(), redis2.as_ref()));
        
        // Transients should be different instances
        let memory1 = container.resolve::<MemoryCache>().unwrap();
        let memory2 = container.resolve::<MemoryCache>().unwrap();
        
        // Different Arc instances for transients
        assert!(!std::ptr::eq(memory1.as_ref(), memory2.as_ref()));
    }
}