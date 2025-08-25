//! Email format validator

use crate::error::{ValidationError, ValidationResult};
use crate::traits::ValidationRule;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

/// Validator for email address format
#[derive(Debug, Clone)]
pub struct EmailValidator {
    /// Custom error message
    pub message: Option<String>,
    /// Allow international domain names
    pub allow_unicode: bool,
    /// Require TLD (top-level domain)
    pub require_tld: bool,
    /// Custom regex pattern (overrides default)
    pub custom_pattern: Option<Regex>,
}

impl EmailValidator {
    /// Create a new email validator with default settings
    pub fn new() -> Self {
        Self {
            message: None,
            allow_unicode: false,
            require_tld: true,
            custom_pattern: None,
        }
    }

    /// Set custom error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Allow unicode characters in domain names (internationalized domains)
    pub fn allow_unicode(mut self, allow: bool) -> Self {
        self.allow_unicode = allow;
        self
    }

    /// Require top-level domain (e.g., .com, .org)
    pub fn require_tld(mut self, require: bool) -> Self {
        self.require_tld = require;
        self
    }

    /// Use custom regex pattern for validation
    pub fn custom_pattern(mut self, pattern: Regex) -> Self {
        self.custom_pattern = Some(pattern);
        self
    }

    /// Get the appropriate regex pattern based on configuration
    fn get_pattern(&self) -> Result<Regex, regex::Error> {
        if let Some(ref pattern) = self.custom_pattern {
            return Ok(pattern.clone());
        }

        // Basic email regex pattern
        // This is a simplified pattern that catches most common cases
        // For production use, consider using a dedicated email validation library
        let pattern = if self.allow_unicode {
            if self.require_tld {
                // Unicode-aware with TLD requirement
                r"^[^\s@.]+[^\s@]*@[^\s@.]+[^\s@]*\.[^\s@]+$"
            } else {
                // Unicode-aware without TLD requirement
                r"^[^\s@.]+[^\s@]*@[^\s@.]+[^\s@]*$"
            }
        } else if self.require_tld {
            // ASCII-only with TLD requirement (no consecutive dots)
            r"^[a-zA-Z0-9]([a-zA-Z0-9._%+-]*[a-zA-Z0-9])?@[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?\.[a-zA-Z]{2,}$"
        } else {
            // ASCII-only without TLD requirement (no consecutive dots)
            r"^[a-zA-Z0-9]([a-zA-Z0-9._%+-]*[a-zA-Z0-9])?@[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?$"
        };

        Regex::new(pattern)
    }

    /// Validate email format
    fn validate_email_format(&self, email: &str) -> bool {
        // Basic checks first
        if email.is_empty() {
            return false;
        }

        // Must contain exactly one @ symbol
        let at_count = email.matches('@').count();
        if at_count != 1 {
            return false;
        }

        // Split into local and domain parts
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }

        let local_part = parts[0];
        let domain_part = parts[1];

        // Local part cannot be empty
        if local_part.is_empty() {
            return false;
        }

        // Domain part cannot be empty
        if domain_part.is_empty() {
            return false;
        }

        // Local part length check (RFC 5321 limit)
        if local_part.len() > 64 {
            return false;
        }

        // Domain part length check
        if domain_part.len() > 255 {
            return false;
        }

        // Use regex for detailed format validation
        match self.get_pattern() {
            Ok(regex) => regex.is_match(email),
            Err(_) => false, // If regex compilation fails, consider invalid
        }
    }
}

impl Default for EmailValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationRule for EmailValidator {
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()> {
        // Skip validation for null values
        if value.is_null() {
            return Ok(());
        }

        let email = match value.as_str() {
            Some(email) => email,
            None => {
                return Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string for email validation", field),
                    "invalid_type",
                )
                .into());
            }
        };

        if !self.validate_email_format(email) {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| format!("{} must be a valid email address", field));

            return Err(ValidationError::with_code(field, message, "invalid_email").into());
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "email"
    }

    fn parameters(&self) -> Option<Value> {
        let mut params = serde_json::Map::new();

        if let Some(ref message) = self.message {
            params.insert("message".to_string(), Value::String(message.clone()));
        }
        params.insert("allow_unicode".to_string(), Value::Bool(self.allow_unicode));
        params.insert("require_tld".to_string(), Value::Bool(self.require_tld));

        if !params.is_empty() {
            Some(Value::Object(params))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_validator_valid_emails() {
        let validator = EmailValidator::new();

        let valid_emails = vec![
            "test@example.com",
            "user.name@domain.co.uk",
            "first+last@subdomain.example.org",
            "user123@test-domain.com",
            "a@b.co",
        ];

        for email in valid_emails {
            let result = validator
                .validate(&Value::String(email.to_string()), "email")
                .await;
            assert!(result.is_ok(), "Email '{}' should be valid", email);
        }
    }

    #[tokio::test]
    async fn test_email_validator_invalid_emails() {
        let validator = EmailValidator::new();

        let toolong_email = format!("toolong{}@domain.com", "a".repeat(60));
        let invalid_emails = vec![
            "",                   // Empty
            "plainaddress",       // No @
            "@missingdomain.com", // No local part
            "missing@.com",       // No domain name
            "double@@domain.com", // Double @
            "spaces @domain.com", // Spaces
            &toolong_email,       // Local part too long
            "test@",              // No domain
            "test@domain",        // No TLD (when required)
        ];

        for email in invalid_emails {
            let result = validator
                .validate(&Value::String(email.to_string()), "email")
                .await;
            assert!(result.is_err(), "Email '{}' should be invalid", email);
        }
    }

    #[tokio::test]
    async fn test_email_validator_without_tld_requirement() {
        let validator = EmailValidator::new().require_tld(false);

        // Should be valid without TLD requirement
        let result = validator
            .validate(&Value::String("test@localhost".to_string()), "email")
            .await;
        assert!(result.is_ok());

        let result = validator
            .validate(&Value::String("admin@intranet".to_string()), "email")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_email_validator_unicode_domain() {
        let validator = EmailValidator::new().allow_unicode(true);

        // Should handle unicode domains when allowed
        let result = validator
            .validate(&Value::String("test@тест.рф".to_string()), "email")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_email_validator_custom_pattern() {
        let custom_regex = Regex::new(r"^[a-z]+@company\.com$").unwrap();
        let validator = EmailValidator::new().custom_pattern(custom_regex);

        // Should match custom pattern
        let result = validator
            .validate(&Value::String("john@company.com".to_string()), "email")
            .await;
        assert!(result.is_ok());

        // Should not match custom pattern
        let result = validator
            .validate(&Value::String("john@otherdomain.com".to_string()), "email")
            .await;
        assert!(result.is_err());

        let result = validator
            .validate(&Value::String("John@company.com".to_string()), "email")
            .await;
        assert!(result.is_err()); // Uppercase not allowed in custom pattern
    }

    #[tokio::test]
    async fn test_email_validator_custom_message() {
        let validator = EmailValidator::new().message("Please enter a valid email address");

        let result = validator
            .validate(&Value::String("invalid-email".to_string()), "email")
            .await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("email").unwrap();
        assert_eq!(
            field_errors[0].message,
            "Please enter a valid email address"
        );
    }

    #[tokio::test]
    async fn test_email_validator_with_null() {
        let validator = EmailValidator::new();

        // Null values should be skipped
        let result = validator.validate(&Value::Null, "email").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_email_validator_invalid_type() {
        let validator = EmailValidator::new();

        // Numbers should fail type validation
        let result = validator
            .validate(&Value::Number(serde_json::Number::from(42)), "email")
            .await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("email").unwrap();
        assert_eq!(field_errors[0].code, "invalid_type");
    }

    #[tokio::test]
    async fn test_email_validator_edge_cases() {
        let validator = EmailValidator::new();

        // Test some edge cases
        let edge_cases = vec![
            ("aa@bb.cc", true),            // Minimal valid email
            ("test@test@test.com", false), // Multiple @ symbols
            ("test@domain.com", true),     // Valid email
        ];

        for (email, should_be_valid) in edge_cases {
            let result = validator
                .validate(&Value::String(email.to_string()), "email")
                .await;
            if should_be_valid {
                assert!(result.is_ok(), "Email '{}' should be valid", email);
            } else {
                assert!(result.is_err(), "Email '{}' should be invalid", email);
            }
        }
    }
}
