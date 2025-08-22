//! Migration utilities for transitioning from old Container to new IoC Container
//! 
//! Provides compatibility adapters, migration tools, and progressive migration support.

use std::sync::Arc;
use std::any::TypeId;
use std::collections::HashMap;

use crate::container::{Container, IocContainer, ServiceBinder};
use crate::foundation::traits::Service;
use crate::errors::CoreError;

/// Compatibility adapter that wraps the old Container API around the new IoC Container
pub struct LegacyContainerAdapter {
    ioc_container: IocContainer,
    legacy_services: HashMap<TypeId, Arc<dyn std::any::Any + Send + Sync>>,
}

impl LegacyContainerAdapter {
    /// Create a new legacy adapter from an IoC container
    pub fn new(ioc_container: IocContainer) -> Self {
        Self {
            ioc_container,
            legacy_services: HashMap::new(),
        }
    }

    /// Register a legacy service (for services that can't be easily migrated)
    pub fn register_legacy<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.legacy_services.insert(type_id, Arc::new(service));
        Ok(())
    }

    /// Get the underlying IoC container
    pub fn ioc_container(&self) -> &IocContainer {
        &self.ioc_container
    }

    /// Get a mutable reference to the underlying IoC container
    pub fn ioc_container_mut(&mut self) -> &mut IocContainer {
        &mut self.ioc_container
    }
}

impl LegacyContainerAdapter {
    /// Register a service (delegates to IoC container when possible)
    pub fn register<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + 'static,
    {
        // Try to register in IoC container first
        if !self.ioc_container.is_built() {
            self.ioc_container.bind_instance::<T, T>(service.clone());
            Ok(())
        } else {
            // Fallback to legacy storage if container is already built
            self.register_legacy(service)
        }
    }

    /// Register a singleton service
    pub fn register_singleton<T>(&mut self, service: T) -> Result<(), CoreError>
    where
        T: Service + Clone + Default + 'static,
    {
        if !self.ioc_container.is_built() {
            self.ioc_container.bind_singleton::<T, T>();
            // Store the instance for immediate availability
            let type_id = TypeId::of::<T>();
            self.legacy_services.insert(type_id, Arc::new(service));
            Ok(())
        } else {
            self.register_legacy(service)
        }
    }

    /// Register a transient service (not directly supported by IoC, use factory)
    pub fn register_transient<T>(&mut self, factory: Box<dyn Fn() -> T + Send + Sync>) -> Result<(), CoreError>
    where
        T: Service + 'static,
    {
        if !self.ioc_container.is_built() {
            self.ioc_container.bind_factory::<T, _, _>(move || {
                let service = factory();
                Ok(service)
            });
            Ok(())
        } else {
            Err(CoreError::InvalidServiceDescriptor {
                message: "Cannot register transient services after container is built".to_string(),
            })
        }
    }

    /// Resolve a service (tries IoC container first, then legacy storage)
    pub fn resolve<T>(&self) -> Result<Arc<T>, CoreError>
    where
        T: Service + Clone + 'static,
    {
        // Try IoC container first
        if let Ok(service) = self.ioc_container.resolve::<T>() {
            return Ok(service);
        }

        // Fallback to legacy storage
        let type_id = TypeId::of::<T>();
        if let Some(service_any) = self.legacy_services.get(&type_id) {
            if let Ok(service) = service_any.clone().downcast::<T>() {
                return Ok(service);
            }
        }

        Err(CoreError::ServiceNotFound {
            service_type: std::any::type_name::<T>().to_string(),
        })
    }

    /// Try to resolve a service, returning None if not found
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Service + Clone + 'static,
    {
        self.resolve::<T>().ok()
    }

    /// Check if a service is registered
    pub fn contains<T>(&self) -> bool
    where
        T: Service + 'static,
    {
        self.ioc_container.contains::<T>() || {
            let type_id = TypeId::of::<T>();
            self.legacy_services.contains_key(&type_id)
        }
    }

    /// Validate the container configuration
    pub fn validate(&self) -> Result<(), CoreError> {
        self.ioc_container.validate()
    }

    /// Initialize the container and all its services
    pub async fn initialize(&mut self) -> Result<(), CoreError> {
        if !self.ioc_container.is_built() {
            self.ioc_container.build()?;
        }
        self.ioc_container.initialize_async().await
    }

    /// Check if the container is initialized
    pub fn is_initialized(&self) -> bool {
        self.ioc_container.is_built()
    }

    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.ioc_container.service_count() + self.legacy_services.len()
    }

    /// Get a list of all registered service types
    pub fn registered_services(&self) -> Vec<TypeId> {
        let mut services = Vec::new();
        
        // Add IoC container services
        for service_id in self.ioc_container.registered_services() {
            services.push(service_id.type_id);
        }
        
        // Add legacy services
        services.extend(self.legacy_services.keys().cloned());
        
        services
    }
}

/// Migration analyzer that scans code for old container usage
pub struct MigrationAnalyzer;

impl MigrationAnalyzer {
    /// Create a new migration analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze container compatibility between old and new
    pub fn analyze_container_compatibility(
        old_container: &Container,
        new_container: &IocContainer,
    ) -> CompatibilityReport {
        let old_services = old_container.registered_services();
        let new_services: std::collections::HashSet<_> = new_container
            .registered_services()
            .into_iter()
            .map(|id| id.type_id)
            .collect();

        let missing_services: Vec<_> = old_services
            .into_iter()
            .filter(|type_id| !new_services.contains(type_id))
            .collect();

        CompatibilityReport {
            migration_required: !missing_services.is_empty(),
            missing_services,
            compatible_services: new_services.len(),
        }
    }

    /// Generate migration suggestions for a container
    pub fn generate_migration_suggestions(
        _container: &Container,
    ) -> Vec<MigrationSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest converting singleton registrations
        suggestions.push(MigrationSuggestion {
            suggestion_type: SuggestionType::ConvertSingletonRegistration,
            description: "Convert container.register_singleton() calls to use IoC container binding".to_string(),
            code_example: Some("container.bind_singleton::<MyService, MyService>()".to_string()),
            priority: MigrationPriority::High,
        });

        // Suggest converting resolve calls
        suggestions.push(MigrationSuggestion {
            suggestion_type: SuggestionType::UpdateResolveCall,
            description: "Update resolve calls to use new IoC container API".to_string(),
            code_example: Some("ioc_container.resolve::<MyService>()".to_string()),
            priority: MigrationPriority::Medium,
        });

        // Suggest using dependency injection
        suggestions.push(MigrationSuggestion {
            suggestion_type: SuggestionType::UseDependencyInjection,
            description: "Consider using #[inject] macro for automatic dependency injection".to_string(),
            code_example: Some("#[inject(my_service: MyService)] struct MyController".to_string()),
            priority: MigrationPriority::Low,
        });

        suggestions
    }
}

/// Report on container compatibility analysis
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub missing_services: Vec<TypeId>,
    pub compatible_services: usize,
    pub migration_required: bool,
}

/// Migration suggestion for improving container usage
#[derive(Debug, Clone)]
pub struct MigrationSuggestion {
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub code_example: Option<String>,
    pub priority: MigrationPriority,
}

/// Type of migration suggestion
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionType {
    ConvertSingletonRegistration,
    ConvertTransientRegistration,
    UpdateResolveCall,
    UseDependencyInjection,
    AddContainerBuilder,
    MigrateToIocContainer,
}

/// Priority level for migration suggestions
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Progressive migration helper that supports gradual migration
pub struct ProgressiveMigrator {
    migration_percentage: f32,
    old_container: Container,
    new_container: IocContainer,
}

impl ProgressiveMigrator {
    /// Create a new progressive migrator
    pub fn new(
        old_container: Container,
        new_container: IocContainer,
        migration_percentage: f32,
    ) -> Self {
        Self {
            migration_percentage: migration_percentage.clamp(0.0, 1.0),
            old_container,
            new_container,
        }
    }

    /// Set migration percentage (0.0 = all old, 1.0 = all new)
    pub fn set_migration_percentage(&mut self, percentage: f32) {
        self.migration_percentage = percentage.clamp(0.0, 1.0);
    }

    /// Resolve a service using progressive migration
    pub fn resolve<T>(&self) -> Result<Arc<T>, CoreError>
    where
        T: Send + Sync + Clone + Service + 'static,
    {
        // Use a deterministic method to decide which container to use
        let use_new = self.should_use_new_container();

        if use_new {
            // Try new container first, fallback to old
            self.new_container.resolve::<T>()
                .or_else(|_| self.old_container.resolve::<T>())
        } else {
            // Try old container first, fallback to new
            self.old_container.resolve::<T>()
                .or_else(|_| self.new_container.resolve::<T>())
        }
    }

    /// Determine if we should use the new container for this request
    fn should_use_new_container(&self) -> bool {
        // Simple implementation - in production you might use request hash, user ID, etc.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        let hash = hasher.finish();
        
        (hash % 100) < (self.migration_percentage * 100.0) as u64
    }

    /// Get current migration percentage
    pub fn migration_percentage(&self) -> f32 {
        self.migration_percentage
    }
}

/// Migration validation utilities
pub struct MigrationValidator;

impl MigrationValidator {
    /// Validate that services in old container are available in new container
    pub fn validate_service_parity(
        old_container: &Container,
        new_container: &IocContainer,
    ) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let old_services = old_container.registered_services();
        let new_service_ids = new_container.registered_services();
        let new_types: std::collections::HashSet<_> = new_service_ids
            .iter()
            .map(|id| id.type_id)
            .collect();

        for service_type in old_services {
            if !new_types.contains(&service_type) {
                errors.push(format!(
                    "Service {:?} is registered in old container but missing in new container",
                    service_type
                ));
            }
        }

        // Check for services that might have different lifetimes
        for service_id in new_service_ids {
            if let Some(descriptor) = new_container.get_service_info_by_id(&service_id) {
                warnings.push(format!(
                    "Service {:?} lifetime should be verified: {}",
                    service_id.type_id,
                    descriptor
                ));
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Run comprehensive migration validation
    pub fn run_comprehensive_validation(
        old_container: &Container,
        new_container: &IocContainer,
    ) -> Result<ValidationSummary, CoreError> {
        let service_parity = Self::validate_service_parity(old_container, new_container);
        let container_validation = new_container.validate();

        Ok(ValidationSummary {
            service_parity,
            container_valid: container_validation.is_ok(),
            container_errors: if let Err(e) = container_validation {
                vec![e.to_string()]
            } else {
                vec![]
            },
        })
    }
}

/// Result of migration validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Summary of comprehensive migration validation
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    pub service_parity: ValidationResult,
    pub container_valid: bool,
    pub container_errors: Vec<String>,
}

/// Extension trait for IocContainer to add migration helpers
pub trait MigrationExtensions {
    /// Get service information by service ID (internal helper)
    fn get_service_info_by_id(&self, service_id: &crate::container::ServiceId) -> Option<String>;
    
    /// Check if container is ready for migration
    fn is_migration_ready(&self) -> bool;
}

impl MigrationExtensions for IocContainer {
    fn get_service_info_by_id(&self, service_id: &crate::container::ServiceId) -> Option<String> {
        // This would normally access internal descriptor information
        // For now, return a placeholder
        Some(format!("Service {:?}", service_id))
    }

    fn is_migration_ready(&self) -> bool {
        self.is_built() && self.service_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::traits::Service;

    // Test service for migration testing
    #[derive(Clone, Debug, PartialEq)]
    struct TestMigrationService {
        name: String,
    }

    impl Service for TestMigrationService {
        fn name(&self) -> &str {
            &self.name
        }
    }

    unsafe impl Send for TestMigrationService {}
    unsafe impl Sync for TestMigrationService {}

    #[tokio::test]
    async fn test_legacy_container_adapter() {
        let ioc_container = IocContainer::new();
        let mut adapter = LegacyContainerAdapter::new(ioc_container);

        let service = TestMigrationService {
            name: "test_service".to_string(),
        };

        // Register in legacy adapter
        adapter.register(service.clone()).expect("Failed to register service");

        // Should be able to resolve
        let resolved = adapter.resolve::<TestMigrationService>()
            .expect("Failed to resolve service");

        assert_eq!(resolved.name, "test_service");
    }

    #[test]
    fn test_migration_analyzer_suggestions() {
        let container = Container::new();
        let suggestions = MigrationAnalyzer::generate_migration_suggestions(&container);

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.suggestion_type == SuggestionType::ConvertSingletonRegistration));
        assert!(suggestions.iter().any(|s| s.suggestion_type == SuggestionType::UpdateResolveCall));
    }

    #[test]
    fn test_compatibility_report() {
        let old_container = Container::new();
        let new_container = IocContainer::new();

        let report = MigrationAnalyzer::analyze_container_compatibility(&old_container, &new_container);

        // Empty containers should be compatible
        assert!(!report.migration_required);
        assert_eq!(report.missing_services.len(), 0);
    }

    #[tokio::test]
    async fn test_progressive_migrator() {
        let mut old_container = Container::new();
        let service = TestMigrationService {
            name: "old_service".to_string(),
        };
        old_container.register(service).expect("Failed to register in old container");
        old_container.initialize().await.expect("Failed to initialize old container");

        let mut new_container = IocContainer::new();
        let new_service = TestMigrationService {
            name: "new_service".to_string(),
        };
        new_container.bind_instance::<TestMigrationService, TestMigrationService>(new_service);
        new_container.build().expect("Failed to build new container");

        let migrator = ProgressiveMigrator::new(old_container, new_container, 0.5);

        // Should be able to resolve from either container
        let resolved = migrator.resolve::<TestMigrationService>()
            .expect("Failed to resolve service");

        assert!(resolved.name == "old_service" || resolved.name == "new_service");
    }

    #[test]
    fn test_migration_validator() {
        let old_container = Container::new();
        let new_container = IocContainer::new();

        let result = MigrationValidator::validate_service_parity(&old_container, &new_container);
        assert!(result.is_valid); // Empty containers should be valid

        let summary = MigrationValidator::run_comprehensive_validation(&old_container, &new_container)
            .expect("Validation should not fail");
        
        assert!(summary.service_parity.is_valid);
    }

    #[test]
    fn test_migration_extensions() {
        let container = IocContainer::new();
        
        // Empty container is not ready for migration
        assert!(!container.is_migration_ready());
    }
}