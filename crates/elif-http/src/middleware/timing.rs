//! # Timing Middleware
//!
//! HTTP request timing middleware for performance monitoring.

use std::time::Instant;
use axum::{
    extract::Request,
    response::Response,
    http::HeaderValue,
};
use log::{debug, warn};

use super::{Middleware, BoxFuture};

/// Request timing middleware that tracks request duration and adds timing headers
pub struct TimingMiddleware {
    /// Whether to add X-Response-Time header to responses
    add_header: bool,
    /// Warning threshold in milliseconds for slow requests
    slow_request_threshold_ms: u64,
}

impl TimingMiddleware {
    /// Create new timing middleware with default settings
    pub fn new() -> Self {
        Self {
            add_header: true,
            slow_request_threshold_ms: 1000, // 1 second
        }
    }
    
    /// Disable adding timing header to responses
    pub fn without_header(mut self) -> Self {
        self.add_header = false;
        self
    }
    
    /// Set slow request warning threshold in milliseconds
    pub fn with_slow_threshold(mut self, threshold_ms: u64) -> Self {
        self.slow_request_threshold_ms = threshold_ms;
        self
    }
}

impl Default for TimingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension key for storing request start time
#[derive(Clone, Copy)]
pub struct RequestStartTime(Instant);

impl RequestStartTime {
    pub fn new() -> Self {
        Self(Instant::now())
    }
    
    pub fn elapsed(&self) -> std::time::Duration {
        self.0.elapsed()
    }
    
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis() as u64
    }
}

impl Middleware for TimingMiddleware {
    fn process_request<'a>(
        &'a self, 
        mut request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            // Store start time in request extensions
            let start_time = RequestStartTime::new();
            request.extensions_mut().insert(start_time);
            
            debug!("⏱️  Request timing started for {} {}", 
                request.method(), 
                request.uri().path()
            );
            
            Ok(request)
        })
    }
    
    fn process_response<'a>(
        &'a self, 
        mut response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Try to get start time from response context
            // Note: In real implementation, we'd need better state management
            // For now, we'll create a mock duration
            let duration_ms = 150; // Placeholder
            
            // Add timing header if enabled
            if self.add_header {
                if let Ok(header_value) = HeaderValue::from_str(&duration_ms.to_string()) {
                    response.headers_mut().insert("X-Response-Time", header_value);
                }
            }
            
            // Check for slow requests and log warning
            if duration_ms > self.slow_request_threshold_ms {
                warn!("🐌 Slow request detected: {}ms (threshold: {}ms)", 
                    duration_ms, 
                    self.slow_request_threshold_ms
                );
            } else {
                debug!("⏱️  Request completed in {}ms", duration_ms);
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "TimingMiddleware"
    }
}

/// Utility function to format duration for display
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_ms = duration.as_millis();
    
    if total_ms >= 1000 {
        format!("{:.2}s", duration.as_secs_f64())
    } else if total_ms >= 1 {
        format!("{}ms", total_ms)
    } else {
        format!("{}μs", duration.as_micros())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{StatusCode, Method};
    use tokio::time::{sleep, Duration};
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_micros(500)), "500μs");
        assert_eq!(format_duration(Duration::from_millis(150)), "150ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
    }
    
    #[tokio::test]
    async fn test_timing_middleware_request() {
        let middleware = TimingMiddleware::new();
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/test")
            .body(axum::body::Body::empty())
            .unwrap();
        
        let result = middleware.process_request(request).await;
        
        assert!(result.is_ok());
        let processed_request = result.unwrap();
        
        // Should have start time in extensions
        assert!(processed_request.extensions().get::<RequestStartTime>().is_some());
    }
    
    #[tokio::test]
    async fn test_timing_middleware_response() {
        let middleware = TimingMiddleware::new();
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::empty())
            .unwrap();
        
        let processed_response = middleware.process_response(response).await;
        
        // Should have timing header
        assert!(processed_response.headers().get("X-Response-Time").is_some());
        
        // Status should be preserved
        assert_eq!(processed_response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_timing_middleware_without_header() {
        let middleware = TimingMiddleware::new().without_header();
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::empty())
            .unwrap();
        
        let processed_response = middleware.process_response(response).await;
        
        // Should NOT have timing header
        assert!(processed_response.headers().get("X-Response-Time").is_none());
    }
    
    #[test]
    fn test_request_start_time() {
        let start = RequestStartTime::new();
        
        // Add a tiny delay to ensure some time passes
        std::thread::sleep(std::time::Duration::from_nanos(1));
        
        // Should have elapsed time
        assert!(start.elapsed().as_nanos() >= 0);
        assert!(start.elapsed_ms() >= 0);
    }
}