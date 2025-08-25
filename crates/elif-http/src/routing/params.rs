//! Route parameter extraction and validation

use axum::extract::Path;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during parameter extraction
#[derive(Error, Debug)]
pub enum ParamError {
    #[error("Missing parameter: {0}")]
    Missing(String),
    #[error("Invalid parameter format: {0}")]
    InvalidFormat(String),
    #[error("Parameter validation failed: {0}")]
    ValidationFailed(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

/// A single route parameter with validation
#[derive(Debug, Clone)]
pub struct RouteParam {
    pub name: String,
    pub value: String,
    pub param_type: ParamType,
}

/// Supported parameter types
#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    String,
    Integer,
    Uuid,
    Custom(String), // For custom validation patterns
}

impl RouteParam {
    pub fn new(name: String, value: String, param_type: ParamType) -> Self {
        Self {
            name,
            value,
            param_type,
        }
    }

    /// Validate the parameter value against its type
    pub fn validate(&self) -> Result<(), ParamError> {
        match &self.param_type {
            ParamType::String => Ok(()), // Strings are always valid
            ParamType::Integer => self.value.parse::<i64>().map(|_| ()).map_err(|_| {
                ParamError::ValidationFailed(format!(
                    "Parameter '{}' must be an integer",
                    self.name
                ))
            }),
            ParamType::Uuid => uuid::Uuid::parse_str(&self.value).map(|_| ()).map_err(|_| {
                ParamError::ValidationFailed(format!(
                    "Parameter '{}' must be a valid UUID",
                    self.name
                ))
            }),
            ParamType::Custom(_pattern) => {
                // TODO: Implement regex validation for custom patterns
                Ok(())
            }
        }
    }

    /// Get the typed value as T
    pub fn as_typed<T>(&self) -> Result<T, ParamError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        self.validate()?;
        self.value.parse::<T>().map_err(|e| {
            ParamError::InvalidFormat(format!(
                "Cannot convert '{}' to target type: {}",
                self.value, e
            ))
        })
    }
}

/// Container for extracted path parameters
#[derive(Debug, Default)]
pub struct PathParams {
    params: HashMap<String, RouteParam>,
}

impl PathParams {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    pub fn add_param(&mut self, param: RouteParam) {
        self.params.insert(param.name.clone(), param);
    }

    /// Get a parameter by name
    pub fn get(&self, name: &str) -> Option<&RouteParam> {
        self.params.get(name)
    }

    /// Get parameter value as string
    pub fn get_str(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|p| p.value.as_str())
    }

    /// Get parameter value as typed value
    pub fn get_typed<T>(&self, name: &str) -> Result<T, ParamError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        self.get(name)
            .ok_or_else(|| ParamError::Missing(name.to_string()))?
            .as_typed()
    }

    /// Get all parameters
    pub fn all(&self) -> &HashMap<String, RouteParam> {
        &self.params
    }

    /// Validate all parameters
    pub fn validate_all(&self) -> Result<(), ParamError> {
        for param in self.params.values() {
            param.validate()?;
        }
        Ok(())
    }
}

/// Extract path parameters from axum Path
impl<T> From<Path<T>> for PathParams
where
    T: DeserializeOwned + Send + 'static,
{
    fn from(_path: Path<T>) -> Self {
        // This is a placeholder implementation
        // In practice, we'd need to work with the actual extracted values
        PathParams::new()
    }
}

/// Builder for creating typed parameter extractors
#[derive(Debug)]
pub struct ParamExtractor {
    param_specs: HashMap<String, ParamType>,
}

impl ParamExtractor {
    pub fn new() -> Self {
        Self {
            param_specs: HashMap::new(),
        }
    }

    /// Specify a parameter type
    pub fn param(mut self, name: &str, param_type: ParamType) -> Self {
        self.param_specs.insert(name.to_string(), param_type);
        self
    }

    /// Extract and validate parameters from a path
    pub fn extract_from_path(
        &self,
        path: &str,
        route_pattern: &str,
    ) -> Result<PathParams, ParamError> {
        let mut params = PathParams::new();

        // Parse route pattern to find parameter names
        let pattern_parts: Vec<&str> = route_pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return Err(ParamError::InvalidFormat(
                "Path structure mismatch".to_string(),
            ));
        }

        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
                let param_name = &pattern_part[1..pattern_part.len() - 1];
                let param_type = self
                    .param_specs
                    .get(param_name)
                    .cloned()
                    .unwrap_or(ParamType::String);

                let param =
                    RouteParam::new(param_name.to_string(), path_part.to_string(), param_type);

                param.validate()?;
                params.add_param(param);
            }
        }

        Ok(params)
    }
}

impl Default for ParamExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_param_validation() {
        let param = RouteParam::new("id".to_string(), "123".to_string(), ParamType::Integer);
        assert!(param.validate().is_ok());

        let invalid_param =
            RouteParam::new("id".to_string(), "abc".to_string(), ParamType::Integer);
        assert!(invalid_param.validate().is_err());
    }

    #[test]
    fn test_param_extractor() {
        let extractor = ParamExtractor::new()
            .param("id", ParamType::Integer)
            .param("slug", ParamType::String);

        let params = extractor
            .extract_from_path("/users/123/posts/hello", "/users/{id}/posts/{slug}")
            .unwrap();

        assert_eq!(params.get_str("id"), Some("123"));
        assert_eq!(params.get_str("slug"), Some("hello"));
        assert_eq!(params.get_typed::<i64>("id").unwrap(), 123);
    }
}
