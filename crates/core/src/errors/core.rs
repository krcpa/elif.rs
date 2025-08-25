use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Core error type for the elif framework
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Service not found: {service_type}")]
    ServiceNotFound { service_type: String },

    #[error("Invalid service scope: {scope}")]
    InvalidServiceScope { scope: String },

    #[error("Lock error on resource: {resource}")]
    LockError { resource: String },

    #[error("Lifecycle error in component '{component}' during '{operation}': {source}")]
    LifecycleError {
        component: String,
        operation: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("System error: {message}")]
    SystemError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Module error: {message}")]
    Module { message: String },

    #[error("Provider error: {message}")]
    Provider { message: String },

    #[error("Codegen error: {message}")]
    Codegen { message: String },

    #[error("Template error: {message}")]
    Template { message: String },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Circular dependency detected: {path} (cycle at: {cycle_service})")]
    CircularDependency { path: String, cycle_service: String },

    #[error("Invalid service descriptor: {message}")]
    InvalidServiceDescriptor { message: String },

    #[error("Dependency resolution failed for '{service_type}': {message}")]
    DependencyResolutionFailed {
        service_type: String,
        message: String,
    },

    #[error("API error: {code} - {message}")]
    Api {
        code: String,
        message: String,
        hint: Option<String>,
    },

    #[error("Service initialization failed for '{service_type}': {source}")]
    ServiceInitializationFailed {
        service_type: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl CoreError {
    /// Create a new validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a new configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a new service not found error
    pub fn service_not_found(service_type: impl Into<String>) -> Self {
        Self::ServiceNotFound {
            service_type: service_type.into(),
        }
    }

    /// Create a new system error
    pub fn system_error(message: impl Into<String>) -> Self {
        Self::SystemError {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new system error with source
    pub fn system_error_with_source(
        message: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::SystemError {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a new API error
    pub fn api_error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Api {
            code: code.into(),
            message: message.into(),
            hint: None,
        }
    }

    /// Add a hint to an API error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        if let Self::Api {
            hint: ref mut h, ..
        } = self
        {
            *h = Some(hint.into());
        }
        self
    }

    /// Check if the error is a validation error
    pub fn is_validation(&self) -> bool {
        matches!(self, Self::Validation { .. })
    }

    /// Check if the error is a configuration error
    pub fn is_configuration(&self) -> bool {
        matches!(self, Self::Configuration { .. })
    }

    /// Check if the error is a service error
    pub fn is_service(&self) -> bool {
        matches!(self, Self::ServiceNotFound { .. })
    }
}

/// Error definition for the error catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDefinition {
    pub code: String,
    pub http: u16,
    pub message: String,
    pub hint: String,
}

/// API error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiError,
}

/// API error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint: None,
        }
    }

    /// Add a hint to the API error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Api {
                code,
                message,
                hint,
            } => Self {
                code,
                message,
                hint,
            },
            CoreError::Validation { message } => Self::new("VALIDATION_ERROR", message),
            CoreError::Configuration { message } => Self::new("CONFIG_ERROR", message),
            CoreError::ServiceNotFound { service_type } => Self::new(
                "SERVICE_NOT_FOUND",
                format!("Service not found: {}", service_type),
            ),
            _ => Self::new("INTERNAL_ERROR", error.to_string()),
        }
    }
}

impl From<ApiError> for ApiErrorResponse {
    fn from(error: ApiError) -> Self {
        Self { error }
    }
}

/// Type alias for error catalog
pub type ErrorCatalog = HashMap<String, ErrorDefinition>;

/// Legacy alias for backward compatibility
pub type ElifError = CoreError;
