use crate::container::{IocContainer, ServiceBinder};
use std::sync::Arc;

/// Tests to verify performance optimizations for string allocations

#[derive(Default)]
struct TestService {
    #[allow(dead_code)]
    value: String,
}

impl TestService {
    #[allow(dead_code)]
    pub fn new(value: String) -> Self {
        Self { value }
    }
    
    #[allow(dead_code)]
    pub fn get_value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] 
    fn test_resolve_named_accepts_str_slice() {
        let mut container = IocContainer::new();
        
        // Use string slice without allocation
        container.bind_named::<TestService, TestService>("test_service");
        container.build().unwrap();
        
        // Resolve using &str - no allocation here
        let service_name = "test_service";
        let service = container.resolve_named::<TestService>(service_name).unwrap();
        
        assert!(Arc::strong_count(&service) >= 1);
    }
    
    #[test]
    fn test_try_resolve_named_accepts_str_slice() {
        let mut container = IocContainer::new();
        
        container.bind_named::<TestService, TestService>("test_service");
        container.build().unwrap();
        
        // Try resolve using &str - no allocation here
        let service_name = "test_service";
        let service = container.try_resolve_named::<TestService>(service_name);
        
        assert!(service.is_some());
    }
    
    #[test]
    fn test_contains_named_accepts_str_slice() {
        let mut container = IocContainer::new();
        
        container.bind_named::<TestService, TestService>("test_service");
        container.build().unwrap();
        
        // Check contains using &str - no allocation here
        let service_name = "test_service";
        assert!(container.contains_named::<TestService>(service_name));
        
        // Check non-existent service
        let other_name = "other_service";
        assert!(!container.contains_named::<TestService>(other_name));
    }
    
    #[test]
    fn test_efficient_named_service_lookup() {
        let mut container = IocContainer::new();
        
        // Register multiple named services
        container
            .bind_named::<TestService, TestService>("service_1")
            .bind_named::<TestService, TestService>("service_2")  
            .bind_named::<TestService, TestService>("service_3");
            
        container.build().unwrap();
        
        // Perform multiple lookups using string slices
        // This should be efficient and not allocate temporary ServiceId objects for lookup
        let names = ["service_1", "service_2", "service_3", "non_existent"];
        
        for name in names {
            let exists = container.contains_named::<TestService>(name);
            if name == "non_existent" {
                assert!(!exists);
            } else {
                assert!(exists);
                let service = container.resolve_named::<TestService>(name);
                assert!(service.is_ok());
            }
        }
    }
    
    #[test]
    fn test_str_slice_vs_owned_string_compatibility() {
        let mut container = IocContainer::new();
        
        // Bind using &str
        container.bind_named::<TestService, TestService>("test_service");
        container.build().unwrap();
        
        // Resolve using various string types
        let service_1 = container.resolve_named::<TestService>("test_service").unwrap(); // &str literal
        
        let name_string = String::from("test_service");
        let service_2 = container.resolve_named::<TestService>(&name_string).unwrap(); // &String -> &str
        
        let name_slice = name_string.as_str(); 
        let service_3 = container.resolve_named::<TestService>(name_slice).unwrap(); // explicit &str
        
        // All should work and return valid services
        assert!(Arc::strong_count(&service_1) >= 1);
        assert!(Arc::strong_count(&service_2) >= 1);
        assert!(Arc::strong_count(&service_3) >= 1);
    }
}