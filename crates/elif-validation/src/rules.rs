//! Validation rules builder and composition system

use crate::error::{ValidationErrors, ValidationResult};
use crate::traits::{ValidateField, ValidateRequest, ValidationRule};
use crate::validators::*;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use service_builder::builder;

/// Collection of validation rules for a specific field or request
#[derive(Clone)]
pub struct Rules {
    /// Field-level validation rules
    field_rules: HashMap<String, Vec<Arc<dyn ValidationRule>>>,
    /// Global request-level validation rules
    request_rules: Vec<Arc<dyn ValidationRule>>,
}

impl std::fmt::Debug for Rules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rules")
            .field("field_rules_count", &self.field_rules.len())
            .field("request_rules_count", &self.request_rules.len())
            .field("validated_fields", &self.get_validated_fields())
            .finish()
    }
}

impl Rules {
    /// Create a new empty rules collection
    pub fn new() -> Self {
        Self {
            field_rules: HashMap::new(),
            request_rules: Vec::new(),
        }
    }

    /// Add a validation rule for a specific field
    pub fn field<R>(mut self, field: impl Into<String>, rule: R) -> Self
    where
        R: ValidationRule + 'static,
    {
        let field = field.into();
        self.field_rules
            .entry(field)
            .or_insert_with(Vec::new)
            .push(Arc::new(rule));
        self
    }

    /// Add multiple validation rules for a specific field
    pub fn field_rules<R>(mut self, field: impl Into<String>, rules: Vec<R>) -> Self
    where
        R: ValidationRule + 'static,
    {
        let field = field.into();
        let rule_arcs: Vec<Arc<dyn ValidationRule>> = rules
            .into_iter()
            .map(|r| Arc::new(r) as Arc<dyn ValidationRule>)
            .collect();
        
        self.field_rules
            .entry(field)
            .or_insert_with(Vec::new)
            .extend(rule_arcs);
        self
    }

    /// Add a request-level validation rule (cross-field validation)
    pub fn request<R>(mut self, rule: R) -> Self
    where
        R: ValidationRule + 'static,
    {
        self.request_rules.push(Arc::new(rule));
        self
    }

    /// Get rules for a specific field
    pub fn get_field_rules(&self, field: &str) -> Option<&Vec<Arc<dyn ValidationRule>>> {
        self.field_rules.get(field)
    }

    /// Get all request-level rules
    pub fn get_request_rules(&self) -> &Vec<Arc<dyn ValidationRule>> {
        &self.request_rules
    }

    /// Check if there are any rules defined
    pub fn is_empty(&self) -> bool {
        self.field_rules.is_empty() && self.request_rules.is_empty()
    }

    /// Get the number of field rules
    pub fn field_rule_count(&self) -> usize {
        self.field_rules.len()
    }

    /// Get the number of request rules
    pub fn request_rule_count(&self) -> usize {
        self.request_rules.len()
    }

    /// Get all field names that have validation rules
    pub fn get_validated_fields(&self) -> Vec<&String> {
        self.field_rules.keys().collect()
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidateField for Rules {
    async fn validate_field(&self, field: &str, value: &Value) -> ValidationResult<()> {
        if let Some(rules) = self.field_rules.get(field) {
            let mut errors = ValidationErrors::new();
            
            for rule in rules {
                if let Err(rule_errors) = rule.validate(value, field).await {
                    errors.merge(rule_errors);
                }
            }
            
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        } else {
            // No rules defined for this field - consider it valid
            Ok(())
        }
    }
}

#[async_trait]
impl ValidateRequest for Rules {
    async fn validate_request(&self, data: &HashMap<String, Value>) -> ValidationResult<()> {
        let mut errors = ValidationErrors::new();
        
        // Apply request-level validation rules
        for rule in &self.request_rules {
            // For request-level rules, we pass the entire data as a JSON object
            let data_value = Value::Object(
                data.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<serde_json::Map<String, Value>>()
            );
            
            if let Err(rule_errors) = rule.validate(&data_value, "request").await {
                errors.merge(rule_errors);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Configuration for Rules builder - contains the accumulated rules
#[derive(Clone)]
#[builder]
pub struct RulesBuilderConfig {
    #[builder(default)]
    pub field_rules: HashMap<String, Vec<Arc<dyn ValidationRule>>>,
    
    #[builder(default)]
    pub request_rules: Vec<Arc<dyn ValidationRule>>,
}

impl std::fmt::Debug for RulesBuilderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RulesBuilderConfig")
            .field("field_rules_count", &self.field_rules.len())
            .field("request_rules_count", &self.request_rules.len())
            .finish()
    }
}

impl RulesBuilderConfig {
    /// Build a Rules from the builder config
    pub fn build_rules(self) -> Rules {
        Rules {
            field_rules: self.field_rules,
            request_rules: self.request_rules,
        }
    }
}

// Add convenience methods to the generated builder
impl RulesBuilderConfigBuilder {
    /// Add a validation rule for a specific field
    pub fn field_rule<R>(self, field: impl Into<String>, rule: R) -> Self
    where
        R: ValidationRule + 'static,
    {
        let field = field.into();
        let mut field_rules = self.field_rules.clone().unwrap_or_default();
        field_rules
            .entry(field)
            .or_insert_with(Vec::new)
            .push(Arc::new(rule));
        self.field_rules(field_rules)
    }
    
    /// Add multiple validation rules for a specific field
    pub fn field_rules_vec<R>(self, field: impl Into<String>, rules: Vec<R>) -> Self
    where
        R: ValidationRule + 'static,
    {
        let field = field.into();
        let rule_arcs: Vec<Arc<dyn ValidationRule>> = rules
            .into_iter()
            .map(|r| Arc::new(r) as Arc<dyn ValidationRule>)
            .collect();
        
        let mut field_rules = self.field_rules.clone().unwrap_or_default();
        field_rules
            .entry(field)
            .or_insert_with(Vec::new)
            .extend(rule_arcs);
        self.field_rules(field_rules)
    }
    
    /// Add a request-level validation rule
    pub fn request_rule<R>(self, rule: R) -> Self
    where
        R: ValidationRule + 'static,
    {
        let mut request_rules = self.request_rules.clone().unwrap_or_default();
        request_rules.push(Arc::new(rule));
        self.request_rules(request_rules)
    }
    
    pub fn build_config(self) -> RulesBuilderConfig {
        self.build_with_defaults().unwrap()
    }
}

/// Builder for creating common validation rule combinations
pub struct RulesBuilder {
    builder_config: RulesBuilderConfigBuilder,
}

impl RulesBuilder {
    /// Create a new rules builder
    pub fn new() -> Self {
        Self {
            builder_config: RulesBuilderConfig::builder(),
        }
    }

    /// Build and return the rules
    pub fn build(self) -> Rules {
        self.builder_config.build_config().build_rules()
    }

    /// Add validation rules for a required string field
    pub fn required_string(
        mut self, 
        field: impl Into<String>, 
        min_length: Option<usize>, 
        max_length: Option<usize>
    ) -> Self {
        let field = field.into();
        
        // Add required validator
        self.builder_config = self.builder_config.field_rule(field.clone(), RequiredValidator::new());
        
        // Add length validator if constraints are specified
        if min_length.is_some() || max_length.is_some() {
            let mut length_validator = LengthValidator::new();
            if let Some(min) = min_length {
                length_validator = length_validator.min(min);
            }
            if let Some(max) = max_length {
                length_validator = length_validator.max(max);
            }
            self.builder_config = self.builder_config.field_rule(field, length_validator);
        }
        
        self
    }

    /// Add validation rules for a required email field
    pub fn required_email(mut self, field: impl Into<String>) -> Self {
        let field = field.into();
        
        self.builder_config = self.builder_config
            .field_rule(field.clone(), RequiredValidator::new())
            .field_rule(field, EmailValidator::new());
        
        self
    }

    /// Add validation rules for an optional email field
    pub fn optional_email(mut self, field: impl Into<String>) -> Self {
        let field = field.into();
        
        // Only add email validation - no required validation
        self.builder_config = self.builder_config.field_rule(field, EmailValidator::new());
        
        self
    }

    /// Add validation rules for a required numeric field
    pub fn required_number(
        mut self, 
        field: impl Into<String>, 
        min: Option<f64>, 
        max: Option<f64>
    ) -> Self {
        let field = field.into();
        
        self.builder_config = self.builder_config.field_rule(field.clone(), RequiredValidator::new());
        
        let mut numeric_validator = NumericValidator::new();
        if let Some(min_val) = min {
            numeric_validator = numeric_validator.min(min_val);
        }
        if let Some(max_val) = max {
            numeric_validator = numeric_validator.max(max_val);
        }
        
        self.builder_config = self.builder_config.field_rule(field, numeric_validator);
        
        self
    }

    /// Add validation rules for a required integer field
    pub fn required_integer(
        mut self, 
        field: impl Into<String>, 
        min: Option<f64>, 
        max: Option<f64>
    ) -> Self {
        let field = field.into();
        
        self.builder_config = self.builder_config.field_rule(field.clone(), RequiredValidator::new());
        
        let mut numeric_validator = NumericValidator::new().integer_only(true);
        if let Some(min_val) = min {
            numeric_validator = numeric_validator.min(min_val);
        }
        if let Some(max_val) = max {
            numeric_validator = numeric_validator.max(max_val);
        }
        
        self.builder_config = self.builder_config.field_rule(field, numeric_validator);
        
        self
    }

    /// Add validation rules for a field that must match a pattern
    pub fn pattern(mut self, field: impl Into<String>, pattern: &str) -> Self {
        let field = field.into();
        
        if let Ok(pattern_validator) = PatternValidator::new(pattern) {
            self.builder_config = self.builder_config.field_rule(field, pattern_validator);
        }
        
        self
    }

    /// Add validation rules for a field that must be one of the allowed values
    pub fn one_of(mut self, field: impl Into<String>, allowed_values: Vec<String>) -> Self {
        let field = field.into();
        
        let custom_validator = CustomValidator::one_of(
            format!("{}_one_of", field),
            allowed_values
        );
        
        self.builder_config = self.builder_config.field_rule(field, custom_validator);
        
        self
    }

    /// Add a custom validation rule
    pub fn custom<R>(mut self, field: impl Into<String>, rule: R) -> Self
    where
        R: ValidationRule + 'static,
    {
        self.builder_config = self.builder_config.field_rule(field, rule);
        self
    }

    /// Add a request-level validation rule
    pub fn request_rule<R>(mut self, rule: R) -> Self
    where
        R: ValidationRule + 'static,
    {
        self.builder_config = self.builder_config.request_rule(rule);
        self
    }
}

impl Default for RulesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ValidationError;
    use crate::traits::Validate;

    #[tokio::test]
    async fn test_rules_field_validation() {
        let rules = Rules::new()
            .field("name", RequiredValidator::new())
            .field("name", LengthValidator::new().min(2).max(50))
            .field("email", EmailValidator::new());

        // Valid name
        let result = rules.validate_field("name", &Value::String("John".to_string())).await;
        assert!(result.is_ok());

        // Invalid name (too short)
        let result = rules.validate_field("name", &Value::String("J".to_string())).await;
        assert!(result.is_err());

        // Valid email
        let result = rules.validate_field("email", &Value::String("john@example.com".to_string())).await;
        assert!(result.is_ok());

        // Invalid email
        let result = rules.validate_field("email", &Value::String("not-an-email".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_request_validation() {
        let password_confirmation_rule = CustomValidator::new("password_confirmation", |value, _field| {
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

        let rules = Rules::new().request(password_confirmation_rule);

        // Valid matching passwords
        let mut data = HashMap::new();
        data.insert("password".to_string(), Value::String("secret123".to_string()));
        data.insert("password_confirmation".to_string(), Value::String("secret123".to_string()));

        let result = rules.validate_request(&data).await;
        assert!(result.is_ok());

        // Invalid non-matching passwords
        let mut data = HashMap::new();
        data.insert("password".to_string(), Value::String("secret123".to_string()));
        data.insert("password_confirmation".to_string(), Value::String("different".to_string()));

        let result = rules.validate_request(&data).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_required_string() {
        let rules = RulesBuilder::new()
            .required_string("name", Some(2), Some(50))
            .build();

        // Valid string
        let result = rules.validate_field("name", &Value::String("John".to_string())).await;
        assert!(result.is_ok());

        // Empty string (should fail required)
        let result = rules.validate_field("name", &Value::String("".to_string())).await;
        assert!(result.is_err());

        // Too short string
        let result = rules.validate_field("name", &Value::String("J".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_required_email() {
        let rules = RulesBuilder::new()
            .required_email("email")
            .build();

        // Valid email
        let result = rules.validate_field("email", &Value::String("test@example.com".to_string())).await;
        assert!(result.is_ok());

        // Empty email (should fail required)
        let result = rules.validate_field("email", &Value::String("".to_string())).await;
        assert!(result.is_err());

        // Invalid email format
        let result = rules.validate_field("email", &Value::String("not-an-email".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_optional_email() {
        let rules = RulesBuilder::new()
            .optional_email("email")
            .build();

        // Valid email
        let result = rules.validate_field("email", &Value::String("test@example.com".to_string())).await;
        assert!(result.is_ok());

        // Null email (should pass - it's optional)
        let result = rules.validate_field("email", &Value::Null).await;
        assert!(result.is_ok());

        // Invalid email format (should still fail)
        let result = rules.validate_field("email", &Value::String("not-an-email".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_required_number() {
        let rules = RulesBuilder::new()
            .required_number("age", Some(0.0), Some(120.0))
            .build();

        // Valid number
        let result = rules.validate_field("age", &Value::Number(serde_json::Number::from(25))).await;
        assert!(result.is_ok());

        // Null (should fail required)
        let result = rules.validate_field("age", &Value::Null).await;
        assert!(result.is_err());

        // Out of range
        let result = rules.validate_field("age", &Value::Number(serde_json::Number::from(150))).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_required_integer() {
        let rules = RulesBuilder::new()
            .required_integer("count", Some(1.0), Some(100.0))
            .build();

        // Valid integer
        let result = rules.validate_field("count", &Value::Number(serde_json::Number::from(10))).await;
        assert!(result.is_ok());

        // Decimal (should fail integer check)
        let result = rules.validate_field("count", &Value::Number(serde_json::Number::from_f64(10.5).unwrap())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_pattern() {
        let rules = RulesBuilder::new()
            .pattern("code", r"^[A-Z]{3}$")
            .build();

        // Valid pattern
        let result = rules.validate_field("code", &Value::String("ABC".to_string())).await;
        assert!(result.is_ok());

        // Invalid pattern
        let result = rules.validate_field("code", &Value::String("abc".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_builder_one_of() {
        let rules = RulesBuilder::new()
            .one_of("status", vec!["active".to_string(), "inactive".to_string()])
            .build();

        // Valid value
        let result = rules.validate_field("status", &Value::String("active".to_string())).await;
        assert!(result.is_ok());

        // Invalid value
        let result = rules.validate_field("status", &Value::String("unknown".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rules_combined_field_and_request_validation() {
        let password_match_rule = CustomValidator::new("password_match", |value, _field| {
            if let Some(obj) = value.as_object() {
                let password = obj.get("password").and_then(|v| v.as_str());
                let confirmation = obj.get("password_confirmation").and_then(|v| v.as_str());
                
                if let (Some(pwd), Some(conf)) = (password, confirmation) {
                    if pwd == conf {
                        Ok(())
                    } else {
                        Err(ValidationError::new("password_confirmation", "Passwords do not match").into())
                    }
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        });

        let rules = RulesBuilder::new()
            .required_string("password", Some(8), None)
            .required_string("password_confirmation", Some(8), None)
            .request_rule(password_match_rule)
            .build();

        let mut data = HashMap::new();
        data.insert("password".to_string(), Value::String("password123".to_string()));
        data.insert("password_confirmation".to_string(), Value::String("password123".to_string()));

        // Should validate both fields individually and the request as a whole
        let result = rules.validate(&data).await;
        assert!(result.is_ok());

        // Test with non-matching passwords
        let mut data = HashMap::new();
        data.insert("password".to_string(), Value::String("password123".to_string()));
        data.insert("password_confirmation".to_string(), Value::String("different".to_string()));

        let result = rules.validate(&data).await;
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert!(errors.has_field_errors("password_confirmation"));
    }
}