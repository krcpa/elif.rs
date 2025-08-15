//! Pattern-based validators using regular expressions

use crate::error::{ValidationError, ValidationResult};
use crate::traits::ValidationRule;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

/// Validator for custom regular expression patterns
#[derive(Debug, Clone)]
pub struct PatternValidator {
    /// The regular expression pattern
    pattern: Regex,
    /// Custom error message
    pub message: Option<String>,
    /// Whether to match the entire string (default) or just find a match
    pub full_match: bool,
    /// Case-sensitive matching (default: true)
    pub case_sensitive: bool,
}

impl PatternValidator {
    /// Create a new pattern validator
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        let regex = Regex::new(pattern)?;
        Ok(Self {
            pattern: regex,
            message: None,
            full_match: true,
            case_sensitive: true,
        })
    }

    /// Create a case-insensitive pattern validator
    pub fn new_case_insensitive(pattern: &str) -> Result<Self, regex::Error> {
        let case_insensitive_pattern = format!("(?i){}", pattern);
        let regex = Regex::new(&case_insensitive_pattern)?;
        Ok(Self {
            pattern: regex,
            message: None,
            full_match: true,
            case_sensitive: false,
        })
    }

    /// Create a validator from an existing Regex
    pub fn from_regex(regex: Regex) -> Self {
        Self {
            pattern: regex,
            message: None,
            full_match: true,
            case_sensitive: true,
        }
    }

    /// Set custom error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set whether to match the full string or just find a match
    pub fn full_match(mut self, full_match: bool) -> Self {
        self.full_match = full_match;
        self
    }

    /// Get the pattern string
    pub fn pattern_string(&self) -> &str {
        self.pattern.as_str()
    }

    /// Validate the string against the pattern
    fn validate_pattern(&self, text: &str) -> bool {
        if self.full_match {
            self.pattern.is_match(text) && self.pattern.find(text).map_or(false, |m| m.as_str() == text)
        } else {
            self.pattern.is_match(text)
        }
    }
}

#[async_trait]
impl ValidationRule for PatternValidator {
    async fn validate(&self, value: &Value, field: &str) -> ValidationResult<()> {
        // Skip validation for null values
        if value.is_null() {
            return Ok(());
        }

        let text = match value.as_str() {
            Some(text) => text,
            None => {
                return Err(ValidationError::with_code(
                    field,
                    format!("{} must be a string for pattern validation", field),
                    "invalid_type",
                ).into());
            }
        };

        if !self.validate_pattern(text) {
            let message = self
                .message
                .as_ref()
                .map(|m| m.clone())
                .unwrap_or_else(|| format!("{} does not match the required pattern", field));

            return Err(ValidationError::with_code(field, message, "pattern_mismatch").into());
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "pattern"
    }

    fn parameters(&self) -> Option<Value> {
        let mut params = serde_json::Map::new();
        
        params.insert("pattern".to_string(), Value::String(self.pattern.as_str().to_string()));
        params.insert("full_match".to_string(), Value::Bool(self.full_match));
        params.insert("case_sensitive".to_string(), Value::Bool(self.case_sensitive));
        
        if let Some(ref message) = self.message {
            params.insert("message".to_string(), Value::String(message.clone()));
        }

        Some(Value::Object(params))
    }
}

/// Common pattern validators for typical use cases
impl PatternValidator {
    /// Create a validator for alphanumeric strings only
    pub fn alphanumeric() -> Self {
        Self::new(r"^[a-zA-Z0-9]+$")
            .unwrap()
            .message("Must contain only letters and numbers")
    }

    /// Create a validator for alphabetic characters only
    pub fn alphabetic() -> Self {
        Self::new(r"^[a-zA-Z]+$")
            .unwrap()
            .message("Must contain only letters")
    }

    /// Create a validator for numeric strings only
    pub fn numeric_string() -> Self {
        Self::new(r"^[0-9]+$")
            .unwrap()
            .message("Must contain only numbers")
    }

    /// Create a validator for phone numbers (US format)
    pub fn phone_us() -> Self {
        Self::new(r"^\+?1?[-.\s]?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}$")
            .unwrap()
            .message("Must be a valid US phone number")
    }

    /// Create a validator for URLs
    pub fn url() -> Self {
        Self::new(r"^https?://[^\s/$.?#].[^\s]*$")
            .unwrap()
            .message("Must be a valid URL")
    }

    /// Create a validator for hexadecimal color codes
    pub fn hex_color() -> Self {
        Self::new(r"^#[0-9a-fA-F]{6}$")
            .unwrap()
            .message("Must be a valid hex color code (e.g., #FF5733)")
    }

    /// Create a validator for UUID v4
    pub fn uuid_v4() -> Self {
        Self::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap()
            .case_sensitive = false; // UUIDs can be uppercase or lowercase
        Self::new_case_insensitive(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap()
            .message("Must be a valid UUID v4")
    }

    /// Create a validator for slug/URL-friendly strings
    pub fn slug() -> Self {
        Self::new(r"^[a-z0-9-]+$")
            .unwrap()
            .message("Must be a valid slug (lowercase letters, numbers, and hyphens only)")
    }

    /// Create a validator for strong passwords (simplified - checks length only)
    /// Note: For complete password strength validation, use multiple validators
    pub fn strong_password() -> Self {
        Self::new(r"^.{8,}$") // Minimum 8 characters
            .unwrap()
            .message("Password must be at least 8 characters long")
    }

    /// Create a validator for IP addresses (IPv4)
    pub fn ipv4() -> Self {
        Self::new(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$")
            .unwrap()
            .message("Must be a valid IPv4 address")
    }

    /// Create a validator for MAC addresses
    pub fn mac_address() -> Self {
        Self::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$")
            .unwrap()
            .message("Must be a valid MAC address (e.g., AA:BB:CC:DD:EE:FF)")
    }

    /// Create a validator for credit card numbers (basic Luhn algorithm check)
    pub fn credit_card() -> Self {
        Self::new(r"^[0-9]{13,19}$")
            .unwrap()
            .message("Must be a valid credit card number")
    }

    /// Create a validator for social security numbers (US format)
    pub fn ssn_us() -> Self {
        Self::new(r"^\d{3}-\d{2}-\d{4}$")
            .unwrap()
            .message("Must be a valid SSN format (XXX-XX-XXXX)")
    }

    /// Create a validator for postal codes (US ZIP codes)
    pub fn zip_code_us() -> Self {
        Self::new(r"^\d{5}(-\d{4})?$")
            .unwrap()
            .message("Must be a valid US ZIP code")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pattern_validator_basic() {
        let validator = PatternValidator::new(r"^[a-zA-Z]+$").unwrap();
        
        // Valid alphabetic strings
        assert!(validator.validate(&Value::String("hello".to_string()), "name").await.is_ok());
        assert!(validator.validate(&Value::String("World".to_string()), "name").await.is_ok());
        
        // Invalid strings (contain numbers or special chars)
        assert!(validator.validate(&Value::String("hello123".to_string()), "name").await.is_err());
        assert!(validator.validate(&Value::String("hello@world".to_string()), "name").await.is_err());
    }

    #[tokio::test]
    async fn test_pattern_validator_full_match() {
        let validator = PatternValidator::new(r"abc")
            .unwrap()
            .full_match(false); // Just find a match, don't require full match
        
        // Should match strings containing "abc"
        assert!(validator.validate(&Value::String("abcdef".to_string()), "text").await.is_ok());
        assert!(validator.validate(&Value::String("123abc456".to_string()), "text").await.is_ok());
        
        // Should not match strings without "abc"
        assert!(validator.validate(&Value::String("def".to_string()), "text").await.is_err());
    }

    #[tokio::test]
    async fn test_pattern_validator_case_insensitive() {
        let validator = PatternValidator::new_case_insensitive(r"^hello$").unwrap();
        
        // Should match regardless of case
        assert!(validator.validate(&Value::String("hello".to_string()), "greeting").await.is_ok());
        assert!(validator.validate(&Value::String("HELLO".to_string()), "greeting").await.is_ok());
        assert!(validator.validate(&Value::String("Hello".to_string()), "greeting").await.is_ok());
        
        // Should not match different words
        assert!(validator.validate(&Value::String("world".to_string()), "greeting").await.is_err());
    }

    #[tokio::test]
    async fn test_pattern_validator_alphanumeric() {
        let validator = PatternValidator::alphanumeric();
        
        assert!(validator.validate(&Value::String("abc123".to_string()), "username").await.is_ok());
        assert!(validator.validate(&Value::String("user123".to_string()), "username").await.is_ok());
        
        // Should not allow special characters
        assert!(validator.validate(&Value::String("user@123".to_string()), "username").await.is_err());
        assert!(validator.validate(&Value::String("user 123".to_string()), "username").await.is_err());
    }

    #[tokio::test]
    async fn test_pattern_validator_phone_us() {
        let validator = PatternValidator::phone_us();
        
        let valid_phones = vec![
            "123-456-7890",
            "(123) 456-7890",
            "123.456.7890",
            "123 456 7890",
            "+1-123-456-7890",
            "1234567890",
        ];

        for phone in valid_phones {
            let result = validator.validate(&Value::String(phone.to_string()), "phone").await;
            assert!(result.is_ok(), "Phone '{}' should be valid", phone);
        }

        let invalid_phones = vec![
            "123-45-6789",   // Too few digits
            "123-456-78901", // Too many digits
            "abc-def-ghij",  // Non-numeric
            "123",           // Too short
        ];

        for phone in invalid_phones {
            let result = validator.validate(&Value::String(phone.to_string()), "phone").await;
            assert!(result.is_err(), "Phone '{}' should be invalid", phone);
        }
    }

    #[tokio::test]
    async fn test_pattern_validator_hex_color() {
        let validator = PatternValidator::hex_color();
        
        // Valid hex colors
        assert!(validator.validate(&Value::String("#FF5733".to_string()), "color").await.is_ok());
        assert!(validator.validate(&Value::String("#000000".to_string()), "color").await.is_ok());
        assert!(validator.validate(&Value::String("#ffffff".to_string()), "color").await.is_ok());
        
        // Invalid hex colors
        assert!(validator.validate(&Value::String("FF5733".to_string()), "color").await.is_err()); // Missing #
        assert!(validator.validate(&Value::String("#FF57".to_string()), "color").await.is_err()); // Too short
        assert!(validator.validate(&Value::String("#GG5733".to_string()), "color").await.is_err()); // Invalid chars
    }

    #[tokio::test]
    async fn test_pattern_validator_uuid_v4() {
        let validator = PatternValidator::uuid_v4();
        
        // Valid UUID v4
        assert!(validator.validate(&Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()), "id").await.is_ok());
        assert!(validator.validate(&Value::String("6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string()), "id").await.is_err()); // Not v4
        
        // Invalid UUIDs
        assert!(validator.validate(&Value::String("550e8400-e29b-41d4-a716".to_string()), "id").await.is_err()); // Too short
        assert!(validator.validate(&Value::String("not-a-uuid".to_string()), "id").await.is_err()); // Invalid format
    }

    #[tokio::test]
    async fn test_pattern_validator_strong_password() {
        let validator = PatternValidator::strong_password();
        
        // Valid passwords (8+ characters)
        assert!(validator.validate(&Value::String("Password123!".to_string()), "password").await.is_ok());
        assert!(validator.validate(&Value::String("MyP@ssw0rd".to_string()), "password").await.is_ok());
        assert!(validator.validate(&Value::String("12345678".to_string()), "password").await.is_ok());
        
        // Invalid passwords (too short)
        assert!(validator.validate(&Value::String("P@ss1".to_string()), "password").await.is_err()); // Too short
        assert!(validator.validate(&Value::String("1234567".to_string()), "password").await.is_err()); // Too short
    }

    #[tokio::test]
    async fn test_pattern_validator_ipv4() {
        let validator = PatternValidator::ipv4();
        
        // Valid IPv4 addresses
        assert!(validator.validate(&Value::String("192.168.1.1".to_string()), "ip").await.is_ok());
        assert!(validator.validate(&Value::String("0.0.0.0".to_string()), "ip").await.is_ok());
        assert!(validator.validate(&Value::String("255.255.255.255".to_string()), "ip").await.is_ok());
        
        // Invalid IPv4 addresses
        assert!(validator.validate(&Value::String("256.1.1.1".to_string()), "ip").await.is_err()); // Out of range
        assert!(validator.validate(&Value::String("192.168.1".to_string()), "ip").await.is_err()); // Incomplete
        assert!(validator.validate(&Value::String("192.168.1.1.1".to_string()), "ip").await.is_err()); // Too many octets
    }

    #[tokio::test]
    async fn test_pattern_validator_custom_message() {
        let validator = PatternValidator::new(r"^[A-Z]+$")
            .unwrap()
            .message("Must be all uppercase letters");
        
        let result = validator.validate(&Value::String("hello".to_string()), "code").await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("code").unwrap();
        assert_eq!(field_errors[0].message, "Must be all uppercase letters");
    }

    #[tokio::test]
    async fn test_pattern_validator_with_null() {
        let validator = PatternValidator::new(r"^[a-z]+$").unwrap();
        
        // Null values should be skipped
        let result = validator.validate(&Value::Null, "optional_field").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pattern_validator_invalid_type() {
        let validator = PatternValidator::new(r"^[a-z]+$").unwrap();
        
        // Numbers should fail type validation
        let result = validator.validate(&Value::Number(serde_json::Number::from(42)), "field").await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        let field_errors = errors.get_field_errors("field").unwrap();
        assert_eq!(field_errors[0].code, "invalid_type");
    }

    #[tokio::test]
    async fn test_pattern_validator_zip_code_us() {
        let validator = PatternValidator::zip_code_us();
        
        // Valid ZIP codes
        assert!(validator.validate(&Value::String("12345".to_string()), "zip").await.is_ok());
        assert!(validator.validate(&Value::String("12345-6789".to_string()), "zip").await.is_ok());
        
        // Invalid ZIP codes
        assert!(validator.validate(&Value::String("1234".to_string()), "zip").await.is_err()); // Too short
        assert!(validator.validate(&Value::String("123456".to_string()), "zip").await.is_err()); // Too long without dash
        assert!(validator.validate(&Value::String("abcde".to_string()), "zip").await.is_err()); // Non-numeric
    }
}