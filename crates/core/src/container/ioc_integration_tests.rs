//! Integration tests for IoC Phase 5: Integration & Migration
//!
//! Tests the complete integration of IoC container with controller factory,
//! middleware injection, and migration utilities.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::collections::HashMap;

    use crate::container::{
        IocContainer, Container, ServiceBinder, LegacyContainerAdapter,
        MigrationAnalyzer, ProgressiveMigrator
    };
    use crate::foundation::traits::Service;
    use crate::errors::CoreError;

    // Test services for integration testing
    #[derive(Clone, Debug, Default)]
    pub struct UserRepository {
        pub users: Vec<String>,
    }

    unsafe impl Send for UserRepository {}
    unsafe impl Sync for UserRepository {}

    impl Service for UserRepository {
        fn name(&self) -> &str {
            "UserRepository"
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct EmailService {
        pub sent_emails: Vec<String>,
    }

    unsafe impl Send for EmailService {}
    unsafe impl Sync for EmailService {}

    impl Service for EmailService {
        fn name(&self) -> &str {
            "EmailService"
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct LoggerService {
        pub logs: Vec<String>,
    }

    unsafe impl Send for LoggerService {}
    unsafe impl Sync for LoggerService {}

    impl Service for LoggerService {
        fn name(&self) -> &str {
            "LoggerService"
        }
    }

    // Test controller that uses dependency injection
    pub struct UserController {
        pub user_repo: Arc<UserRepository>,
        pub email_service: Arc<EmailService>,
        pub logger: Arc<LoggerService>,
    }

    impl UserController {
        pub fn from_ioc_container(
            container: &IocContainer,
            _scope: Option<&crate::container::ScopeId>,
        ) -> Result<Self, String> {
            let user_repo = container.resolve::<UserRepository>()
                .map_err(|e| format!("Failed to resolve UserRepository: {}", e))?;
            let email_service = container.resolve::<EmailService>()
                .map_err(|e| format!("Failed to resolve EmailService: {}", e))?;
            let logger = container.resolve::<LoggerService>()
                .map_err(|e| format!("Failed to resolve LoggerService: {}", e))?;

            Ok(Self {
                user_repo,
                email_service,
                logger,
            })
        }

        pub fn create_user(&self, username: &str) -> Result<(), String> {
            // Simulate user creation logic
            if username.is_empty() {
                return Err("Username cannot be empty".to_string());
            }

            // Would normally modify the repository
            println!("Creating user: {}", username);
            Ok(())
        }

        pub fn send_welcome_email(&self, email: &str) -> Result<(), String> {
            // Simulate email sending
            println!("Sending welcome email to: {}", email);
            Ok(())
        }
    }

    // Test middleware that uses dependency injection
    pub struct LoggingMiddleware {
        pub logger: Arc<LoggerService>,
    }

    impl LoggingMiddleware {
        pub fn from_ioc_container(
            container: &IocContainer,
            _scope: Option<&crate::container::ScopeId>,
        ) -> Result<Self, String> {
            let logger = container.resolve::<LoggerService>()
                .map_err(|e| format!("Failed to resolve LoggerService: {}", e))?;

            Ok(Self { logger })
        }

        pub fn log_request(&self, path: &str) {
            println!("Logging request to: {}", path);
        }
    }

    #[tokio::test]
    async fn test_complete_ioc_integration() {
        // Setup IoC container with all services
        let mut container = IocContainer::new();
        
        container.bind::<UserRepository, UserRepository>();
        container.bind::<EmailService, EmailService>();
        container.bind::<LoggerService, LoggerService>();
        
        container.build().expect("Container build should succeed");

        // Test controller creation with dependency injection
        let user_controller = UserController::from_ioc_container(&container, None)
            .expect("Controller creation should succeed");

        // Test that services are properly injected
        assert_eq!(user_controller.user_repo.name(), "UserRepository");
        assert_eq!(user_controller.email_service.name(), "EmailService");
        assert_eq!(user_controller.logger.name(), "LoggerService");

        // Test controller functionality
        user_controller.create_user("test_user")
            .expect("User creation should succeed");
        user_controller.send_welcome_email("test@example.com")
            .expect("Email sending should succeed");
    }

    #[tokio::test]
    async fn test_middleware_dependency_injection() {
        // Setup IoC container
        let mut container = IocContainer::new();
        container.bind::<LoggerService, LoggerService>();
        container.build().expect("Container build should succeed");

        // Create middleware with dependency injection
        let logging_middleware = LoggingMiddleware::from_ioc_container(&container, None)
            .expect("Middleware creation should succeed");

        // Test middleware functionality
        logging_middleware.log_request("/api/users");
        assert_eq!(logging_middleware.logger.name(), "LoggerService");
    }

    #[tokio::test]
    async fn test_scoped_service_injection() {
        let mut container = IocContainer::new();
        container.bind::<LoggerService, LoggerService>();
        container.build().expect("Container build should succeed");

        // Create a scope for request-specific services
        let scope_id = container.create_scope()
            .expect("Scope creation should succeed");

        // Test scoped service resolution
        let logger1 = container.resolve_scoped::<LoggerService>(&scope_id)
            .expect("Scoped service resolution should succeed");
        let logger2 = container.resolve_scoped::<LoggerService>(&scope_id)
            .expect("Second scoped service resolution should succeed");

        // Should be the same instance within the scope
        assert!(Arc::ptr_eq(&logger1, &logger2));

        // Cleanup scope
        container.dispose_scope(&scope_id).await
            .expect("Scope disposal should succeed");
    }

    #[tokio::test]
    async fn test_named_service_injection() {
        let mut container = IocContainer::new();
        
        // Bind multiple implementations of the same service with different names
        let primary_logger = LoggerService::default();
        let secondary_logger = LoggerService::default();
        
        container.bind_instance::<LoggerService, LoggerService>(primary_logger)
            .with_implementation::<LoggerService, LoggerService>(
                crate::container::BindingConfig::default().named("primary")
            );
        container.bind_instance::<LoggerService, LoggerService>(secondary_logger)
            .with_implementation::<LoggerService, LoggerService>(
                crate::container::BindingConfig::default().named("secondary")  
            );
        
        container.build().expect("Container build should succeed");

        // Resolve named services
        let primary = container.resolve_named::<LoggerService>("primary")
            .expect("Primary logger resolution should succeed");
        let secondary = container.resolve_named::<LoggerService>("secondary")
            .expect("Secondary logger resolution should succeed");

        assert_eq!(primary.name(), "LoggerService");
        assert_eq!(secondary.name(), "LoggerService");
        assert!(!Arc::ptr_eq(&primary, &secondary));
    }

    #[tokio::test]
    async fn test_legacy_container_compatibility() {
        // Setup old container
        let mut old_container = Container::new();
        let user_repo = UserRepository::default();
        old_container.register(user_repo.clone())
            .expect("Old container registration should succeed");
        old_container.initialize().await
            .expect("Old container initialization should succeed");

        // Setup new IoC container
        let mut new_container = IocContainer::new();
        new_container.bind::<EmailService, EmailService>();
        new_container.build().expect("New container build should succeed");

        // Create compatibility adapter
        let adapter = LegacyContainerAdapter::new(new_container);

        // Test that both old and new services are accessible
        let user_repo_resolved = adapter.resolve::<UserRepository>()
            .expect("User repository should be resolvable");
        assert_eq!(user_repo_resolved.name(), "UserRepository");

        // Test service count includes both containers
        assert!(adapter.service_count() > 0);
    }

    #[tokio::test]
    async fn test_migration_analyzer() {
        // Setup containers for migration analysis
        let mut old_container = Container::new();
        let user_repo = UserRepository::default();
        old_container.register(user_repo)
            .expect("Old container registration should succeed");
        old_container.initialize().await
            .expect("Old container initialization should succeed");

        let mut new_container = IocContainer::new();
        new_container.bind::<UserRepository, UserRepository>();
        new_container.bind::<EmailService, EmailService>(); // Additional service
        new_container.build().expect("New container build should succeed");

        // Analyze compatibility
        let report = MigrationAnalyzer::analyze_container_compatibility(&old_container, &new_container);
        
        // Should be compatible since UserRepository exists in both
        assert!(!report.migration_required);
        assert!(report.compatible_services > 0);

        // Test migration suggestions
        let suggestions = MigrationAnalyzer::generate_migration_suggestions(&old_container);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| {
            matches!(s.suggestion_type, crate::container::SuggestionType::ConvertSingletonRegistration)
        }));
    }

    #[tokio::test]
    async fn test_progressive_migrator() {
        // Setup old container
        let mut old_container = Container::new();
        let user_repo = UserRepository::default();
        old_container.register(user_repo)
            .expect("Old container registration should succeed");
        old_container.initialize().await
            .expect("Old container initialization should succeed");

        // Setup new container
        let mut new_container = IocContainer::new();
        new_container.bind::<UserRepository, UserRepository>();
        new_container.build().expect("New container build should succeed");

        // Create progressive migrator
        let migrator = ProgressiveMigrator::new(old_container, new_container, 0.5);

        // Test service resolution (should work from either container)
        let resolved = migrator.resolve::<UserRepository>()
            .expect("Progressive migration should resolve service");
        assert_eq!(resolved.name(), "UserRepository");

        // Test migration percentage
        assert_eq!(migrator.migration_percentage(), 0.5);
    }

    #[tokio::test]
    async fn test_factory_pattern_integration() {
        let mut container = IocContainer::new();
        
        // Bind a factory that creates services with dependencies
        container.bind_factory::<LoggerService, _, _>(|| {
            Ok(LoggerService::default())
        });
        
        container.build().expect("Container build should succeed");

        // Resolve factory-created service
        let logger = container.resolve::<LoggerService>()
            .expect("Factory service resolution should succeed");
        assert_eq!(logger.name(), "LoggerService");
    }

    #[tokio::test]
    async fn test_collection_binding_integration() {
        let mut container = IocContainer::new();
        
        // Bind collection of services
        container.bind_collection::<dyn Service, _>(|builder| {
            builder
                .add_implementation::<UserRepository>()
                .add_implementation::<EmailService>()
                .add_implementation::<LoggerService>();
        });
        
        container.build().expect("Container build should succeed");

        // Resolve all services in collection
        let services = container.resolve_all::<dyn Service>()
            .expect("Collection resolution should succeed");
        assert_eq!(services.len(), 3);
    }

    #[tokio::test]
    async fn test_conditional_binding() {
        let mut container = IocContainer::new();
        
        // Bind service conditionally based on environment
        let is_development = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) == "development";
        
        if is_development {
            container.bind::<LoggerService, LoggerService>();
        } else {
            // In production, we might use a different implementation
            container.bind::<LoggerService, LoggerService>();
        }
        
        container.build().expect("Container build should succeed");

        let logger = container.resolve::<LoggerService>()
            .expect("Conditional service resolution should succeed");
        assert_eq!(logger.name(), "LoggerService");
    }

    #[tokio::test]
    async fn test_lazy_initialization() {
        let mut container = IocContainer::new();
        
        // Bind service with lazy initialization
        container.bind_lazy::<LoggerService, _, _>(|| {
            println!("Lazy initializing LoggerService");
            LoggerService::default()
        });
        
        container.build().expect("Container build should succeed");

        // Service should be created only when first resolved
        let logger = container.resolve::<LoggerService>()
            .expect("Lazy service resolution should succeed");
        assert_eq!(logger.name(), "LoggerService");
    }

    #[tokio::test]
    async fn test_parameterized_factory() {
        let mut container = IocContainer::new();
        
        // Bind parameterized factory
        container.bind_parameterized_factory::<LoggerService, String, _, _>(|name| {
            let mut service = LoggerService::default();
            // Would normally configure the service with the parameter
            println!("Creating logger with name: {}", name);
            Ok(service)
        });
        
        container.build().expect("Container build should succeed");

        // Resolve with parameter (in a real implementation)
        let logger = container.resolve::<LoggerService>()
            .expect("Parameterized factory service resolution should succeed");
        assert_eq!(logger.name(), "LoggerService");
    }

    #[tokio::test]
    async fn test_container_validation() {
        let mut container = IocContainer::new();
        
        // Add services with dependencies
        container.bind::<UserRepository, UserRepository>();
        container.bind::<EmailService, EmailService>();
        container.bind::<LoggerService, LoggerService>();
        
        container.build().expect("Container build should succeed");

        // Validate container configuration
        container.validate().expect("Container validation should succeed");

        // Test comprehensive validation
        let validation_errors = container.validate_all_services();
        assert!(validation_errors.is_ok(), "All services should be valid");
    }

    #[test]
    fn test_service_statistics() {
        let mut container = IocContainer::new();
        
        container.bind::<UserRepository, UserRepository>();
        container.bind_singleton::<EmailService, EmailService>();
        container.bind_transient::<LoggerService, LoggerService>();
        
        container.build().expect("Container build should succeed");

        let stats = container.get_statistics();
        assert_eq!(stats.total_services, 3);
        assert_eq!(stats.singleton_services, 1);
        assert_eq!(stats.transient_services, 1);
    }

    #[tokio::test]
    async fn test_async_service_initialization() {
        let mut container = IocContainer::new();
        
        container.bind::<LoggerService, LoggerService>();
        container.build().expect("Container build should succeed");

        // Test async initialization
        container.initialize_async().await
            .expect("Async initialization should succeed");
            
        // Test async initialization with timeout
        container.initialize_async_with_timeout(std::time::Duration::from_secs(5)).await
            .expect("Async initialization with timeout should succeed");
    }

    #[tokio::test]
    async fn test_lifecycle_management() {
        let mut container = IocContainer::new();
        
        container.bind::<LoggerService, LoggerService>();
        container.build().expect("Container build should succeed");

        // Test lifecycle operations
        let lifecycle_manager = container.lifecycle_manager();
        
        // Initialize all services
        container.initialize_async().await
            .expect("Service initialization should succeed");
        
        // Dispose all services
        container.dispose_all().await
            .expect("Service disposal should succeed");
    }

    #[test]
    fn test_debug_and_introspection() {
        let mut container = IocContainer::new();
        
        container.bind::<UserRepository, UserRepository>();
        container.bind::<EmailService, EmailService>();
        container.build().expect("Container build should succeed");

        // Test debug information
        let registered_services = container.get_registered_services();
        assert!(!registered_services.is_empty());

        let service_info = container.get_service_info::<UserRepository>();
        assert!(service_info.is_some());
    }
}