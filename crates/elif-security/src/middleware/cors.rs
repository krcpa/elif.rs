//! CORS (Cross-Origin Resource Sharing) middleware implementation
//!
//! Provides secure cross-origin request handling with configurable policies.

use std::collections::HashSet;
use elif_http::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::{ElifRequest, ElifMethod},
    response::{ElifResponse, ElifStatusCode},
};
use crate::{SecurityError, SecurityResult};

pub use crate::config::CorsConfig;

/// CORS middleware that handles cross-origin requests
#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    /// Create new CORS middleware with configuration
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }
    
    /// Create CORS middleware with default permissive settings
    pub fn permissive() -> Self {
        Self::new(CorsConfig {
            allowed_origins: None, // Allow all origins
            allow_credentials: false,
            ..CorsConfig::default()
        })
    }
    
    /// Create CORS middleware with strict settings  
    pub fn strict() -> Self {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("https://localhost:3000".to_string());
        
        Self::new(CorsConfig {
            allowed_origins: Some(allowed_origins),
            allow_credentials: true,
            max_age: Some(300), // 5 minutes
            ..CorsConfig::default()
        })
    }
    
    /// Builder method to set allowed origins
    pub fn allow_origin(mut self, origin: &str) -> Self {
        let origins = self.config.allowed_origins.get_or_insert_with(HashSet::new);
        origins.insert(origin.to_string());
        self
    }
    
    /// Builder method to allow all origins (not recommended for production)
    pub fn allow_any_origin(mut self) -> Self {
        self.config.allowed_origins = None;
        self
    }
    
    /// Builder method to set allowed methods
    pub fn allow_methods(mut self, methods: Vec<ElifMethod>) -> Self {
        self.config.allowed_methods = methods
            .into_iter()
            .map(|m| m.to_string())
            .collect();
        self
    }
    
    /// Builder method to set allowed headers
    pub fn allow_headers(mut self, headers: Vec<&str>) -> Self {
        self.config.allowed_headers = headers
            .into_iter()
            .map(|h| h.to_lowercase())
            .collect();
        self
    }
    
    /// Builder method to expose headers
    pub fn expose_headers(mut self, headers: Vec<&str>) -> Self {
        self.config.exposed_headers = headers
            .into_iter()
            .map(|h| h.to_lowercase())
            .collect();
        self
    }
    
    /// Builder method to allow credentials
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.config.allow_credentials = allow;
        self
    }
    
    /// Builder method to set max age for preflight cache
    pub fn max_age(mut self, seconds: u32) -> Self {
        self.config.max_age = Some(seconds);
        self
    }
    
    /// Check if the request origin is allowed
    fn is_origin_allowed(&self, origin: &str) -> bool {
        match &self.config.allowed_origins {
            None => true, // Allow all origins
            Some(origins) => origins.contains(origin) || origins.contains("*"),
        }
    }
    
    /// Check if the request method is allowed
    fn is_method_allowed(&self, method: &str) -> bool {
        self.config.allowed_methods.contains(method)
    }
    
    /// Check if the request headers are allowed
    fn are_headers_allowed(&self, headers: &elif_http::response::ElifHeaderMap) -> bool {
        if let Some(requested_headers) = headers.get_str("access-control-request-headers") {
            if let Ok(requested_headers_str) = requested_headers.to_str() {
                for header in requested_headers_str.split(',') {
                    let header = header.trim().to_lowercase();
                    if !self.config.allowed_headers.contains(&header) {
                        return false;
                    }
                }
            }
        }
        true
    }
    
    /// Add CORS headers to response
    fn add_cors_headers(&self, response: &mut Response, origin: Option<&str>) -> SecurityResult<()> {
        let headers = response.headers_mut();
        
        // Add Access-Control-Allow-Origin
        if let Some(origin) = origin {
            if self.is_origin_allowed(origin) {
                if self.config.allowed_origins.is_none() || 
                   self.config.allowed_origins.as_ref().unwrap().contains("*") {
                    headers.insert(
                        "access-control-allow-origin",
                        HeaderValue::from_static("*"),
                    );
                } else {
                    headers.insert(
                        "access-control-allow-origin",
                        HeaderValue::from_str(origin).map_err(|_| SecurityError::CorsViolation {
                            message: "Invalid origin header".to_string(),
                        })?,
                    );
                }
            }
        }
        
        // Add Access-Control-Allow-Credentials
        if self.config.allow_credentials {
            headers.insert(
                "access-control-allow-credentials",
                HeaderValue::from_static("true"),
            );
        }
        
        // Add Access-Control-Expose-Headers
        if !self.config.exposed_headers.is_empty() {
            let exposed_headers = self.config.exposed_headers
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            headers.insert(
                "access-control-expose-headers",
                HeaderValue::from_str(&exposed_headers).map_err(|_| SecurityError::CorsViolation {
                    message: "Invalid exposed headers".to_string(),
                })?,
            );
        }
        
        Ok(())
    }
    
    /// Handle preflight OPTIONS request
    fn handle_preflight(&self, request: &Request) -> SecurityResult<Response> {
        let headers = request.headers();
        
        // Check origin
        let origin = headers.get("origin")
            .and_then(|v| v.to_str().ok());
        
        if let Some(origin) = origin {
            if !self.is_origin_allowed(origin) {
                return Err(SecurityError::CorsViolation {
                    message: format!("Origin '{}' not allowed", origin),
                });
            }
        }
        
        // Check method
        if let Some(method) = headers.get("access-control-request-method") {
            if let Ok(method_str) = method.to_str() {
                if !self.is_method_allowed(method_str) {
                    return Err(SecurityError::CorsViolation {
                        message: format!("Method '{}' not allowed", method_str),
                    });
                }
            }
        }
        
        // Check headers
        if !self.are_headers_allowed(headers) {
            return Err(SecurityError::CorsViolation {
                message: "Requested headers not allowed".to_string(),
            });
        }
        
        // Create preflight response
        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap();
        
        let response_headers = response.headers_mut();
        
        // Add origin
        if let Some(origin) = origin {
            response_headers.insert(
                "access-control-allow-origin",
                HeaderValue::from_str(origin).unwrap(),
            );
        }
        
        // Add allowed methods
        let methods = self.config.allowed_methods
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        response_headers.insert(
            "access-control-allow-methods",
            HeaderValue::from_str(&methods).unwrap(),
        );
        
        // Add allowed headers
        if !self.config.allowed_headers.is_empty() {
            let headers = self.config.allowed_headers
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            response_headers.insert(
                "access-control-allow-headers",
                HeaderValue::from_str(&headers).unwrap(),
            );
        }
        
        // Add max age
        if let Some(max_age) = self.config.max_age {
            response_headers.insert(
                "access-control-max-age",
                HeaderValue::from_str(&max_age.to_string()).unwrap(),
            );
        }
        
        // Add credentials
        if self.config.allow_credentials {
            response_headers.insert(
                "access-control-allow-credentials",
                HeaderValue::from_static("true"),
            );
        }
        
        Ok(response)
    }
    
    /// Handle preflight OPTIONS request for V2 middleware system
    fn handle_preflight_v2(&self, request: &ElifRequest) -> SecurityResult<ElifResponse> {
        // Check if request method is allowed for preflight
        if let Some(request_method) = request.headers.get_str("access-control-request-method") {
            if let Ok(method_str) = request_method.to_str() {
                if !self.is_method_allowed(method_str) {
                    return Err(SecurityError::CorsViolation {
                        message: format!("Method '{}' not allowed", method_str),
                    });
                }
            }
        }
        
        // Check if request headers are allowed
        if !self.are_headers_allowed(&request.headers) {
            return Err(SecurityError::CorsViolation {
                message: "Headers not allowed".to_string(),
            });
        }
        
        // Create preflight response
        let mut response = ElifResponse::with_status(ElifStatusCode::OK);
        
        // Add preflight headers
        if let Err(e) = self.add_cors_headers_v2(&mut response, 
            request.headers.get_str("origin")
                .and_then(|h| h.to_str().ok())) {
            return Err(e);
        }
        
        // Add method headers
        if !self.config.allowed_methods.is_empty() {
            let methods = self.config.allowed_methods.join(", ");
            if let Err(_) = response.add_header("access-control-allow-methods", &methods) {
                return Err(SecurityError::CorsViolation {
                    message: "Failed to add allowed methods header".to_string(),
                });
            }
        }
        
        // Add headers header
        if !self.config.allowed_headers.is_empty() {
            let headers = self.config.allowed_headers.join(", ");
            if let Err(_) = response.add_header("access-control-allow-headers", &headers) {
                return Err(SecurityError::CorsViolation {
                    message: "Failed to add allowed headers header".to_string(),
                });
            }
        }
        
        // Add max age
        if let Some(max_age) = self.config.max_age {
            if let Err(_) = response.add_header("access-control-max-age", &max_age.to_string()) {
                return Err(SecurityError::CorsViolation {
                    message: "Failed to add max age header".to_string(),
                });
            }
        }
        
        Ok(response)
    }
    
    /// Add CORS headers to response for V2 middleware system
    fn add_cors_headers_v2(&self, response: &mut ElifResponse, origin: Option<&str>) -> SecurityResult<()> {
        // Add Access-Control-Allow-Origin
        if let Some(origin_str) = origin {
            if self.is_origin_allowed(origin_str) {
                let origin_header = if self.config.allowed_origins.is_none() || 
                   self.config.allowed_origins.as_ref().unwrap().contains("*") {
                    "*"
                } else {
                    origin_str
                };
                
                response.add_header("access-control-allow-origin", origin_header)
                    .map_err(|_| SecurityError::CorsViolation {
                        message: "Failed to add origin header".to_string(),
                    })?;
            }
        }
        
        // Add Access-Control-Allow-Credentials
        if self.config.allow_credentials {
            response.add_header("access-control-allow-credentials", "true")
                .map_err(|_| SecurityError::CorsViolation {
                    message: "Failed to add credentials header".to_string(),
                })?;
        }
        
        // Add Access-Control-Expose-Headers
        if !self.config.exposed_headers.is_empty() {
            let exposed = self.config.exposed_headers.join(", ");
            response.add_header("access-control-expose-headers", &exposed)
                .map_err(|_| SecurityError::CorsViolation {
                    message: "Failed to add exposed headers".to_string(),
                })?;
        }
        
        Ok(())
    }
}

/// Extension type to store CORS origin in request
#[derive(Debug, Clone)]
struct CorsOrigin(Option<String>);

impl Middleware for CorsMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Extract origin header
            let origin = request.headers.get_str("origin")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            
            // Handle preflight OPTIONS request
            if request.method == ElifMethod::OPTIONS {
                return match self.handle_preflight_v2(&request) {
                    Ok(response) => response,
                    Err(_) => ElifResponse::with_status(ElifStatusCode::FORBIDDEN)
                        .text("CORS policy violation"),
                };
            }
            
            // Check origin for non-preflight requests
            if let Some(ref origin_str) = origin {
                if !self.is_origin_allowed(origin_str) {
                    return ElifResponse::with_status(ElifStatusCode::FORBIDDEN)
                        .text("CORS policy violation: origin not allowed");
                }
            }
            
            // Request is valid, proceed to next middleware/handler
            let mut response = next.run(request).await;
            
            // Add CORS headers to response
            if let Some(ref origin_str) = origin {
                if let Err(_) = self.add_cors_headers_v2(&mut response, Some(origin_str)) {
                    tracing::warn!("Failed to add CORS headers to response");
                }
            } else if let Err(_) = self.add_cors_headers_v2(&mut response, None) {
                tracing::warn!("Failed to add CORS headers to response");
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "CorsMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::middleware::MiddlewarePipeline;
    use axum::http::Method;
    
    #[tokio::test]
    async fn test_cors_preflight_request() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        
        // Create preflight OPTIONS request
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/")
            .header("Origin", "https://example.com")
            .header("Access-Control-Request-Method", "GET")
            .body(Body::empty())
            .unwrap();
        
        // Test preflight handling directly
        match cors.handle_preflight(&request) {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
                assert!(response.headers().get("access-control-allow-origin").is_some());
                assert!(response.headers().get("access-control-allow-methods").is_some());
            },
            Err(e) => panic!("Preflight request should succeed: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn test_cors_middleware_pipeline() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        let pipeline = MiddlewarePipeline::new().add(cors);
        
        // Test normal request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header("Origin", "https://example.com")
            .body(Body::empty())
            .unwrap();
        
        // Process request through pipeline
        let processed_request = pipeline.process_request(request).await;
        
        // Request should pass through for normal GET request
        assert!(processed_request.is_ok());
        
        // Test response processing
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        
        let processed_response = pipeline.process_response(response).await;
        
        // Response should have CORS headers added
        assert_eq!(processed_response.status(), StatusCode::OK);
        // Note: CORS headers depend on the specific implementation
    }
    
    #[tokio::test]
    async fn test_cors_preflight_in_pipeline() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        let pipeline = MiddlewarePipeline::new().add(cors);
        
        // Create preflight OPTIONS request
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/")
            .header("Origin", "https://example.com")
            .header("Access-Control-Request-Method", "GET")
            .body(Body::empty())
            .unwrap();
        
        // Process request through pipeline - should return early response
        let result = pipeline.process_request(request).await;
        
        match result {
            Err(response) => {
                // Preflight should return early response
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
                assert!(response.headers().get("access-control-allow-origin").is_some());
            },
            Ok(_) => panic!("Preflight request should return early response"),
        }
    }
    
    #[tokio::test]
    async fn test_cors_origin_not_allowed() {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("https://trusted.com".to_string());
        
        let config = CorsConfig {
            allowed_origins: Some(allowed_origins),
            ..CorsConfig::default()
        };
        
        let cors = CorsMiddleware::new(config);
        let pipeline = MiddlewarePipeline::new().add(cors);
        
        // Test request from disallowed origin
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header("Origin", "https://evil.com")
            .body(Body::empty())
            .unwrap();
        
        // Process request through pipeline
        let result = pipeline.process_request(request).await;
        
        match result {
            Err(response) => {
                // Should be rejected with 403
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
            },
            Ok(_) => panic!("Request from disallowed origin should be rejected"),
        }
    }
    
    #[tokio::test]
    async fn test_cors_builder_methods() {
        let cors = CorsMiddleware::new(CorsConfig::default())
            .allow_origin("https://example.com")
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_headers(vec!["content-type", "authorization"])
            .allow_credentials(true)
            .max_age(3600);
        
        assert!(cors.config.allowed_origins.as_ref().unwrap().contains("https://example.com"));
        assert!(cors.config.allowed_methods.contains("GET"));
        assert!(cors.config.allowed_methods.contains("POST"));
        assert!(cors.config.allowed_headers.contains("content-type"));
        assert!(cors.config.allow_credentials);
        assert_eq!(cors.config.max_age, Some(3600));
    }
    
    #[tokio::test]
    async fn test_cors_method_not_allowed() {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert("GET".to_string());
        
        let config = CorsConfig {
            allowed_methods: allowed_methods,
            ..CorsConfig::default()
        };
        
        let cors = CorsMiddleware::new(config);
        
        // Test preflight for disallowed method
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/")
            .header("Origin", "https://example.com")
            .header("Access-Control-Request-Method", "DELETE")
            .body(Body::empty())
            .unwrap();
        
        // Test preflight handling directly
        match cors.handle_preflight(&request) {
            Ok(_) => panic!("Preflight for disallowed method should fail"),
            Err(_) => {
                // Should be rejected
                // The error handling will convert this to a 403 response in the middleware
            },
        }
    }
}