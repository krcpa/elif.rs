//! Test for optional services combined with other injection attributes
//! This test verifies the fix for the bug where optional services combined
//! with named services would generate incorrect field types

use elif_http_derive::inject;
use std::sync::Arc;

// Mock services for testing
trait CacheService: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    #[allow(dead_code)]
    fn set(&self, key: &str, value: &str);
}

// Mock implementation
#[derive(Default)]
struct MockCacheService;
impl CacheService for MockCacheService {
    fn get(&self, key: &str) -> Option<String> {
        if key == "test" {
            Some("cached_value".to_string())
        } else {
            None
        }
    }

    fn set(&self, key: &str, value: &str) {
        println!("Cache set: {} = {}", key, value);
    }
}

impl elif_core::foundation::traits::FrameworkComponent for MockCacheService {}
impl elif_core::foundation::traits::Service for MockCacheService {}
impl Clone for MockCacheService {
    fn clone(&self) -> Self {
        MockCacheService
    }
}

// Test controller with optional named service
// This is the key test case: Optional<T> with a named service
#[inject(
    cache: Option<MockCacheService> = "redis_cache"
)]
pub struct OptionalNamedController;

impl OptionalNamedController {
    pub fn test_optional_named(&self) -> String {
        match &self.cache {
            Some(cache) => cache.get("test").unwrap_or("No cached value".to_string()),
            None => "No cache available".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_core::container::{IocContainer, ServiceBinder};

    #[test]
    fn test_optional_with_named_service() {
        // This test verifies that cache: Option<MockCacheService> = "redis_cache"
        // generates a field of type Option<Arc<MockCacheService>>
        // NOT Arc<MockCacheService> (which was the bug)

        let mut container = IocContainer::new();
        container.bind_named::<MockCacheService, MockCacheService>("redis_cache");
        container.build().unwrap();

        let controller = OptionalNamedController::from_ioc_container(&container, None).unwrap();

        // The field should be Option<Arc<MockCacheService>>
        assert!(controller.cache.is_some());
        let result = controller.test_optional_named();
        assert_eq!(result, "cached_value");
    }

    #[test]
    fn test_optional_named_service_missing() {
        // Test that optional named services can be None
        let mut container = IocContainer::new();
        container.build().unwrap();

        let controller = OptionalNamedController::from_ioc_container(&container, None).unwrap();

        // The field should be None when named service isn't registered
        assert!(controller.cache.is_none());
        assert_eq!(controller.test_optional_named(), "No cache available");
    }

    #[test]
    fn test_field_type_is_correct() {
        // This test verifies the actual field type at compile time
        // If this compiles, the field has the correct type
        let controller = OptionalNamedController {
            cache: Some(Arc::new(MockCacheService)),
        };

        // This assignment will only compile if the field type is Option<Arc<MockCacheService>>
        let _: Option<Arc<MockCacheService>> = controller.cache;
    }

    #[test]
    fn test_generated_from_ioc_uses_try_resolve_named() {
        // This test ensures the generated code uses try_resolve_named for optional named services
        // The test passing means the macro correctly generated code using try_resolve_named

        let mut container = IocContainer::new();
        // Register with a different name
        container.bind_named::<MockCacheService, MockCacheService>("memcached");
        container.build().unwrap();

        // Should succeed with None since "redis_cache" isn't registered
        let controller = OptionalNamedController::from_ioc_container(&container, None).unwrap();
        assert!(controller.cache.is_none());

        // Now register with the correct name
        let mut container2 = IocContainer::new();
        container2.bind_named::<MockCacheService, MockCacheService>("redis_cache");
        container2.build().unwrap();

        // Should succeed with Some since "redis_cache" is registered
        let controller2 = OptionalNamedController::from_ioc_container(&container2, None).unwrap();
        assert!(controller2.cache.is_some());
    }
}
