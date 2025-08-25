//! Integration tests for the #[module] and app! macros
//! These tests verify that the generated code compiles and works correctly.

#![allow(dead_code)]

use elif_http_derive::module;

// Note: We only test the #[module] attribute macro in integration tests
// The app! macro will be tested once elif-core supports it

// Mock services for testing
pub trait UserService: Send + Sync {
    fn get_user(&self, id: u32) -> String;
}

pub trait EmailService: Send + Sync {
    fn send_email(&self, to: &str, subject: &str) -> bool;
}

pub trait CacheService: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str);
}

// Mock implementations
pub struct MockUserService;
impl UserService for MockUserService {
    fn get_user(&self, id: u32) -> String {
        format!("User {}", id)
    }
}

pub struct SmtpEmailService;
impl EmailService for SmtpEmailService {
    fn send_email(&self, to: &str, subject: &str) -> bool {
        println!("Sending email to {} with subject: {}", to, subject);
        true
    }
}

pub struct MockEmailService;
impl EmailService for MockEmailService {
    fn send_email(&self, to: &str, subject: &str) -> bool {
        println!("Mock sending email to {} with subject: {}", to, subject);
        true
    }
}

pub struct RedisCacheService;
impl CacheService for RedisCacheService {
    fn get(&self, key: &str) -> Option<String> {
        println!("Redis getting key: {}", key);
        None
    }

    fn set(&self, key: &str, value: &str) {
        println!("Redis setting key: {} = {}", key, value);
    }
}

// Mock controllers for testing
pub struct UserController;
pub struct PostController;
pub struct AuthController;

#[cfg(test)]
mod basic_module_tests {
    use super::*;

    #[test]
    fn test_basic_module_compilation() {
        #[module(
            providers: [MockUserService],
            controllers: [UserController]
        )]
        pub struct BasicModule;

        // Test that the module descriptor method exists
        let descriptor = BasicModule::module_descriptor();
        assert_eq!(descriptor.name(), "BasicModule");
    }

    #[test]
    fn test_trait_mapping_module_compilation() {
        // Test simplified syntax (without dyn)
        #[module(
            providers: [
                MockUserService,
                EmailService => SmtpEmailService
            ],
            controllers: [UserController, PostController]
        )]
        pub struct TraitMappingModule;

        let descriptor = TraitMappingModule::module_descriptor();
        assert_eq!(descriptor.name(), "TraitMappingModule");
    }

    #[test]
    fn test_explicit_dyn_syntax_still_works() {
        // Test that explicit dyn syntax is still supported
        #[module(
            providers: [
                MockUserService,
                dyn EmailService => SmtpEmailService
            ],
            controllers: [UserController]
        )]
        pub struct ExplicitDynModule;

        let descriptor = ExplicitDynModule::module_descriptor();
        assert_eq!(descriptor.name(), "ExplicitDynModule");
    }

    #[test]
    fn test_named_trait_mapping_compilation() {
        #[module(
            providers: [
                dyn EmailService => SmtpEmailService @ "smtp",
                dyn CacheService => RedisCacheService @ "redis"
            ],
            controllers: [UserController]
        )]
        pub struct NamedMappingModule;

        let descriptor = NamedMappingModule::module_descriptor();
        assert_eq!(descriptor.name(), "NamedMappingModule");
    }

    #[test]
    fn test_imports_and_exports_compilation() {
        // First define a dependency module
        #[module(
            providers: [MockUserService],
            exports: [MockUserService]
        )]
        pub struct UserModule;

        // Then define a module that imports from it
        #[module(
            imports: [UserModule],
            providers: [dyn EmailService => SmtpEmailService],
            controllers: [PostController],
            exports: [dyn EmailService]
        )]
        pub struct PostModule;

        let descriptor = PostModule::module_descriptor();
        assert_eq!(descriptor.name(), "PostModule");
    }

    #[test]
    fn test_complex_module_compilation() {
        #[module(
            providers: [
                MockUserService,
                dyn EmailService => SmtpEmailService @ "smtp",
                dyn CacheService => RedisCacheService @ "redis"
            ],
            controllers: [UserController, PostController, AuthController],
            imports: [],
            exports: [MockUserService, dyn EmailService]
        )]
        pub struct ComplexModule;

        let descriptor = ComplexModule::module_descriptor();
        assert_eq!(descriptor.name(), "ComplexModule");
    }
}

// Composition tests are commented out for now until elif-core supports the module loader API
// #[cfg(test)]
// mod composition_tests {
//     use super::*;
//
//     #[test]
//     fn test_basic_composition_compilation() {
//         // Will be implemented in Epic 4 (Runtime Integration)
//     }
// }

#[cfg(test)]
mod syntax_validation_tests {
    use super::*;

    // These are compilation tests - they should compile successfully
    // Error cases are tested in UI tests with trybuild

    #[test]
    fn test_empty_sections_compilation() {
        #[module(
            providers: [],
            controllers: [],
            imports: [],
            exports: []
        )]
        pub struct EmptyModule;

        let descriptor = EmptyModule::module_descriptor();
        assert_eq!(descriptor.name(), "EmptyModule");
    }

    #[test]
    fn test_partial_sections_compilation() {
        #[module(
            providers: [MockUserService],
            controllers: [UserController]
        )]
        pub struct PartialModule;

        let descriptor = PartialModule::module_descriptor();
        assert_eq!(descriptor.name(), "PartialModule");
    }

    #[test]
    fn test_single_providers_only() {
        #[module(providers: [MockUserService])]
        pub struct ProvidersOnlyModule;

        let descriptor = ProvidersOnlyModule::module_descriptor();
        assert_eq!(descriptor.name(), "ProvidersOnlyModule");
    }

    #[test]
    fn test_controllers_only() {
        #[module(controllers: [UserController])]
        pub struct ControllersOnlyModule;

        let descriptor = ControllersOnlyModule::module_descriptor();
        assert_eq!(descriptor.name(), "ControllersOnlyModule");
    }
}

// Test that module structs can be used normally
#[cfg(test)]
mod module_struct_tests {
    use super::*;

    #[test]
    fn test_module_struct_instantiation() {
        #[module(
            providers: [MockUserService],
            controllers: [UserController]
        )]
        pub struct InstantiableModule {
            pub name: String,
        }

        // Should be able to create instances normally
        let module = InstantiableModule {
            name: "test_module".to_string(),
        };

        assert_eq!(module.name, "test_module");

        // And still have the generated method
        let descriptor = InstantiableModule::module_descriptor();
        assert_eq!(descriptor.name(), "InstantiableModule");
    }

    #[test]
    fn test_module_struct_with_methods() {
        #[module(
            providers: [MockUserService],
            controllers: [UserController]
        )]
        pub struct ModuleWithMethods;

        impl ModuleWithMethods {
            pub fn custom_method(&self) -> &'static str {
                "custom method works"
            }
        }

        let module = ModuleWithMethods;
        assert_eq!(module.custom_method(), "custom method works");

        let descriptor = ModuleWithMethods::module_descriptor();
        assert_eq!(descriptor.name(), "ModuleWithMethods");
    }
}
