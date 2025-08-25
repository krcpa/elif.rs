//! API demonstration for the new IoC container
//! This shows the target usage patterns defined in issue #281

#[cfg(test)]
mod tests {
    use crate::container::{IocContainerBuilder, ServiceBinder};
    use crate::errors::CoreError;
    use std::sync::Arc;

    // Example traits
    trait UserRepository: Send + Sync {
        fn find(&self, id: u32) -> Option<String>;
    }

    trait EmailService: Send + Sync {
        fn send(&self, to: &str, message: &str) -> Result<(), String>;
    }

    trait Logger: Send + Sync {
        fn log(&self, level: &str, message: &str);
    }

    // Example implementations
    #[derive(Default)]
    struct PostgresUserRepository;

    unsafe impl Send for PostgresUserRepository {}
    unsafe impl Sync for PostgresUserRepository {}

    impl UserRepository for PostgresUserRepository {
        fn find(&self, id: u32) -> Option<String> {
            Some(format!("User {}", id))
        }
    }

    #[derive(Default)]
    struct SmtpEmailService;

    unsafe impl Send for SmtpEmailService {}
    unsafe impl Sync for SmtpEmailService {}

    impl EmailService for SmtpEmailService {
        fn send(&self, _to: &str, _message: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FileLogger;

    unsafe impl Send for FileLogger {}
    unsafe impl Sync for FileLogger {}

    impl Logger for FileLogger {
        fn log(&self, _level: &str, _message: &str) {
            // Log to file
        }
    }

    // Service with dependencies (simplified without constructor injection for now)
    #[derive(Default)]
    struct UserService;

    unsafe impl Send for UserService {}
    unsafe impl Sync for UserService {}

    impl UserService {
        #[allow(dead_code)]
        pub fn create_user(&self, _name: &str) -> Result<u32, String> {
            // In full implementation, this would use injected repository and email service
            Ok(42)
        }
    }

    /// Demonstrates the target API from issue #281
    #[test]
    fn test_target_ioc_api() {
        // Target API usage from the issue
        let mut builder = IocContainerBuilder::new();

        // Interface to implementation binding
        builder
            .bind::<PostgresUserRepository, PostgresUserRepository>() // Will be interface binding
            .bind_singleton::<SmtpEmailService, SmtpEmailService>()
            .bind_factory::<FileLogger, _, _>(|| Ok(FileLogger::default()));

        let container = builder.build().unwrap();

        // Resolution works
        let repo = container.resolve::<PostgresUserRepository>().unwrap();
        let email = container.resolve::<SmtpEmailService>().unwrap();
        let logger = container.resolve::<FileLogger>().unwrap();

        // Validate functionality
        assert!(repo.find(1).is_some());
        assert!(email.send("test@example.com", "Hello").is_ok());
        logger.log("info", "Test message");

        // Validate singleton behavior
        let email2 = container.resolve::<SmtpEmailService>().unwrap();
        assert!(Arc::ptr_eq(&email, &email2));
    }

    /// Demonstrates named services as specified in the issue
    #[test]
    fn test_named_services_api() {
        let mut builder = IocContainerBuilder::new();

        // Named services for multiple implementations
        builder
            .bind_named::<PostgresUserRepository, PostgresUserRepository>("primary")
            .bind_named::<PostgresUserRepository, PostgresUserRepository>("backup");

        let container = builder.build().unwrap();

        // Resolve by name
        let primary = container
            .resolve_named::<PostgresUserRepository>("primary")
            .unwrap();
        let backup = container
            .resolve_named::<PostgresUserRepository>("backup")
            .unwrap();

        assert!(primary.find(1).is_some());
        assert!(backup.find(1).is_some());

        // Different instances for different names
        assert!(!Arc::ptr_eq(&primary, &backup));
    }

    /// Demonstrates error handling as required by the issue
    #[test]
    fn test_error_handling_api() {
        let container = IocContainerBuilder::new().build().unwrap();

        // Service not found
        let result = container.resolve::<UserService>();
        assert!(result.is_err());

        match result {
            Err(CoreError::ServiceNotFound { service_type }) => {
                assert!(service_type.contains("UserService"));
                // Verify we're NOT getting "unknown" anymore
                assert!(!service_type.contains("unknown"));
            }
            _ => panic!("Expected ServiceNotFound error"),
        }

        // Named service not found
        let result = container.resolve_named::<UserService>("nonexistent");
        assert!(result.is_err());

        match result {
            Err(CoreError::ServiceNotFound { service_type }) => {
                assert!(service_type.contains("UserService"));
                assert!(service_type.contains("nonexistent"));
                // Verify we're NOT getting "unknown" anymore
                assert!(!service_type.contains("unknown"));
            }
            _ => panic!("Expected ServiceNotFound error"),
        }
    }

    /// Demonstrates container validation as specified
    #[test]
    fn test_container_validation() {
        let mut builder = IocContainerBuilder::new();

        builder
            .bind::<UserService, UserService>()
            .bind::<SmtpEmailService, SmtpEmailService>()
            .bind::<FileLogger, FileLogger>();

        let container = builder.build().unwrap();

        // Validation should pass
        assert!(container.validate().is_ok());

        // Service count
        assert_eq!(container.service_count(), 3);

        // Contains checks
        assert!(container.contains::<UserService>());
        assert!(container.contains::<SmtpEmailService>());
        assert!(container.contains::<FileLogger>());
        assert!(!container.contains::<String>());
    }

    /// Demonstrates performance characteristics
    #[test]
    fn test_performance_characteristics() {
        let mut builder = IocContainerBuilder::new();

        // Add multiple services
        for i in 0..100 {
            builder.bind_named::<FileLogger, FileLogger>(&format!("logger_{}", i));
        }

        let container = builder.build().unwrap();

        // Resolution should be fast
        let start = std::time::Instant::now();
        for i in 0..100 {
            let _logger = container
                .resolve_named::<FileLogger>(&format!("logger_{}", i))
                .unwrap();
        }
        let duration = start.elapsed();

        // Should resolve 100 services quickly (< 10ms)
        assert!(
            duration.as_millis() < 10,
            "Resolution took too long: {:?}",
            duration
        );

        assert_eq!(container.service_count(), 100);
    }
}
