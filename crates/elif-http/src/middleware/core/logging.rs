//! # Logging Middleware
//!
//! HTTP request/response logging middleware for observability.

use std::time::Instant;
use log::{info, debug, warn, error};
use crate::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
};

/// HTTP request logging middleware that logs request details and response status
#[derive(Debug)]
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
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let log_response_headers = self.log_response_headers;
        Box::pin(async move {
            // Store start time
            let start_time = Instant::now();
            
            // Log basic request info
            info!("→ {} {}", 
                request.method, 
                request.uri.path()
            );
            
            // Log headers (excluding sensitive ones)
            debug!("Request headers:");
            for name in request.headers.keys() {
                if !is_sensitive_header(name.as_str()) {
                    if let Some(value) = request.headers.get_str(name.as_str()) {
                        if let Ok(value_str) = value.to_str() {
                            debug!("  {}: {}", name, value_str);
                        }
                    }
                }
            }
            
            // Continue to next middleware/handler
            let response = next.run(request).await;
            
            // Calculate duration
            let duration_ms = start_time.elapsed().as_millis();
            
            // Log response info
            let status = response.status_code();
            if status.is_success() {
                info!("← {:?} {}ms", status, duration_ms);
            } else if status.is_redirection() {
                info!("← {:?} {}ms (Redirect)", status, duration_ms);
            } else if status.is_client_error() {
                warn!("← {:?} {}ms (Client Error)", status, duration_ms);
            } else if status.is_server_error() {
                error!("← {:?} {}ms (Server Error)", status, duration_ms);
            } else {
                info!("← {:?} {}ms (Informational)", status, duration_ms);
            }
            
            // Log response headers if enabled
            if log_response_headers {
                debug!("Response headers:");
                for (name, value) in response.headers().iter() {
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
    use crate::middleware::v2::MiddlewarePipelineV2;
    use crate::request::{ElifRequest, ElifMethod};
    use crate::response::headers::ElifHeaderMap;
    use crate::response::{ElifResponse, ElifStatusCode};
    
    #[test]
    fn test_sensitive_header_detection() {
        assert!(is_sensitive_header("Authorization"));
        assert!(is_sensitive_header("cookie"));
        assert!(is_sensitive_header("X-API-Key"));
        assert!(!is_sensitive_header("Content-Type"));
        assert!(!is_sensitive_header("User-Agent"));
    }
    
    #[tokio::test]
    async fn test_logging_middleware_v2() {
        let middleware = LoggingMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("content-type".parse().unwrap(), "application/json".parse().unwrap());
        headers.insert("authorization".parse().unwrap(), "Bearer secret".parse().unwrap());
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            headers,
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Success"
                }))
            })
        }).await;
        
        // Should complete successfully
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }
    
    #[test]
    fn test_logging_middleware_builder() {
        let middleware = LoggingMiddleware::new()
            .with_body_logging()
            .with_response_headers();
        
        assert!(middleware.log_body);
        assert!(middleware.log_response_headers);
    }
    
    #[tokio::test]
    async fn test_logging_different_status_codes() {
        let middleware = LoggingMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let headers = ElifHeaderMap::new();
        
        // Test success status (2xx)
        let request = ElifRequest::new(ElifMethod::GET, "/success".parse().unwrap(), headers.clone());
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move { ElifResponse::ok().text("Success") })
        }).await;
        assert!(response.status_code().is_success());
        
        // Test redirect status (3xx)
        let request = ElifRequest::new(ElifMethod::GET, "/redirect".parse().unwrap(), headers.clone());
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move { ElifResponse::with_status(ElifStatusCode::FOUND).text("Redirect") })
        }).await;
        assert!(response.status_code().is_redirection());
        
        // Test client error (4xx)
        let request = ElifRequest::new(ElifMethod::GET, "/client-error".parse().unwrap(), headers.clone());
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move { ElifResponse::with_status(ElifStatusCode::NOT_FOUND).text("Not Found") })
        }).await;
        assert!(response.status_code().is_client_error());
        
        // Test server error (5xx)
        let request = ElifRequest::new(ElifMethod::GET, "/server-error".parse().unwrap(), headers);
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move { ElifResponse::with_status(ElifStatusCode::INTERNAL_SERVER_ERROR).text("Error") })
        }).await;
        assert!(response.status_code().is_server_error());
    }
}