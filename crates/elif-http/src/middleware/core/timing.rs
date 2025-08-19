//! # Timing Middleware
//!
//! HTTP request timing middleware for performance monitoring.

use std::time::Instant;
use log::{debug, warn};
use crate::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
};

/// Request timing middleware that tracks request duration and adds timing headers
#[derive(Debug)]
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
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let add_header = self.add_header;
        let slow_threshold = self.slow_request_threshold_ms;
        
        Box::pin(async move {
            // Store start time
            let start_time = Instant::now();
            
            debug!("⏱️  Request timing started for {} {}", 
                request.method, 
                request.uri.path()
            );
            
            // Continue to next middleware/handler
            let mut response = next.run(request).await;
            
            // Calculate duration
            let duration = start_time.elapsed();
            let duration_ms = duration.as_millis() as u64;
            
            // Add timing header if enabled
            if add_header {
                if let Err(e) = response.add_header("X-Response-Time", &duration_ms.to_string()) {
                    warn!("Failed to add X-Response-Time header: {}", e);
                }
            }
            
            // Check for slow requests and log warning
            if duration_ms > slow_threshold {
                warn!("🐌 Slow request detected: {}ms (threshold: {}ms)", 
                    duration_ms, 
                    slow_threshold
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
    use crate::middleware::v2::MiddlewarePipelineV2;
    use crate::request::{ElifRequest, ElifMethod};
    use crate::response::headers::ElifHeaderMap;
    use crate::response::{ElifResponse, ElifStatusCode};
    use tokio::time::Duration;
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_micros(500)), "500μs");
        assert_eq!(format_duration(Duration::from_millis(150)), "150ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
    }
    
    #[tokio::test]
    async fn test_timing_middleware_v2() {
        let middleware = TimingMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let headers = ElifHeaderMap::new();
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            headers,
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                // Add small delay to test timing
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                ElifResponse::ok().text("Success")
            })
        }).await;
        
        // Should complete successfully and have the timing header
        assert_eq!(response.status_code(), ElifStatusCode::OK);
        assert!(response.has_header("x-response-time"));
    }
    
    #[tokio::test]
    async fn test_timing_middleware_without_header() {
        let middleware = TimingMiddleware::new().without_header();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Success")
            })
        }).await;
        
        // Should NOT have timing header
        assert!(!response.has_header("x-response-time"));
    }
    
    #[test]
    fn test_request_start_time() {
        let start = RequestStartTime::new();
        
        // Add a tiny delay to ensure some time passes
        std::thread::sleep(std::time::Duration::from_millis(1000));
        
        // Should have elapsed time
        assert!(start.elapsed().as_nanos() > 0);
        assert!(start.elapsed_ms() > 0);
    }
    
    #[test]
    fn test_timing_middleware_builder() {
        let middleware = TimingMiddleware::new()
            .with_slow_threshold(500)
            .without_header();
        
        assert_eq!(middleware.slow_request_threshold_ms, 500);
        assert!(!middleware.add_header);
    }
}