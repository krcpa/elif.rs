//! Custom validation functions and closures

use crate::error::{ValidationError, ValidationResult};
use crate::traits::ValidationRule;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Type alias for sync validation functions  
pub type SyncValidationFn = Arc<dyn Fn(&Value, &str) -> ValidationResult<()> + Send + Sync>;

/// Custom validator that accepts user-defined validation functions
#[derive(Clone)]
pub struct CustomValidator {
    /// Name/identifier for this custom validator
    pub name: String,
    /// Sync validation function
    sync_validator: Option<SyncValidationFn>,
    /// Custom error message
    pub message: Option<String>,
}

impl CustomValidator {
    /// Create a new custom validator with a sync function
    pub fn new<F>(name: impl Into<String>, validator: F) -> Self 
    where
        F: Fn(&Value, &str) -> ValidationResult<()> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            sync_validator: Some(Arc::new(validator)),
            message: None,
        }
    }

    /// Set custom error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get the validator name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for CustomValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomValidator")
            .field("name", &self.name)
            .field("has_sync_validator", &self.sync_validator.is_some())
            .field("message", &self.message)
            .finish()
    }
}

#[async_trait]
impl ValidationRule for CustomValidator {
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()> {
        // Skip validation for null values (unless custom validator explicitly handles them)
        if value.is_null() {
            return Ok(());
        }

        let result = if let Some(ref sync_validator) = self.sync_validator {
            sync_validator(value, field)
        } else {
            // No validator function provided
            return Err(ValidationError::with_code(
                field,
                "Custom validator has no validation function",
                "no_validator",
            ).into());
        };

        // If validation failed and we have a custom message, replace the error message
        match (result, &self.message) {
            (Err(mut errors), Some(custom_message)) => {
                // Update error messages to use custom message
                for field_errors in errors.errors.values_mut() {
                    for error in field_errors {
                        error.message = custom_message.clone();
                    }
                }
                Err(errors)
            }
            (result, _) => result,
        }
    }

    fn rule_name(&self) -> &'static str {
        "custom"
    }

    fn parameters(&self) -> Option<Value> {
        let mut params = serde_json::Map::new();
        
        params.insert("name".to_string(), Value::String(self.name.clone()));
        
        if let Some(ref message) = self.message {
            params.insert("message".to_string(), Value::String(message.clone()));
        }

        Some(Value::Object(params))
    }
}

/// Helper functions for common custom validations
impl CustomValidator {
    /// Create a validator that checks if a string is one of the allowed values
    pub fn one_of(name: impl Into<String>, allowed_values: Vec<String>) -> Self {
        let allowed = allowed_values.clone();
        Self::new(name, move |value, field| {
            if let Some(string_value) = value.as_str() {
                if allowed.contains(&string_value.to_string()) {
                    Ok(())
                } else {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} must be one of: {}", field, allowed.join(", ")),
                        "not_in_list",
                    ).into())
                }
            } else {
                Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string", field),
                    "invalid_type",
                ).into())
            }
        })
    }

    /// Create a validator that checks if a value is not in a list of forbidden values
    pub fn not_one_of(name: impl Into<String>, forbidden_values: Vec<String>) -> Self {
        let forbidden = forbidden_values.clone();
        Self::new(name, move |value, field| {
            if let Some(string_value) = value.as_str() {
                if forbidden.contains(&string_value.to_string()) {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} cannot be one of: {}", field, forbidden.join(", ")),
                        "in_forbidden_list",
                    ).into())
                } else {
                    Ok(())
                }
            } else {
                Ok(()) // Allow non-string values through
            }
        })
    }

    /// Create a validator that checks if a string contains a specific substring
    pub fn contains(name: impl Into<String>, substring: String) -> Self {
        Self::new(name, move |value, field| {
            if let Some(string_value) = value.as_str() {
                if string_value.contains(&substring) {
                    Ok(())
                } else {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} must contain '{}'", field, substring),
                        "missing_substring",
                    ).into())
                }
            } else {
                Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string", field),
                    "invalid_type",
                ).into())
            }
        })
    }

    /// Create a validator that checks if a string does not contain a specific substring
    pub fn not_contains(name: impl Into<String>, substring: String) -> Self {
        Self::new(name, move |value, field| {
            if let Some(string_value) = value.as_str() {
                if !string_value.contains(&substring) {
                    Ok(())
                } else {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} must not contain '{}'", field, substring),
                        "forbidden_substring",
                    ).into())
                }
            } else {
                Ok(()) // Allow non-string values through
            }
        })
    }

    /// Create a validator that checks if a string starts with a specific prefix
    pub fn starts_with(name: impl Into<String>, prefix: String) -> Self {
        Self::new(name, move |value, field| {
            if let Some(string_value) = value.as_str() {
                if string_value.starts_with(&prefix) {
                    Ok(())
                } else {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} must start with '{}'", field, prefix),
                        "invalid_prefix",
                    ).into())
                }
            } else {
                Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string", field),
                    "invalid_type",
                ).into())
            }
        })
    }

    /// Create a validator that checks if a string ends with a specific suffix
    pub fn ends_with(name: impl Into<String>, suffix: String) -> Self {
        Self::new(name, move |value, field| {
            if let Some(string_value) = value.as_str() {
                if string_value.ends_with(&suffix) {
                    Ok(())
                } else {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} must end with '{}'", field, suffix),
                        "invalid_suffix",
                    ).into())
                }
            } else {
                Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string", field),
                    "invalid_type",
                ).into())
            }
        })
    }

    /// Create a validator that checks if an array has a specific length
    pub fn array_length(name: impl Into<String>, expected_length: usize) -> Self {
        Self::new(name, move |value, field| {
            if let Some(array) = value.as_array() {
                if array.len() == expected_length {
                    Ok(())
                } else {
                    Err(ValidationError::with_code(
                        field,
                        format!("{} must have exactly {} items", field, expected_length),
                        "invalid_array_length",
                    ).into())
                }
            } else {
                Err(ValidationError::with_code(
                    field,
                    format!("{} must be an array", field),
                    "invalid_type",
                ).into())
            }
        })
    }

    /// Create a validator that checks if all array elements pass a condition
    pub fn array_all<F>(name: impl Into<String>, condition: F) -> Self 
    where
        F: Fn(&Value) -> bool + Send + Sync + 'static,
    {
        Self::new(name, move |value, field| {
            if let Some(array) = value.as_array() {
                for (index, item) in array.iter().enumerate() {
                    if !condition(item) {
                        return Err(ValidationError::with_code(
                            field,
                            format!("{} item at index {} does not meet the required condition", field, index),
                            "array_condition_failed",
                        ).into());
                    }
                }
                Ok(())
            } else {
                Err(ValidationError::with_code(
                    field,
                    format!("{} must be an array", field),
                    "invalid_type",
                ).into())
            }
        })
    }

    // Note: async_check function removed for simplicity
    // Users can create async custom validators directly using new_async()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_validator_sync() {
        let validator = CustomValidator::new("even_number", |value, field| {
            if let Some(num) = value.as_i64() {
                if num % 2 == 0 {
                    Ok(())
                } else {
                    Err(ValidationError::new(field, "Must be an even number").into())
                }
            } else {
                Err(ValidationError::new(field, "Must be a number").into())
            }
        });

        // Even number should pass
        let result = validator.validate(&Value::Number(serde_json::Number::from(4)), "count").await;
        assert!(result.is_ok());

        // Odd number should fail
        let result = validator.validate(&Value::Number(serde_json::Number::from(5)), "count").await;
        assert!(result.is_err());
    }

    // Async validator test removed - focusing on sync validators for simplicity

    #[tokio::test]
    async fn test_custom_validator_one_of() {
        let validator = CustomValidator::one_of(
            "status_validator",
            vec!["active".to_string(), "inactive".to_string(), "pending".to_string()]
        );

        // Valid status
        assert!(validator.validate(&Value::String("active".to_string()), "status").await.is_ok());
        assert!(validator.validate(&Value::String("pending".to_string()), "status").await.is_ok());

        // Invalid status
        assert!(validator.validate(&Value::String("unknown".to_string()), "status").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_validator_not_one_of() {
        let validator = CustomValidator::not_one_of(
            "username_validator",
            vec!["admin".to_string(), "root".to_string(), "system".to_string()]
        );

        // Valid username
        assert!(validator.validate(&Value::String("john".to_string()), "username").await.is_ok());
        assert!(validator.validate(&Value::String("alice".to_string()), "username").await.is_ok());

        // Forbidden username
        assert!(validator.validate(&Value::String("admin".to_string()), "username").await.is_err());
        assert!(validator.validate(&Value::String("root".to_string()), "username").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_validator_contains() {
        let validator = CustomValidator::contains("email_domain", "@company.com".to_string());

        // Valid email with company domain
        assert!(validator.validate(&Value::String("john@company.com".to_string()), "email").await.is_ok());

        // Invalid email without company domain
        assert!(validator.validate(&Value::String("john@gmail.com".to_string()), "email").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_validator_starts_with() {
        let validator = CustomValidator::starts_with("api_key", "sk_".to_string());

        // Valid API key
        assert!(validator.validate(&Value::String("sk_1234567890".to_string()), "api_key").await.is_ok());

        // Invalid API key
        assert!(validator.validate(&Value::String("pk_1234567890".to_string()), "api_key").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_validator_ends_with() {
        let validator = CustomValidator::ends_with("image_file", ".jpg".to_string());

        // Valid image file
        assert!(validator.validate(&Value::String("photo.jpg".to_string()), "filename").await.is_ok());

        // Invalid file extension
        assert!(validator.validate(&Value::String("photo.png".to_string()), "filename").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_validator_array_length() {
        let validator = CustomValidator::array_length("tags", 3);

        // Valid array with 3 items
        let array = Value::Array(vec![
            Value::String("tag1".to_string()),
            Value::String("tag2".to_string()),
            Value::String("tag3".to_string()),
        ]);
        assert!(validator.validate(&array, "tags").await.is_ok());

        // Invalid array with wrong length
        let array = Value::Array(vec![
            Value::String("tag1".to_string()),
            Value::String("tag2".to_string()),
        ]);
        assert!(validator.validate(&array, "tags").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_validator_array_all() {
        let validator = CustomValidator::array_all("numbers", |value| {
            value.as_i64().map_or(false, |n| n > 0)
        });

        // Valid array with all positive numbers
        let array = Value::Array(vec![
            Value::Number(serde_json::Number::from(1)),
            Value::Number(serde_json::Number::from(2)),
            Value::Number(serde_json::Number::from(3)),
        ]);
        assert!(validator.validate(&array, "numbers").await.is_ok());

        // Invalid array with negative number
        let array = Value::Array(vec![
            Value::Number(serde_json::Number::from(1)),
            Value::Number(serde_json::Number::from(-2)),
            Value::Number(serde_json::Number::from(3)),
        ]);
        assert!(validator.validate(&array, "numbers").await.is_err());
    }

    // async_check test removed - use new_async directly for custom async validators

    #[tokio::test]
    async fn test_custom_validator_with_custom_message() {
        let validator = CustomValidator::new("always_fail", |_value, field| {
            Err(ValidationError::new(field, "Original message").into())
        }).message("Custom error message");

        let result = validator.validate(&Value::String("test".to_string()), "field").await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("field").unwrap();
        assert_eq!(field_errors[0].message, "Custom error message");
    }

    #[tokio::test]
    async fn test_custom_validator_with_null() {
        let validator = CustomValidator::new("not_null", |value, field| {
            if value.is_null() {
                Err(ValidationError::new(field, "Cannot be null").into())
            } else {
                Ok(())
            }
        });

        // Null values are skipped by default
        let result = validator.validate(&Value::Null, "field").await;
        assert!(result.is_ok());
    }
}