//! Integration tests for the IoC container implementation

#[cfg(test)]
mod tests {
    use crate::container::{IocContainerBuilder, ServiceBinder, DependencyGraph, ServiceScope};
    use crate::container::descriptor::ServiceDescriptor;
    use crate::errors::CoreError;
    use std::sync::Arc;

    // Test service hierarchy
    trait Logger: Send + Sync {
        fn log(&self, message: &str);
    }

    #[derive(Default)]
    struct FileLogger;
    
    unsafe impl Send for FileLogger {}
    unsafe impl Sync for FileLogger {}

    impl Logger for FileLogger {
        fn log(&self, message: &str) {
            println!("LOG: {}", message);
        }
    }

    trait Database: Send + Sync {
        fn query(&self, sql: &str) -> Vec<String>;
    }

    #[derive(Default)]
    struct PostgresDatabase;
    
    unsafe impl Send for PostgresDatabase {}
    unsafe impl Sync for PostgresDatabase {}

    impl Database for PostgresDatabase {
        fn query(&self, _sql: &str) -> Vec<String> {
            vec!["result1".to_string(), "result2".to_string()]
        }
    }

    #[derive(Default)]
    struct UserService {
        // These would be injected in the full implementation
    }
    
    unsafe impl Send for UserService {}
    unsafe impl Sync for UserService {}

    impl UserService {
        pub fn get_users(&self) -> Vec<String> {
            vec!["user1".to_string(), "user2".to_string()]
        }
    }

    #[test]
    fn test_complete_ioc_container_workflow() {
        // Create and configure container
        let mut builder = IocContainerBuilder::new();
        
        builder
            .bind_singleton::<FileLogger, FileLogger>()
            .bind::<PostgresDatabase, PostgresDatabase>()
            .bind_transient::<UserService, UserService>();
        
        // Build container (validates dependencies)
        let container = builder.build().unwrap();
        
        // Resolve services
        let logger = container.resolve::<FileLogger>().unwrap();
        let db = container.resolve::<PostgresDatabase>().unwrap();
        let service = container.resolve::<UserService>().unwrap();
        
        // Test functionality
        logger.log("Test message");
        let results = db.query("SELECT * FROM users");
        let users = service.get_users();
        
        assert_eq!(results.len(), 2);
        assert_eq!(users.len(), 2);
        
        // Test singleton behavior
        let logger2 = container.resolve::<FileLogger>().unwrap();
        assert!(Arc::ptr_eq(&logger, &logger2));
        
        // Test transient behavior
        let service2 = container.resolve::<UserService>().unwrap();
        assert!(!Arc::ptr_eq(&service, &service2));
    }

    #[test]
    fn test_dependency_graph_validation() {
        // Create descriptors with dependencies
        let logger_descriptor = ServiceDescriptor::bind::<FileLogger, FileLogger>()
            .with_lifetime(ServiceScope::Singleton)
            .build();
        
        let db_descriptor = ServiceDescriptor::bind::<PostgresDatabase, PostgresDatabase>()
            .with_lifetime(ServiceScope::Transient)
            .depends_on::<FileLogger>()
            .build();
        
        let service_descriptor = ServiceDescriptor::bind::<UserService, UserService>()
            .with_lifetime(ServiceScope::Transient)
            .depends_on::<PostgresDatabase>()
            .depends_on::<FileLogger>()
            .build();
        
        let descriptors = vec![service_descriptor, db_descriptor, logger_descriptor];
        
        // Build dependency graph
        let graph = DependencyGraph::build_from_descriptors(&descriptors);
        
        // Should not have cycles
        assert!(graph.detect_cycles().is_ok());
        
        // Test topological sort
        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        
        // Logger should come before Database and UserService
        let logger_pos = sorted.iter().position(|id| id.type_id == std::any::TypeId::of::<FileLogger>()).unwrap();
        let db_pos = sorted.iter().position(|id| id.type_id == std::any::TypeId::of::<PostgresDatabase>()).unwrap();
        let service_pos = sorted.iter().position(|id| id.type_id == std::any::TypeId::of::<UserService>()).unwrap();
        
        assert!(logger_pos < db_pos);
        assert!(logger_pos < service_pos);
        assert!(db_pos < service_pos);
    }

    #[test] 
    fn test_circular_dependency_detection() {
        // Create descriptors with circular dependencies
        let service_a_descriptor = ServiceDescriptor::bind::<UserService, UserService>()
            .depends_on_named::<PostgresDatabase>("B")
            .build();
        
        let service_b_descriptor = ServiceDescriptor::bind_named::<PostgresDatabase, PostgresDatabase>("B")
            .depends_on_named::<FileLogger>("C")
            .build();
        
        let service_c_descriptor = ServiceDescriptor::bind_named::<FileLogger, FileLogger>("C")
            .depends_on::<UserService>()
            .build();
        
        let descriptors = vec![service_a_descriptor, service_b_descriptor, service_c_descriptor];
        
        // Build dependency graph - should detect cycle
        let graph = DependencyGraph::build_from_descriptors(&descriptors);
        let result = graph.detect_cycles();
        
        assert!(result.is_err());
        
        if let Err(CoreError::CircularDependency { path, cycle_service }) = result {
            assert!(!path.is_empty());
            assert!(!cycle_service.is_empty());
        }
    }

    #[test]
    fn test_service_count_and_validation() {
        let mut builder = IocContainerBuilder::new();
        
        builder
            .bind::<FileLogger, FileLogger>()
            .bind::<PostgresDatabase, PostgresDatabase>()
            .bind::<UserService, UserService>();
        
        let container = builder.build().unwrap();
        
        // Test service count
        assert_eq!(container.service_count(), 3);
        
        // Test service presence
        assert!(container.contains::<FileLogger>());
        assert!(container.contains::<PostgresDatabase>());
        assert!(container.contains::<UserService>());
        assert!(!container.contains::<String>());
        
        // Test validation
        assert!(container.validate().is_ok());
    }

    #[test]
    fn test_factory_with_configuration() {
        let mut builder = IocContainerBuilder::new();
        
        // Factory that creates configured service
        builder.bind_factory::<PostgresDatabase, _, _>(|| {
            Ok(PostgresDatabase::default())
        });
        
        let container = builder.build().unwrap();
        
        let db = container.resolve::<PostgresDatabase>().unwrap();
        let results = db.query("SELECT 1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_error_handling() {
        let container = IocContainerBuilder::new().build().unwrap();
        
        // Service not found
        let result = container.resolve::<UserService>();
        assert!(result.is_err());
        
        if let Err(CoreError::ServiceNotFound { service_type }) = result {
            assert!(service_type.contains("UserService"));
        }
        
        // Named service not found
        let result = container.resolve_named::<UserService>("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_container_cannot_be_modified_after_build() {
        let mut container = IocContainerBuilder::new().build().unwrap();
        
        // This should panic
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            container.bind::<UserService, UserService>();
        })).expect_err("Should panic when trying to bind after build");
    }
}