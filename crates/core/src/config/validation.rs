use thiserror::Error;

/// Configuration error type
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required field: {field}. {hint}")]
    MissingRequired { field: String, hint: String },

    #[error("Invalid value for field '{field}': '{value}'. Expected: {expected}")]
    InvalidValue {
        field: String,
        value: String,
        expected: String,
    },

    #[error("Configuration validation failed: {message}")]
    ValidationFailed { message: String },

    #[error("Environment variable error: {message}")]
    EnvironmentError { message: String },

    #[error("File system error: {message}")]
    FileSystemError { message: String },

    #[error("Parsing error: {message}")]
    ParsingError { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ConfigError {
    /// Create a missing required field error
    pub fn missing_required(field: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::MissingRequired {
            field: field.into(),
            hint: hint.into(),
        }
    }

    /// Create an invalid value error
    pub fn invalid_value(
        field: impl Into<String>,
        value: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self::InvalidValue {
            field: field.into(),
            value: value.into(),
            expected: expected.into(),
        }
    }

    /// Create a validation failed error
    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            message: message.into(),
        }
    }

    /// Create an environment error
    pub fn environment_error(message: impl Into<String>) -> Self {
        Self::EnvironmentError {
            message: message.into(),
        }
    }
}

/// Trait for validating configuration values
pub trait ConfigValidator<T> {
    /// Validate a configuration value
    fn validate(&self, value: &T) -> Result<(), ConfigError>;
}

/// Port number validator
pub struct PortValidator {
    pub min: u16,
    pub max: u16,
}

impl Default for PortValidator {
    fn default() -> Self {
        Self { min: 1, max: 65535 }
    }
}

impl ConfigValidator<u16> for PortValidator {
    fn validate(&self, value: &u16) -> Result<(), ConfigError> {
        if *value < self.min || *value > self.max {
            return Err(ConfigError::invalid_value(
                "port",
                value.to_string(),
                format!("port between {} and {}", self.min, self.max),
            ));
        }
        Ok(())
    }
}

/// URL validator
pub struct UrlValidator {
    pub schemes: Vec<String>,
    pub require_host: bool,
}

impl Default for UrlValidator {
    fn default() -> Self {
        Self {
            schemes: vec!["http".to_string(), "https".to_string()],
            require_host: true,
        }
    }
}

impl ConfigValidator<String> for UrlValidator {
    fn validate(&self, value: &String) -> Result<(), ConfigError> {
        // Basic URL validation (in a real implementation, use a proper URL parser)
        if value.is_empty() {
            return Err(ConfigError::invalid_value(
                "url",
                value.clone(),
                "non-empty URL",
            ));
        }

        // Check scheme
        let has_valid_scheme = self
            .schemes
            .iter()
            .any(|scheme| value.starts_with(&format!("{}://", scheme)));

        if !has_valid_scheme {
            return Err(ConfigError::invalid_value(
                "url",
                value.clone(),
                format!("URL with scheme: {}", self.schemes.join(", ")),
            ));
        }

        // Check for host if required
        if self.require_host && !value.contains("://") {
            return Err(ConfigError::invalid_value(
                "url",
                value.clone(),
                "URL with host",
            ));
        }

        Ok(())
    }
}

/// Required field validator
pub struct RequiredValidator;

impl<T> ConfigValidator<Option<T>> for RequiredValidator {
    fn validate(&self, value: &Option<T>) -> Result<(), ConfigError> {
        if value.is_none() {
            return Err(ConfigError::missing_required(
                "field",
                "This field is required",
            ));
        }
        Ok(())
    }
}

/// String length validator
pub struct LengthValidator {
    pub min_length: usize,
    pub max_length: Option<usize>,
}

impl LengthValidator {
    pub fn min(min_length: usize) -> Self {
        Self {
            min_length,
            max_length: None,
        }
    }

    pub fn range(min_length: usize, max_length: usize) -> Self {
        Self {
            min_length,
            max_length: Some(max_length),
        }
    }
}

impl ConfigValidator<String> for LengthValidator {
    fn validate(&self, value: &String) -> Result<(), ConfigError> {
        if value.len() < self.min_length {
            return Err(ConfigError::invalid_value(
                "string",
                value.clone(),
                format!("string with at least {} characters", self.min_length),
            ));
        }

        if let Some(max_length) = self.max_length {
            if value.len() > max_length {
                return Err(ConfigError::invalid_value(
                    "string",
                    value.clone(),
                    format!("string with at most {} characters", max_length),
                ));
            }
        }

        Ok(())
    }
}

/// Composite validator that runs multiple validators
pub struct CompositeValidator<T> {
    validators: Vec<Box<dyn ConfigValidator<T>>>,
}

impl<T> CompositeValidator<T> {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    pub fn add_validator(mut self, validator: Box<dyn ConfigValidator<T>>) -> Self {
        self.validators.push(validator);
        self
    }
}

impl<T> Default for CompositeValidator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ConfigValidator<T> for CompositeValidator<T> {
    fn validate(&self, value: &T) -> Result<(), ConfigError> {
        for validator in &self.validators {
            validator.validate(value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_validator() {
        let validator = PortValidator::default();

        assert!(validator.validate(&80).is_ok());
        assert!(validator.validate(&443).is_ok());
        assert!(validator.validate(&65535).is_ok());
        assert!(validator.validate(&0).is_err());
    }

    #[test]
    fn test_url_validator() {
        let validator = UrlValidator::default();

        assert!(validator
            .validate(&"https://example.com".to_string())
            .is_ok());
        assert!(validator
            .validate(&"http://localhost:3000".to_string())
            .is_ok());
        assert!(validator
            .validate(&"ftp://example.com".to_string())
            .is_err());
        assert!(validator.validate(&"not-a-url".to_string()).is_err());
    }

    #[test]
    fn test_length_validator() {
        let validator = LengthValidator::range(3, 10);

        assert!(validator.validate(&"hello".to_string()).is_ok());
        assert!(validator.validate(&"hi".to_string()).is_err()); // Too short
        assert!(validator.validate(&"this is too long".to_string()).is_err()); // Too long
    }
}
