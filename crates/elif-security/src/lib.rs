//! # elif-security
//!
//! Security middleware and utilities for the elif.rs web framework.
//! Provides CORS, CSRF protection, rate limiting, and other security features.

pub mod config;
pub mod middleware;

// Re-export main types
pub use config::*;
pub use middleware::cors::{CorsMiddleware, CorsConfig};
pub use middleware::csrf::{CsrfMiddleware, CsrfConfig};

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
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Security policy violation: {message}")]
    PolicyViolation { message: String },
}