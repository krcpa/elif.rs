//! # Request ID Middleware
//!
//! Provides unique request ID generation and tracking for distributed systems.
//! Supports X-Request-ID header forwarding and custom ID generation strategies.

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::ElifRequest;
use crate::response::ElifResponse;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Request ID generation strategy
#[derive(Debug)]
pub enum RequestIdStrategy {
    /// Generate UUID v4 (random)
    UuidV4,
    /// Generate UUID v1 (timestamp-based)
    UuidV1,
    /// Use incrementing counter (not suitable for distributed systems)
    Counter(AtomicU64),
    /// Use custom prefix with UUID
    PrefixedUuid(String),
    /// Use custom function to generate request ID
    Custom(fn() -> String),
}

impl Default for RequestIdStrategy {
    fn default() -> Self {
        Self::UuidV4
    }
}

impl Clone for RequestIdStrategy {
    fn clone(&self) -> Self {
        match self {
            Self::UuidV4 => Self::UuidV4,
            Self::UuidV1 => Self::UuidV1,
            Self::Counter(counter) => {
                // Create new counter starting from current value
                Self::Counter(AtomicU64::new(counter.load(Ordering::Relaxed)))
            }
            Self::PrefixedUuid(prefix) => Self::PrefixedUuid(prefix.clone()),
            Self::Custom(func) => Self::Custom(*func),
        }
    }
}

impl RequestIdStrategy {
    /// Generate a new request ID using this strategy
    pub fn generate(&self) -> String {
        match self {
            Self::UuidV4 => Uuid::new_v4().to_string(),
            Self::UuidV1 => {
                // UUID v1 requires MAC address and timestamp
                // For simplicity, we'll use v4 with timestamp prefix
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                format!("{}-{}", timestamp, Uuid::new_v4())
            }
            Self::Counter(counter) => {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                format!("req-{:016x}", count)
            }
            Self::PrefixedUuid(prefix) => {
                format!("{}-{}", prefix, Uuid::new_v4())
            }
            Self::Custom(generator) => generator(),
        }
    }
}

/// Configuration for request ID middleware
#[derive(Debug)]
pub struct RequestIdConfig {
    /// Header name for request ID (default: "x-request-id")
    pub header_name: String,
    /// Request ID generation strategy
    pub strategy: RequestIdStrategy,
    /// Whether to generate new ID if one already exists
    pub override_existing: bool,
    /// Whether to add request ID to response headers
    pub add_to_response: bool,
    /// Whether to log request ID
    pub log_request_id: bool,
}

impl Clone for RequestIdConfig {
    fn clone(&self) -> Self {
        Self {
            header_name: self.header_name.clone(),
            strategy: self.strategy.clone(),
            override_existing: self.override_existing,
            add_to_response: self.add_to_response,
            log_request_id: self.log_request_id,
        }
    }
}

impl Default for RequestIdConfig {
    fn default() -> Self {
        Self {
            header_name: "x-request-id".to_string(),
            strategy: RequestIdStrategy::default(),
            override_existing: false,
            add_to_response: true,
            log_request_id: true,
        }
    }
}

/// Middleware for request ID generation and tracking
#[derive(Debug)]
pub struct RequestIdMiddleware {
    config: RequestIdConfig,
}

impl RequestIdMiddleware {
    /// Create new request ID middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: RequestIdConfig::default(),
        }
    }
    
    /// Create request ID middleware with custom configuration
    pub fn with_config(config: RequestIdConfig) -> Self {
        Self { config }
    }
    
    /// Set custom header name for request ID
    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.config.header_name = name.into();
        self
    }
    
    /// Set request ID generation strategy
    pub fn strategy(mut self, strategy: RequestIdStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }
    
    /// Use UUID v4 strategy (default)
    pub fn uuid_v4(mut self) -> Self {
        self.config.strategy = RequestIdStrategy::UuidV4;
        self
    }
    
    /// Use UUID v1 strategy (timestamp-based)
    pub fn uuid_v1(mut self) -> Self {
        self.config.strategy = RequestIdStrategy::UuidV1;
        self
    }
    
    /// Use counter strategy (not recommended for distributed systems)
    pub fn counter(mut self) -> Self {
        self.config.strategy = RequestIdStrategy::Counter(AtomicU64::new(0));
        self
    }
    
    /// Use prefixed UUID strategy
    pub fn prefixed(mut self, prefix: impl Into<String>) -> Self {
        self.config.strategy = RequestIdStrategy::PrefixedUuid(prefix.into());
        self
    }
    
    /// Use custom ID generation function
    pub fn custom_generator(mut self, generator: fn() -> String) -> Self {
        self.config.strategy = RequestIdStrategy::Custom(generator);
        self
    }
    
    /// Override existing request ID if present
    pub fn override_existing(mut self) -> Self {
        self.config.override_existing = true;
        self
    }
    
    /// Don't add request ID to response headers
    pub fn no_response_header(mut self) -> Self {
        self.config.add_to_response = false;
        self
    }
    
    /// Disable request ID logging
    pub fn no_logging(mut self) -> Self {
        self.config.log_request_id = false;
        self
    }
    
    /// Extract or generate request ID from request
    fn get_or_generate_request_id(&self, request: &ElifRequest) -> String {
        // Check if request already has a request ID
        if !self.config.override_existing {
            if let Some(existing_id) = request.header(&self.config.header_name) {
                if let Ok(id_str) = existing_id.to_str() {
                    if !id_str.trim().is_empty() {
                        return id_str.to_string();
                    }
                }
            }
        }
        
        // Generate new request ID
        self.config.strategy.generate()
    }
    
    /// Add request ID to request headers
    fn add_request_id_to_request(&self, mut request: ElifRequest, request_id: &str) -> ElifRequest {
        let header_name = match HeaderName::from_bytes(self.config.header_name.as_bytes()) {
            Ok(name) => name,
            Err(_) => return request, // Invalid header name, skip
        };
        
        let header_value = match HeaderValue::from_str(request_id) {
            Ok(value) => value,
            Err(_) => return request, // Invalid header value, skip
        };
        
        request.headers.insert(header_name, header_value);
        request
    }
    
    /// Add request ID to response headers
    fn add_request_id_to_response(&self, response: ElifResponse, request_id: &str) -> ElifResponse {
        if !self.config.add_to_response {
            return response;
        }
        
        let header_name = match self.config.header_name.as_str() {
            "x-request-id" => "x-request-id",
            "request-id" => "request-id", 
            "x-trace-id" => "x-trace-id",
            _ => &self.config.header_name,
        };
        
        response.header(header_name, request_id).unwrap_or_else(|_| {
            // If we can't add the header for some reason, return a new response with error
            ElifResponse::internal_server_error().json_value(serde_json::json!({
                "error": {
                    "code": "internal_error",
                    "message": "Failed to add request ID to response"
                }
            }))
        })
    }
    
    /// Log request ID if enabled
    fn log_request_id(&self, request_id: &str, method: &axum::http::Method, path: &str) {
        if self.config.log_request_id {
            tracing::info!(
                request_id = request_id,
                method = %method,
                path = path,
                "Request started"
            );
        }
    }
}

impl Default for RequestIdMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for RequestIdMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        // Generate or extract request ID
        let request_id = self.get_or_generate_request_id(&request);
        let method = request.method.clone();
        let path = request.path().to_string();
        
        // Log request ID
        self.log_request_id(&request_id, &method, &path);
        
        // Add request ID to request headers
        let updated_request = self.add_request_id_to_request(request, &request_id);
        
        let config = self.config.clone();
        let request_id_clone = request_id.clone();
        
        Box::pin(async move {
            // Execute next middleware/handler
            let response = next.run(updated_request).await;
            
            // Add request ID to response headers
            let middleware = RequestIdMiddleware { config };
            middleware.add_request_id_to_response(response, &request_id_clone)
        })
    }
    
    fn name(&self) -> &'static str {
        "RequestIdMiddleware"
    }
}

/// Extension trait to easily get request ID from ElifRequest
pub trait RequestIdExt {
    /// Get the request ID from the request headers
    fn request_id(&self) -> Option<String>;
    
    /// Get the request ID with fallback header names
    fn request_id_with_fallbacks(&self) -> Option<String>;
}

impl RequestIdExt for ElifRequest {
    fn request_id(&self) -> Option<String> {
        self.header("x-request-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
    
    fn request_id_with_fallbacks(&self) -> Option<String> {
        // Try common request ID header names
        let header_names = [
            "x-request-id",
            "request-id", 
            "x-trace-id",
            "x-correlation-id",
            "x-session-id",
        ];
        
        for header_name in &header_names {
            if let Some(value) = self.header(header_name) {
                if let Ok(id_str) = value.to_str() {
                    if !id_str.trim().is_empty() {
                        return Some(id_str.to_string());
                    }
                }
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifResponse;
    use axum::http::{HeaderMap, Method, StatusCode};
    use crate::request::ElifRequest;
    
    #[test]
    fn test_request_id_strategies() {
        // UUID v4
        let uuid_strategy = RequestIdStrategy::UuidV4;
        let id1 = uuid_strategy.generate();
        let id2 = uuid_strategy.generate();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // Standard UUID length
        
        // Counter
        let counter_strategy = RequestIdStrategy::Counter(AtomicU64::new(0));
        let id1 = counter_strategy.generate();
        let id2 = counter_strategy.generate();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("req-"));
        assert!(id2.starts_with("req-"));
        
        // Prefixed UUID
        let prefixed_strategy = RequestIdStrategy::PrefixedUuid("api".to_string());
        let id = prefixed_strategy.generate();
        assert!(id.starts_with("api-"));
        assert_eq!(id.len(), 40); // "api-" + 36-char UUID
        
        // Custom
        let custom_strategy = RequestIdStrategy::Custom(|| "custom-123".to_string());
        let id = custom_strategy.generate();
        assert_eq!(id, "custom-123");
    }
    
    #[test]
    fn test_request_id_config() {
        let config = RequestIdConfig::default();
        assert_eq!(config.header_name, "x-request-id");
        assert!(!config.override_existing);
        assert!(config.add_to_response);
        assert!(config.log_request_id);
    }
    
    #[tokio::test]
    async fn test_request_id_middleware_basic() {
        let middleware = RequestIdMiddleware::new();
        
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            HeaderMap::new(),
        );
        
        let next = Next::new(|req| {
            Box::pin(async move {
                // Verify request has request ID
                assert!(req.request_id().is_some());
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Check response has request ID header
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert!(parts.headers.contains_key("x-request-id"));
    }
    
    #[tokio::test]
    async fn test_request_id_middleware_existing_id() {
        let middleware = RequestIdMiddleware::new();
        
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "existing-123".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|req| {
            Box::pin(async move {
                // Should preserve existing request ID
                assert_eq!(req.request_id(), Some("existing-123".to_string()));
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        
        // Response should have the same request ID
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert_eq!(
            parts.headers.get("x-request-id").unwrap(),
            "existing-123"
        );
    }
    
    #[tokio::test]
    async fn test_request_id_middleware_override() {
        let middleware = RequestIdMiddleware::new().override_existing();
        
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "existing-123".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|req| {
            Box::pin(async move {
                // Should have new request ID, not the existing one
                let request_id = req.request_id().unwrap();
                assert_ne!(request_id, "existing-123");
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        
        // Response should have new request ID
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        let response_id = parts.headers.get("x-request-id").unwrap().to_str().unwrap();
        assert_ne!(response_id, "existing-123");
    }
    
    #[tokio::test]
    async fn test_request_id_custom_header() {
        let middleware = RequestIdMiddleware::new().header_name("x-trace-id");
        
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            HeaderMap::new(),
        );
        
        let next = Next::new(|req| {
            Box::pin(async move {
                // Check custom header name
                assert!(req.header("x-trace-id").is_some());
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert!(parts.headers.contains_key("x-trace-id"));
    }
    
    #[tokio::test] 
    async fn test_request_id_prefixed() {
        let middleware = RequestIdMiddleware::new().prefixed("api");
        
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            HeaderMap::new(),
        );
        
        let next = Next::new(|req| {
            Box::pin(async move {
                let request_id = req.request_id().unwrap();
                assert!(request_id.starts_with("api-"));
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        let response_id = parts.headers.get("x-request-id").unwrap().to_str().unwrap();
        assert!(response_id.starts_with("api-"));
    }
    
    #[tokio::test]
    async fn test_request_id_counter() {
        let middleware = RequestIdMiddleware::new().counter();
        
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            HeaderMap::new(),
        );
        
        let next = Next::new(|req| {
            Box::pin(async move {
                let request_id = req.request_id().unwrap();
                assert!(request_id.starts_with("req-"));
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_request_id_no_response_header() {
        let middleware = RequestIdMiddleware::new().no_response_header();
        
        let request = ElifRequest::new(
            Method::GET,
            "/api/test".parse().unwrap(),
            HeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Success")
            })
        });
        
        let response = middleware.handle(request, next).await;
        
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert!(!parts.headers.contains_key("x-request-id"));
    }
    
    #[test]
    fn test_request_id_extension_trait() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "test-123".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/test".parse().unwrap(),
            headers,
        );
        
        assert_eq!(request.request_id(), Some("test-123".to_string()));
        
        // Test with fallbacks
        let mut headers = HeaderMap::new();
        headers.insert("x-trace-id", "trace-456".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/test".parse().unwrap(),
            headers,
        );
        
        assert_eq!(request.request_id_with_fallbacks(), Some("trace-456".to_string()));
    }
    
    #[tokio::test]
    async fn test_builder_pattern() {
        let middleware = RequestIdMiddleware::new()
            .header_name("x-custom-id")
            .prefixed("test")
            .override_existing()
            .no_response_header()
            .no_logging();
        
        assert_eq!(middleware.config.header_name, "x-custom-id");
        assert!(middleware.config.override_existing);
        assert!(!middleware.config.add_to_response);
        assert!(!middleware.config.log_request_id);
        assert!(matches!(middleware.config.strategy, RequestIdStrategy::PrefixedUuid(_)));
    }
}