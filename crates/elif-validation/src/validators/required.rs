//! Required field validator

use crate::error::{ValidationError, ValidationResult};
use crate::traits::ValidationRule;
use async_trait::async_trait;
use serde_json::Value;

/// Validator that ensures a field is present and not empty
#[derive(Debug, Clone)]
pub struct RequiredValidator {
    /// Custom error message
    pub message: Option<String>,
}

impl RequiredValidator {
    /// Create a new required validator with default message
    pub fn new() -> Self {
        Self { message: None }
    }

    /// Create a required validator with custom message
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }

    /// Check if a value is considered empty
    fn is_empty(&self, value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(s) => s.trim().is_empty(),
            Value::Array(arr) => arr.is_empty(),
            Value::Object(obj) => obj.is_empty(),
            _ => false,
        }
    }
}

impl Default for RequiredValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationRule for RequiredValidator {
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()> {
        if self.is_empty(value) {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| format!("{} is required", field));

            Err(ValidationError::with_code(field, message, "required").into())
        } else {
            Ok(())
        }
    }

    fn rule_name(&self) -> &'static str {
        "required"
    }

    fn parameters(&self) -> Option<Value> {
        self.message.as_ref().map(|msg| {
            serde_json::json!({
                "message": msg
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_required_validator_with_null() {
        let validator = RequiredValidator::new();
        let result = validator.validate(&Value::Null, "email").await;

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.has_field_errors("email"));
    }

    #[tokio::test]
    async fn test_required_validator_with_empty_string() {
        let validator = RequiredValidator::new();
        let result = validator
            .validate(&Value::String("".to_string()), "name")
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_required_validator_with_whitespace_string() {
        let validator = RequiredValidator::new();
        let result = validator
            .validate(&Value::String("   ".to_string()), "name")
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_required_validator_with_valid_string() {
        let validator = RequiredValidator::new();
        let result = validator
            .validate(&Value::String("John".to_string()), "name")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_required_validator_with_empty_array() {
        let validator = RequiredValidator::new();
        let result = validator.validate(&Value::Array(vec![]), "tags").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_required_validator_with_filled_array() {
        let validator = RequiredValidator::new();
        let result = validator
            .validate(
                &Value::Array(vec![Value::String("tag1".to_string())]),
                "tags",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_required_validator_with_custom_message() {
        let validator = RequiredValidator::with_message("This field cannot be empty");
        let result = validator.validate(&Value::Null, "email").await;

        assert!(result.is_err());
        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("email").unwrap();
        assert_eq!(field_errors[0].message, "This field cannot be empty");
    }

    #[tokio::test]
    async fn test_required_validator_with_numbers() {
        let validator = RequiredValidator::new();

        // Numbers are never considered empty (including 0)
        assert!(validator
            .validate(&Value::Number(serde_json::Number::from(0)), "count")
            .await
            .is_ok());
        assert!(validator
            .validate(&Value::Number(serde_json::Number::from(42)), "count")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_required_validator_with_boolean() {
        let validator = RequiredValidator::new();

        // Booleans are never considered empty (including false)
        assert!(validator
            .validate(&Value::Bool(false), "active")
            .await
            .is_ok());
        assert!(validator
            .validate(&Value::Bool(true), "active")
            .await
            .is_ok());
    }
}
