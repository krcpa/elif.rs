//! Request validation utilities

use crate::errors::{HttpError, HttpResult};

/// Validation trait for request data
pub trait Validate {
    fn validate(&self) -> HttpResult<()>;
}

/// Helper functions for common validation patterns
pub fn validate_required<T>(field: &Option<T>, field_name: &str) -> HttpResult<()> {
    if field.is_none() {
        return Err(HttpError::bad_request(format!(
            "{} is required",
            field_name
        )));
    }
    Ok(())
}

pub fn validate_min_length(value: &str, min: usize, field_name: &str) -> HttpResult<()> {
    if value.len() < min {
        return Err(HttpError::bad_request(format!(
            "{} must be at least {} characters long",
            field_name, min
        )));
    }
    Ok(())
}

pub fn validate_max_length(value: &str, max: usize, field_name: &str) -> HttpResult<()> {
    if value.len() > max {
        return Err(HttpError::bad_request(format!(
            "{} must be at most {} characters long",
            field_name, max
        )));
    }
    Ok(())
}

pub fn validate_email(email: &str, field_name: &str) -> HttpResult<()> {
    // Basic email validation - must have @ and . with content around them
    if !email.contains('@') || !email.contains('.') {
        return Err(HttpError::bad_request(format!(
            "{} must be a valid email address",
            field_name
        )));
    }

    // Must have at least one character before @
    let at_pos = email.find('@').unwrap();
    if at_pos == 0 {
        return Err(HttpError::bad_request(format!(
            "{} must be a valid email address",
            field_name
        )));
    }

    // Must have content after @ and a dot in the domain part
    let domain_part = &email[at_pos + 1..];
    if domain_part.is_empty() || !domain_part.contains('.') {
        return Err(HttpError::bad_request(format!(
            "{} must be a valid email address",
            field_name
        )));
    }

    // Domain must have content after the last dot
    let last_dot_pos = domain_part.rfind('.').unwrap();
    if last_dot_pos == domain_part.len() - 1 {
        return Err(HttpError::bad_request(format!(
            "{} must be a valid email address",
            field_name
        )));
    }

    Ok(())
}

pub fn validate_range<T: PartialOrd>(value: T, min: T, max: T, field_name: &str) -> HttpResult<()> {
    if value < min || value > max {
        return Err(HttpError::bad_request(format!(
            "{} must be between {} and {}",
            field_name,
            std::any::type_name::<T>(),
            std::any::type_name::<T>()
        )));
    }
    Ok(())
}

pub fn validate_pattern(value: &str, pattern: &str, field_name: &str) -> HttpResult<()> {
    if !regex_simple_match(value, pattern) {
        return Err(HttpError::bad_request(format!(
            "{} does not match required pattern",
            field_name
        )));
    }
    Ok(())
}

// Simple pattern matching without regex dependency - for basic patterns
fn regex_simple_match(value: &str, pattern: &str) -> bool {
    match pattern {
        "alphanumeric" => value.chars().all(char::is_alphanumeric),
        "numeric" => value.chars().all(char::is_numeric),
        "alpha" => value.chars().all(char::is_alphabetic),
        _ => true, // Default to true for unknown patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_required_success() {
        let value = Some("test");
        let result = validate_required(&value, "test_field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_required_failure() {
        let value: Option<String> = None;
        let result = validate_required(&value, "test_field");
        assert!(result.is_err());

        if let Err(HttpError::BadRequest { message }) = result {
            assert!(message.contains("test_field is required"));
        } else {
            panic!("Expected BadRequest error");
        }
    }

    #[test]
    fn test_validate_min_length_success() {
        let result = validate_min_length("hello world", 5, "message");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_min_length_failure() {
        let result = validate_min_length("hi", 5, "message");
        assert!(result.is_err());

        if let Err(HttpError::BadRequest { message }) = result {
            assert!(message.contains("message must be at least 5 characters long"));
        } else {
            panic!("Expected BadRequest error");
        }
    }

    #[test]
    fn test_validate_min_length_exact() {
        let result = validate_min_length("exact", 5, "message");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_max_length_success() {
        let result = validate_max_length("short", 10, "message");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_max_length_failure() {
        let result = validate_max_length("this is a very long message", 10, "message");
        assert!(result.is_err());

        if let Err(HttpError::BadRequest { message }) = result {
            assert!(message.contains("message must be at most 10 characters long"));
        } else {
            panic!("Expected BadRequest error");
        }
    }

    #[test]
    fn test_validate_max_length_exact() {
        let result = validate_max_length("exactly10c", 10, "message");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_valid() {
        let valid_emails = [
            "user@example.com",
            "test.email@domain.org",
            "user+tag@example.co.uk",
        ];

        for email in valid_emails {
            let result = validate_email(email, "email");
            assert!(result.is_ok(), "Failed to validate email: {}", email);
        }
    }

    #[test]
    fn test_validate_email_invalid() {
        let invalid_emails = [
            "invalid",
            "no-at-sign.com",
            "no-domain@",
            "@no-user.com",
            "no.dot@domain",
        ];

        for email in invalid_emails {
            let result = validate_email(email, "email");
            assert!(
                result.is_err(),
                "Should have failed to validate email: {}",
                email
            );

            if let Err(HttpError::BadRequest { message }) = result {
                assert!(message.contains("email must be a valid email address"));
            } else {
                panic!("Expected BadRequest error");
            }
        }
    }

    #[test]
    fn test_validate_range_success() {
        let result = validate_range(5, 1, 10, "number");
        assert!(result.is_ok());

        let result = validate_range(1, 1, 10, "number");
        assert!(result.is_ok());

        let result = validate_range(10, 1, 10, "number");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_failure() {
        let result = validate_range(0, 1, 10, "number");
        assert!(result.is_err());

        let result = validate_range(11, 1, 10, "number");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pattern_alphanumeric() {
        let result = validate_pattern("test123", "alphanumeric", "username");
        assert!(result.is_ok());

        let result = validate_pattern("test-123", "alphanumeric", "username");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pattern_numeric() {
        let result = validate_pattern("12345", "numeric", "id");
        assert!(result.is_ok());

        let result = validate_pattern("123a5", "numeric", "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pattern_alpha() {
        let result = validate_pattern("hello", "alpha", "name");
        assert!(result.is_ok());

        let result = validate_pattern("hello123", "alpha", "name");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pattern_unknown() {
        // Unknown patterns should default to true
        let result = validate_pattern("anything", "unknown_pattern", "field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_trait_implementation() {
        struct TestStruct {
            name: Option<String>,
            email: String,
            age: u32,
        }

        impl Validate for TestStruct {
            fn validate(&self) -> HttpResult<()> {
                validate_required(&self.name, "name")?;
                validate_email(&self.email, "email")?;
                validate_range(self.age, 0, 120, "age")?;
                Ok(())
            }
        }

        let valid_struct = TestStruct {
            name: Some("John".to_string()),
            email: "john@example.com".to_string(),
            age: 25,
        };
        assert!(valid_struct.validate().is_ok());

        let invalid_struct = TestStruct {
            name: None,
            email: "invalid-email".to_string(),
            age: 150,
        };
        assert!(invalid_struct.validate().is_err());
    }

    #[test]
    fn test_chained_validations() {
        fn validate_user_input(name: &Option<String>, email: &str, age: u32) -> HttpResult<()> {
            validate_required(name, "name")?;
            if let Some(name_value) = name {
                validate_min_length(name_value, 2, "name")?;
                validate_max_length(name_value, 50, "name")?;
            }
            validate_email(email, "email")?;
            validate_range(age, 13, 120, "age")?;
            Ok(())
        }

        let result = validate_user_input(&Some("John".to_string()), "john@example.com", 25);
        assert!(result.is_ok());

        let result = validate_user_input(&None, "john@example.com", 25);
        assert!(result.is_err());

        let result = validate_user_input(
            &Some("J".to_string()), // Too short
            "john@example.com",
            25,
        );
        assert!(result.is_err());
    }
}
