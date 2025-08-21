//! Integration tests for the #[inject] macro
//! These tests verify that the generated code compiles and works correctly.

use elif_http_derive::inject;
use std::sync::Arc;

// Mock services for testing
trait UserService: Send + Sync {
    fn get_user(&self, id: u32) -> String;
}

trait EmailService: Send + Sync {  
    fn send_email(&self, to: &str, subject: &str) -> bool;
}

trait CacheService: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
}

// Mock implementations
struct MockUserService;
impl UserService for MockUserService {
    fn get_user(&self, id: u32) -> String {
        format!("User {}", id)
    }
}

// Implement required traits for container registration
impl elif_core::foundation::traits::FrameworkComponent for MockUserService {}
impl elif_core::foundation::traits::Service for MockUserService {}
impl Clone for MockUserService {
    fn clone(&self) -> Self {
        MockUserService
    }
}

struct MockEmailService;
impl EmailService for MockEmailService {
    fn send_email(&self, to: &str, subject: &str) -> bool {
        println!("Sending email to {} with subject: {}", to, subject);
        true
    }
}

impl elif_core::foundation::traits::FrameworkComponent for MockEmailService {}
impl elif_core::foundation::traits::Service for MockEmailService {}
impl Clone for MockEmailService {
    fn clone(&self) -> Self {
        MockEmailService
    }
}

struct MockCacheService;
impl CacheService for MockCacheService {
    fn get(&self, key: &str) -> Option<String> {
        if key == "user" {
            Some("cached_value".to_string())
        } else {
            None
        }
    }
    
}

impl elif_core::foundation::traits::FrameworkComponent for MockCacheService {}
impl elif_core::foundation::traits::Service for MockCacheService {}
impl Clone for MockCacheService {
    fn clone(&self) -> Self {
        MockCacheService
    }
}

// Test controller with basic service injection
#[inject(user_service: MockUserService, email_service: MockEmailService)]
pub struct TestController;

impl TestController {
    pub fn handle_user_request(&self, user_id: u32) -> String {
        let user = self.user_service.get_user(user_id);
        let sent = self.email_service.send_email("user@example.com", "Welcome");
        format!("User: {}, Email sent: {}", user, sent)
    }
}

// Test controller with optional service
#[inject(user_service: MockUserService, cache_service: Option<MockCacheService>)]
pub struct OptionalServiceController;

impl OptionalServiceController {
    pub fn get_cached_user(&self, user_id: u32) -> String {
        if let Some(cache) = &self.cache_service {
            if let Some(cached) = cache.get("user") {
                return cached;
            }
        }
        
        self.user_service.get_user(user_id)
    }
}

// Test controller with multiple services
#[inject(user_service: MockUserService, email_service: MockEmailService, cache_service: MockCacheService)]
pub struct MultiServiceController;

#[cfg(test)]
mod tests {
    use super::*;
    
    
    #[test]
    fn test_inject_macro_generates_fields() {
        // This test verifies that the macro generates the expected struct fields
        
        // Create mock services as concrete types
        let user_service = Arc::new(MockUserService);
        let email_service = Arc::new(MockEmailService);
        
        // Create controller (manually for now - in real implementation this would use from_container)
        let controller = TestController {
            user_service,
            email_service,
        };
        
        // Test that the injected services work
        let result = controller.handle_user_request(123);
        assert!(result.contains("User 123"));
        assert!(result.contains("Email sent: true"));
    }
    
    #[test]
    fn test_optional_service_injection() {
        // Test with optional service present
        let user_service = Arc::new(MockUserService);
        let cache_service = Some(Arc::new(MockCacheService));
        
        let controller = OptionalServiceController {
            user_service: user_service.clone(),
            cache_service,
        };
        
        let result = controller.get_cached_user(456);
        assert_eq!(result, "User 456"); // Since cache doesn't have "user" key
        
        // Test with optional service absent
        let controller_no_cache = OptionalServiceController {
            user_service,
            cache_service: None,
        };
        
        let result = controller_no_cache.get_cached_user(789);
        assert_eq!(result, "User 789");
    }
    
    #[test]
    fn test_generated_from_container_method_exists() {
        // This test just verifies that the from_container method is generated
        // In a real test, we'd set up a proper container and call it
        
        // Verify the method exists (this will compile-fail if it doesn't)
        let _method_exists = TestController::from_container;
        let _optional_method_exists = OptionalServiceController::from_container;
        let _multi_service_method_exists = MultiServiceController::from_container;
    }
    
    #[test]
    fn test_from_container_runtime_behavior() {
        // This test verifies the actual runtime behavior of from_container
        use elif_core::container::Container;
        
        // Create and configure container
        let mut container = Container::new();
        
        // Register services
        container.register_singleton(MockUserService).unwrap();
        container.register_singleton(MockEmailService).unwrap();
        container.register_singleton(MockCacheService).unwrap();
        
        // Test basic controller creation
        let controller = TestController::from_container(&container).unwrap();
        let result = controller.handle_user_request(999);
        assert_eq!(result, "User: User 999, Email sent: true");
        
        // Test optional service controller with service present
        let optional_controller = OptionalServiceController::from_container(&container).unwrap();
        let cached_result = optional_controller.get_cached_user(123);
        assert_eq!(cached_result, "User 123"); // Not cached, so fetches from user service
        
        // Test multi-service controller
        let multi_controller = MultiServiceController::from_container(&container).unwrap();
        // Just verify it was created successfully
        assert!(multi_controller.user_service.get_user(1) == "User 1");
    }
    
    #[test]
    fn test_from_container_missing_service() {
        // Test error handling when required service is missing
        use elif_core::container::Container;
        
        // Create empty container
        let container = Container::new();
        
        // Try to create controller without registering services
        let result = TestController::from_container(&container);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.contains("Failed to inject service"));
        }
    }
    
    #[test]
    fn test_from_container_optional_service_missing() {
        // Test that optional services can be None
        use elif_core::container::Container;
        
        let mut container = Container::new();
        // Only register required service, not optional cache
        container.register_singleton(MockUserService).unwrap();
        
        // Should succeed with cache_service as None
        let controller = OptionalServiceController::from_container(&container).unwrap();
        assert!(controller.cache_service.is_none());
        
        // Should still work without cache
        let result = controller.get_cached_user(456);
        assert_eq!(result, "User 456");
    }
}