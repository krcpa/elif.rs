//! Core validation traits for the elif framework

use crate::error::{ValidationError, ValidationErrors, ValidationResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Core validation trait that all validators must implement
#[async_trait]
pub trait ValidationRule: Send + Sync {
    /// Validate a single value
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()>;
    
    /// Get the validation rule name/type
    fn rule_name(&self) -> &'static str;
    
    /// Get validation rule parameters/configuration as JSON
    fn parameters(&self) -> Option<Value> {
        None
    }
}

/// Trait for validating individual fields
#[async_trait]
pub trait ValidateField: Send + Sync {
    /// Validate a single field value
    async fn validate_field(&self, field: &str, value: &Value) -> ValidationResult<()>;
}

/// Trait for validating entire requests/objects
#[async_trait]  
pub trait ValidateRequest: Send + Sync {
    /// Validate the entire request data
    async fn validate_request(&self, data: &HashMap<String, Value>) -> ValidationResult<()>;
}

/// Main validation trait that combines field and request validation
#[async_trait]
pub trait Validate: ValidateField + ValidateRequest + Send + Sync {
    /// Validate both individual fields and the entire request
    async fn validate(&self, data: &HashMap<String, Value>) -> ValidationResult<()> {
        let mut errors = ValidationErrors::new();
        
        // First validate individual fields
        for (field, value) in data {
            if let Err(field_errors) = self.validate_field(field, value).await {
                errors.merge(field_errors);
            }
        }
        
        // Then validate the entire request for cross-field rules
        if let Err(request_errors) = self.validate_request(data).await {
            errors.merge(request_errors);
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Auto-implementation of Validate for types that implement both ValidateField and ValidateRequest
impl<T> Validate for T where T: ValidateField + ValidateRequest + Send + Sync {}

/// Trait for types that can be converted to a validation value
pub trait ToValidationValue {
    fn to_validation_value(&self) -> Value;
}

impl ToValidationValue for String {
    fn to_validation_value(&self) -> Value {
        Value::String(self.clone())
    }
}

impl ToValidationValue for &str {
    fn to_validation_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl ToValidationValue for i32 {
    fn to_validation_value(&self) -> Value {
        Value::Number(serde_json::Number::from(*self))
    }
}

impl ToValidationValue for i64 {
    fn to_validation_value(&self) -> Value {
        Value::Number(serde_json::Number::from(*self))
    }
}

impl ToValidationValue for f64 {
    fn to_validation_value(&self) -> Value {
        Value::Number(serde_json::Number::from_f64(*self).unwrap_or(serde_json::Number::from(0)))
    }
}

impl ToValidationValue for bool {
    fn to_validation_value(&self) -> Value {
        Value::Bool(*self)
    }
}

impl ToValidationValue for Value {
    fn to_validation_value(&self) -> Value {
        self.clone()
    }
}

impl<T> ToValidationValue for Option<T> 
where 
    T: ToValidationValue,
{
    fn to_validation_value(&self) -> Value {
        match self {
            Some(value) => value.to_validation_value(),
            None => Value::Null,
        }
    }
}

impl<T> ToValidationValue for Vec<T>
where 
    T: ToValidationValue,
{
    fn to_validation_value(&self) -> Value {
        let values: Vec<Value> = self.iter()
            .map(|item| item.to_validation_value())
            .collect();
        Value::Array(values)
    }
}

/// Helper trait for creating validation errors
pub trait CreateValidationError {
    fn validation_error(field: &str, message: &str) -> ValidationError {
        ValidationError::new(field, message)
    }
    
    fn validation_error_with_code(field: &str, message: &str, code: &str) -> ValidationError {
        ValidationError::with_code(field, message, code)
    }
}

impl<T> CreateValidationError for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestValidator;

    #[async_trait]
    impl ValidateField for TestValidator {
        async fn validate_field(&self, field: &str, value: &Value) -> ValidationResult<()> {
            if field == "email" && value.as_str().map(|s| !s.contains('@')).unwrap_or(true) {
                return Err(ValidationErrors::from_error(
                    ValidationError::new(field, "Invalid email format")
                ));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl ValidateRequest for TestValidator {
        async fn validate_request(&self, data: &HashMap<String, Value>) -> ValidationResult<()> {
            if data.get("password").is_some() && data.get("password_confirmation").is_none() {
                return Err(ValidationErrors::from_error(
                    ValidationError::new("password_confirmation", "Password confirmation required")
                ));
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_field_validation() {
        let validator = TestValidator;
        let value = Value::String("invalid-email".to_string());
        
        let result = validator.validate_field("email", &value).await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert!(errors.has_field_errors("email"));
    }

    #[tokio::test]
    async fn test_request_validation() {
        let validator = TestValidator;
        let mut data = HashMap::new();
        data.insert("password".to_string(), Value::String("secret".to_string()));
        
        let result = validator.validate_request(&data).await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert!(errors.has_field_errors("password_confirmation"));
    }

    #[tokio::test]
    async fn test_combined_validation() {
        let validator = TestValidator;
        let mut data = HashMap::new();
        data.insert("email".to_string(), Value::String("invalid-email".to_string()));
        data.insert("password".to_string(), Value::String("secret".to_string()));
        
        let result = validator.validate(&data).await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert!(errors.has_field_errors("email"));
        assert!(errors.has_field_errors("password_confirmation"));
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_to_validation_value() {
        assert_eq!("hello".to_validation_value(), Value::String("hello".to_string()));
        assert_eq!(42i32.to_validation_value(), Value::Number(serde_json::Number::from(42)));
        assert_eq!(true.to_validation_value(), Value::Bool(true));
        
        let opt_str: Option<String> = Some("test".to_string());
        assert_eq!(opt_str.to_validation_value(), Value::String("test".to_string()));
        
        let opt_none: Option<String> = None;
        assert_eq!(opt_none.to_validation_value(), Value::Null);
    }
}