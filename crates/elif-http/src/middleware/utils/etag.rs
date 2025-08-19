//! # ETag Middleware
//!
//! Provides HTTP ETag header generation and conditional request handling.
//! Supports both strong and weak ETags for caching optimization.

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::{ElifRequest, ElifMethod};
use crate::response::{ElifResponse, ElifHeaderValue};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// ETag type
#[derive(Debug, Clone, PartialEq)]
pub enum ETagType {
    /// Strong ETag (exact match required)
    Strong(String),
    /// Weak ETag (semantic equivalence)
    Weak(String),
}

impl ETagType {
    /// Parse ETag from header value
    pub fn from_header_value(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.starts_with("W/") {
            // Weak ETag: W/"value"
            if value.len() > 3 && value.starts_with("W/\"") && value.ends_with('"') {
                let etag_value = &value[3..value.len()-1];
                Some(Self::Weak(etag_value.to_string()))
            } else {
                None
            }
        } else if value.starts_with('"') && value.ends_with('"') {
            // Strong ETag: "value"
            let etag_value = &value[1..value.len()-1];
            Some(Self::Strong(etag_value.to_string()))
        } else {
            None
        }
    }
    
    /// Format ETag for response header
    pub fn to_header_value(&self) -> String {
        match self {
            Self::Strong(value) => format!("\"{}\"", value),
            Self::Weak(value) => format!("W/\"{}\"", value),
        }
    }
    
    /// Get the ETag value (without quotes or weak prefix)
    pub fn value(&self) -> &str {
        match self {
            Self::Strong(value) | Self::Weak(value) => value,
        }
    }
    
    /// Check if this ETag matches another for conditional requests
    /// For If-None-Match, both strong and weak comparison allowed
    pub fn matches_for_if_none_match(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
    
    /// Check if this ETag matches another for If-Match (strong comparison only)
    pub fn matches_for_if_match(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Strong(a), Self::Strong(b)) => a == b,
            _ => false, // Weak ETags don't match for If-Match
        }
    }
}

/// ETag generation strategy
#[derive(Debug, Clone)]
pub enum ETagStrategy {
    /// Generate ETag from response body hash
    BodyHash,
    /// Generate weak ETag from response body hash
    WeakBodyHash,
    /// Use custom function to generate ETag
    Custom(fn(&[u8], &HeaderMap) -> Option<ETagType>),
}

impl Default for ETagStrategy {
    fn default() -> Self {
        Self::BodyHash
    }
}

/// Configuration for ETag middleware
#[derive(Debug, Clone)]
pub struct ETagConfig {
    /// Strategy for generating ETags
    pub strategy: ETagStrategy,
    /// Minimum response size to generate ETags for
    pub min_size: usize,
    /// Maximum response size to generate ETags for  
    pub max_size: usize,
    /// Content types to generate ETags for
    pub content_types: Vec<String>,
}

impl Default for ETagConfig {
    fn default() -> Self {
        Self {
            strategy: ETagStrategy::default(),
            min_size: 0,
            max_size: 10 * 1024 * 1024, // 10MB
            content_types: vec![
                "text/html".to_string(),
                "text/css".to_string(),
                "text/javascript".to_string(),
                "text/plain".to_string(),
                "application/json".to_string(),
                "application/javascript".to_string(),
                "application/xml".to_string(),
                "text/xml".to_string(),
                "image/svg+xml".to_string(),
            ],
        }
    }
}

/// Middleware for generating ETags and handling conditional requests
#[derive(Debug)]
pub struct ETagMiddleware {
    config: ETagConfig,
}

impl ETagMiddleware {
    /// Create new ETag middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: ETagConfig::default(),
        }
    }
    
    /// Create ETag middleware with custom configuration
    pub fn with_config(config: ETagConfig) -> Self {
        Self { config }
    }
    
    /// Set ETag generation strategy
    pub fn strategy(mut self, strategy: ETagStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }
    
    /// Set minimum size for ETag generation
    pub fn min_size(mut self, min_size: usize) -> Self {
        self.config.min_size = min_size;
        self
    }
    
    /// Set maximum size for ETag generation
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.config.max_size = max_size;
        self
    }
    
    /// Add content type for ETag generation
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.config.content_types.push(content_type.into());
        self
    }
    
    /// Use weak ETags (faster generation, semantic equivalence)
    pub fn weak(mut self) -> Self {
        self.config.strategy = ETagStrategy::WeakBodyHash;
        self
    }
    
    /// Check if response should have ETag generated
    fn should_generate_etag(&self, headers: &HeaderMap, body_size: usize) -> bool {
        // Check size limits
        if body_size < self.config.min_size || body_size > self.config.max_size {
            return false;
        }
        
        // Don't generate ETag if already present
        if headers.contains_key("etag") {
            return false;
        }
        
        // Check content type
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                let content_type_lower = content_type_str.to_lowercase();
                return self.config.content_types.iter().any(|ct| {
                    content_type_lower.starts_with(&ct.to_lowercase())
                });
            }
        }
        
        // Generate ETag for responses without content-type header
        true
    }
    
    /// Generate ETag for response body
    fn generate_etag(&self, body: &[u8], headers: &HeaderMap) -> Option<ETagType> {
        match &self.config.strategy {
            ETagStrategy::BodyHash => {
                let mut hasher = DefaultHasher::new();
                body.hash(&mut hasher);
                // Hash relevant headers (content-type, etc.)
                for (name, value) in headers.iter() {
                    name.as_str().hash(&mut hasher);
                    if let Ok(value_str) = value.to_str() {
                        value_str.hash(&mut hasher);
                    }
                }
                let hash = hasher.finish();
                Some(ETagType::Strong(format!("{:x}", hash)))
            }
            ETagStrategy::WeakBodyHash => {
                let mut hasher = DefaultHasher::new();
                body.hash(&mut hasher);
                let hash = hasher.finish();
                Some(ETagType::Weak(format!("{:x}", hash)))
            }
            ETagStrategy::Custom(func) => func(body, headers),
        }
    }
    
    /// Parse If-None-Match header
    fn parse_if_none_match(&self, header_value: &str) -> Vec<ETagType> {
        let mut etags = Vec::new();
        
        // Handle "*" case
        if header_value.trim() == "*" {
            return etags; // Return empty vec, will be handled specially
        }
        
        // Parse comma-separated ETags
        for etag_str in header_value.split(',') {
            if let Some(etag) = ETagType::from_header_value(etag_str) {
                etags.push(etag);
            }
        }
        
        etags
    }
    
    /// Parse If-Match header
    fn parse_if_match(&self, header_value: &str) -> Vec<ETagType> {
        let mut etags = Vec::new();
        
        // Handle "*" case
        if header_value.trim() == "*" {
            return etags; // Return empty vec, will be handled specially
        }
        
        // Parse comma-separated ETags
        for etag_str in header_value.split(',') {
            if let Some(etag) = ETagType::from_header_value(etag_str) {
                etags.push(etag);
            }
        }
        
        etags
    }
    
    /// Check If-None-Match condition
    fn check_if_none_match(&self, request_etags: &[ETagType], response_etag: &ETagType) -> bool {
        if request_etags.is_empty() {
            return true; // No condition to check
        }
        
        // If any ETag matches, condition fails (return 304)
        !request_etags.iter().any(|req_etag| {
            response_etag.matches_for_if_none_match(req_etag)
        })
    }
    
    /// Check If-Match condition
    fn check_if_match(&self, request_etags: &[ETagType], response_etag: &ETagType) -> bool {
        if request_etags.is_empty() {
            return true; // No condition to check
        }
        
        // If any ETag matches with strong comparison, condition passes
        request_etags.iter().any(|req_etag| {
            response_etag.matches_for_if_match(req_etag)
        })
    }
    
    /// Handle conditional requests and add ETag to response with extracted headers
    async fn process_response_with_headers(
        &self, 
        response: ElifResponse,
        if_none_match: Option<ElifHeaderValue>,
        if_match: Option<ElifHeaderValue>,
        request_method: ElifMethod
    ) -> ElifResponse {
        // Convert elif types to axum types for internal processing
        let axum_if_none_match = if_none_match.as_ref().map(|v| v.to_axum());
        let axum_if_match = if_match.as_ref().map(|v| v.to_axum());
        let axum_method = request_method.to_axum();
        
        let axum_response = response.into_axum_response();
        let (parts, body) = axum_response.into_parts();
        
        // Collect body bytes
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => {
                // If we can't read the body, return as-is
                let response = axum::response::Response::from_parts(parts, axum::body::Body::empty());
                return ElifResponse::from_axum_response(response).await;
            }
        };
        
        // Check if we should generate ETag
        if !self.should_generate_etag(&parts.headers, body_bytes.len()) {
            let response = axum::response::Response::from_parts(parts, axum::body::Body::from(body_bytes));
            return ElifResponse::from_axum_response(response).await;
        }
        
        // Generate ETag
        let etag = match self.generate_etag(&body_bytes, &parts.headers) {
            Some(etag) => etag,
            None => {
                // ETag generation failed, return original response
                let response = axum::response::Response::from_parts(parts, axum::body::Body::from(body_bytes));
                return ElifResponse::from_axum_response(response).await;
            }
        };
        
        // Check conditional request headers
        
        // Handle If-None-Match (typically used with GET/HEAD for caching)
        if let Some(if_none_match) = axum_if_none_match {
            if let Ok(if_none_match_str) = if_none_match.to_str() {
                let request_etags = self.parse_if_none_match(if_none_match_str);
                
                // Special case: "*" matches any ETag
                // RFC 7232: For GET/HEAD, return 304. For others, return 412 if resource exists.
                if if_none_match_str.trim() == "*" {
                    return if axum_method == &axum::http::Method::GET || axum_method == &axum::http::Method::HEAD {
                        // Return 304 Not Modified for GET/HEAD
                        ElifResponse::from_axum_response(
                            axum::response::Response::builder()
                                .status(axum::http::StatusCode::NOT_MODIFIED)
                                .header("etag", etag.to_header_value())
                                .body(axum::body::Body::empty())
                                .unwrap()
                        ).await
                    } else {
                        // Return 412 Precondition Failed for state-changing methods
                        ElifResponse::from_axum_response(
                            axum::response::Response::builder()
                                .status(axum::http::StatusCode::PRECONDITION_FAILED)
                                .header("etag", etag.to_header_value())
                                .body(axum::body::Body::from(
                                    serde_json::to_vec(&serde_json::json!({
                                        "error": {
                                            "code": "precondition_failed",
                                            "message": "If-None-Match: * failed - resource exists"
                                        }
                                    })).unwrap_or_default()
                                ))
                                .unwrap()
                        ).await
                    };
                }
                
                if !self.check_if_none_match(&request_etags, &etag) {
                    // ETag matches - behavior depends on request method
                    return if axum_method == &axum::http::Method::GET || axum_method == &axum::http::Method::HEAD {
                        // Return 304 Not Modified for GET/HEAD
                        ElifResponse::from_axum_response(
                            axum::response::Response::builder()
                                .status(axum::http::StatusCode::NOT_MODIFIED)
                                .header("etag", etag.to_header_value())
                                .body(axum::body::Body::empty())
                                .unwrap()
                        ).await
                    } else {
                        // Return 412 Precondition Failed for state-changing methods
                        ElifResponse::from_axum_response(
                            axum::response::Response::builder()
                                .status(axum::http::StatusCode::PRECONDITION_FAILED)
                                .header("etag", etag.to_header_value())
                                .body(axum::body::Body::from(
                                    serde_json::to_vec(&serde_json::json!({
                                        "error": {
                                            "code": "precondition_failed",
                                            "message": "If-None-Match precondition failed - resource unchanged"
                                        }
                                    })).unwrap_or_default()
                                ))
                                .unwrap()
                        ).await
                    };
                }
            }
        }
        
        // Handle If-Match (typically used with PUT/POST for conflict detection)
        if let Some(if_match) = axum_if_match {
            if let Ok(if_match_str) = if_match.to_str() {
                let request_etags = self.parse_if_match(if_match_str);
                
                // Special case: "*" matches if resource exists
                if if_match_str.trim() == "*" {
                    // Resource exists (we have a response), so condition passes
                } else if !self.check_if_match(&request_etags, &etag) {
                    // No ETag matches with strong comparison, return 412 Precondition Failed
                    return ElifResponse::from_axum_response(
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::PRECONDITION_FAILED)
                            .header("etag", etag.to_header_value())
                            .body(axum::body::Body::from(
                                serde_json::to_vec(&serde_json::json!({
                                    "error": {
                                        "code": "precondition_failed",
                                        "message": "Request ETag does not match current resource ETag"
                                    }
                                })).unwrap_or_default()
                            ))
                            .unwrap()
                    ).await;
                }
            }
        }
        
        // Add ETag header to successful response
        let mut new_parts = parts;
        new_parts.headers.insert(
            HeaderName::from_static("etag"),
            HeaderValue::from_str(&etag.to_header_value()).unwrap(),
        );
        
        // Add Cache-Control header if not present
        if !new_parts.headers.contains_key("cache-control") {
            new_parts.headers.insert(
                HeaderName::from_static("cache-control"),
                HeaderValue::from_static("private, max-age=0"),
            );
        }
        
        let response = axum::response::Response::from_parts(
            new_parts,
            axum::body::Body::from(body_bytes),
        );
        
        ElifResponse::from_axum_response(response).await
    }
}

impl Default for ETagMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for ETagMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let config = self.config.clone();
        
        Box::pin(async move {
            // Extract needed headers and method before moving request
            let if_none_match = request.header("if-none-match").cloned();
            let if_match = request.header("if-match").cloned();
            let method = request.method.clone();
            
            let response = next.run(request).await;
            
            // Process response to add ETag and handle conditional requests
            let middleware = ETagMiddleware { config };
            middleware.process_response_with_headers(response, if_none_match, if_match, method).await
        })
    }
    
    fn name(&self) -> &'static str {
        "ETagMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifResponse;
    use crate::request::ElifRequest;
    
    #[test]
    fn test_etag_parsing() {
        // Test strong ETag
        let etag = ETagType::from_header_value("\"abc123\"").unwrap();
        assert_eq!(etag, ETagType::Strong("abc123".to_string()));
        assert_eq!(etag.to_header_value(), "\"abc123\"");
        
        // Test weak ETag
        let etag = ETagType::from_header_value("W/\"abc123\"").unwrap();
        assert_eq!(etag, ETagType::Weak("abc123".to_string()));
        assert_eq!(etag.to_header_value(), "W/\"abc123\"");
        
        // Test invalid ETag
        assert!(ETagType::from_header_value("invalid").is_none());
        assert!(ETagType::from_header_value("\"unclosed").is_none());
    }
    
    #[test]
    fn test_etag_matching() {
        let strong1 = ETagType::Strong("abc123".to_string());
        let strong2 = ETagType::Strong("abc123".to_string());
        let strong3 = ETagType::Strong("def456".to_string());
        let weak1 = ETagType::Weak("abc123".to_string());
        
        // If-None-Match allows both strong and weak comparison
        assert!(strong1.matches_for_if_none_match(&strong2));
        assert!(strong1.matches_for_if_none_match(&weak1));
        assert!(!strong1.matches_for_if_none_match(&strong3));
        
        // If-Match requires strong comparison
        assert!(strong1.matches_for_if_match(&strong2));
        assert!(!strong1.matches_for_if_match(&weak1));
        assert!(!strong1.matches_for_if_match(&strong3));
    }
    
    #[test]
    fn test_etag_config() {
        let config = ETagConfig::default();
        assert_eq!(config.min_size, 0);
        assert_eq!(config.max_size, 10 * 1024 * 1024);
        assert!(config.content_types.contains(&"application/json".to_string()));
    }
    
    #[test]
    fn test_should_generate_etag() {
        let middleware = ETagMiddleware::new();
        
        // Test with JSON content type
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        assert!(middleware.should_generate_etag(&headers, 1024));
        
        // Test with existing ETag
        headers.insert("etag", "\"existing\"".parse().unwrap());
        assert!(!middleware.should_generate_etag(&headers, 1024));
        
        // Test with unsupported content type
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "image/jpeg".parse().unwrap());
        assert!(!middleware.should_generate_etag(&headers, 1024));
    }
    
    #[tokio::test]
    async fn test_etag_generation() {
        let middleware = ETagMiddleware::new();
        
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/api/data".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        
        // Convert to check headers
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert!(parts.headers.contains_key("etag"));
    }
    
    #[tokio::test]
    async fn test_if_none_match_304() {
        let middleware = ETagMiddleware::new();
        
        // First request to get ETag
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/api/data".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        
        let etag_header = parts.headers.get("etag").unwrap();
        let etag_value = etag_header.to_str().unwrap();
        
        // Second request with If-None-Match
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let header_name = crate::response::headers::ElifHeaderName::from_str("if-none-match").unwrap();
        let header_value = crate::response::headers::ElifHeaderValue::from_str(etag_value).unwrap();
        headers.insert(header_name, header_value);
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::NOT_MODIFIED);
    }
    
    #[tokio::test]
    async fn test_if_match_412() {
        let middleware = ETagMiddleware::new();
        
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let header_name = crate::response::headers::ElifHeaderName::from_str("if-match").unwrap();
        let header_value = crate::response::headers::ElifHeaderValue::from_str("\"non-matching-etag\"").unwrap();
        headers.insert(header_name, header_value);
        let request = ElifRequest::new(
            crate::request::ElifMethod::PUT,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Updated!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::PRECONDITION_FAILED);
    }
    
    #[tokio::test]
    async fn test_if_none_match_star_put_request() {
        let middleware = ETagMiddleware::new();
        
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let header_name = crate::response::headers::ElifHeaderName::from_str("if-none-match").unwrap();
        let header_value = crate::response::headers::ElifHeaderValue::from_str("*").unwrap();
        headers.insert(header_name, header_value);
        let request = ElifRequest::new(
            crate::request::ElifMethod::PUT,  // State-changing method
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Created!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        // Should return 412 for PUT with If-None-Match: * when resource exists
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::PRECONDITION_FAILED);
    }
    
    #[tokio::test]
    async fn test_if_none_match_star_get_request() {
        let middleware = ETagMiddleware::new();
        
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let header_name = crate::response::headers::ElifHeaderName::from_str("if-none-match").unwrap();
        let header_value = crate::response::headers::ElifHeaderValue::from_str("*").unwrap();
        headers.insert(header_name, header_value);
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,  // Safe method
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Data"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        // Should return 304 for GET with If-None-Match: *
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::NOT_MODIFIED);
    }
    
    #[tokio::test] 
    async fn test_if_none_match_etag_put_request() {
        let middleware = ETagMiddleware::new();
        
        // First request to get ETag
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/api/data".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Data"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        
        let etag_header = parts.headers.get("etag").unwrap();
        let etag_value = etag_header.to_str().unwrap();
        
        // Second request - PUT with matching ETag
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let header_name = crate::response::headers::ElifHeaderName::from_str("if-none-match").unwrap();
        let header_value = crate::response::headers::ElifHeaderValue::from_str(etag_value).unwrap();
        headers.insert(header_name, header_value);
        let request = ElifRequest::new(
            crate::request::ElifMethod::PUT,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Data"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        // Should return 412 for PUT when ETag matches
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::PRECONDITION_FAILED);
    }
    
    #[tokio::test]
    async fn test_weak_etag_strategy() {
        let middleware = ETagMiddleware::new().weak();
        
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/api/data".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        
        let etag_header = parts.headers.get("etag").unwrap();
        let etag_value = etag_header.to_str().unwrap();
        assert!(etag_value.starts_with("W/"));
    }
    
    #[test]
    fn test_etag_middleware_builder() {
        let middleware = ETagMiddleware::new()
            .min_size(1024)
            .max_size(5 * 1024 * 1024)
            .content_type("application/xml")
            .weak();
        
        assert_eq!(middleware.config.min_size, 1024);
        assert_eq!(middleware.config.max_size, 5 * 1024 * 1024);
        assert!(middleware.config.content_types.contains(&"application/xml".to_string()));
        assert!(matches!(middleware.config.strategy, ETagStrategy::WeakBodyHash));
    }
}