use crate::container::descriptor::ServiceId;
use crate::errors::CoreError;
use std::any::Any;
use std::sync::Arc;

/// Trait for services that can be automatically resolved by the IoC container
pub trait Injectable: Send + Sync + 'static {
    /// Get the list of dependencies this service requires
    fn dependencies() -> Vec<ServiceId>;

    /// Create an instance of this service, resolving dependencies from the container
    fn create<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError>
    where
        Self: Sized;
}

/// Trait for resolving dependencies during service construction
/// Note: This trait is not dyn compatible due to generic methods
/// Implementors must provide concrete container instances
pub trait DependencyResolver {
    /// Resolve a service by type
    fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError>;

    /// Resolve a named service
    fn resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Result<Arc<T>, CoreError>;

    /// Try to resolve a service, returning None if not found
    fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;

    /// Try to resolve a named service, returning None if not found
    fn try_resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>>;
}

/// Automatically derive Injectable for types with constructor injection
pub trait AutowireService: Send + Sync + 'static {
    /// The concrete type this service implements
    type ServiceType: Injectable;

    /// Create an instance with auto-wiring
    fn autowire<R: DependencyResolver>(resolver: &R) -> Result<Self::ServiceType, CoreError>;
}

/// Helper trait for extracting constructor parameters
pub trait ConstructorParameter: Send + Sync + 'static {
    /// Get the service ID for this parameter type
    fn service_id() -> ServiceId;

    /// Resolve this parameter from the container
    fn resolve<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError>
    where
        Self: Sized;
}

/// Implementation for Arc<T> parameters
impl<T: Send + Sync + 'static> ConstructorParameter for Arc<T> {
    fn service_id() -> ServiceId {
        ServiceId::of::<T>()
    }

    fn resolve<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError> {
        resolver.resolve::<T>()
    }
}

// Note: Cannot implement for Arc<dyn T> directly due to Rust limitations
// Instead, users should implement for specific trait objects like Arc<dyn Repository>

/// Implementation for Option<Arc<T>> parameters (optional dependencies)
impl<T: Send + Sync + 'static> ConstructorParameter for Option<Arc<T>> {
    fn service_id() -> ServiceId {
        ServiceId::of::<T>()
    }

    fn resolve<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError> {
        Ok(resolver.try_resolve::<T>())
    }
}

// Note: Cannot implement for Option<Arc<dyn T>> directly due to Rust limitations
// Instead, users should implement for specific trait objects like Option<Arc<dyn Logger>>

/// Metadata about a constructor parameter
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// The type name of the parameter
    pub type_name: &'static str,
    /// The service ID this parameter requires
    pub service_id: ServiceId,
    /// Whether this parameter is optional
    pub is_optional: bool,
}

/// Metadata about a service constructor
#[derive(Debug, Clone)]
pub struct ConstructorInfo {
    /// The service type name
    pub service_type: &'static str,
    /// Parameters required by the constructor
    pub parameters: Vec<ParameterInfo>,
}

/// Trait for services that provide constructor metadata
pub trait ConstructorMetadata {
    /// Get information about the constructor
    fn constructor_info() -> ConstructorInfo;
}

/// Factory trait for creating services with dependency injection
pub trait InjectableFactory<T: Send + Sync + 'static> {
    /// Create an instance with the given parameters
    fn create_instance<R: DependencyResolver>(resolver: &R) -> Result<T, CoreError>;

    /// Get the dependencies this factory requires
    fn dependencies() -> Vec<ServiceId>;
}

/// Helper macro to implement Injectable for services with constructor injection
/// This would typically be provided by a proc macro, but we'll implement it manually for now
pub trait InjectableHelper {
    /// Extract dependencies from constructor signature
    fn extract_dependencies() -> Vec<ServiceId>;

    /// Create instance by resolving constructor parameters
    fn create_with_resolver<R: DependencyResolver>(
        resolver: &R,
    ) -> Result<Box<dyn Any + Send + Sync>, CoreError>;
}

/// Implementation for services with no dependencies
impl Injectable for () {
    fn dependencies() -> Vec<ServiceId> {
        Vec::new()
    }

    fn create<R: DependencyResolver>(_resolver: &R) -> Result<Self, CoreError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Mock resolver for testing
    struct MockResolver {
        services: HashMap<ServiceId, Box<dyn Any + Send + Sync>>,
    }

    impl MockResolver {
        fn new() -> Self {
            Self {
                services: HashMap::new(),
            }
        }

        fn register<T: Send + Sync + 'static>(&mut self, instance: T) {
            let service_id = ServiceId::of::<T>();
            self.services
                .insert(service_id, Box::new(Arc::new(instance)));
        }

        #[allow(dead_code)]
        fn register_trait<T: ?Sized + Send + Sync + 'static>(&mut self, instance: Arc<T>) {
            let service_id = ServiceId::of::<T>();
            self.services.insert(service_id, Box::new(instance));
        }
    }

    impl DependencyResolver for MockResolver {
        fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, CoreError> {
            let service_id = ServiceId::of::<T>();
            let instance =
                self.services
                    .get(&service_id)
                    .ok_or_else(|| CoreError::ServiceNotFound {
                        service_type: std::any::type_name::<T>().to_string(),
                    })?;

            let arc_instance =
                instance
                    .downcast_ref::<Arc<T>>()
                    .ok_or_else(|| CoreError::ServiceNotFound {
                        service_type: std::any::type_name::<T>().to_string(),
                    })?;

            Ok(arc_instance.clone())
        }

        fn resolve_named<T: Send + Sync + 'static>(
            &self,
            _name: &str,
        ) -> Result<Arc<T>, CoreError> {
            // For now, just delegate to resolve
            self.resolve::<T>()
        }

        fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
            self.resolve::<T>().ok()
        }

        fn try_resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
            self.resolve_named::<T>(name).ok()
        }
    }

    // Test service interfaces
    #[allow(dead_code)]
    trait Repository: Send + Sync {
        fn find(&self, id: u32) -> Option<String>;
    }

    #[allow(dead_code)]
    trait EmailService: Send + Sync {
        fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
    }

    #[allow(dead_code)]
    trait Logger: Send + Sync {
        fn log(&self, message: &str);
    }

    // Test implementations
    #[derive(Default)]
    struct PostgresRepository;

    impl Repository for PostgresRepository {
        fn find(&self, _id: u32) -> Option<String> {
            Some("user data".to_string())
        }
    }

    #[derive(Default)]
    struct SmtpEmailService;

    impl EmailService for SmtpEmailService {
        fn send(&self, _to: &str, _subject: &str, _body: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FileLogger;

    impl Logger for FileLogger {
        fn log(&self, _message: &str) {
            // Log to file
        }
    }

    // Service with dependencies - would typically use a derive macro
    struct UserService {
        repository: Arc<PostgresRepository>,
        email_service: Arc<SmtpEmailService>,
        logger: Option<Arc<FileLogger>>, // Optional dependency
    }

    impl UserService {
        pub fn new(
            repository: Arc<PostgresRepository>,
            email_service: Arc<SmtpEmailService>,
            logger: Option<Arc<FileLogger>>,
        ) -> Self {
            Self {
                repository,
                email_service,
                logger,
            }
        }

        pub fn create_user(&self, name: &str) -> Result<String, String> {
            if let Some(logger) = &self.logger {
                logger.log(&format!("Creating user: {}", name));
            }

            // Use repository to check for existing user
            let _existing = self.repository.find(1);

            let user_id = format!("user_{}", name);
            self.email_service.send(
                &format!("{}@example.com", name),
                "Welcome",
                "Welcome to our service",
            )?;

            Ok(user_id)
        }
    }

    // Manual Injectable implementation - would be generated by proc macro
    impl Injectable for UserService {
        fn dependencies() -> Vec<ServiceId> {
            vec![
                ServiceId::of::<PostgresRepository>(),
                ServiceId::of::<SmtpEmailService>(),
                ServiceId::of::<FileLogger>(), // Optional, but still listed
            ]
        }

        fn create<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError> {
            let repository = resolver.resolve::<PostgresRepository>()?;
            let email_service = resolver.resolve::<SmtpEmailService>()?;
            let logger = resolver.try_resolve::<FileLogger>(); // Optional

            Ok(UserService::new(repository, email_service, logger))
        }
    }

    #[test]
    fn test_parameter_type_extraction() {
        assert_eq!(Arc::<String>::service_id(), ServiceId::of::<String>());
        assert_eq!(
            Option::<Arc<String>>::service_id(),
            ServiceId::of::<String>()
        );
    }

    #[test]
    fn test_injectable_with_dependencies() {
        let mut resolver = MockResolver::new();

        // Register dependencies as concrete types
        resolver.register(PostgresRepository);
        resolver.register(SmtpEmailService);
        resolver.register(FileLogger);

        // Create service with auto-wiring
        let user_service = UserService::create(&resolver).unwrap();

        // Test that it works
        let result = user_service.create_user("john");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "user_john");
    }

    #[test]
    fn test_optional_dependencies() {
        let mut resolver = MockResolver::new();

        // Register only required dependencies (not the optional logger)
        resolver.register(PostgresRepository);
        resolver.register(SmtpEmailService);

        // Should still work without optional dependency
        let user_service = UserService::create(&resolver).unwrap();

        let result = user_service.create_user("jane");
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_required_dependency() {
        let mut resolver = MockResolver::new();

        // Register only partial dependencies
        resolver.register(PostgresRepository);
        // Missing EmailService

        // Should fail with missing dependency error
        let result = UserService::create(&resolver);
        assert!(result.is_err());

        if let Err(CoreError::ServiceNotFound { .. }) = result {
            // Expected
        } else {
            panic!("Expected ServiceNotFound error");
        }
    }

    #[test]
    fn test_dependency_list() {
        let deps = UserService::dependencies();
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&ServiceId::of::<PostgresRepository>()));
        assert!(deps.contains(&ServiceId::of::<SmtpEmailService>()));
        assert!(deps.contains(&ServiceId::of::<FileLogger>()));
    }
}
