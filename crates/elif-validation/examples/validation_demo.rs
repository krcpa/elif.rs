//! Comprehensive validation demo showing all validator types

use elif_validation::{
    RulesBuilder, ValidationError, ValidationRule, Validate,
    RequiredValidator, LengthValidator, EmailValidator, 
    NumericValidator, PatternValidator, CustomValidator,
};
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 elif-validation Demo");
    println!("========================\n");

    // Demo 1: Individual Validators
    demo_individual_validators().await?;
    
    // Demo 2: Rules Builder
    demo_rules_builder().await?;
    
    // Demo 3: Custom Validators
    demo_custom_validators().await?;
    
    // Demo 4: Complex Validation Scenarios
    demo_complex_scenarios().await?;

    println!("✅ All validation demos completed successfully!");
    Ok(())
}

async fn demo_individual_validators() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Demo 1: Individual Validators");
    println!("---------------------------------");

    // Required Validator
    let required = RequiredValidator::new();
    
    println!("Required Validator:");
    match required.validate(&Value::String("hello".to_string()), "name").await {
        Ok(()) => println!("  ✅ 'hello' is valid (not empty)"),
        Err(_) => println!("  ❌ 'hello' failed validation"),
    }
    
    match required.validate(&Value::String("".to_string()), "name").await {
        Ok(()) => println!("  ❌ Empty string should fail"),
        Err(errors) => println!("  ✅ Empty string correctly failed: {}", errors.errors["name"][0].message),
    }

    // Email Validator
    let email = EmailValidator::new();
    
    println!("\nEmail Validator:");
    match email.validate(&Value::String("user@example.com".to_string()), "email").await {
        Ok(()) => println!("  ✅ 'user@example.com' is valid"),
        Err(_) => println!("  ❌ Valid email failed validation"),
    }
    
    match email.validate(&Value::String("invalid-email".to_string()), "email").await {
        Ok(()) => println!("  ❌ Invalid email should fail"),
        Err(errors) => println!("  ✅ Invalid email correctly failed: {}", errors.errors["email"][0].message),
    }

    // Length Validator
    let length = LengthValidator::new().min(3).max(20);
    
    println!("\nLength Validator (3-20 chars):");
    match length.validate(&Value::String("hello".to_string()), "text").await {
        Ok(()) => println!("  ✅ 'hello' is valid (5 chars)"),
        Err(_) => println!("  ❌ 'hello' should be valid"),
    }
    
    match length.validate(&Value::String("hi".to_string()), "text").await {
        Ok(()) => println!("  ❌ 'hi' should fail (too short)"),
        Err(errors) => println!("  ✅ 'hi' correctly failed: {}", errors.errors["text"][0].message),
    }

    // Numeric Validator
    let numeric = NumericValidator::new().range(1.0, 100.0).integer_only(true);
    
    println!("\nNumeric Validator (1-100, integers only):");
    match numeric.validate(&Value::Number(serde_json::Number::from(42)), "score").await {
        Ok(()) => println!("  ✅ 42 is valid"),
        Err(_) => println!("  ❌ 42 should be valid"),
    }
    
    match numeric.validate(&Value::Number(serde_json::Number::from(150)), "score").await {
        Ok(()) => println!("  ❌ 150 should fail (too high)"),
        Err(errors) => println!("  ✅ 150 correctly failed: {}", errors.errors["score"][0].message),
    }

    // Pattern Validator
    let pattern = PatternValidator::new(r"^[A-Z]{3}\d{3}$")?;
    
    println!("\nPattern Validator (3 uppercase letters + 3 digits):");
    match pattern.validate(&Value::String("ABC123".to_string()), "code").await {
        Ok(()) => println!("  ✅ 'ABC123' is valid"),
        Err(_) => println!("  ❌ 'ABC123' should be valid"),
    }
    
    match pattern.validate(&Value::String("abc123".to_string()), "code").await {
        Ok(()) => println!("  ❌ 'abc123' should fail (lowercase)"),
        Err(errors) => println!("  ✅ 'abc123' correctly failed: {}", errors.errors["code"][0].message),
    }

    println!();
    Ok(())
}

async fn demo_rules_builder() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Demo 2: Rules Builder");
    println!("------------------------");

    let rules = RulesBuilder::new()
        .required_email("email")
        .required_string("name", Some(2), Some(50))
        .required_integer("age", Some(1.0), Some(120.0))
        .pattern("phone", r"^\+?[\d\s-()]+$")
        .one_of("status", vec!["active".to_string(), "inactive".to_string()])
        .build();

    println!("Created validation rules for user registration form\n");

    // Test valid data
    let mut valid_data = HashMap::new();
    valid_data.insert("email".to_string(), Value::String("john@example.com".to_string()));
    valid_data.insert("name".to_string(), Value::String("John Doe".to_string()));
    valid_data.insert("age".to_string(), Value::Number(serde_json::Number::from(25)));
    valid_data.insert("phone".to_string(), Value::String("+1-555-123-4567".to_string()));
    valid_data.insert("status".to_string(), Value::String("active".to_string()));

    println!("Testing valid user data:");
    match rules.validate(&valid_data).await {
        Ok(()) => println!("  ✅ All validation rules passed!"),
        Err(errors) => println!("  ❌ Unexpected validation errors: {}", errors),
    }

    // Test invalid data
    let mut invalid_data = HashMap::new();
    invalid_data.insert("email".to_string(), Value::String("not-an-email".to_string()));
    invalid_data.insert("name".to_string(), Value::String("J".to_string())); // Too short
    invalid_data.insert("age".to_string(), Value::Number(serde_json::Number::from(150))); // Too old
    invalid_data.insert("phone".to_string(), Value::String("invalid-phone".to_string()));
    invalid_data.insert("status".to_string(), Value::String("unknown".to_string()));

    println!("\nTesting invalid user data:");
    match rules.validate(&invalid_data).await {
        Ok(()) => println!("  ❌ Validation should have failed"),
        Err(errors) => {
            println!("  ✅ Validation correctly failed with {} field errors:", errors.len());
            for (field, field_errors) in &errors.errors {
                for error in field_errors {
                    println!("    - {}: {}", field, error.message);
                }
            }
        }
    }

    println!();
    Ok(())
}

async fn demo_custom_validators() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️  Demo 3: Custom Validators");
    println!("-----------------------------");

    // Custom sync validator for even numbers
    let even_validator = CustomValidator::new_sync("even_number", |value, field| {
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

    println!("Custom Even Number Validator:");
    match even_validator.validate(&Value::Number(serde_json::Number::from(4)), "count").await {
        Ok(()) => println!("  ✅ 4 is even"),
        Err(_) => println!("  ❌ 4 should be valid"),
    }

    match even_validator.validate(&Value::Number(serde_json::Number::from(5)), "count").await {
        Ok(()) => println!("  ❌ 5 should fail (odd number)"),
        Err(errors) => println!("  ✅ 5 correctly failed: {}", errors.errors["count"][0].message),
    }

    // Custom async validator (simulating database check)
    let unique_username = CustomValidator::new_async("unique_username", |value, field| async move {
        // Simulate async database lookup
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        if let Some(username) = value.as_str() {
            // For demo purposes, consider "admin" and "root" as taken
            if username == "admin" || username == "root" {
                Err(ValidationError::new(field, "Username is already taken").into())
            } else {
                Ok(())
            }
        } else {
            Err(ValidationError::new(field, "Username must be a string").into())
        }
    });

    println!("\nCustom Async Username Validator:");
    match unique_username.validate(&Value::String("john".to_string()), "username").await {
        Ok(()) => println!("  ✅ 'john' is available"),
        Err(_) => println!("  ❌ 'john' should be available"),
    }

    match unique_username.validate(&Value::String("admin".to_string()), "username").await {
        Ok(()) => println!("  ❌ 'admin' should be taken"),
        Err(errors) => println!("  ✅ 'admin' correctly failed: {}", errors.errors["username"][0].message),
    }

    // Built-in custom validators
    let status_validator = CustomValidator::one_of("status", vec![
        "draft".to_string(),
        "published".to_string(),
        "archived".to_string(),
    ]);

    println!("\nBuilt-in Custom 'One Of' Validator:");
    match status_validator.validate(&Value::String("published".to_string()), "status").await {
        Ok(()) => println!("  ✅ 'published' is valid"),
        Err(_) => println!("  ❌ 'published' should be valid"),
    }

    match status_validator.validate(&Value::String("invalid".to_string()), "status").await {
        Ok(()) => println!("  ❌ 'invalid' should fail"),
        Err(errors) => println!("  ✅ 'invalid' correctly failed: {}", errors.errors["status"][0].message),
    }

    println!();
    Ok(())
}

async fn demo_complex_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧩 Demo 4: Complex Validation Scenarios");
    println!("---------------------------------------");

    // Password confirmation validator (cross-field validation)
    let password_confirmation = CustomValidator::new_sync("password_confirmation", |value, _field| {
        if let Some(obj) = value.as_object() {
            let password = obj.get("password").and_then(|v| v.as_str());
            let confirmation = obj.get("password_confirmation").and_then(|v| v.as_str());
            
            match (password, confirmation) {
                (Some(pwd), Some(conf)) if pwd == conf => Ok(()),
                (Some(_), Some(_)) => Err(ValidationError::new("password_confirmation", "Passwords do not match").into()),
                (Some(_), None) => Err(ValidationError::new("password_confirmation", "Password confirmation is required").into()),
                _ => Ok(()), // No password field, skip validation
            }
        } else {
            Ok(())
        }
    });

    let rules = RulesBuilder::new()
        .required_string("password", Some(8), Some(128))
        .pattern("password", r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]+$") // Strong password
        .required_string("password_confirmation", Some(8), Some(128))
        .request_rule(password_confirmation)
        .build();

    println!("Password validation with confirmation matching:");

    // Valid password data
    let mut valid_password = HashMap::new();
    valid_password.insert("password".to_string(), Value::String("MySecureP@ssw0rd".to_string()));
    valid_password.insert("password_confirmation".to_string(), Value::String("MySecureP@ssw0rd".to_string()));

    match rules.validate(&valid_password).await {
        Ok(()) => println!("  ✅ Strong matching passwords validated successfully"),
        Err(errors) => println!("  ❌ Unexpected validation error: {}", errors),
    }

    // Mismatched passwords
    let mut mismatched_password = HashMap::new();
    mismatched_password.insert("password".to_string(), Value::String("MySecureP@ssw0rd".to_string()));
    mismatched_password.insert("password_confirmation".to_string(), Value::String("DifferentP@ssw0rd".to_string()));

    match rules.validate(&mismatched_password).await {
        Ok(()) => println!("  ❌ Mismatched passwords should fail validation"),
        Err(errors) => {
            println!("  ✅ Mismatched passwords correctly failed:");
            for (field, field_errors) in &errors.errors {
                for error in field_errors {
                    println!("    - {}: {}", field, error.message);
                }
            }
        }
    }

    // Weak password
    let mut weak_password = HashMap::new();
    weak_password.insert("password".to_string(), Value::String("weak".to_string()));
    weak_password.insert("password_confirmation".to_string(), Value::String("weak".to_string()));

    match rules.validate(&weak_password).await {
        Ok(()) => println!("  ❌ Weak password should fail validation"),
        Err(errors) => {
            println!("  ✅ Weak password correctly failed:");
            for (field, field_errors) in &errors.errors {
                for error in field_errors {
                    println!("    - {}: {}", field, error.message);
                }
            }
        }
    }

    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demo_functions() {
        // Ensure all demo functions run without panicking
        assert!(demo_individual_validators().await.is_ok());
        assert!(demo_rules_builder().await.is_ok());
        assert!(demo_custom_validators().await.is_ok());
        assert!(demo_complex_scenarios().await.is_ok());
    }
}