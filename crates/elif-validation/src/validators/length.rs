//! Length-based validators for strings and collections

use crate::error::{ValidationError, ValidationResult};
use crate::traits::ValidationRule;
use async_trait::async_trait;
use serde_json::Value;

/// Validator for string/array length constraints
#[derive(Debug, Clone)]
pub struct LengthValidator {
    /// Minimum length (inclusive)
    pub min: Option<usize>,
    /// Maximum length (inclusive)
    pub max: Option<usize>,
    /// Exact length required
    pub exact: Option<usize>,
    /// Custom error message
    pub message: Option<String>,
}

impl LengthValidator {
    /// Create a new length validator with no constraints
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            exact: None,
            message: None,
        }
    }

    /// Set minimum length constraint
    pub fn min(mut self, min: usize) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum length constraint
    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }

    /// Set exact length requirement
    pub fn exact(mut self, exact: usize) -> Self {
        self.exact = Some(exact);
        self
    }

    /// Set length range (min and max)
    pub fn range(mut self, min: usize, max: usize) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Set custom error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get the length of a value (supports strings and arrays)
    fn get_length(&self, value: &Value) -> Option<usize> {
        match value {
            Value::String(s) => Some(s.chars().count()), // Unicode-aware length
            Value::Array(arr) => Some(arr.len()),
            _ => None,
        }
    }

    /// Generate appropriate error message based on constraints
    fn create_error_message(&self, field: &str, actual_length: usize) -> String {
        if let Some(ref custom_message) = self.message {
            return custom_message.clone();
        }

        if let Some(exact) = self.exact {
            return format!("{} must be exactly {} characters long", field, exact);
        }

        match (self.min, self.max) {
            (Some(min), Some(max)) if min == max => {
                format!("{} must be exactly {} characters long", field, min)
            }
            (Some(min), Some(max)) => {
                format!(
                    "{} must be between {} and {} characters long",
                    field, min, max
                )
            }
            (Some(min), None) => {
                format!("{} must be at least {} characters long", field, min)
            }
            (None, Some(max)) => {
                format!("{} must be at most {} characters long", field, max)
            }
            (None, None) => {
                format!("{} has invalid length: {}", field, actual_length)
            }
        }
    }
}

impl Default for LengthValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationRule for LengthValidator {
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()> {
        // Skip validation for null values (use RequiredValidator for null checks)
        if value.is_null() {
            return Ok(());
        }

        let length = match self.get_length(value) {
            Some(len) => len,
            None => {
                return Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string or array for length validation", field),
                    "invalid_type",
                )
                .into());
            }
        };

        // Check exact length first
        if let Some(exact) = self.exact {
            if length != exact {
                return Err(ValidationError::with_code(
                    field,
                    self.create_error_message(field, length),
                    "length_exact",
                )
                .into());
            }
            return Ok(());
        }

        // Check minimum length
        if let Some(min) = self.min {
            if length < min {
                return Err(ValidationError::with_code(
                    field,
                    self.create_error_message(field, length),
                    "length_min",
                )
                .into());
            }
        }

        // Check maximum length
        if let Some(max) = self.max {
            if length > max {
                return Err(ValidationError::with_code(
                    field,
                    self.create_error_message(field, length),
                    "length_max",
                )
                .into());
            }
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "length"
    }

    fn parameters(&self) -> Option<Value> {
        let mut params = serde_json::Map::new();

        if let Some(min) = self.min {
            params.insert(
                "min".to_string(),
                Value::Number(serde_json::Number::from(min)),
            );
        }
        if let Some(max) = self.max {
            params.insert(
                "max".to_string(),
                Value::Number(serde_json::Number::from(max)),
            );
        }
        if let Some(exact) = self.exact {
            params.insert(
                "exact".to_string(),
                Value::Number(serde_json::Number::from(exact)),
            );
        }
        if let Some(ref message) = self.message {
            params.insert("message".to_string(), Value::String(message.clone()));
        }

        if params.is_empty() {
            None
        } else {
            Some(Value::Object(params))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_length_validator_min_constraint() {
        let validator = LengthValidator::new().min(3);

        // Too short
        let result = validator
            .validate(&Value::String("hi".to_string()), "name")
            .await;
        assert!(result.is_err());

        // Exact minimum
        let result = validator
            .validate(&Value::String("bob".to_string()), "name")
            .await;
        assert!(result.is_ok());

        // Longer than minimum
        let result = validator
            .validate(&Value::String("alice".to_string()), "name")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_length_validator_max_constraint() {
        let validator = LengthValidator::new().max(5);

        // Within limit
        let result = validator
            .validate(&Value::String("hello".to_string()), "name")
            .await;
        assert!(result.is_ok());

        // Too long
        let result = validator
            .validate(&Value::String("hello world".to_string()), "name")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_length_validator_exact_constraint() {
        let validator = LengthValidator::new().exact(4);

        // Correct length
        let result = validator
            .validate(&Value::String("test".to_string()), "code")
            .await;
        assert!(result.is_ok());

        // Too short
        let result = validator
            .validate(&Value::String("hi".to_string()), "code")
            .await;
        assert!(result.is_err());

        // Too long
        let result = validator
            .validate(&Value::String("testing".to_string()), "code")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_length_validator_range() {
        let validator = LengthValidator::new().range(3, 10);

        // Too short
        let result = validator
            .validate(&Value::String("hi".to_string()), "password")
            .await;
        assert!(result.is_err());

        // Within range
        let result = validator
            .validate(&Value::String("secret".to_string()), "password")
            .await;
        assert!(result.is_ok());

        // Too long
        let result = validator
            .validate(&Value::String("very_long_password".to_string()), "password")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_length_validator_with_arrays() {
        let validator = LengthValidator::new().min(2).max(4);

        // Too few items
        let result = validator
            .validate(
                &Value::Array(vec![Value::String("item1".to_string())]),
                "tags",
            )
            .await;
        assert!(result.is_err());

        // Within range
        let result = validator
            .validate(
                &Value::Array(vec![
                    Value::String("tag1".to_string()),
                    Value::String("tag2".to_string()),
                ]),
                "tags",
            )
            .await;
        assert!(result.is_ok());

        // Too many items
        let result = validator
            .validate(
                &Value::Array(vec![
                    Value::String("tag1".to_string()),
                    Value::String("tag2".to_string()),
                    Value::String("tag3".to_string()),
                    Value::String("tag4".to_string()),
                    Value::String("tag5".to_string()),
                ]),
                "tags",
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_length_validator_unicode_support() {
        let validator = LengthValidator::new().max(5);

        // Unicode characters should be counted correctly
        let result = validator
            .validate(&Value::String("café".to_string()), "name")
            .await;
        assert!(result.is_ok());

        let result = validator
            .validate(&Value::String("🦀🚀✨".to_string()), "emoji")
            .await;
        assert!(result.is_ok());

        // This should be 6 characters, exceeding the max of 5
        let result = validator
            .validate(&Value::String("🦀🚀✨🎉🔥💯".to_string()), "emoji")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_length_validator_with_null() {
        let validator = LengthValidator::new().min(1);

        // Null values should be skipped (not validated)
        let result = validator.validate(&Value::Null, "optional_field").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_length_validator_invalid_type() {
        let validator = LengthValidator::new().min(1);

        // Numbers should fail type validation
        let result = validator
            .validate(&Value::Number(serde_json::Number::from(42)), "age")
            .await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("age").unwrap();
        assert_eq!(field_errors[0].code, "invalid_type");
    }

    #[tokio::test]
    async fn test_length_validator_custom_message() {
        let validator = LengthValidator::new()
            .min(8)
            .message("Password must be strong");

        let result = validator
            .validate(&Value::String("weak".to_string()), "password")
            .await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("password").unwrap();
        assert_eq!(field_errors[0].message, "Password must be strong");
    }
}
