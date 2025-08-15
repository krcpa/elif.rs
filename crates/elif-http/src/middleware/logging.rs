//! # Logging Middleware
//!
//! HTTP request/response logging middleware for observability.

use std::time::Instant;
use axum::{
    extract::Request,
    response::Response,
    http::{Method, Uri},
};
use log::{info, debug, error};

use super::{Middleware, BoxFuture};

/// HTTP request logging middleware that logs request details and response status
pub struct LoggingMiddleware {
    /// Whether to log request body (careful with sensitive data)
    log_body: bool,
    /// Whether to log response headers
    log_response_headers: bool,
}

impl LoggingMiddleware {
    /// Create new logging middleware with default settings
    pub fn new() -> Self {
        Self {
            log_body: false,
            log_response_headers: false,
        }
    }
    
    /// Enable request body logging (use with caution for sensitive data)
    pub fn with_body_logging(mut self) -> Self {
        self.log_body = true;
        self
    }
    
    /// Enable response headers logging
    pub fn with_response_headers(mut self) -> Self {
        self.log_response_headers = true;
        self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggingMiddleware {
    fn process_request<'a>(
        &'a self, 
        request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            let method = request.method();
            let uri = request.uri();
            let headers = request.headers();
            
            // Log basic request info
            info!("→ {} {} HTTP/{:?}", 
                method, 
                uri.path_and_query().map_or("/", |p| p.as_str()),
                request.version()
            );
            
            // Log headers (excluding sensitive ones)
            debug!("Request headers:");
            for (name, value) in headers.iter() {
                // Skip sensitive headers
                if !is_sensitive_header(name.as_str()) {
                    if let Ok(value_str) = value.to_str() {
                        debug!("  {}: {}", name, value_str);
                    }
                }
            }
            
            // Store start time for response logging
            let start_time = Instant::now();
            
            // Add start time to request extensions for response logging
            let mut request = request;
            request.extensions_mut().insert(start_time);
            
            Ok(request)
        })
    }
    
    fn process_response<'a>(
        &'a self, 
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let status = response.status();
            let headers = response.headers();
            
            // Try to get request start time from extensions
            // Note: In a real implementation, we'd need better state management
            let duration_ms = 100; // Placeholder - would calculate from stored start time
            
            // Log response info
            if status.is_success() {
                info!("← {} {}ms", status, duration_ms);
            } else if status.is_client_error() {
                error!("← {} {}ms (Client Error)", status, duration_ms);
            } else if status.is_server_error() {
                error!("← {} {}ms (Server Error)", status, duration_ms);
            } else {
                info!("← {} {}ms", status, duration_ms);
            }
            
            // Log response headers if enabled
            if self.log_response_headers {
                debug!("Response headers:");
                for (name, value) in headers.iter() {
                    if let Ok(value_str) = value.to_str() {
                        debug!("  {}: {}", name, value_str);
                    }
                }
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}

/// Check if a header name is sensitive and should not be logged
fn is_sensitive_header(name: &str) -> bool {
    let sensitive_headers = [
        "authorization",
        "cookie",
        "set-cookie", 
        "x-api-key",
        "x-auth-token",
        "bearer",
    ];
    
    let name_lower = name.to_lowercase();
    sensitive_headers.iter().any(|&sensitive| {
        name_lower.contains(sensitive)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{StatusCode, Method, HeaderName, HeaderValue};
    
    #[test]
    fn test_sensitive_header_detection() {
        assert!(is_sensitive_header("Authorization"));
        assert!(is_sensitive_header("cookie"));
        assert!(is_sensitive_header("X-API-Key"));
        assert!(!is_sensitive_header("Content-Type"));
        assert!(!is_sensitive_header("User-Agent"));
    }
    
    #[tokio::test]
    async fn test_logging_middleware_request() {
        let middleware = LoggingMiddleware::new();
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/test")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer secret")
            .body(axum::body::Body::empty())
            .unwrap();
        
        let result = middleware.process_request(request).await;
        
        assert!(result.is_ok());
        let processed_request = result.unwrap();
        
        // Should have start time in extensions
        assert!(processed_request.extensions().get::<Instant>().is_some());
        
        // Original headers should be preserved
        assert_eq!(
            processed_request.headers().get("Content-Type").unwrap(),
            "application/json"
        );
    }
    
    #[tokio::test]
    async fn test_logging_middleware_response() {
        let middleware = LoggingMiddleware::new();
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();
        
        let processed_response = middleware.process_response(response).await;
        
        // Response should be unchanged
        assert_eq!(processed_response.status(), StatusCode::OK);
        assert_eq!(
            processed_response.headers().get("Content-Type").unwrap(),
            "application/json"
        );
    }
}