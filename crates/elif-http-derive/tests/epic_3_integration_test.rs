//! Integration tests for Epic 3: Module Descriptor Generation
//!
//! Tests the complete module descriptor generation system including:
//! - Module attribute macro parsing and descriptor generation
//! - Auto-configuration function generation  
//! - Module composition macro functionality
//! - Validation of circular imports and missing exports
//! - Integration with IoC container

use elif_core::modules::{
    ModuleAutoConfiguration, ModuleComposition, ModuleDependencyValidator, ModuleDescriptor,
    ServiceDescriptor, ServiceLifecycle,
};
use elif_http_derive::module;

/// Test service for module system
#[derive(Default)]
pub struct TestUserService {
    pub name: String,
}

/// Test repository trait
pub trait TestRepository: Send + Sync {
    fn find(&self, id: u32) -> Option<String>;
}

/// Test repository implementation
#[derive(Default)]
pub struct TestSqlRepository {
    pub connection: String,
}

impl TestRepository for TestSqlRepository {
    fn find(&self, _id: u32) -> Option<String> {
        Some("test data".to_string())
    }
}

/// Test controller
#[derive(Default)]
pub struct TestUserController {
    pub service: Option<String>, // Placeholder for dependency injection
}

/// Test module with comprehensive features  
#[module(
    providers: [
        TestUserService,
        TestSqlRepository
    ],
    controllers: [TestUserController],
    imports: [],
    exports: [TestUserService, TestSqlRepository]
)]
pub struct TestModule;

/// Database module for import testing
#[module(
    providers: [TestSqlRepository],
    controllers: [],
    imports: [],
    exports: [TestSqlRepository]
)]
pub struct DatabaseModule;

/// Business logic module that imports database
#[module(
    providers: [TestUserService],
    controllers: [TestUserController],
    imports: [DatabaseModule],
    exports: [TestUserService]
)]
pub struct BusinessModule;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epic_3_module_descriptor_generation() {
        // Test that the module descriptor is correctly generated
        let descriptor = TestModule::module_descriptor();

        // Verify basic module information
        assert_eq!(descriptor.name, "TestModule");
        assert_eq!(descriptor.service_count(), 2); // TestUserService + TestSqlRepository
        assert_eq!(descriptor.controller_count(), 1);
        assert_eq!(descriptor.exports.len(), 2);
        assert!(descriptor.imports.is_empty());

        // Verify provider descriptors
        let providers = &descriptor.providers;

        // Check concrete service
        let user_service = providers
            .iter()
            .find(|p| p.service_name == "TestUserService")
            .expect("TestUserService provider should exist");
        assert_eq!(user_service.lifecycle, ServiceLifecycle::default());
        assert!(!user_service.is_trait_service);
        assert!(user_service.name.is_none());

        // Check repository service
        let repo_service = providers
            .iter()
            .find(|p| p.service_name == "TestSqlRepository")
            .expect("TestSqlRepository provider should exist");
        assert!(!repo_service.is_trait_service); // It's a concrete service, not a trait mapping
        assert!(repo_service.name.is_none());

        // Verify controller descriptors
        let controllers = &descriptor.controllers;
        assert_eq!(controllers.len(), 1);
        assert_eq!(controllers[0].controller_name, "TestUserController");

        // Verify exports
        assert!(descriptor.exports.contains(&"TestUserService".to_string()));
        assert!(descriptor
            .exports
            .contains(&"TestSqlRepository".to_string()));
    }

    #[test]
    fn test_epic_3_module_auto_configuration_trait() {
        // Verify that the ModuleAutoConfiguration trait is implemented
        let descriptor1 = TestModule::module_descriptor();
        let descriptor2 = <TestModule as ModuleAutoConfiguration>::module_descriptor();

        assert_eq!(descriptor1.name, descriptor2.name);
        assert_eq!(descriptor1.service_count(), descriptor2.service_count());
    }

    #[test]
    fn test_epic_3_module_composition() {
        // Test the module composition functionality using ModuleComposition directly
        let composition = ModuleComposition::new()
            .with_module(TestModule::module_descriptor())
            .with_module(DatabaseModule::module_descriptor())
            .with_module(BusinessModule::module_descriptor())
            .with_overrides(vec![ServiceDescriptor::new::<TestUserService>(
                "OverrideService",
                ServiceLifecycle::Singleton,
            )]);

        let composition_result = composition.compose().unwrap();

        // Verify the composition creates a valid module descriptor
        assert_eq!(composition_result.name, "ComposedApplication");

        // Should have services from all modules
        assert!(composition_result.service_count() > 0);

        // Should include controllers from all modules
        assert!(composition_result.controller_count() > 0);

        // Should merge imports and exports appropriately
        assert!(composition_result.has_exports());
    }

    #[test]
    fn test_epic_3_circular_dependency_validation() {
        // Create a simple circular dependency scenario for testing
        let module_a = ModuleDescriptor::new("ModuleA")
            .with_imports(vec!["ModuleB".to_string()])
            .with_exports(vec!["ServiceA".to_string()]);

        let module_b = ModuleDescriptor::new("ModuleB")
            .with_imports(vec!["ModuleA".to_string()]) // Creates A -> B -> A cycle
            .with_exports(vec!["ServiceB".to_string()]);

        let modules = vec![module_a, module_b];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();

        // Should detect the circular dependency
        assert!(result.is_err(), "Should detect circular dependency");

        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(
                e,
                elif_core::modules::ModuleValidationError::CircularImport { .. }
            )),
            "Should contain CircularImport error"
        );
    }

    #[test]
    fn test_epic_3_missing_export_validation() {
        // Test validation of missing exports
        let importing_module = ModuleDescriptor::new("ImportingModule")
            .with_imports(vec!["NonExistentModule".to_string()]);

        let modules = vec![importing_module];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();

        // Should detect missing export
        assert!(result.is_err(), "Should detect missing export");

        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(
                e,
                elif_core::modules::ModuleValidationError::MissingExport { .. }
            )),
            "Should contain MissingExport error"
        );
    }

    #[test]
    fn test_epic_3_topological_sort() {
        // Test proper dependency ordering
        let modules = vec![
            BusinessModule::module_descriptor(), // Depends on DatabaseModule
            TestModule::module_descriptor(),     // No dependencies
            DatabaseModule::module_descriptor(), // No dependencies
        ];

        let validator = ModuleDependencyValidator::new(&modules);
        let sorted = validator.topological_sort().unwrap();

        // DatabaseModule should come before BusinessModule since BusinessModule imports it
        let database_pos = sorted.iter().position(|m| m == "DatabaseModule").unwrap();
        let business_pos = sorted.iter().position(|m| m == "BusinessModule").unwrap();

        assert!(
            database_pos < business_pos,
            "DatabaseModule should come before BusinessModule in topological order"
        );
    }

    #[test]
    fn test_epic_3_complex_composition_scenario() {
        // Test a complex composition with multiple modules, imports, and overrides
        let composition = ModuleComposition::new()
            .with_module(DatabaseModule::module_descriptor())
            .with_module(BusinessModule::module_descriptor())
            .with_module(TestModule::module_descriptor())
            .with_overrides(vec![ServiceDescriptor::new::<TestUserService>(
                "OverrideUserService",
                ServiceLifecycle::Transient,
            )
            .with_name("business")]);

        let result = composition.compose();

        // Should succeed with proper validation and ordering
        assert!(
            result.is_ok(),
            "Complex composition should succeed: {:?}",
            result
        );

        let final_descriptor = result.unwrap();

        // Verify final composition
        assert_eq!(final_descriptor.name, "ComposedApplication");
        assert!(final_descriptor.service_count() > 0);
        assert!(final_descriptor.controller_count() > 0);

        // Should include the override
        let has_override = final_descriptor.providers.iter().any(|p| {
            p.service_name == "OverrideUserService" && p.name.as_deref() == Some("business")
        });
        assert!(has_override, "Should contain override service");
    }

    #[test]
    fn test_epic_3_self_import_detection() {
        // Test detection of self-imports
        let self_importing_module =
            ModuleDescriptor::new("SelfModule").with_imports(vec!["SelfModule".to_string()]);

        let modules = vec![self_importing_module];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();

        assert!(result.is_err(), "Should detect self-import");

        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(
                e,
                elif_core::modules::ModuleValidationError::SelfImport { .. }
            )),
            "Should contain SelfImport error"
        );
    }

    #[test]
    fn test_epic_3_duplicate_exports_detection() {
        // Test detection of duplicate exports
        let duplicate_export_module = ModuleDescriptor::new("DuplicateModule").with_exports(vec![
            "Service1".to_string(),
            "Service2".to_string(),
            "Service1".to_string(), // Duplicate
        ]);

        let modules = vec![duplicate_export_module];
        let validator = ModuleDependencyValidator::new(&modules);
        let result = validator.validate();

        assert!(result.is_err(), "Should detect duplicate exports");

        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(
                e,
                elif_core::modules::ModuleValidationError::DuplicateExport { .. }
            )),
            "Should contain DuplicateExport error"
        );
    }

    #[test]
    fn test_epic_3_empty_module() {
        // Test handling of empty modules
        let empty_descriptor = TestEmptyModule::module_descriptor();

        assert_eq!(empty_descriptor.name, "TestEmptyModule");
        assert_eq!(empty_descriptor.service_count(), 0);
        assert_eq!(empty_descriptor.controller_count(), 0);
        assert!(empty_descriptor.imports.is_empty());
        assert!(empty_descriptor.exports.is_empty());

        // Should validate successfully
        let modules = vec![empty_descriptor];
        let validator = ModuleDependencyValidator::new(&modules);
        assert!(validator.validate().is_ok());
    }
}

/// Empty module for testing edge cases
#[module(
    providers: [],
    controllers: [],
    imports: [],
    exports: []
)]
pub struct TestEmptyModule;
