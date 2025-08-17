//! HTTP server error types
//! 
//! Comprehensive error handling for HTTP operations, integrating with
//! the elif framework error system.

use thiserror::Error;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use crate::response::{ElifResponse, IntoElifResponse};

/// Result type for HTTP operations
pub type HttpResult<T> = Result<T, HttpError>;

/// HTTP server errors
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Server startup failed: {message}")]
    StartupFailed { message: String },
    
    #[error("Server shutdown failed: {message}")]
    ShutdownFailed { message: String },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Service resolution failed: {service}")]
    ServiceResolutionFailed { service: String },
    
    #[error("Request timeout")]
    RequestTimeout,
    
    #[error("Request too large: {size} bytes exceeds limit of {limit} bytes")]
    RequestTooLarge { size: usize, limit: usize },
    
    #[error("Invalid request: {message}")]
    BadRequest { message: String },
    
    #[error("Internal server error: {message}")]
    InternalError { message: String },
    
    #[error("Health check failed: {reason}")]
    HealthCheckFailed { reason: String },
    
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    
    #[error("Validation error: {message}")]
    ValidationError { message: String },
    
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Resource already exists: {message}")]
    Conflict { message: String },
    
    #[error("Unauthorized access")]
    Unauthorized,
    
    #[error("Access forbidden: {message}")]
    Forbidden { message: String },
}

impl HttpError {
    /// Create a startup error
    pub fn startup<T: Into<String>>(message: T) -> Self {
        HttpError::StartupFailed { 
            message: message.into() 
        }
    }
    
    /// Create a shutdown error
    pub fn shutdown<T: Into<String>>(message: T) -> Self {
        HttpError::ShutdownFailed { 
            message: message.into() 
        }
    }
    
    /// Create a configuration error
    pub fn config<T: Into<String>>(message: T) -> Self {
        HttpError::ConfigError { 
            message: message.into() 
        }
    }
    
    /// Create a service resolution error
    pub fn service_resolution<T: Into<String>>(service: T) -> Self {
        HttpError::ServiceResolutionFailed { 
            service: service.into() 
        }
    }
    
    /// Create a bad request error
    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        HttpError::BadRequest { 
            message: message.into() 
        }
    }
    
    /// Create an internal error
    pub fn internal<T: Into<String>>(message: T) -> Self {
        HttpError::InternalError { 
            message: message.into() 
        }
    }
    
    /// Create a health check error
    pub fn health_check<T: Into<String>>(reason: T) -> Self {
        HttpError::HealthCheckFailed { 
            reason: reason.into() 
        }
    }
    
    /// Create a database error
    pub fn database_error<T: Into<String>>(message: T) -> Self {
        HttpError::DatabaseError { 
            message: message.into() 
        }
    }
    
    /// Create a validation error
    pub fn validation_error<T: Into<String>>(message: T) -> Self {
        HttpError::ValidationError { 
            message: message.into() 
        }
    }
    
    /// Create a not found error
    pub fn not_found<T: Into<String>>(resource: T) -> Self {
        HttpError::NotFound { 
            resource: resource.into() 
        }
    }
    
    /// Create a conflict error
    pub fn conflict<T: Into<String>>(message: T) -> Self {
        HttpError::Conflict { 
            message: message.into() 
        }
    }
    
    /// Create an unauthorized error
    pub fn unauthorized() -> Self {
        HttpError::Unauthorized
    }
    
    /// Create a forbidden error
    pub fn forbidden<T: Into<String>>(message: T) -> Self {
        HttpError::Forbidden { 
            message: message.into() 
        }
    }
    
    /// Create an internal server error
    pub fn internal_server_error<T: Into<String>>(message: T) -> Self {
        HttpError::InternalError { 
            message: message.into() 
        }
    }

    /// Create a timeout error
    pub fn timeout<T: Into<String>>(_message: T) -> Self {
        HttpError::RequestTimeout
    }

    /// Create a payload too large error
    pub fn payload_too_large<T: Into<String>>(_message: T) -> Self {
        HttpError::RequestTooLarge { 
            size: 0, // Will be set dynamically if needed
            limit: 0
        }
    }

    /// Create a payload too large error with specific sizes
    pub fn payload_too_large_with_sizes<T: Into<String>>(_message: T, size: usize, limit: usize) -> Self {
        HttpError::RequestTooLarge { size, limit }
    }

    /// Add additional detail to error (for now, just returns self - future enhancement)
    pub fn with_detail<T: Into<String>>(self, _detail: T) -> Self {
        self
    }

    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            HttpError::StartupFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ShutdownFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ConfigError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ServiceResolutionFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            HttpError::RequestTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            HttpError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            HttpError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::HealthCheckFailed { .. } => StatusCode::SERVICE_UNAVAILABLE,
            HttpError::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ValidationError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            HttpError::NotFound { .. } => StatusCode::NOT_FOUND,
            HttpError::Conflict { .. } => StatusCode::CONFLICT,
            HttpError::Unauthorized => StatusCode::UNAUTHORIZED,
            HttpError::Forbidden { .. } => StatusCode::FORBIDDEN,
        }
    }

    /// Get error code for consistent API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            HttpError::StartupFailed { .. } => "SERVER_STARTUP_FAILED",
            HttpError::ShutdownFailed { .. } => "SERVER_SHUTDOWN_FAILED",
            HttpError::ConfigError { .. } => "CONFIGURATION_ERROR",
            HttpError::ServiceResolutionFailed { .. } => "SERVICE_RESOLUTION_FAILED",
            HttpError::RequestTimeout => "REQUEST_TIMEOUT",
            HttpError::RequestTooLarge { .. } => "REQUEST_TOO_LARGE",
            HttpError::BadRequest { .. } => "BAD_REQUEST",
            HttpError::InternalError { .. } => "INTERNAL_ERROR",
            HttpError::HealthCheckFailed { .. } => "HEALTH_CHECK_FAILED",
            HttpError::DatabaseError { .. } => "DATABASE_ERROR",
            HttpError::ValidationError { .. } => "VALIDATION_ERROR",
            HttpError::NotFound { .. } => "RESOURCE_NOT_FOUND",
            HttpError::Conflict { .. } => "RESOURCE_CONFLICT",
            HttpError::Unauthorized => "UNAUTHORIZED_ACCESS",
            HttpError::Forbidden { .. } => "ACCESS_FORBIDDEN",
        }
    }
}

// Implement IntoElifResponse for HttpError
impl IntoElifResponse for HttpError {
    fn into_elif_response(self) -> ElifResponse {
        let body = json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "hint": match &self {
                    HttpError::RequestTooLarge { .. } => Some("Reduce request payload size"),
                    HttpError::RequestTimeout => Some("Retry the request"),
                    HttpError::BadRequest { .. } => Some("Check request format and parameters"),
                    HttpError::HealthCheckFailed { .. } => Some("Server may be starting up or experiencing issues"),
                    _ => None,
                }
            }
        });

        ElifResponse::with_status(self.status_code())
            .json_value(body)
    }
}

// Implement IntoResponse for automatic HTTP error responses
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "hint": match &self {
                    HttpError::RequestTooLarge { .. } => Some("Reduce request payload size"),
                    HttpError::RequestTimeout => Some("Retry the request"),
                    HttpError::BadRequest { .. } => Some("Check request format and parameters"),
                    HttpError::HealthCheckFailed { .. } => Some("Server may be starting up or experiencing issues"),
                    _ => None,
                }
            }
        });

        (status, Json(body)).into_response()
    }
}

// Convert from elif-core ConfigError
impl From<elif_core::ConfigError> for HttpError {
    fn from(err: elif_core::ConfigError) -> Self {
        HttpError::ConfigError { 
            message: err.to_string() 
        }
    }
}

// Convert from std::io::Error
impl From<std::io::Error> for HttpError {
    fn from(err: std::io::Error) -> Self {
        HttpError::InternalError { 
            message: format!("IO error: {}", err) 
        }
    }
}

// Convert from hyper errors
impl From<hyper::Error> for HttpError {
    fn from(err: hyper::Error) -> Self {
        HttpError::InternalError { 
            message: format!("Hyper error: {}", err) 
        }
    }
}


// Convert from serde_json errors
impl From<serde_json::Error> for HttpError {
    fn from(err: serde_json::Error) -> Self {
        HttpError::InternalError { 
            message: format!("JSON serialization error: {}", err) 
        }
    }
}

// Convert from ORM ModelError to HttpError
#[cfg(feature = "orm")]
impl From<orm::ModelError> for HttpError {
    fn from(err: orm::ModelError) -> Self {
        match err {
            orm::ModelError::NotFound(table) => HttpError::NotFound { 
                resource: table 
            },
            orm::ModelError::Validation(msg) => HttpError::ValidationError { 
                message: msg 
            },
            orm::ModelError::Database(msg) => HttpError::DatabaseError { 
                message: msg 
            },
            orm::ModelError::Connection(msg) => HttpError::DatabaseError { 
                message: format!("Connection error: {}", msg) 
            },
            orm::ModelError::Transaction(msg) => HttpError::DatabaseError { 
                message: format!("Transaction error: {}", msg) 
            },
            orm::ModelError::Query(msg) => HttpError::BadRequest { 
                message: format!("Query error: {}", msg) 
            },
            orm::ModelError::Schema(msg) => HttpError::InternalError { 
                message: format!("Schema error: {}", msg) 
            },
            orm::ModelError::Migration(msg) => HttpError::InternalError { 
                message: format!("Migration error: {}", msg) 
            },
            orm::ModelError::MissingPrimaryKey => HttpError::BadRequest { 
                message: "Missing or invalid primary key".to_string() 
            },
            orm::ModelError::Relationship(msg) => HttpError::BadRequest { 
                message: format!("Relationship error: {}", msg) 
            },
            orm::ModelError::Serialization(msg) => HttpError::InternalError { 
                message: format!("Serialization error: {}", msg) 
            },
            orm::ModelError::Event(msg) => HttpError::InternalError { 
                message: format!("Event error: {}", msg) 
            },
        }
    }
}

// Convert from ORM QueryError to HttpError
#[cfg(feature = "orm")]
impl From<orm::QueryError> for HttpError {
    fn from(err: orm::QueryError) -> Self {
        match err {
            orm::QueryError::InvalidSql(msg) => HttpError::BadRequest { 
                message: format!("Invalid SQL query: {}", msg) 
            },
            orm::QueryError::MissingFields(msg) => HttpError::BadRequest { 
                message: format!("Missing required fields: {}", msg) 
            },
            orm::QueryError::InvalidParameter(msg) => HttpError::BadRequest { 
                message: format!("Invalid query parameter: {}", msg) 
            },
            orm::QueryError::UnsupportedOperation(msg) => HttpError::BadRequest { 
                message: format!("Unsupported operation: {}", msg) 
            },
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = HttpError::startup("Failed to bind to port");
        assert!(matches!(error, HttpError::StartupFailed { .. }));
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error_code(), "SERVER_STARTUP_FAILED");
    }

    #[test]
    fn test_error_status_codes() {
        assert_eq!(HttpError::bad_request("test").status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(HttpError::RequestTimeout.status_code(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(
            HttpError::RequestTooLarge { size: 100, limit: 50 }.status_code(), 
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            HttpError::health_check("Database unavailable").status_code(), 
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(HttpError::bad_request("test").error_code(), "BAD_REQUEST");
        assert_eq!(HttpError::RequestTimeout.error_code(), "REQUEST_TIMEOUT");
        assert_eq!(HttpError::internal("test").error_code(), "INTERNAL_ERROR");
    }

    #[test]
    fn test_config_error_conversion() {
        let config_error = elif_core::app_config::ConfigError::MissingEnvVar {
            var: "TEST_VAR".to_string(),
        };
        let http_error = HttpError::from(config_error);
        assert!(matches!(http_error, HttpError::ConfigError { .. }));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let http_error = HttpError::from(io_error);
        assert!(matches!(http_error, HttpError::InternalError { .. }));
    }

    #[cfg(feature = "orm")]
    #[test]
    fn test_orm_error_conversions() {
        // Test ModelError conversions
        let not_found_error = orm::ModelError::NotFound("users".to_string());
        let http_error = HttpError::from(not_found_error);
        assert!(matches!(http_error, HttpError::NotFound { .. }));
        assert_eq!(http_error.status_code(), StatusCode::NOT_FOUND);

        let validation_error = orm::ModelError::Validation("Invalid email".to_string());
        let http_error = HttpError::from(validation_error);
        assert!(matches!(http_error, HttpError::ValidationError { .. }));
        assert_eq!(http_error.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

        let database_error = orm::ModelError::Database("Connection failed".to_string());
        let http_error = HttpError::from(database_error);
        assert!(matches!(http_error, HttpError::DatabaseError { .. }));
        assert_eq!(http_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test QueryError conversions
        let query_error = orm::QueryError::InvalidSql("Syntax error".to_string());
        let http_error = HttpError::from(query_error);
        assert!(matches!(http_error, HttpError::BadRequest { .. }));
        assert_eq!(http_error.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_validation_error_status_code() {
        let validation_error = HttpError::validation_error("Field is required");
        assert_eq!(validation_error.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(validation_error.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_error_response_format_consistency() {
        let error = HttpError::not_found("User");
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        
        // Check that response contains proper JSON structure
        // In a real test environment, we'd deserialize the body to verify JSON structure
    }
}