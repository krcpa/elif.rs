use thiserror::Error;

/// Result type for OpenAPI operations
pub type OpenApiResult<T> = Result<T, OpenApiError>;

/// Errors that can occur during OpenAPI generation
#[derive(Debug, Error)]
pub enum OpenApiError {
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML serialization/deserialization error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// I/O error (file operations, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Schema generation error
    #[error("Schema generation error: {0}")]
    Schema(String),

    /// Route discovery error
    #[error("Route discovery error: {0}")]
    RouteDiscovery(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Export format error
    #[error("Export format error: {0}")]
    Export(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Generic error with context
    #[error("OpenAPI error: {0}")]
    Generic(String),
}

impl OpenApiError {
    /// Create a new schema generation error
    pub fn schema_error<T: ToString>(msg: T) -> Self {
        Self::Schema(msg.to_string())
    }

    /// Create a new route discovery error
    pub fn route_discovery_error<T: ToString>(msg: T) -> Self {
        Self::RouteDiscovery(msg.to_string())
    }

    /// Create a new configuration error
    pub fn config_error<T: ToString>(msg: T) -> Self {
        Self::Config(msg.to_string())
    }

    /// Create a new export format error
    pub fn export_error<T: ToString>(msg: T) -> Self {
        Self::Export(msg.to_string())
    }

    /// Create a new validation error
    pub fn validation_error<T: ToString>(msg: T) -> Self {
        Self::Validation(msg.to_string())
    }

    /// Create a generic error
    pub fn generic<T: ToString>(msg: T) -> Self {
        Self::Generic(msg.to_string())
    }
}
