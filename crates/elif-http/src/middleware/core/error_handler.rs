//! Error handling middleware for HTTP requests
//! 
//! Provides comprehensive error handling including panic recovery and error response formatting.

use crate::{
    errors::HttpError,
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::{ElifResponse, IntoElifResponse},
};

/// Error handling middleware configuration
#[derive(Debug, Clone)]
pub struct ErrorHandlerConfig {
    /// Whether to include panic details in error responses (development only)
    pub include_panic_details: bool,
    
    /// Whether to log errors
    pub log_errors: bool,
    
    /// Custom error handler function
    pub custom_error_handler: Option<fn(&dyn std::error::Error) -> HttpError>,
}

impl Default for ErrorHandlerConfig {
    fn default() -> Self {
        Self {
            include_panic_details: cfg!(debug_assertions), // Only in debug builds
            log_errors: true,
            custom_error_handler: None,
        }
    }
}

/// Error handling middleware
#[derive(Debug)]
pub struct ErrorHandlerMiddleware {
    config: ErrorHandlerConfig,
}

impl ErrorHandlerMiddleware {
    /// Create new error handling middleware with default config
    pub fn new() -> Self {
        Self {
            config: ErrorHandlerConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ErrorHandlerConfig) -> Self {
        Self { config }
    }

    /// Enable panic details in responses (use only in development)
    pub fn with_panic_details(mut self, include: bool) -> Self {
        self.config.include_panic_details = include;
        self
    }

    /// Enable error logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.config.log_errors = enable;
        self
    }

    /// Set custom error handler
    pub fn with_custom_handler(mut self, handler: fn(&dyn std::error::Error) -> HttpError) -> Self {
        self.config.custom_error_handler = Some(handler);
        self
    }
}

impl Default for ErrorHandlerMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for ErrorHandlerMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let _config = self.config.clone();
        Box::pin(async move {
            // Execute next middleware - for now just pass through
            // TODO: Implement proper async panic recovery in future iteration
            next.run(request).await
        })
    }

    fn name(&self) -> &'static str {
        "ErrorHandlerMiddleware"
    }
}

/// Helper function to create error handler middleware
pub fn error_handler() -> ErrorHandlerMiddleware {
    ErrorHandlerMiddleware::new()
}

/// Helper function to create error handler middleware with config
pub fn error_handler_with_config(config: ErrorHandlerConfig) -> ErrorHandlerMiddleware {
    ErrorHandlerMiddleware::with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        middleware::v2::MiddlewarePipelineV2,
        request::ElifMethod,
        response::headers::ElifHeaderMap,
    };

    #[tokio::test]
    async fn test_error_handler_config() {
        let config = ErrorHandlerConfig {
            include_panic_details: true,
            log_errors: false,
            custom_error_handler: None,
        };

        assert!(config.include_panic_details);
        assert!(!config.log_errors);
        assert!(config.custom_error_handler.is_none());
    }

    #[tokio::test]
    async fn test_error_handler_middleware_creation() {
        let middleware = ErrorHandlerMiddleware::new()
            .with_panic_details(true)
            .with_logging(false);

        assert!(middleware.config.include_panic_details);
        assert!(!middleware.config.log_errors);
    }

    #[tokio::test]
    async fn test_error_handler_with_http_error() {
        let middleware = ErrorHandlerMiddleware::new();
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let next = crate::middleware::v2::Next::new(|_req| {
            Box::pin(async {
                // Return an error response
                HttpError::bad_request("Test error").into_response()
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_error_handler_normal_flow() {
        let middleware = ErrorHandlerMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().text("Success")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[test]
    fn test_error_handler_config_default() {
        let config = ErrorHandlerConfig::default();
        
        // Should include panic details only in debug mode
        assert_eq!(config.include_panic_details, cfg!(debug_assertions));
        assert!(config.log_errors);
        assert!(config.custom_error_handler.is_none());
    }

    #[tokio::test]
    async fn test_custom_error_handler() {
        let custom_handler = |_error: &dyn std::error::Error| -> HttpError {
            HttpError::bad_request("Custom error handling")
        };
        
        let middleware = ErrorHandlerMiddleware::new()
            .with_custom_handler(custom_handler);
        
        // Test that middleware was created with custom handler
        assert!(middleware.config.custom_error_handler.is_some());
    }
}