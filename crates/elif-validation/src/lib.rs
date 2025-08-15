//! # elif-validation
//! 
//! Pure validation system for the elif framework - similar to NestJS class-validator.
//! Framework-agnostic validation with built-in validators and custom rules.

pub mod error;
pub mod traits;
pub mod validators;
pub mod rules;

// Re-exports for easy access
pub use error::{ValidationError, ValidationErrors, ValidationResult};
pub use traits::{Validate, ValidateField, ValidateRequest, ValidationRule};
pub use rules::{Rules, RulesBuilder};

// Built-in validators
pub use validators::{
    email::EmailValidator,
    length::LengthValidator, 
    numeric::NumericValidator,
    pattern::PatternValidator,
    required::RequiredValidator,
    custom::CustomValidator,
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