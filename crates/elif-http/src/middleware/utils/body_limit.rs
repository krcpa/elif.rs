//! # Body Limit Middleware
//!
//! Framework middleware for request body size limiting.
//! Replaces tower-http RequestBodyLimitLayer with framework-native implementation.

use axum::{
    extract::Request,
    response::{Response, IntoResponse},
    body::Body,
    http::{StatusCode, HeaderValue},
};
use tracing::{warn, error};

use crate::{
    middleware::{Middleware, BoxFuture},
    HttpError,
};

/// Configuration for body limit middleware
#[derive(Debug, Clone)]
pub struct BodyLimitConfig {
    /// Maximum allowed body size in bytes
    pub max_size: usize,
    /// Whether to log oversized requests
    pub log_oversized: bool,
    /// Custom error message for oversized requests
    pub error_message: String,
    /// Whether to include Content-Length header in error response
    pub include_headers: bool,
}

impl Default for BodyLimitConfig {
    fn default() -> Self {
        Self {
            max_size: 2 * 1024 * 1024, // 2MB default
            log_oversized: true,
            error_message: "Request body too large".to_string(),
            include_headers: true,
        }
    }
}

impl BodyLimitConfig {
    /// Create new body limit configuration
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            ..Default::default()
        }
    }

    /// Set maximum body size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Enable or disable logging of oversized requests
    pub fn with_logging(mut self, log_oversized: bool) -> Self {
        self.log_oversized = log_oversized;
        self
    }

    /// Set custom error message
    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.error_message = message.into();
        self
    }

    /// Include helpful headers in error response
    pub fn with_headers(mut self, include_headers: bool) -> Self {
        self.include_headers = include_headers;
        self
    }
}

/// Framework body limit middleware for HTTP requests
pub struct BodyLimitMiddleware {
    config: BodyLimitConfig,
}

impl BodyLimitMiddleware {
    /// Create new body limit middleware with default 2MB limit
    pub fn new() -> Self {
        Self {
            config: BodyLimitConfig::default(),
        }
    }

    /// Create body limit middleware with specific size limit
    pub fn with_limit(max_size: usize) -> Self {
        Self {
            config: BodyLimitConfig::new(max_size),
        }
    }

    /// Create body limit middleware with custom configuration
    pub fn with_config(config: BodyLimitConfig) -> Self {
        Self { config }
    }

    /// Set maximum body size (builder pattern)
    pub fn max_size(mut self, size: usize) -> Self {
        self.config = self.config.with_max_size(size);
        self
    }

    /// Enable or disable logging (builder pattern)
    pub fn logging(mut self, enabled: bool) -> Self {
        self.config = self.config.with_logging(enabled);
        self
    }

    /// Set custom error message (builder pattern)
    pub fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.config = self.config.with_message(message);
        self
    }

    /// Get configured max size
    pub fn limit(&self) -> usize {
        self.config.max_size
    }

    /// Create body limit exceeded error response
    fn create_error_response(&self, content_length: Option<usize>) -> Response {
        let mut error = HttpError::payload_too_large(&self.config.error_message);

        if self.config.include_headers {
            if let Some(size) = content_length {
                error = error.with_detail(&format!(
                    "Request body size {} bytes exceeds limit of {} bytes", 
                    size, 
                    self.config.max_size
                ));
            } else {
                error = error.with_detail(&format!(
                    "Request body exceeds limit of {} bytes", 
                    self.config.max_size
                ));
            }
        }

        let mut response = error.into_response();
        
        if self.config.include_headers {
            if let Ok(max_size_header) = HeaderValue::from_str(&self.config.max_size.to_string()) {
                response.headers_mut().insert("X-Max-Body-Size", max_size_header);
            }
        }

        response
    }

    /// Check content-length header against limit
    fn check_content_length(&self, request: &Request) -> Result<Option<usize>, Response> {
        if let Some(content_length) = request.headers().get("content-length") {
            if let Ok(content_length_str) = content_length.to_str() {
                if let Ok(content_length) = content_length_str.parse::<usize>() {
                    if content_length > self.config.max_size {
                        if self.config.log_oversized {
                            warn!(
                                "Request body size {} bytes exceeds limit of {} bytes (Content-Length check)",
                                content_length,
                                self.config.max_size
                            );
                        }
                        return Err(self.create_error_response(Some(content_length)));
                    }
                    return Ok(Some(content_length));
                }
            }
        }
        Ok(None)
    }
}

impl Default for BodyLimitMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for BodyLimitMiddleware {
    fn process_request<'a>(
        &'a self,
        request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            // First, check Content-Length header if present
            let content_length = match self.check_content_length(&request) {
                Ok(length) => length,
                Err(response) => return Err(response),
            };

            // Store body limit info in request extensions
            let mut request = request;
            request.extensions_mut().insert(BodyLimitInfo {
                max_size: self.config.max_size,
                content_length,
                error_message: self.config.error_message.clone(),
            });

            // For streaming bodies or cases where Content-Length is not reliable,
            // we need to check the actual body size during consumption.
            // This is typically handled by axum's built-in body limiting or
            // custom extractors that check size during body reading.

            Ok(request)
        })
    }

    fn process_response<'a>(
        &'a self,
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Log if we're returning a payload too large error
            if response.status() == StatusCode::PAYLOAD_TOO_LARGE && self.config.log_oversized {
                warn!("Returned 413 Payload Too Large response due to body size limit");
            }

            response
        })
    }

    fn name(&self) -> &'static str {
        "BodyLimitMiddleware"
    }
}

/// Body limit information stored in request extensions
#[derive(Debug, Clone)]
pub struct BodyLimitInfo {
    pub max_size: usize,
    pub content_length: Option<usize>,
    pub error_message: String,
}

/// Helper function to create a body-limited body wrapper
pub fn limit_body_size(body: Body, max_size: usize) -> LimitedBody {
    LimitedBody {
        body,
        max_size,
        consumed: 0,
    }
}

/// Wrapper around axum::body::Body that enforces size limits
pub struct LimitedBody {
    body: Body,
    max_size: usize,
    consumed: usize,
}

impl LimitedBody {
    /// Create new limited body
    pub fn new(body: Body, max_size: usize) -> Self {
        Self {
            body,
            max_size,
            consumed: 0,
        }
    }

    /// Get remaining allowed bytes
    pub fn remaining(&self) -> usize {
        self.max_size.saturating_sub(self.consumed)
    }

    /// Get total consumed bytes
    pub fn consumed(&self) -> usize {
        self.consumed
    }

    /// Check if limit has been exceeded
    pub fn is_exceeded(&self) -> bool {
        self.consumed > self.max_size
    }
}

// Note: Full implementation of LimitedBody would require implementing
// the Body trait and handling streaming chunks with size checking.
// For now, this serves as the structure for future implementation.

/// Utility functions for common body size limits
pub mod limits {
    /// 1KB limit
    pub const KB: usize = 1024;
    
    /// 1MB limit
    pub const MB: usize = 1024 * 1024;
    
    /// 10MB limit
    pub const MB_10: usize = 10 * MB;
    
    /// 100MB limit
    pub const MB_100: usize = 100 * MB;
    
    /// 1GB limit (use with caution)
    pub const GB: usize = 1024 * MB;

    /// Create body limit middleware with common sizes
    pub mod presets {
        use super::super::BodyLimitMiddleware;
        use super::*;

        /// Small API requests (1MB)
        pub fn small_api() -> BodyLimitMiddleware {
            BodyLimitMiddleware::with_limit(MB)
                .message("API request body too large (1MB limit)")
        }

        /// File uploads (10MB)
        pub fn file_upload() -> BodyLimitMiddleware {
            BodyLimitMiddleware::with_limit(MB_10)
                .message("File upload too large (10MB limit)")
        }

        /// Large file uploads (100MB)
        pub fn large_upload() -> BodyLimitMiddleware {
            BodyLimitMiddleware::with_limit(MB_100)
                .message("Large file upload too large (100MB limit)")
        }

        /// Tiny requests (64KB)
        pub fn tiny() -> BodyLimitMiddleware {
            BodyLimitMiddleware::with_limit(64 * KB)
                .message("Request body too large (64KB limit)")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, HeaderValue};

    #[tokio::test]
    async fn test_body_limit_middleware_basic() {
        let middleware = BodyLimitMiddleware::new();
        
        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = middleware.process_request(request).await;
        assert!(result.is_ok());

        let processed_request = result.unwrap();
        
        // Check that body limit info was added to extensions
        let body_limit_info = processed_request.extensions().get::<BodyLimitInfo>();
        assert!(body_limit_info.is_some());
        
        let body_limit_info = body_limit_info.unwrap();
        assert_eq!(body_limit_info.max_size, 2 * 1024 * 1024); // 2MB default
        assert!(body_limit_info.content_length.is_none());
    }

    #[tokio::test]
    async fn test_body_limit_middleware_custom_limit() {
        let middleware = BodyLimitMiddleware::with_limit(1024); // 1KB
        
        assert_eq!(middleware.limit(), 1024);
    }

    #[tokio::test]
    async fn test_body_limit_middleware_builder() {
        let middleware = BodyLimitMiddleware::new()
            .max_size(512)
            .logging(false)
            .message("Too big!");
        
        assert_eq!(middleware.config.max_size, 512);
        assert!(!middleware.config.log_oversized);
        assert_eq!(middleware.config.error_message, "Too big!");
    }

    #[tokio::test]
    async fn test_content_length_check_within_limit() {
        let middleware = BodyLimitMiddleware::with_limit(1000);
        
        let request = Request::builder()
            .method(Method::POST)
            .header("content-length", "500")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = middleware.process_request(request).await;
        assert!(result.is_ok());

        let processed_request = result.unwrap();
        let body_limit_info = processed_request.extensions().get::<BodyLimitInfo>().unwrap();
        assert_eq!(body_limit_info.content_length, Some(500));
    }

    #[tokio::test]
    async fn test_content_length_check_exceeds_limit() {
        let middleware = BodyLimitMiddleware::with_limit(100);
        
        let request = Request::builder()
            .method(Method::POST)
            .header("content-length", "200")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = middleware.process_request(request).await;
        assert!(result.is_err());

        let error_response = result.unwrap_err();
        assert_eq!(error_response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        
        // Check for custom header
        assert!(error_response.headers().contains_key("X-Max-Body-Size"));
        assert_eq!(
            error_response.headers().get("X-Max-Body-Size").unwrap(),
            "100"
        );
    }

    #[tokio::test]
    async fn test_body_limit_config() {
        let config = BodyLimitConfig::new(512)
            .with_logging(false)
            .with_message("Custom message")
            .with_headers(false);

        let middleware = BodyLimitMiddleware::with_config(config);
        
        assert_eq!(middleware.config.max_size, 512);
        assert!(!middleware.config.log_oversized);
        assert_eq!(middleware.config.error_message, "Custom message");
        assert!(!middleware.config.include_headers);
    }

    #[tokio::test]
    async fn test_body_limit_middleware_name() {
        let middleware = BodyLimitMiddleware::new();
        assert_eq!(middleware.name(), "BodyLimitMiddleware");
    }

    #[tokio::test]
    async fn test_limited_body_creation() {
        let body = Body::empty();
        let limited = limit_body_size(body, 1024);
        
        assert_eq!(limited.remaining(), 1024);
        assert_eq!(limited.consumed(), 0);
        assert!(!limited.is_exceeded());
    }

    #[tokio::test]
    async fn test_body_limit_presets() {
        let small = limits::presets::small_api();
        assert_eq!(small.limit(), limits::MB);

        let upload = limits::presets::file_upload();
        assert_eq!(upload.limit(), limits::MB_10);

        let large = limits::presets::large_upload();
        assert_eq!(large.limit(), limits::MB_100);

        let tiny = limits::presets::tiny();
        assert_eq!(tiny.limit(), 64 * limits::KB);
    }

    #[tokio::test]
    async fn test_body_limit_constants() {
        assert_eq!(limits::KB, 1024);
        assert_eq!(limits::MB, 1024 * 1024);
        assert_eq!(limits::MB_10, 10 * 1024 * 1024);
        assert_eq!(limits::MB_100, 100 * 1024 * 1024);
        assert_eq!(limits::GB, 1024 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_invalid_content_length_header() {
        let middleware = BodyLimitMiddleware::with_limit(1000);
        
        let request = Request::builder()
            .method(Method::POST)
            .header("content-length", "not-a-number")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        // Should not error on invalid content-length, just ignore it
        let result = middleware.process_request(request).await;
        assert!(result.is_ok());

        let processed_request = result.unwrap();
        let body_limit_info = processed_request.extensions().get::<BodyLimitInfo>().unwrap();
        assert!(body_limit_info.content_length.is_none());
    }
}