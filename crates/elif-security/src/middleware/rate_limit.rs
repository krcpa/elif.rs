//! Rate limiting middleware implementation
//!
//! Provides configurable rate limiting to prevent abuse and ensure fair usage.

use axum::{
    extract::Request,
    http::{StatusCode, HeaderValue},
    response::Response,
    body::Body,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use elif_http::middleware::{Middleware, BoxFuture};
use crate::SecurityError;

pub use crate::config::{RateLimitConfig, RateLimitIdentifier};

/// Rate limiting middleware that tracks requests and enforces limits
#[derive(Debug)]
pub struct RateLimitMiddleware {
    config: RateLimitConfig,
    storage: Arc<Mutex<InMemoryStorage>>,
}

impl Clone for RateLimitMiddleware {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            storage: Arc::clone(&self.storage),
        }
    }
}

/// In-memory storage for rate limiting data
#[derive(Debug, Default)]
struct InMemoryStorage {
    /// Maps identifier -> (request_count, window_start_time)
    counters: HashMap<String, (u32, u64)>,
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Current request count in window
    pub current: u32,
    /// Maximum allowed requests
    pub limit: u32,
    /// Seconds remaining in current window
    pub reset_time: u64,
    /// Whether the request should be allowed
    pub allowed: bool,
}

impl RateLimitMiddleware {
    /// Create new rate limiting middleware with configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            storage: Arc::new(Mutex::new(InMemoryStorage::default())),
        }
    }
    
    /// Create middleware with default configuration (100 requests per minute by IP)
    pub fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
    
    /// Create strict rate limiting (10 requests per minute by IP)
    pub fn strict() -> Self {
        let config = RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
            identifier: RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        };
        Self::new(config)
    }
    
    /// Create permissive rate limiting (1000 requests per minute by IP)
    pub fn permissive() -> Self {
        let config = RateLimitConfig {
            max_requests: 1000,
            window_seconds: 60,
            identifier: RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        };
        Self::new(config)
    }
    
    /// Extract identifier from request based on configuration
    fn extract_identifier(&self, request: &Request) -> Option<String> {
        match &self.config.identifier {
            RateLimitIdentifier::IpAddress => {
                // Try to get real IP from forwarded headers first
                if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
                    if let Ok(forwarded_str) = forwarded_for.to_str() {
                        return forwarded_str.split(',').next().map(|ip| ip.trim().to_string());
                    }
                }
                if let Some(real_ip) = request.headers().get("x-real-ip") {
                    if let Ok(real_ip_str) = real_ip.to_str() {
                        return Some(real_ip_str.to_string());
                    }
                }
                // Fall back to connection info (not available in this context, use placeholder)
                Some("127.0.0.1".to_string())
            }
            RateLimitIdentifier::UserId => {
                // Extract from Authorization header or custom user header
                request.headers().get("x-user-id")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            }
            RateLimitIdentifier::ApiKey => {
                // Extract from Authorization header or API key header
                request.headers().get("x-api-key")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            }
            RateLimitIdentifier::CustomHeader(header_name) => {
                request.headers().get(header_name)
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            }
        }
    }
    
    /// Check if path is exempt from rate limiting
    fn is_exempt_path(&self, path: &str) -> bool {
        self.config.exempt_paths.iter().any(|exempt_path| {
            // Support glob-style matching
            if exempt_path.ends_with('*') {
                let prefix = &exempt_path[..exempt_path.len() - 1];
                path.starts_with(prefix)
            } else {
                path == exempt_path
            }
        })
    }
    
    /// Check rate limit for identifier and update counters
    fn check_rate_limit(&self, identifier: &str) -> Result<RateLimitInfo, SecurityError> {
        let mut storage = self.storage.lock().map_err(|_| {
            SecurityError::RateLimitError("Failed to acquire storage lock".to_string())
        })?;
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SecurityError::RateLimitError("Failed to get current time".to_string()))?
            .as_secs();
        
        let window_size = self.config.window_seconds as u64;
        let max_requests = self.config.max_requests;
        
        // Get or create counter entry
        let (count, window_start) = storage.counters
            .get(identifier)
            .copied()
            .unwrap_or((0, current_time));
        
        // Check if we're in a new window
        let time_since_window_start = current_time.saturating_sub(window_start);
        
        let (new_count, new_window_start, reset_time) = if time_since_window_start >= window_size {
            // New window - reset counter
            (1u32, current_time, current_time + window_size)
        } else {
            // Same window - increment counter
            let remaining_window = window_size - time_since_window_start;
            (count + 1, window_start, window_start + window_size)
        };
        
        // Update storage
        storage.counters.insert(identifier.to_string(), (new_count, new_window_start));
        
        // Clean up old entries (simple cleanup strategy)
        if storage.counters.len() > 10000 {
            storage.counters.retain(|_, &mut (_, start)| {
                current_time.saturating_sub(start) < window_size * 2
            });
        }
        
        Ok(RateLimitInfo {
            current: new_count,
            limit: max_requests,
            reset_time: reset_time.saturating_sub(current_time),
            allowed: new_count <= max_requests,
        })
    }
    
    /// Create rate limit exceeded response
    fn rate_limit_response(&self, info: &RateLimitInfo) -> Response {
        let mut response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(Body::from(format!(
                r#"{{"error":{{"code":"RATE_LIMIT_EXCEEDED","message":"Rate limit exceeded. Try again in {} seconds.","limit":{},"current":{},"reset_time":{}}}}}"#,
                info.reset_time, info.limit, info.current, info.reset_time
            )))
            .unwrap();
        
        // Add rate limit headers
        let headers = response.headers_mut();
        headers.insert("X-RateLimit-Limit", HeaderValue::from(info.limit));
        headers.insert("X-RateLimit-Remaining", 
            HeaderValue::from(info.limit.saturating_sub(info.current)));
        headers.insert("X-RateLimit-Reset", HeaderValue::from(info.reset_time));
        headers.insert("Retry-After", HeaderValue::from(info.reset_time));
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        response
    }
}

impl Middleware for RateLimitMiddleware {
    fn process_request<'a>(
        &'a self,
        request: Request,
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            // Check if path is exempt
            if self.is_exempt_path(request.uri().path()) {
                return Ok(request);
            }
            
            // Extract identifier
            let identifier = match self.extract_identifier(&request) {
                Some(id) => id,
                None => {
                    // If we can't identify the request, allow it but log warning
                    // Log warning using print for now - can be upgraded to proper logging later
                    eprintln!("Rate limiting: Could not extract identifier from request");
                    return Ok(request);
                }
            };
            
            // Check rate limit
            match self.check_rate_limit(&identifier) {
                Ok(info) => {
                    if info.allowed {
                        // Add rate limit headers to successful requests
                        let mut request = request;
                        if let Ok(remaining) = HeaderValue::try_from(info.limit.saturating_sub(info.current)) {
                            request.headers_mut().insert("X-RateLimit-Remaining", remaining);
                        }
                        Ok(request)
                    } else {
                        // Rate limit exceeded
                        // Log warning using print for now - can be upgraded to proper logging later
                        eprintln!("Rate limit exceeded for identifier: {}, current: {}, limit: {}", 
                            identifier, info.current, info.limit);
                        Err(self.rate_limit_response(&info))
                    }
                }
                Err(err) => {
                    // Log error using print for now - can be upgraded to proper logging later
                    eprintln!("Rate limiting check failed: {}", err);
                    // On error, allow the request but log the error
                    Ok(request)
                }
            }
        })
    }
    
    fn process_response<'a>(
        &'a self,
        mut response: Response,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Add rate limit headers to response if not already present
            let headers = response.headers_mut();
            if !headers.contains_key("X-RateLimit-Limit") {
                headers.insert("X-RateLimit-Limit", HeaderValue::from(self.config.max_requests));
            }
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "RateLimit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[tokio::test]
    async fn test_rate_limit_middleware_basic() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 60,
            identifier: RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        };
        
        let middleware = RateLimitMiddleware::new(config);
        
        // First request should be allowed
        let request1 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result1 = middleware.process_request(request1).await;
        assert!(result1.is_ok());
        
        // Second request should be allowed
        let request2 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result2 = middleware.process_request(request2).await;
        assert!(result2.is_ok());
        
        // Third request should be rate limited
        let request3 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result3 = middleware.process_request(request3).await;
        assert!(result3.is_err());
        
        // Check response is rate limit error
        if let Err(response) = result3 {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            assert!(response.headers().contains_key("X-RateLimit-Limit"));
            assert!(response.headers().contains_key("Retry-After"));
        }
    }
    
    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let middleware = RateLimitMiddleware::strict(); // 10 requests per minute
        
        // Request from first IP
        let request1 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result1 = middleware.process_request(request1).await;
        assert!(result1.is_ok());
        
        // Request from different IP should be allowed
        let request2 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.2")
            .body(Body::empty())
            .unwrap();
        
        let result2 = middleware.process_request(request2).await;
        assert!(result2.is_ok());
    }
    
    #[tokio::test]
    async fn test_rate_limit_exempt_paths() {
        let mut exempt_paths = std::collections::HashSet::new();
        exempt_paths.insert("/health".to_string());
        exempt_paths.insert("/api/v1/public/*".to_string());
        
        let config = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
            identifier: RateLimitIdentifier::IpAddress,
            exempt_paths,
        };
        
        let middleware = RateLimitMiddleware::new(config);
        
        // Health check should be exempt
        let health_request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result = middleware.process_request(health_request).await;
        assert!(result.is_ok());
        
        // Public API should be exempt (wildcard match)
        let public_request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/public/status")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result = middleware.process_request(public_request).await;
        assert!(result.is_ok());
        
        // Regular API should be rate limited after using up quota
        let api_request1 = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result1 = middleware.process_request(api_request1).await;
        assert!(result1.is_ok());
        
        // Second request should be rate limited
        let api_request2 = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/users")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result2 = middleware.process_request(api_request2).await;
        assert!(result2.is_err());
    }
    
    #[tokio::test]
    async fn test_rate_limit_user_id_identifier() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 60,
            identifier: RateLimitIdentifier::UserId,
            exempt_paths: std::collections::HashSet::new(),
        };
        
        let middleware = RateLimitMiddleware::new(config);
        
        // First request with user ID
        let request1 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-user-id", "user123")
            .body(Body::empty())
            .unwrap();
        
        let result1 = middleware.process_request(request1).await;
        assert!(result1.is_ok());
        
        // Second request with same user ID
        let request2 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-user-id", "user123")
            .body(Body::empty())
            .unwrap();
        
        let result2 = middleware.process_request(request2).await;
        assert!(result2.is_ok());
        
        // Third request should be rate limited
        let request3 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-user-id", "user123")
            .body(Body::empty())
            .unwrap();
        
        let result3 = middleware.process_request(request3).await;
        assert!(result3.is_err());
    }
    
    #[tokio::test]
    async fn test_rate_limit_middleware_name() {
        let middleware = RateLimitMiddleware::default();
        assert_eq!(middleware.name(), "RateLimit");
    }
    
    #[tokio::test]
    async fn test_rate_limit_response_headers() {
        let config = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
            identifier: RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        };
        
        let middleware = RateLimitMiddleware::new(config);
        
        // Use up the quota
        let request1 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let _result1 = middleware.process_request(request1).await;
        
        // Second request should return rate limit response with headers
        let request2 = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();
        
        let result2 = middleware.process_request(request2).await;
        assert!(result2.is_err());
        
        if let Err(response) = result2 {
            let headers = response.headers();
            assert!(headers.contains_key("X-RateLimit-Limit"));
            assert!(headers.contains_key("X-RateLimit-Remaining"));
            assert!(headers.contains_key("X-RateLimit-Reset"));
            assert!(headers.contains_key("Retry-After"));
            assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        }
    }
}