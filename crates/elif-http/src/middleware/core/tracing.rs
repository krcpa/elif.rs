//! # Tracing Middleware
//!
//! Framework middleware for HTTP request tracing and observability using V2 system.
//! Replaces tower-http TraceLayer with framework-native implementation.

use std::time::Instant;
use tracing::{info, warn, error, Span, Level};
use uuid::Uuid;

use crate::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::ElifResponse,
};

/// Configuration for tracing middleware
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Whether to trace request bodies
    pub trace_bodies: bool,
    /// Whether to trace response bodies  
    pub trace_response_bodies: bool,
    /// Maximum body size to trace (in bytes)
    pub max_body_size: usize,
    /// Log level for requests
    pub level: Level,
    /// Whether to include sensitive headers in traces
    pub include_sensitive_headers: bool,
    /// Headers considered sensitive (will be redacted)
    pub sensitive_headers: Vec<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            trace_bodies: false,
            trace_response_bodies: false,
            max_body_size: 1024,
            level: Level::INFO,
            include_sensitive_headers: false,
            sensitive_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "x-api-key".to_string(),
                "x-auth-token".to_string(),
            ],
        }
    }
}

impl TracingConfig {
    /// Enable body tracing
    pub fn with_body_tracing(mut self) -> Self {
        self.trace_bodies = true;
        self
    }

    /// Enable response body tracing
    pub fn with_response_body_tracing(mut self) -> Self {
        self.trace_response_bodies = true;
        self
    }

    /// Set maximum body size for tracing
    pub fn with_max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Set tracing level
    pub fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Include sensitive headers in traces (not recommended for production)
    pub fn with_sensitive_headers(mut self) -> Self {
        self.include_sensitive_headers = true;
        self
    }

    /// Add custom sensitive header
    pub fn add_sensitive_header(mut self, header: String) -> Self {
        self.sensitive_headers.push(header.to_lowercase());
        self
    }
}

/// Framework tracing middleware for HTTP requests
#[derive(Debug)]
pub struct TracingMiddleware {
    config: TracingConfig,
}

impl TracingMiddleware {
    /// Create new tracing middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: TracingConfig::default(),
        }
    }

    /// Create tracing middleware with custom configuration
    pub fn with_config(config: TracingConfig) -> Self {
        Self { config }
    }

    /// Enable body tracing
    pub fn with_body_tracing(mut self) -> Self {
        self.config = self.config.with_body_tracing();
        self
    }

    /// Set tracing level
    pub fn with_level(mut self, level: Level) -> Self {
        self.config = self.config.with_level(level);
        self
    }

    /// Check if header is sensitive
    fn is_sensitive_header(&self, name: &str) -> bool {
        if self.config.include_sensitive_headers {
            return false;
        }
        
        let name_lower = name.to_lowercase();
        self.config.sensitive_headers.iter().any(|h| h == &name_lower)
    }

    /// Format headers for tracing
    fn format_headers(&self, headers: &crate::response::ElifHeaderMap) -> String {
        let mut header_strings = Vec::new();
        
        for name in headers.keys() {
            let name_str = name.as_str();
            if let Some(value) = headers.get_str(name_str) {
                let value_str = if self.is_sensitive_header(name_str) {
                    "[REDACTED]"
                } else {
                    value.to_str().unwrap_or("[INVALID_UTF8]")
                };
                header_strings.push(format!("{}={}", name_str, value_str));
            }
        }
        
        header_strings.join(", ")
    }

    /// Format response headers for tracing
    fn format_response_headers(&self, headers: &crate::response::ElifHeaderMap) -> String {
        let mut header_strings = Vec::new();
        
        for (name, value) in headers.iter() {
            let name_str = name.as_str();
            let value_str = if self.is_sensitive_header(name_str) {
                "[REDACTED]"
            } else {
                value.to_str().unwrap_or("[INVALID_UTF8]")
            };
            header_strings.push(format!("{}={}", name_str, value_str));
        }
        
        header_strings.join(", ")
    }
}

impl Default for TracingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TracingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let config = self.config.clone();
        Box::pin(async move {
            let start_time = Instant::now();
            let request_id = Uuid::new_v4();
            
            // Create tracing span for this request
            let span = match config.level {
                Level::ERROR => tracing::error_span!(
                    "http_request",
                    method = %request.method,
                    uri = %request.uri,
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::WARN => tracing::warn_span!(
                    "http_request",
                    method = %request.method,
                    uri = %request.uri,
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::INFO => tracing::info_span!(
                    "http_request",
                    method = %request.method,
                    uri = %request.uri,
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::DEBUG => tracing::debug_span!(
                    "http_request",
                    method = %request.method,
                    uri = %request.uri,
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::TRACE => tracing::trace_span!(
                    "http_request",
                    method = %request.method,
                    uri = %request.uri,
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
            };

            // Enter the span for this request
            let _enter = span.enter();

            // Log request details based on level
            match config.level {
                Level::ERROR => error!(
                    "HTTP Request: {} {} (ID: {})",
                    request.method,
                    request.uri,
                    request_id
                ),
                Level::WARN => warn!(
                    "HTTP Request: {} {} (ID: {})",
                    request.method,
                    request.uri, 
                    request_id
                ),
                Level::INFO => info!(
                    "HTTP Request: {} {} (ID: {})",
                    request.method,
                    request.uri,
                    request_id
                ),
                Level::DEBUG => {
                    let headers = {
                        let mut header_strings = Vec::new();
                        
                        for name in request.headers.keys() {
                            let name_str = name.as_str();
                            if let Some(value) = request.headers.get_str(name_str) {
                                let value_str = if config.include_sensitive_headers {
                                    value.to_str().unwrap_or("[INVALID_UTF8]")
                                } else {
                                    let name_lower = name_str.to_lowercase();
                                    if config.sensitive_headers.iter().any(|h| h == &name_lower) {
                                        "[REDACTED]"
                                    } else {
                                        value.to_str().unwrap_or("[INVALID_UTF8]")
                                    }
                                };
                                header_strings.push(format!("{}={}", name_str, value_str));
                            }
                        }
                        
                        header_strings.join(", ")
                    };
                    tracing::debug!(
                        "HTTP Request: {} {} (ID: {}) - Headers: {}",
                        request.method,
                        request.uri,
                        request_id,
                        headers
                    );
                },
                Level::TRACE => {
                    let headers = {
                        let mut header_strings = Vec::new();
                        
                        for name in request.headers.keys() {
                            let name_str = name.as_str();
                            if let Some(value) = request.headers.get_str(name_str) {
                                let value_str = if config.include_sensitive_headers {
                                    value.to_str().unwrap_or("[INVALID_UTF8]")
                                } else {
                                    let name_lower = name_str.to_lowercase();
                                    if config.sensitive_headers.iter().any(|h| h == &name_lower) {
                                        "[REDACTED]"
                                    } else {
                                        value.to_str().unwrap_or("[INVALID_UTF8]")
                                    }
                                };
                                header_strings.push(format!("{}={}", name_str, value_str));
                            }
                        }
                        
                        header_strings.join(", ")
                    };
                    tracing::trace!(
                        "HTTP Request: {} {} (ID: {}) - Headers: {} - Body tracing: {}",
                        request.method,
                        request.uri,
                        request_id,
                        headers,
                        config.trace_bodies
                    );
                }
            }

            // Continue to next middleware/handler
            let response = next.run(request).await;
            
            // Calculate duration and log response
            let duration = start_time.elapsed();
            let status = response.status_code();
            
            match config.level {
                Level::ERROR if status.is_server_error() => {
                    error!("HTTP Response: {:?} (Server Error) - Duration: {:?} (ID: {})", status, duration, request_id);
                },
                Level::WARN if status.is_client_error() => {
                    warn!("HTTP Response: {:?} (Client Error) - Duration: {:?} (ID: {})", status, duration, request_id);
                },
                Level::INFO => {
                    info!("HTTP Response: {:?} - Duration: {:?} (ID: {})", status, duration, request_id);
                },
                Level::DEBUG => {
                    let headers = {
                        let mut header_strings = Vec::new();
                        
                        for (name, value) in response.headers().iter() {
                            let name_str = name.as_str();
                            let value_str = if config.include_sensitive_headers {
                                value.to_str().unwrap_or("[INVALID_UTF8]")
                            } else {
                                let name_lower = name_str.to_lowercase();
                                if config.sensitive_headers.iter().any(|h| h == &name_lower) {
                                    "[REDACTED]"
                                } else {
                                    value.to_str().unwrap_or("[INVALID_UTF8]")
                                }
                            };
                            header_strings.push(format!("{}={}", name_str, value_str));
                        }
                        
                        header_strings.join(", ")
                    };
                    tracing::debug!(
                        "HTTP Response: {:?} - Duration: {:?} - Headers: {} (ID: {})",
                        status,
                        duration,
                        headers,
                        request_id
                    );
                },
                Level::TRACE => {
                    let headers = {
                        let mut header_strings = Vec::new();
                        
                        for (name, value) in response.headers().iter() {
                            let name_str = name.as_str();
                            let value_str = if config.include_sensitive_headers {
                                value.to_str().unwrap_or("[INVALID_UTF8]")
                            } else {
                                let name_lower = name_str.to_lowercase();
                                if config.sensitive_headers.iter().any(|h| h == &name_lower) {
                                    "[REDACTED]"
                                } else {
                                    value.to_str().unwrap_or("[INVALID_UTF8]")
                                }
                            };
                            header_strings.push(format!("{}={}", name_str, value_str));
                        }
                        
                        header_strings.join(", ")
                    };
                    tracing::trace!(
                        "HTTP Response: {:?} - Duration: {:?} - Headers: {} - Body tracing: {} (ID: {})",
                        status,
                        duration,
                        headers,
                        config.trace_response_bodies,
                        request_id
                    );
                },
                _ => {} // Skip logging for other combinations
            }

            response
        })
    }

    fn name(&self) -> &'static str {
        "TracingMiddleware"
    }
}

/// Request metadata for tracing context
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub request_id: Uuid,
    pub start_time: Instant,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::v2::MiddlewarePipelineV2;
    use crate::request::{ElifRequest, ElifMethod};
    use crate::response::{ElifResponse, ElifStatusCode, ElifHeaderMap};

    #[tokio::test]
    async fn test_tracing_middleware_v2() {
        let middleware = TracingMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("content-type".parse().unwrap(), "application/json".parse().unwrap());
        headers.insert("authorization".parse().unwrap(), "Bearer secret".parse().unwrap());
        
        let request = ElifRequest::new(
            ElifMethod::GET,
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
    async fn test_tracing_config_customization() {
        let config = TracingConfig::default()
            .with_body_tracing()
            .with_level(Level::DEBUG)
            .with_max_body_size(2048)
            .add_sensitive_header("x-custom-secret".to_string());

        let middleware = TracingMiddleware::with_config(config);
        assert!(middleware.config.trace_bodies);
        assert_eq!(middleware.config.level, Level::DEBUG);
        assert_eq!(middleware.config.max_body_size, 2048);
        assert!(middleware.config.sensitive_headers.contains(&"x-custom-secret".to_string()));
    }

    #[tokio::test]
    async fn test_sensitive_header_detection() {
        let middleware = TracingMiddleware::new();
        
        assert!(middleware.is_sensitive_header("Authorization"));
        assert!(middleware.is_sensitive_header("COOKIE"));
        assert!(middleware.is_sensitive_header("x-api-key"));
        assert!(!middleware.is_sensitive_header("content-type"));
        assert!(!middleware.is_sensitive_header("accept"));
    }

    #[tokio::test]
    async fn test_tracing_middleware_name() {
        let middleware = TracingMiddleware::new();
        assert_eq!(middleware.name(), "TracingMiddleware");
    }
}