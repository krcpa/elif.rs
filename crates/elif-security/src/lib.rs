//! # elif-security
//!
//! Security middleware and utilities for the elif.rs web framework.
//! Provides CORS, CSRF protection, rate limiting, and other security features.

pub mod config;
pub mod integration;
pub mod middleware;

// Re-export main types
pub use config::*;
pub use integration::{
    basic_security_pipeline, development_security_pipeline, strict_security_pipeline,
    SecurityMiddlewareConfig, SecurityMiddlewareConfigBuilder,
};
pub use middleware::cors::{CorsConfig, CorsMiddleware};
pub use middleware::csrf::{
    CsrfConfig, CsrfMiddleware, CsrfMiddlewareConfig, CsrfMiddlewareConfigBuilder,
};
pub use middleware::rate_limit::{RateLimitConfig, RateLimitIdentifier, RateLimitMiddleware};
pub use middleware::sanitization::{SanitizationConfig, SanitizationMiddleware};
pub use middleware::security_headers::{SecurityHeadersConfig, SecurityHeadersMiddleware};

/// Common result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Security-related errors
#[derive(thiserror::Error, Debug)]
pub enum SecurityError {
    #[error("CORS violation: {message}")]
    CorsViolation { message: String },

    #[error("CSRF token validation failed")]
    CsrfValidationFailed,

    #[error("Rate limit exceeded: {limit} requests per {window_seconds} seconds")]
    RateLimitExceeded { limit: u32, window_seconds: u32 },

    #[error("Rate limiting error: {0}")]
    RateLimitError(String),

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Security policy violation: {message}")]
    PolicyViolation { message: String },
}
