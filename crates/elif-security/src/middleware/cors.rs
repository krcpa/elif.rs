//! CORS (Cross-Origin Resource Sharing) middleware implementation
//!
//! Provides secure cross-origin request handling with configurable policies.

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    body::Body,
};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use tower::{Layer, Service};
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
    pub fn allow_methods(mut self, methods: Vec<Method>) -> Self {
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
    fn are_headers_allowed(&self, headers: &HeaderMap) -> bool {
        if let Some(requested_headers) = headers.get("access-control-request-headers") {
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
}

/// Tower layer for CORS middleware
#[derive(Clone)]
pub struct CorsLayer {
    middleware: CorsMiddleware,
}

impl CorsLayer {
    pub fn new(config: CorsConfig) -> Self {
        Self {
            middleware: CorsMiddleware::new(config),
        }
    }
}

impl<S> Layer<S> for CorsLayer {
    type Service = CorsService<S>;
    
    fn layer(&self, inner: S) -> Self::Service {
        CorsService {
            middleware: self.middleware.clone(),
            inner,
        }
    }
}

/// Tower service for CORS middleware
#[derive(Clone)]
pub struct CorsService<S> {
    middleware: CorsMiddleware,
    inner: S,
}

impl<S> Service<Request> for CorsService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    
    fn call(&mut self, request: Request) -> Self::Future {
        let middleware = self.middleware.clone();
        let mut inner = self.inner.clone();
        
        Box::pin(async move {
            // Handle preflight OPTIONS request
            if request.method() == Method::OPTIONS {
                match middleware.handle_preflight(&request) {
                    Ok(response) => return Ok(response),
                    Err(_) => {
                        // Return 403 Forbidden for CORS violations
                        return Ok(Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Body::from("CORS policy violation"))
                            .unwrap());
                    }
                }
            }
            
            // Get origin for later use
            let origin = request.headers().get("origin")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            
            // Check origin for non-preflight requests
            if let Some(ref origin_str) = origin {
                if !middleware.is_origin_allowed(origin_str) {
                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::from("CORS policy violation: origin not allowed"))
                        .unwrap());
                }
            }
            
            // Call the inner service
            let mut response = inner.call(request).await?;
            
            // Add CORS headers to response
            if let Err(_) = middleware.add_cors_headers(&mut response, origin.as_deref()) {
                // Log error but don't fail the request
                log::warn!("Failed to add CORS headers");
            }
            
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use axum_test::TestServer;
    use http::StatusCode;
    
    async fn hello_handler() -> &'static str {
        "Hello, World!"
    }
    
    #[tokio::test]
    async fn test_cors_preflight_request() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        let layer = CorsLayer::new(cors.config.clone());
        
        let app = Router::new()
            .route("/", get(hello_handler))
            .layer(layer);
        
        let server = TestServer::new(app).unwrap();
        
        // Test preflight request using method that creates OPTIONS request
        let response = server
            .method("OPTIONS".parse().unwrap(), "/")
            .add_header("Origin", "https://example.com")
            .add_header("Access-Control-Request-Method", "GET")
            .await;
        
        response.assert_status(StatusCode::NO_CONTENT);
        assert!(response.headers().get("access-control-allow-origin").is_some());
        assert!(response.headers().get("access-control-allow-methods").is_some());
    }
    
    #[tokio::test]
    async fn test_cors_simple_request() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        let layer = CorsLayer::new(cors.config.clone());
        
        let app = Router::new()
            .route("/", get(hello_handler))
            .layer(layer);
        
        let server = TestServer::new(app).unwrap();
        
        // Test simple request with origin
        let response = server
            .get("/")
            .add_header("Origin", "https://example.com")
            .await;
        
        response.assert_status_ok();
        response.assert_text("Hello, World!");
        assert!(response.headers().get("access-control-allow-origin").is_some());
    }
    
    #[tokio::test]
    async fn test_cors_origin_not_allowed() {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("https://trusted.com".to_string());
        
        let config = CorsConfig {
            allowed_origins: Some(allowed_origins),
            ..CorsConfig::default()
        };
        
        let layer = CorsLayer::new(config);
        
        let app = Router::new()
            .route("/", get(hello_handler))
            .layer(layer);
        
        let server = TestServer::new(app).unwrap();
        
        // Test request from disallowed origin
        let response = server
            .get("/")
            .add_header("Origin", "https://evil.com")
            .await;
        
        response.assert_status(StatusCode::FORBIDDEN);
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
            allowed_methods,
            ..CorsConfig::default()
        };
        
        let layer = CorsLayer::new(config);
        
        let app = Router::new()
            .route("/", get(hello_handler))
            .layer(layer);
        
        let server = TestServer::new(app).unwrap();
        
        // Test preflight for disallowed method
        let response = server
            .method("OPTIONS".parse().unwrap(), "/")
            .add_header("Origin", "https://example.com")
            .add_header("Access-Control-Request-Method", "DELETE")
            .await;
        
        response.assert_status(StatusCode::FORBIDDEN);
    }
}