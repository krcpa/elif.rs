//! Advanced Binding Examples for IOC Phase 4
//! 
//! This module demonstrates the advanced binding features including:
//! - Interface-to-implementation mapping
//! - Named services and tagging
//! - Conditional binding (environment, features, profiles)
//! - Factory patterns and lazy initialization
//! - Generic type support
//! - Collection resolution

use std::collections::HashMap;
use crate::container::{
    IocContainer, ServiceBinder, AdvancedBindingBuilder,
    ServiceScope
};
use crate::errors::CoreError;

// Example interfaces and implementations

/// Cache interface for different storage backends
pub trait Cache: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: String) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

/// Redis cache implementation
#[derive(Default)]
pub struct RedisCache {
    _config: String,
}

impl Cache for RedisCache {
    fn get(&self, key: &str) -> Option<String> {
        println!("RedisCache: Getting key '{}'", key);
        Some(format!("redis_value_{}", key))
    }
    
    fn set(&self, key: &str, value: String) -> Result<(), String> {
        println!("RedisCache: Setting '{}' = '{}'", key, value);
        Ok(())
    }
    
    fn delete(&self, key: &str) -> Result<(), String> {
        println!("RedisCache: Deleting key '{}'", key);
        Ok(())
    }
}

/// In-memory cache implementation
#[derive(Default)]
pub struct MemoryCache {
    storage: HashMap<String, String>,
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<String> {
        println!("MemoryCache: Getting key '{}'", key);
        self.storage.get(key).cloned()
    }
    
    fn set(&self, key: &str, value: String) -> Result<(), String> {
        println!("MemoryCache: Setting '{}' = '{}'", key, value);
        // Note: In real implementation, we'd need interior mutability
        Ok(())
    }
    
    fn delete(&self, key: &str) -> Result<(), String> {
        println!("MemoryCache: Deleting key '{}'", key);
        Ok(())
    }
}

/// Hybrid cache that uses both Redis and Memory
#[derive(Default)]
pub struct HybridCache;

impl Cache for HybridCache {
    fn get(&self, key: &str) -> Option<String> {
        println!("HybridCache: Getting key '{}' (checking memory first, then Redis)", key);
        Some(format!("hybrid_value_{}", key))
    }
    
    fn set(&self, key: &str, value: String) -> Result<(), String> {
        println!("HybridCache: Setting '{}' = '{}' (both memory and Redis)", key, value);
        Ok(())
    }
    
    fn delete(&self, key: &str) -> Result<(), String> {
        println!("HybridCache: Deleting key '{}' (from both memory and Redis)", key);
        Ok(())
    }
}

/// Email service interface
pub trait EmailService: Send + Sync {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
}

/// SMTP email service
#[derive(Default)]
pub struct SmtpEmailService;

impl EmailService for SmtpEmailService {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        println!("SMTP: Sending email to '{}' with subject '{}'", to, subject);
        println!("Body: {}", body);
        Ok(())
    }
}

/// SendGrid email service
#[derive(Default)]
pub struct SendGridEmailService;

impl EmailService for SendGridEmailService {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        println!("SendGrid API: Sending email to '{}' with subject '{}'", to, subject);
        println!("Body: {}", body);
        Ok(())
    }
}

/// Storage interface
pub trait Storage: Send + Sync {
    fn store(&self, path: &str, data: &[u8]) -> Result<String, String>;
    fn retrieve(&self, path: &str) -> Result<Vec<u8>, String>;
}

/// Local file storage
#[derive(Default)]
pub struct LocalStorage;

impl Storage for LocalStorage {
    fn store(&self, path: &str, data: &[u8]) -> Result<String, String> {
        println!("LocalStorage: Storing {} bytes to '{}'", data.len(), path);
        Ok(format!("/local/{}", path))
    }
    
    fn retrieve(&self, path: &str) -> Result<Vec<u8>, String> {
        println!("LocalStorage: Retrieving from '{}'", path);
        Ok(b"local file content".to_vec())
    }
}

/// S3 cloud storage
#[derive(Default)]
pub struct S3Storage;

impl Storage for S3Storage {
    fn store(&self, path: &str, data: &[u8]) -> Result<String, String> {
        println!("S3Storage: Storing {} bytes to '{}'", data.len(), path);
        Ok(format!("https://bucket.s3.amazonaws.com/{}", path))
    }
    
    fn retrieve(&self, path: &str) -> Result<Vec<u8>, String> {
        println!("S3Storage: Retrieving from '{}'", path);
        Ok(b"s3 file content".to_vec())
    }
}

/// Example 1: Multiple implementations with environment-based selection
pub fn example_environment_based_binding() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Redis cache for production environment
    let redis_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("redis")
        .when_env("CACHE_PROVIDER", "redis")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, RedisCache>(redis_config);
    
    // Memory cache for development/test environment
    let memory_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("memory")
        .when_env("CACHE_PROVIDER", "memory")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, MemoryCache>(memory_config);
    
    // Hybrid cache as default
    let hybrid_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .as_default()
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, HybridCache>(hybrid_config);
    
    container.build()?;
    Ok(container)
}

/// Example 2: Feature flag based service selection
pub fn example_feature_flag_binding() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Cloud storage when cloud features are enabled
    let s3_config = AdvancedBindingBuilder::<dyn Storage>::new()
        .when_feature("cloud-storage")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Storage, S3Storage>(s3_config);
    
    // Local storage when cloud features are disabled
    let local_config = AdvancedBindingBuilder::<dyn Storage>::new()
        .when_not_feature("cloud-storage")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Storage, LocalStorage>(local_config);
    
    container.build()?;
    Ok(container)
}

/// Example 3: Profile-based configuration
pub fn example_profile_based_binding() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Production email service
    let smtp_config = AdvancedBindingBuilder::<dyn EmailService>::new()
        .named("production_email")
        .in_profile("production")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn EmailService, SmtpEmailService>(smtp_config);
    
    // Development email service
    let sendgrid_config = AdvancedBindingBuilder::<dyn EmailService>::new()
        .named("dev_email")
        .in_profile("development")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn EmailService, SendGridEmailService>(sendgrid_config);
    
    container.build()?;
    Ok(container)
}

/// Example 4: Custom condition binding
pub fn example_custom_condition_binding() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Use Redis cache if Redis URL is available
    let redis_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("conditional_cache")
        .when(|| std::env::var("REDIS_URL").is_ok())
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, RedisCache>(redis_config);
    
    // Fallback to memory cache
    let memory_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("fallback_cache")
        .when(|| std::env::var("REDIS_URL").is_err())
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, MemoryCache>(memory_config);
    
    container.build()?;
    Ok(container)
}

/// Example 5: Factory patterns and lazy initialization
pub fn example_factory_patterns() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Lazy-initialized expensive cache
    container.bind_lazy::<RedisCache, _, _>(|| {
        println!("Initializing expensive Redis connection...");
        std::thread::sleep(std::time::Duration::from_millis(10)); // Simulate setup time
        RedisCache::default()
    });
    
    // Factory with custom logic
    container.bind_factory::<dyn Cache, _, _>(|| {
        let cache_type = std::env::var("CACHE_TYPE").unwrap_or_else(|_| "memory".to_string());
        match cache_type.as_str() {
            "redis" => Ok(Box::new(RedisCache::default()) as Box<dyn Cache>),
            "memory" => Ok(Box::new(MemoryCache::default()) as Box<dyn Cache>),
            _ => Ok(Box::new(HybridCache::default()) as Box<dyn Cache>),
        }
    });
    
    container.build()?;
    Ok(container)
}

/// Example 6: Collection bindings for plugin architecture
pub fn example_collection_binding() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Register multiple cache implementations as a collection
    let _cache_collection = container.bind_collection::<dyn Cache>()
        .add::<RedisCache>()
        .add::<MemoryCache>()
        .add_named::<HybridCache>("hybrid");
    
    // Register multiple storage providers
    let _storage_collection = container.bind_collection::<dyn Storage>()
        .add::<LocalStorage>()
        .add::<S3Storage>();
    
    container.build()?;
    Ok(container)
}

/// Example 7: Complex multi-condition binding
pub fn example_complex_conditions() -> Result<IocContainer, CoreError> {
    let mut container = IocContainer::new();
    
    // Complex cache configuration for production with Redis
    let production_redis_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("production_cache")
        .in_profile("production")
        .when_env("CACHE_PROVIDER", "redis")
        .when_feature("high-performance")
        .when(|| std::env::var("REDIS_CLUSTER_NODES").is_ok())
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, RedisCache>(production_redis_config);
    
    // Staging configuration
    let staging_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("staging_cache")
        .in_profile("staging")
        .when_env("CACHE_PROVIDER", "hybrid")
        .with_lifetime(ServiceScope::Singleton)
        .config();
    container.with_implementation::<dyn Cache, HybridCache>(staging_config);
    
    // Development fallback
    let dev_config = AdvancedBindingBuilder::<dyn Cache>::new()
        .named("dev_cache")
        .in_profile("development")
        .config();
    container.with_implementation::<dyn Cache, MemoryCache>(dev_config);
    
    container.build()?;
    Ok(container)
}

/// Example usage demonstration
pub fn demonstrate_advanced_binding_features() -> Result<(), CoreError> {
    println!("=== Advanced Binding Features Demo ===\n");
    
    // Example 1: Environment-based binding
    println!("1. Environment-based binding:");
    std::env::set_var("CACHE_PROVIDER", "redis");
    let container1 = example_environment_based_binding()?;
    if let Ok(cache) = container1.resolve_named::<RedisCache>("redis") {
        cache.set("test_key", "test_value".to_string()).ok();
    }
    std::env::remove_var("CACHE_PROVIDER");
    
    // Example 2: Feature flag binding
    println!("\n2. Feature flag binding:");
    std::env::set_var("FEATURE_CLOUD-STORAGE", "1");
    let container2 = example_feature_flag_binding()?;
    if let Ok(storage) = container2.resolve::<S3Storage>() {
        storage.store("test.txt", b"test data").ok();
    }
    std::env::remove_var("FEATURE_CLOUD-STORAGE");
    
    // Example 3: Profile-based binding
    println!("\n3. Profile-based binding:");
    std::env::set_var("PROFILE", "production");
    let container3 = example_profile_based_binding()?;
    if let Ok(email) = container3.resolve_named::<SmtpEmailService>("production_email") {
        email.send_email("user@example.com", "Test", "Hello World").ok();
    }
    std::env::remove_var("PROFILE");
    
    // Example 4: Factory patterns
    println!("\n4. Factory patterns:");
    let container4 = example_factory_patterns()?;
    if let Ok(cache) = container4.resolve::<RedisCache>() {
        cache.set("lazy_key", "lazy_value".to_string()).ok();
    }
    
    // Example 5: Service statistics
    println!("\n5. Service statistics:");
    let stats = container4.get_statistics();
    println!("Total services: {}", stats.total_services);
    println!("Singleton services: {}", stats.singleton_services);
    println!("Cached instances: {}", stats.cached_instances);
    
    // Example 6: Service validation
    println!("\n6. Service validation:");
    match container4.validate_all_services() {
        Ok(()) => println!("All services are valid!"),
        Err(errors) => println!("Validation errors: {}", errors.len()),
    }
    
    println!("\n=== Demo Complete ===");
    Ok(())
}

#[cfg(test)]
mod example_tests {
    use super::*;

    #[test]
    fn test_environment_based_example() {
        std::env::set_var("CACHE_PROVIDER", "memory");
        let result = example_environment_based_binding();
        assert!(result.is_ok());
        std::env::remove_var("CACHE_PROVIDER");
    }

    #[test]
    fn test_feature_flag_example() {
        let result = example_feature_flag_binding();
        assert!(result.is_ok());
    }

    #[test]
    fn test_profile_based_example() {
        std::env::set_var("PROFILE", "development");
        let result = example_profile_based_binding();
        assert!(result.is_ok());
        std::env::remove_var("PROFILE");
    }

    #[test]
    fn test_factory_patterns_example() {
        let result = example_factory_patterns();
        assert!(result.is_ok());
    }

    #[test]
    fn test_demonstration() {
        let result = demonstrate_advanced_binding_features();
        assert!(result.is_ok());
    }
}