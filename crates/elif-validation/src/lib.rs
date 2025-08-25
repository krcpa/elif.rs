//! # elif-validation
//!
//! Pure validation system for the elif framework - similar to NestJS class-validator.
//! Framework-agnostic validation with built-in validators and custom rules.

pub mod error;
pub mod rules;
pub mod traits;
pub mod validators;

// Re-exports for easy access
pub use error::{ValidationError, ValidationErrors, ValidationResult};
pub use rules::{Rules, RulesBuilder};
pub use traits::{Validate, ValidateField, ValidateRequest, ValidationRule};

// Built-in validators
pub use validators::{
    custom::CustomValidator, email::EmailValidator, length::LengthValidator,
    numeric::NumericValidator, pattern::PatternValidator, required::RequiredValidator,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_imports() {
        // Test that all imports are working
        let _error = ValidationError::new("field", "message");
        let _rules = Rules::new();

        // This test just ensures the module structure is correct
        assert!(true);
    }
}
