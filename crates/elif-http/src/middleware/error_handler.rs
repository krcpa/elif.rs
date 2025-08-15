//! Error handling middleware for HTTP requests
//! 
//! Provides comprehensive error handling including panic recovery and error response formatting.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{Response, IntoResponse},
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use crate::error::HttpError;
use crate::response::IntoElifResponse;
use tower::Layer;
use tower::Service;
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;

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

/// Error handling middleware function
pub async fn error_handler_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    error_handler_with_config(request, next, ErrorHandlerConfig::default()).await
}

/// Error handling middleware function with custom config
pub async fn error_handler_with_config(
    request: Request<Body>,
    next: Next,
    config: ErrorHandlerConfig,
) -> Result<Response, StatusCode> {
    // Wrap the next handler call in panic recovery
    let result = catch_unwind(AssertUnwindSafe(|| {
        // Create a future that can be executed
        Box::pin(next.run(request))
    }));

    match result {
        Ok(future) => {
            // No panic occurred, execute the future
            let response = future.await;
            Ok(response)
        }
        Err(panic_info) => {
            // Panic occurred, create error response
            let panic_message = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic occurred".to_string()
            };

            if config.log_errors {
                tracing::error!("Panic in request handler: {}", panic_message);
            }

            let error_message = if config.include_panic_details {
                format!("Internal server error: {}", panic_message)
            } else {
                "Internal server error occurred".to_string()
            };

            let http_error = HttpError::internal(error_message);
            Ok(http_error.into_elif_response().build().unwrap_or_else(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }))
        }
    }
}

/// Tower Layer implementation for error handling
#[derive(Clone)]
pub struct ErrorHandlerLayer {
    config: ErrorHandlerConfig,
}

impl ErrorHandlerLayer {
    /// Create new error handler layer
    pub fn new() -> Self {
        Self {
            config: ErrorHandlerConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ErrorHandlerConfig) -> Self {
        Self { config }
    }
}

impl Default for ErrorHandlerLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for ErrorHandlerLayer {
    type Service = ErrorHandlerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ErrorHandlerService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Error handling service
#[derive(Clone)]
pub struct ErrorHandlerService<S> {
    inner: S,
    config: ErrorHandlerConfig,
}

impl<S> Service<Request<Body>> for ErrorHandlerService<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = StatusCode;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();

        Box::pin(async move {
            // Wrap the service call in panic recovery
            let result = catch_unwind(AssertUnwindSafe(|| {
                Box::pin(inner.call(req))
            }));

            match result {
                Ok(future) => {
                    let response = future.await;
                    match response {
                        Ok(response) => Ok(response),
                        Err(error) => {
                            if config.log_errors {
                                tracing::error!("Service error: {}", error);
                            }

                            let http_error = if let Some(custom_handler) = config.custom_error_handler {
                                custom_handler(&error)
                            } else {
                                HttpError::internal("Service error occurred")
                            };

                            Ok(http_error.into_elif_response().build().unwrap_or_else(|_| {
                                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
                            }))
                        }
                    }
                }
                Err(panic_info) => {
                    let panic_message = if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "Unknown panic occurred".to_string()
                    };

                    if config.log_errors {
                        tracing::error!("Panic in service: {}", panic_message);
                    }

                    let error_message = if config.include_panic_details {
                        format!("Internal server error: {}", panic_message)
                    } else {
                        "Internal server error occurred".to_string()
                    };

                    let http_error = HttpError::internal(error_message);
                    Ok(http_error.into_elif_response().build().unwrap_or_else(|_| {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
                    }))
                }
            }
        })
    }
}

/// Helper function to create error handler layer
pub fn error_handler_layer() -> ErrorHandlerLayer {
    ErrorHandlerLayer::new()
}

/// Helper function to create error handler layer with config
pub fn error_handler_layer_with_config(config: ErrorHandlerConfig) -> ErrorHandlerLayer {
    ErrorHandlerLayer::with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::Request,
        http::StatusCode,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::util::ServiceExt;

    async fn panic_handler() -> impl IntoResponse {
        panic!("Test panic");
    }

    async fn error_handler() -> impl IntoResponse {
        HttpError::bad_request("Test error").into_response()
    }

    async fn ok_handler() -> impl IntoResponse {
        "OK"
    }

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
    async fn test_error_handler_layer_creation() {
        let layer = ErrorHandlerLayer::new();
        
        // Test that it can be applied to a router
        let app = Router::new()
            .route("/", get(ok_handler));

        // Test that layer can be created (layer trait implementation test)
        let _layered_service = layer.layer(app.into_make_service());
    }

    #[test]
    fn test_error_handler_config_default() {
        let config = ErrorHandlerConfig::default();
        
        // Should include panic details only in debug mode
        assert_eq!(config.include_panic_details, cfg!(debug_assertions));
        assert!(config.log_errors);
        assert!(config.custom_error_handler.is_none());
    }
}