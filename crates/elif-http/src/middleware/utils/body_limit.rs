//! # Body Limit Middleware
//!
//! Framework middleware for request body size limiting using V2 system.
//! Replaces tower-http RequestBodyLimitLayer with framework-native implementation.

use tracing::warn;

use crate::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::{ElifResponse, ElifStatusCode},
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
#[derive(Debug)]
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
}

impl Default for BodyLimitMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for BodyLimitMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let config = self.config.clone();
        Box::pin(async move {
            // First, check Content-Length header if present
            let _content_length = {
                if let Some(content_length) = request.headers.get_str("content-length") {
                    if let Ok(content_length_str) = content_length.to_str() {
                        if let Ok(content_length) = content_length_str.parse::<usize>() {
                            if content_length > config.max_size {
                                if config.log_oversized {
                                    warn!(
                                        "Request body size {} bytes exceeds limit of {} bytes (Content-Length check)",
                                        content_length,
                                        config.max_size
                                    );
                                }
                                
                                let mut response = ElifResponse::with_status(ElifStatusCode::PAYLOAD_TOO_LARGE)
                                    .text(format!("Request body size {} bytes exceeds limit of {} bytes", 
                                                content_length, config.max_size));
                                
                                if config.include_headers {
                                    if let Err(e) = response.add_header("X-Max-Body-Size", &config.max_size.to_string()) {
                                        warn!("Failed to add X-Max-Body-Size header: {}", e);
                                    }
                                }
                                
                                return response;
                            }
                            Some(content_length)
                        } else { None }
                    } else { None }
                } else { None }
            };

            // For streaming bodies or cases where Content-Length is not reliable,
            // we need to check the actual body size during consumption.
            // This is typically handled by the framework's built-in body limiting or
            // custom extractors that check size during body reading.

            // Continue to next middleware/handler
            let response = next.run(request).await;

            // Log if we're returning a payload too large error
            if response.status_code() == ElifStatusCode::PAYLOAD_TOO_LARGE && config.log_oversized {
                warn!("Returned 413 Payload Too Large response due to body size limit");
            }

            response
        })
    }

    fn name(&self) -> &'static str {
        "BodyLimitMiddleware"
    }
}

/// Body limit information for tracking
#[derive(Debug, Clone)]
pub struct BodyLimitInfo {
    pub max_size: usize,
    pub content_length: Option<usize>,
    pub error_message: String,
}

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
    use crate::middleware::v2::MiddlewarePipelineV2;
    use crate::request::{ElifRequest, ElifMethod};
    use crate::response::headers::ElifHeaderMap;
    use crate::response::{ElifResponse, ElifStatusCode};

    #[tokio::test]
    async fn test_body_limit_middleware_v2() {
        let middleware = BodyLimitMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let headers = ElifHeaderMap::new();
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/test".parse().unwrap(),
            headers,
        );

        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Success")
            })
        }).await;
        
        assert_eq!(response.status_code(), ElifStatusCode::OK);
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
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("content-length".parse().unwrap(), "500".parse().unwrap());
        
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/test".parse().unwrap(),
            headers,
        );

        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Success")
            })
        }).await;
        
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_content_length_check_exceeds_limit() {
        let middleware = BodyLimitMiddleware::with_limit(100);
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("content-length".parse().unwrap(), "200".parse().unwrap());
        
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/test".parse().unwrap(),
            headers,
        );

        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should not reach here")
            })
        }).await;
        
        assert_eq!(response.status_code(), ElifStatusCode::PAYLOAD_TOO_LARGE);
        assert!(response.has_header("X-Max-Body-Size"));
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
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("content-length".parse().unwrap(), "not-a-number".parse().unwrap());
        
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/test".parse().unwrap(),
            headers,
        );

        // Should not error on invalid content-length, just ignore it
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Success")
            })
        }).await;
        
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }
}