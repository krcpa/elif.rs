//! Numeric value validators

use crate::error::{ValidationError, ValidationResult};
use crate::traits::ValidationRule;
use async_trait::async_trait;
use serde_json::Value;

/// Validator for numeric constraints
#[derive(Debug, Clone)]
pub struct NumericValidator {
    /// Minimum value (inclusive)
    pub min: Option<f64>,
    /// Maximum value (inclusive)
    pub max: Option<f64>,
    /// Allow only integers (no decimals)
    pub integer_only: bool,
    /// Allow only positive numbers (> 0)
    pub positive_only: bool,
    /// Allow only negative numbers (< 0)
    pub negative_only: bool,
    /// Custom error message
    pub message: Option<String>,
}

impl NumericValidator {
    /// Create a new numeric validator with default settings
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            integer_only: false,
            positive_only: false,
            negative_only: false,
            message: None,
        }
    }

    /// Set minimum value constraint
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value constraint
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set value range (min and max)
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Require integer values only (no decimals)
    pub fn integer_only(mut self, integer_only: bool) -> Self {
        self.integer_only = integer_only;
        self
    }

    /// Allow only positive numbers (> 0)
    pub fn positive_only(mut self, positive_only: bool) -> Self {
        self.positive_only = positive_only;
        if positive_only {
            self.negative_only = false;
        }
        self
    }

    /// Allow only negative numbers (< 0)
    pub fn negative_only(mut self, negative_only: bool) -> Self {
        self.negative_only = negative_only;
        if negative_only {
            self.positive_only = false;
        }
        self
    }

    /// Set custom error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Extract numeric value from JSON Value
    fn get_numeric_value(&self, value: &Value) -> Option<f64> {
        match value {
            Value::Number(num) => num.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Check if a number is an integer
    fn is_integer(&self, num: f64) -> bool {
        num.fract() == 0.0
    }

    /// Generate appropriate error message
    fn create_error_message(&self, field: &str, value: f64) -> String {
        if let Some(ref custom_message) = self.message {
            return custom_message.clone();
        }

        if self.positive_only && value <= 0.0 {
            return format!("{} must be a positive number", field);
        }

        if self.negative_only && value >= 0.0 {
            return format!("{} must be a negative number", field);
        }

        if self.integer_only && !self.is_integer(value) {
            return format!("{} must be an integer", field);
        }

        match (self.min, self.max) {
            (Some(min), Some(max)) if min == max => {
                format!("{} must equal {}", field, min)
            }
            (Some(min), Some(max)) => {
                format!("{} must be between {} and {}", field, min, max)
            }
            (Some(min), None) => {
                format!("{} must be at least {}", field, min)
            }
            (None, Some(max)) => {
                format!("{} must be at most {}", field, max)
            }
            (None, None) => {
                format!("{} has invalid numeric value: {}", field, value)
            }
        }
    }
}

impl Default for NumericValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationRule for NumericValidator {
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()> {
        // Skip validation for null values
        if value.is_null() {
            return Ok(());
        }

        let num = match self.get_numeric_value(value) {
            Some(n) => n,
            None => {
                return Err(ValidationError::with_code(
                    field,
                    format!("{} must be a numeric value", field),
                    "invalid_type",
                ).into());
            }
        };

        // Check for NaN or infinity
        if !num.is_finite() {
            return Err(ValidationError::with_code(
                field,
                format!("{} must be a finite number", field),
                "invalid_number",
            ).into());
        }

        // Check integer requirement
        if self.integer_only && !self.is_integer(num) {
            return Err(ValidationError::with_code(
                field,
                self.create_error_message(field, num),
                "not_integer",
            ).into());
        }

        // Check positive/negative requirements
        if self.positive_only && num <= 0.0 {
            return Err(ValidationError::with_code(
                field,
                self.create_error_message(field, num),
                "not_positive",
            ).into());
        }

        if self.negative_only && num >= 0.0 {
            return Err(ValidationError::with_code(
                field,
                self.create_error_message(field, num),
                "not_negative",
            ).into());
        }

        // Check minimum value
        if let Some(min) = self.min {
            if num < min {
                return Err(ValidationError::with_code(
                    field,
                    self.create_error_message(field, num),
                    "below_minimum",
                ).into());
            }
        }

        // Check maximum value
        if let Some(max) = self.max {
            if num > max {
                return Err(ValidationError::with_code(
                    field,
                    self.create_error_message(field, num),
                    "above_maximum",
                ).into());
            }
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "numeric"
    }

    fn parameters(&self) -> Option<Value> {
        let mut params = serde_json::Map::new();
        
        if let Some(min) = self.min {
            params.insert("min".to_string(), Value::Number(
                serde_json::Number::from_f64(min).unwrap_or(serde_json::Number::from(0))
            ));
        }
        if let Some(max) = self.max {
            params.insert("max".to_string(), Value::Number(
                serde_json::Number::from_f64(max).unwrap_or(serde_json::Number::from(0))
            ));
        }
        params.insert("integer_only".to_string(), Value::Bool(self.integer_only));
        params.insert("positive_only".to_string(), Value::Bool(self.positive_only));
        params.insert("negative_only".to_string(), Value::Bool(self.negative_only));
        
        if let Some(ref message) = self.message {
            params.insert("message".to_string(), Value::String(message.clone()));
        }

        Some(Value::Object(params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_numeric_validator_basic() {
        let validator = NumericValidator::new();
        
        // Valid numbers
        assert!(validator.validate(&Value::Number(serde_json::Number::from(42)), "age").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(-10)), "temp").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from_f64(3.14).unwrap()), "pi").await.is_ok());
        
        // String representations of numbers
        assert!(validator.validate(&Value::String("42".to_string()), "age").await.is_ok());
        assert!(validator.validate(&Value::String("3.14".to_string()), "pi").await.is_ok());
        
        // Invalid types
        assert!(validator.validate(&Value::String("not-a-number".to_string()), "age").await.is_err());
        assert!(validator.validate(&Value::Bool(true), "age").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_min_max() {
        let validator = NumericValidator::new().range(0.0, 100.0);
        
        // Within range
        assert!(validator.validate(&Value::Number(serde_json::Number::from(50)), "score").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(0)), "score").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(100)), "score").await.is_ok());
        
        // Out of range
        assert!(validator.validate(&Value::Number(serde_json::Number::from(-1)), "score").await.is_err());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(101)), "score").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_integer_only() {
        let validator = NumericValidator::new().integer_only(true);
        
        // Valid integers
        assert!(validator.validate(&Value::Number(serde_json::Number::from(42)), "count").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(0)), "count").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(-10)), "count").await.is_ok());
        
        // Invalid decimals
        assert!(validator.validate(&Value::Number(serde_json::Number::from_f64(3.14).unwrap()), "count").await.is_err());
        assert!(validator.validate(&Value::String("2.5".to_string()), "count").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_positive_only() {
        let validator = NumericValidator::new().positive_only(true);
        
        // Valid positive numbers
        assert!(validator.validate(&Value::Number(serde_json::Number::from(1)), "amount").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from_f64(0.1).unwrap()), "amount").await.is_ok());
        
        // Invalid non-positive numbers
        assert!(validator.validate(&Value::Number(serde_json::Number::from(0)), "amount").await.is_err());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(-1)), "amount").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_negative_only() {
        let validator = NumericValidator::new().negative_only(true);
        
        // Valid negative numbers
        assert!(validator.validate(&Value::Number(serde_json::Number::from(-1)), "debt").await.is_ok());
        assert!(validator.validate(&Value::Number(serde_json::Number::from_f64(-0.1).unwrap()), "debt").await.is_ok());
        
        // Invalid non-negative numbers
        assert!(validator.validate(&Value::Number(serde_json::Number::from(0)), "debt").await.is_err());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(1)), "debt").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_combined_constraints() {
        let validator = NumericValidator::new()
            .range(1.0, 100.0)
            .integer_only(true)
            .positive_only(true);
        
        // Valid: positive integer in range
        assert!(validator.validate(&Value::Number(serde_json::Number::from(42)), "level").await.is_ok());
        
        // Invalid: decimal
        assert!(validator.validate(&Value::Number(serde_json::Number::from_f64(42.5).unwrap()), "level").await.is_err());
        
        // Invalid: out of range
        assert!(validator.validate(&Value::Number(serde_json::Number::from(0)), "level").await.is_err());
        assert!(validator.validate(&Value::Number(serde_json::Number::from(101)), "level").await.is_err());
        
        // Invalid: negative
        assert!(validator.validate(&Value::Number(serde_json::Number::from(-10)), "level").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_string_parsing() {
        let validator = NumericValidator::new().range(0.0, 10.0);
        
        // Valid string numbers
        assert!(validator.validate(&Value::String("5".to_string()), "rating").await.is_ok());
        assert!(validator.validate(&Value::String("7.5".to_string()), "rating").await.is_ok());
        assert!(validator.validate(&Value::String("0".to_string()), "rating").await.is_ok());
        
        // Invalid string numbers (out of range)
        assert!(validator.validate(&Value::String("-1".to_string()), "rating").await.is_err());
        assert!(validator.validate(&Value::String("11".to_string()), "rating").await.is_err());
        
        // Invalid string (not a number)
        assert!(validator.validate(&Value::String("not-a-number".to_string()), "rating").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_infinity_nan() {
        let validator = NumericValidator::new();
        
        // Test with infinity and NaN strings (should be rejected)
        assert!(validator.validate(&Value::String("inf".to_string()), "value").await.is_err());
        assert!(validator.validate(&Value::String("infinity".to_string()), "value").await.is_err());
        assert!(validator.validate(&Value::String("NaN".to_string()), "value").await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_validator_custom_message() {
        let validator = NumericValidator::new()
            .min(18.0)
            .message("Must be at least 18 years old");
        
        let result = validator.validate(&Value::Number(serde_json::Number::from(16)), "age").await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("age").unwrap();
        assert_eq!(field_errors[0].message, "Must be at least 18 years old");
    }

    #[tokio::test]
    async fn test_numeric_validator_with_null() {
        let validator = NumericValidator::new().min(0.0);
        
        // Null values should be skipped
        let result = validator.validate(&Value::Null, "optional_number").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_numeric_validator_error_codes() {
        let validator = NumericValidator::new()
            .range(0.0, 100.0)
            .integer_only(true)
            .positive_only(true);
        
        // Test below minimum error code
        let result = validator.validate(&Value::Number(serde_json::Number::from(-1)), "value").await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors["value"][0].code, "not_positive");
        
        // Test not integer error code
        let result = validator.validate(&Value::Number(serde_json::Number::from_f64(1.5).unwrap()), "value").await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors["value"][0].code, "not_integer");
        
        // Test above maximum error code
        let result = validator.validate(&Value::Number(serde_json::Number::from(101)), "value").await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors["value"][0].code, "above_maximum");
    }
}