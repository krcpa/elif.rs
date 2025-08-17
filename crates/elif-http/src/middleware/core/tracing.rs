//! # Tracing Middleware
//!
//! Framework middleware for HTTP request tracing and observability.
//! Replaces tower-http TraceLayer with framework-native implementation.

use std::time::Instant;
use axum::{
    extract::Request,
    response::Response,
};
use tracing::{info, warn, error, Span, Level};
use uuid::Uuid;

use crate::middleware::{Middleware, BoxFuture};

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
    fn format_headers(&self, headers: &axum::http::HeaderMap) -> String {
        headers
            .iter()
            .map(|(name, value)| {
                let name_str = name.as_str();
                let value_str = if self.is_sensitive_header(name_str) {
                    "[REDACTED]"
                } else {
                    value.to_str().unwrap_or("[INVALID_UTF8]")
                };
                format!("{}={}", name_str, value_str)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Default for TracingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TracingMiddleware {
    fn process_request<'a>(
        &'a self,
        mut request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            let start_time = Instant::now();
            let request_id = Uuid::new_v4();
            
            // Create tracing span for this request
            let span = match self.config.level {
                Level::ERROR => tracing::error_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::WARN => tracing::warn_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::INFO => tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::DEBUG => tracing::debug_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
                Level::TRACE => tracing::trace_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    remote_addr = tracing::field::Empty,
                ),
            };

            // Store request metadata in extensions
            request.extensions_mut().insert(RequestMetadata {
                request_id,
                start_time,
                span: span.clone(),
            });

            // Enter the span for this request
            let _enter = span.enter();

            // Log request details
            match self.config.level {
                Level::ERROR => error!(
                    "HTTP Request: {} {} (ID: {})",
                    request.method(),
                    request.uri(),
                    request_id
                ),
                Level::WARN => warn!(
                    "HTTP Request: {} {} (ID: {})",
                    request.method(),
                    request.uri(), 
                    request_id
                ),
                Level::INFO => info!(
                    "HTTP Request: {} {} (ID: {})",
                    request.method(),
                    request.uri(),
                    request_id
                ),
                Level::DEBUG => {
                    let headers = self.format_headers(request.headers());
                    tracing::debug!(
                        "HTTP Request: {} {} (ID: {}) - Headers: {}",
                        request.method(),
                        request.uri(),
                        request_id,
                        headers
                    );
                },
                Level::TRACE => {
                    let headers = self.format_headers(request.headers());
                    tracing::trace!(
                        "HTTP Request: {} {} (ID: {}) - Headers: {} - Body tracing: {}",
                        request.method(),
                        request.uri(),
                        request_id,
                        headers,
                        self.config.trace_bodies
                    );
                }
            }

            Ok(request)
        })
    }

    fn process_response<'a>(
        &'a self,
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let status = response.status();
            
            // Try to get request metadata from response extensions
            // Note: In real middleware pipeline, this would be passed through
            // For now, we'll create minimal tracing
            
            match self.config.level {
                Level::ERROR if status.is_server_error() => {
                    error!("HTTP Response: {} (Server Error)", status);
                },
                Level::WARN if status.is_client_error() => {
                    warn!("HTTP Response: {} (Client Error)", status);
                },
                Level::INFO => {
                    info!("HTTP Response: {}", status);
                },
                Level::DEBUG => {
                    let headers = self.format_headers(response.headers());
                    tracing::debug!(
                        "HTTP Response: {} - Headers: {}",
                        status,
                        headers
                    );
                },
                Level::TRACE => {
                    let headers = self.format_headers(response.headers());
                    tracing::trace!(
                        "HTTP Response: {} - Headers: {} - Body tracing: {}",
                        status,
                        headers,
                        self.config.trace_response_bodies
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

/// Request metadata stored in request extensions
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub request_id: Uuid,
    pub start_time: Instant,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, StatusCode, HeaderValue};
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_tracing_middleware_basic() {
        let middleware = TracingMiddleware::new();
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = middleware.process_request(request).await;
        assert!(result.is_ok());

        let processed_request = result.unwrap();
        
        // Check that request metadata was added
        let metadata = processed_request.extensions().get::<RequestMetadata>();
        assert!(metadata.is_some());
        
        let metadata = metadata.unwrap();
        assert!(!metadata.request_id.is_nil());
        assert!(metadata.start_time.elapsed().as_nanos() > 0);
    }

    #[traced_test]
    #[tokio::test] 
    async fn test_tracing_middleware_response() {
        let middleware = TracingMiddleware::new();
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::empty())
            .unwrap();

        let processed_response = middleware.process_response(response).await;
        assert_eq!(processed_response.status(), StatusCode::OK);
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
    async fn test_header_formatting() {
        let middleware = TracingMiddleware::new();
        
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        headers.insert("x-custom", HeaderValue::from_static("value"));

        let formatted = middleware.format_headers(&headers);
        
        assert!(formatted.contains("content-type=application/json"));
        assert!(formatted.contains("authorization=[REDACTED]"));
        assert!(formatted.contains("x-custom=value"));
    }

    #[tokio::test]
    async fn test_tracing_middleware_name() {
        let middleware = TracingMiddleware::new();
        assert_eq!(middleware.name(), "TracingMiddleware");
    }
}