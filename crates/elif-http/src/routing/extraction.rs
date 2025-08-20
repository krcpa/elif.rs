//! Parameter extraction engine for elif.rs
//!
//! This module provides enhanced parameter extraction with type conversion,
//! validation, and error handling for route parameters.

use super::pattern::{ParamConstraint, RoutePattern};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during parameter extraction and conversion
#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error("Missing parameter: {0}")]
    Missing(String),
    #[error("Parameter validation failed for '{param}': {reason}")]
    ValidationFailed { param: String, reason: String },
    #[error("Type conversion failed for parameter '{param}': {error}")]
    ConversionFailed { param: String, error: String },
    #[error("Constraint violation for parameter '{param}': expected {constraint}, got '{value}'")]
    ConstraintViolation { param: String, constraint: String, value: String },
}

/// Extracted and validated route parameters
#[derive(Debug, Clone)]
pub struct ExtractedParams {
    raw_params: HashMap<String, String>,
    pattern: RoutePattern,
}

impl ExtractedParams {
    /// Create new extracted parameters
    pub fn new(raw_params: HashMap<String, String>, pattern: RoutePattern) -> Self {
        Self { raw_params, pattern }
    }

    /// Get a parameter as a raw string
    pub fn get_str(&self, name: &str) -> Option<&str> {
        self.raw_params.get(name).map(|s| s.as_str())
    }

    /// Get a parameter converted to a specific type
    pub fn get<T>(&self, name: &str) -> Result<T, ExtractionError>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        let value = self.raw_params
            .get(name)
            .ok_or_else(|| ExtractionError::Missing(name.to_string()))?;

        // Validate against pattern constraints if available
        if let Some(segment) = self.find_parameter_segment(name) {
            if let super::pattern::PathSegment::Parameter { constraint, .. } = segment {
                if !constraint.validate(value) {
                    return Err(ExtractionError::ConstraintViolation {
                        param: name.to_string(),
                        constraint: format!("{:?}", constraint),
                        value: value.clone(),
                    });
                }
            }
        }

        // Convert to target type
        value.parse::<T>().map_err(|e| ExtractionError::ConversionFailed {
            param: name.to_string(),
            error: e.to_string(),
        })
    }

    /// Get a parameter as an integer
    pub fn get_int(&self, name: &str) -> Result<i64, ExtractionError> {
        self.get::<i64>(name)
    }

    /// Get a parameter as a UUID
    pub fn get_uuid(&self, name: &str) -> Result<Uuid, ExtractionError> {
        self.get::<Uuid>(name)
    }

    /// Get a parameter with a default value if missing
    pub fn get_or<T>(&self, name: &str, default: T) -> Result<T, ExtractionError>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        match self.get(name) {
            Ok(value) => Ok(value),
            Err(ExtractionError::Missing(_)) => Ok(default),
            Err(e) => Err(e),
        }
    }

    /// Get a parameter as an Option (None if missing)
    pub fn get_optional<T>(&self, name: &str) -> Result<Option<T>, ExtractionError>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        match self.get(name) {
            Ok(value) => Ok(Some(value)),
            Err(ExtractionError::Missing(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all parameter names
    pub fn param_names(&self) -> Vec<&String> {
        self.raw_params.keys().collect()
    }

    /// Get all raw parameter values
    pub fn raw_params(&self) -> &HashMap<String, String> {
        &self.raw_params
    }

    /// Validate all parameters against their constraints
    pub fn validate_all(&self) -> Result<(), ExtractionError> {
        for (param_name, param_value) in &self.raw_params {
            if let Some(segment) = self.find_parameter_segment(param_name) {
                if let super::pattern::PathSegment::Parameter { constraint, .. } = segment {
                    if !constraint.validate(param_value) {
                        return Err(ExtractionError::ConstraintViolation {
                            param: param_name.clone(),
                            constraint: format!("{:?}", constraint),
                            value: param_value.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Find the parameter segment in the route pattern
    fn find_parameter_segment(&self, name: &str) -> Option<&super::pattern::PathSegment> {
        self.pattern.segments.iter().find(|segment| match segment {
            super::pattern::PathSegment::Parameter { name: seg_name, .. } => seg_name == name,
            super::pattern::PathSegment::CatchAll { name: seg_name } => seg_name == name,
            _ => false,
        })
    }
}

/// Parameter extractor for route patterns
#[derive(Debug)]
pub struct ParameterExtractor {
    pattern: RoutePattern,
}

impl ParameterExtractor {
    /// Create a new parameter extractor for a route pattern
    pub fn new(pattern: RoutePattern) -> Self {
        Self { pattern }
    }

    /// Extract parameters from a path that matches this pattern
    pub fn extract(&self, path: &str) -> Result<ExtractedParams, ExtractionError> {
        // First verify the path matches the pattern
        if !self.pattern.matches(path) {
            return Err(ExtractionError::ValidationFailed {
                param: "path".to_string(),
                reason: "Path does not match route pattern".to_string(),
            });
        }

        // Extract raw parameters
        let raw_params = self.pattern.extract_params(path);
        
        // Create extracted parameters with validation
        let extracted = ExtractedParams::new(raw_params, self.pattern.clone());
        
        // Validate all parameters upfront
        extracted.validate_all()?;
        
        Ok(extracted)
    }

    /// Get the route pattern
    pub fn pattern(&self) -> &RoutePattern {
        &self.pattern
    }

    /// Get expected parameter names
    pub fn param_names(&self) -> &[String] {
        &self.pattern.param_names
    }
}

/// Builder for creating typed parameter extractors with validation
#[derive(Debug)]
pub struct TypedExtractorBuilder {
    pattern: RoutePattern,
    custom_constraints: HashMap<String, ParamConstraint>,
}

impl TypedExtractorBuilder {
    /// Create a new builder for the given route pattern
    pub fn new(pattern: RoutePattern) -> Self {
        Self {
            pattern,
            custom_constraints: HashMap::new(),
        }
    }

    /// Add a custom constraint for a parameter
    pub fn constraint(mut self, param_name: &str, constraint: ParamConstraint) -> Self {
        self.custom_constraints.insert(param_name.to_string(), constraint);
        self
    }

    /// Add an integer constraint
    pub fn int_param(self, param_name: &str) -> Self {
        self.constraint(param_name, ParamConstraint::Int)
    }

    /// Add a UUID constraint
    pub fn uuid_param(self, param_name: &str) -> Self {
        self.constraint(param_name, ParamConstraint::Uuid)
    }

    /// Add an alphabetic constraint
    pub fn alpha_param(self, param_name: &str) -> Self {
        self.constraint(param_name, ParamConstraint::Alpha)
    }

    /// Add a slug constraint
    pub fn slug_param(self, param_name: &str) -> Self {
        self.constraint(param_name, ParamConstraint::Slug)
    }

    /// Build the parameter extractor
    pub fn build(mut self) -> ParameterExtractor {
        // Apply custom constraints to the pattern
        for segment in &mut self.pattern.segments {
            if let super::pattern::PathSegment::Parameter { name, constraint } = segment {
                if let Some(custom_constraint) = self.custom_constraints.remove(name) {
                    *constraint = custom_constraint;
                }
            }
        }

        ParameterExtractor::new(self.pattern)
    }
}

/// Convenience macros for parameter extraction
#[macro_export]
macro_rules! extract_params {
    ($extracted:expr, $($name:ident: $type:ty),+ $(,)?) => {
        {
            $(
                let $name: $type = $extracted.get(stringify!($name))?;
            )+
        }
    };
}

#[macro_export]
macro_rules! extract_optional_params {
    ($extracted:expr, $($name:ident: $type:ty),+ $(,)?) => {
        {
            $(
                let $name: Option<$type> = $extracted.get_optional(stringify!($name))?;
            )+
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pattern::RoutePattern;

    #[test]
    fn test_basic_parameter_extraction() {
        let pattern = RoutePattern::parse("/users/{id}/posts/{slug}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let extracted = extractor.extract("/users/123/posts/hello-world").unwrap();
        
        assert_eq!(extracted.get_str("id"), Some("123"));
        assert_eq!(extracted.get_str("slug"), Some("hello-world"));
    }

    #[test]
    fn test_typed_parameter_extraction() {
        let pattern = RoutePattern::parse("/users/{id:int}/posts/{slug}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let extracted = extractor.extract("/users/123/posts/hello-world").unwrap();
        
        // Should extract as integer
        assert_eq!(extracted.get_int("id").unwrap(), 123);
        
        // Should extract as string
        assert_eq!(extracted.get::<String>("slug").unwrap(), "hello-world");
    }

    #[test]
    fn test_uuid_parameter_extraction() {
        let pattern = RoutePattern::parse("/users/{id:uuid}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let extracted = extractor.extract(&format!("/users/{}", uuid_str)).unwrap();
        
        let uuid = extracted.get_uuid("id").unwrap();
        assert_eq!(uuid.to_string(), uuid_str);
    }

    #[test]
    fn test_constraint_violations() {
        let pattern = RoutePattern::parse("/users/{id:int}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        // Should fail with non-integer value
        let result = extractor.extract("/users/abc");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExtractionError::ValidationFailed { .. }));
    }

    #[test]
    fn test_optional_parameters() {
        let pattern = RoutePattern::parse("/users/{id}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let extracted = extractor.extract("/users/123").unwrap();
        
        // Existing parameter
        let id: Option<i64> = extracted.get_optional("id").unwrap();
        assert_eq!(id, Some(123));
        
        // Missing parameter
        let missing: Option<String> = extracted.get_optional("missing").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_parameter_with_defaults() {
        let pattern = RoutePattern::parse("/users/{id}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let extracted = extractor.extract("/users/123").unwrap();
        
        // Existing parameter
        let id = extracted.get_or("id", 0i64).unwrap();
        assert_eq!(id, 123);
        
        // Missing parameter with default
        let page = extracted.get_or("page", 1i64).unwrap();
        assert_eq!(page, 1);
    }

    #[test]
    fn test_catch_all_parameter() {
        let pattern = RoutePattern::parse("/files/*path").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let extracted = extractor.extract("/files/docs/images/logo.png").unwrap();
        
        let path: String = extracted.get("path").unwrap();
        assert_eq!(path, "docs/images/logo.png");
    }

    #[test]
    fn test_typed_extractor_builder() {
        let pattern = RoutePattern::parse("/api/{version}/users/{id}").unwrap();
        let extractor = TypedExtractorBuilder::new(pattern)
            .slug_param("version")
            .int_param("id")
            .build();
        
        let extracted = extractor.extract("/api/v1/users/123").unwrap();
        
        assert_eq!(extracted.get::<String>("version").unwrap(), "v1");
        assert_eq!(extracted.get_int("id").unwrap(), 123);
    }

    #[test]
    fn test_custom_regex_constraint() {
        use regex::Regex;
        
        let pattern = RoutePattern::parse("/posts/{slug}").unwrap();
        let regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
        
        let extractor = TypedExtractorBuilder::new(pattern)
            .constraint("slug", ParamConstraint::Custom(regex))
            .build();
        
        // Should match valid slug
        let result = extractor.extract("/posts/hello-world-123");
        assert!(result.is_ok());
        
        // Should fail with invalid characters
        let result = extractor.extract("/posts/Hello_World!");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_constraints() {
        // Test all built-in constraint types
        assert!(ParamConstraint::Int.validate("123"));
        assert!(!ParamConstraint::Int.validate("abc"));
        
        assert!(ParamConstraint::Alpha.validate("hello"));
        assert!(!ParamConstraint::Alpha.validate("hello123"));
        
        assert!(ParamConstraint::Slug.validate("hello-world_123"));
        assert!(!ParamConstraint::Slug.validate("hello world!"));
        
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        assert!(ParamConstraint::Uuid.validate(uuid_str));
        assert!(!ParamConstraint::Uuid.validate("not-a-uuid"));
        
        assert!(ParamConstraint::None.validate("anything"));
        assert!(!ParamConstraint::None.validate("")); // Empty not allowed
    }

    #[test]
    fn test_error_types() {
        let pattern = RoutePattern::parse("/users/{id:int}").unwrap();
        let extractor = ParameterExtractor::new(pattern);
        
        let extracted = extractor.extract("/users/123").unwrap();
        
        // Missing parameter error
        let result: Result<i64, _> = extracted.get("missing");
        assert!(matches!(result.unwrap_err(), ExtractionError::Missing(_)));
        
        // Type conversion error (try to get string as different type)
        // First we need an actual string parameter
        let pattern2 = RoutePattern::parse("/users/{name}").unwrap();
        let extractor2 = ParameterExtractor::new(pattern2);
        let extracted2 = extractor2.extract("/users/john").unwrap();
        
        let result: Result<i64, _> = extracted2.get("name");
        assert!(matches!(result.unwrap_err(), ExtractionError::ConversionFailed { .. }));
    }
}