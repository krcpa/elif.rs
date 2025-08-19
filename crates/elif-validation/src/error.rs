//! Validation error types and handling

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type ValidationResult<T> = Result<T, ValidationErrors>;

/// Individual validation error for a specific field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationError {
    /// The field that failed validation
    pub field: String,
    /// Human-readable error message
    pub message: String,
    /// Error code for programmatic handling
    pub code: String,
    /// Additional context or hints
    pub context: Option<serde_json::Value>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: "validation_failed".to_string(),
            context: None,
        }
    }

    /// Create a validation error with a specific code
    pub fn with_code(field: impl Into<String>, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: code.into(),
            context: None,
        }
    }

    /// Create a validation error with additional context
    pub fn with_context(field: impl Into<String>, message: impl Into<String>, context: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: "validation_failed".to_string(),
            context: Some(context),
        }
    }

    /// Set the error code
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    /// Set additional context
    pub fn context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Collection of validation errors, typically one per field
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub struct ValidationErrors {
    /// Map of field names to their validation errors
    pub errors: HashMap<String, Vec<ValidationError>>,
}

impl ValidationErrors {
    /// Create a new empty validation errors collection
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    /// Add a single validation error
    pub fn add(&mut self, error: ValidationError) {
        self.errors
            .entry(error.field.clone())
            .or_default()
            .push(error);
    }

    /// Add multiple validation errors for a field
    pub fn add_errors(&mut self, field: impl Into<String>, errors: Vec<ValidationError>) {
        let field = field.into();
        self.errors
            .entry(field)
            .or_default()
            .extend(errors);
    }

    /// Add a simple validation error with field and message
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        let error = ValidationError::new(field.into(), message);
        self.add(error);
    }

    /// Check if there are any validation errors
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of fields with errors
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Get total number of validation errors across all fields
    pub fn total_errors(&self) -> usize {
        self.errors.values().map(|v| v.len()).sum()
    }

    /// Get errors for a specific field
    pub fn get_field_errors(&self, field: &str) -> Option<&Vec<ValidationError>> {
        self.errors.get(field)
    }

    /// Check if a specific field has errors
    pub fn has_field_errors(&self, field: &str) -> bool {
        self.errors.contains_key(field) && !self.errors[field].is_empty()
    }

    /// Merge another ValidationErrors into this one
    pub fn merge(&mut self, other: ValidationErrors) {
        for (field, errors) in other.errors {
            self.errors
                .entry(field)
                .or_default()
                .extend(errors);
        }
    }

    /// Create ValidationErrors from a single error
    pub fn from_error(error: ValidationError) -> Self {
        let mut errors = Self::new();
        errors.add(error);
        errors
    }

    /// Convert to a JSON-serializable format for API responses
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "code": "validation_failed",
                "message": "Validation failed",
                "fields": self.errors
            }
        })
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            write!(f, "No validation errors")
        } else {
            write!(f, "Validation failed for {} field(s):", self.errors.len())?;
            for (field, field_errors) in &self.errors {
                for error in field_errors {
                    write!(f, "\n  {}: {}", field, error.message)?;
                }
            }
            Ok(())
        }
    }
}

impl From<ValidationError> for ValidationErrors {
    fn from(error: ValidationError) -> Self {
        Self::from_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError::new("email", "Invalid email format");
        assert_eq!(error.field, "email");
        assert_eq!(error.message, "Invalid email format");
        assert_eq!(error.code, "validation_failed");
        assert!(error.context.is_none());
    }

    #[test]
    fn test_validation_error_with_code() {
        let error = ValidationError::with_code("age", "Must be positive", "positive_number");
        assert_eq!(error.code, "positive_number");
    }

    #[test]
    fn test_validation_errors_collection() {
        let mut errors = ValidationErrors::new();
        
        errors.add_error("email", "Invalid format");
        errors.add_error("age", "Must be positive");
        errors.add_error("email", "Already exists");

        assert_eq!(errors.len(), 2); // Two fields with errors
        assert_eq!(errors.total_errors(), 3); // Three total errors
        assert!(errors.has_field_errors("email"));
        assert!(errors.has_field_errors("age"));
        assert!(!errors.has_field_errors("name"));

        let email_errors = errors.get_field_errors("email").unwrap();
        assert_eq!(email_errors.len(), 2);
    }

    #[test]
    fn test_validation_errors_merge() {
        let mut errors1 = ValidationErrors::new();
        errors1.add_error("field1", "Error 1");

        let mut errors2 = ValidationErrors::new();
        errors2.add_error("field2", "Error 2");
        errors2.add_error("field1", "Error 3");

        errors1.merge(errors2);

        assert_eq!(errors1.len(), 2);
        assert_eq!(errors1.total_errors(), 3);
        assert_eq!(errors1.get_field_errors("field1").unwrap().len(), 2);
    }
}