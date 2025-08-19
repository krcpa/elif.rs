//! Rate limiting middleware implementation
//!
//! Provides configurable rate limiting to prevent abuse and ensure fair usage.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use elif_http::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::{ElifResponse, ElifStatusCode},
};
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
    fn extract_identifier(&self, request: &ElifRequest) -> Option<String> {
        match &self.config.identifier {
            RateLimitIdentifier::IpAddress => {
                // Try to get real IP from forwarded headers first
                if let Some(forwarded_for) = request.headers.get_str("x-forwarded-for") {
                    if let Ok(forwarded_str) = forwarded_for.to_str() {
                        return forwarded_str.split(',').next().map(|ip| ip.trim().to_string());
                    }
                }
                if let Some(real_ip) = request.headers.get_str("x-real-ip") {
                    if let Ok(real_ip_str) = real_ip.to_str() {
                        return Some(real_ip_str.to_string());
                    }
                }
                // Fall back to connection info (not available in this context, use placeholder)
                Some("127.0.0.1".to_string())
            }
            RateLimitIdentifier::UserId => {
                // Extract from Authorization header or custom user header
                request.headers.get_str("x-user-id")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            }
            RateLimitIdentifier::ApiKey => {
                // Extract from Authorization header or API key header
                request.headers.get_str("x-api-key")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            }
            RateLimitIdentifier::CustomHeader(header_name) => {
                request.headers.get_str(header_name)
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
    fn rate_limit_response(&self, info: &RateLimitInfo) -> ElifResponse {
        let json_body = serde_json::json!({
            "error": {
                "code": "RATE_LIMIT_EXCEEDED",
                "message": format!("Rate limit exceeded. Try again in {} seconds.", info.reset_time),
                "limit": info.limit,
                "current": info.current,
                "reset_time": info.reset_time
            }
        });
        
        let mut response = ElifResponse::with_status(ElifStatusCode::TOO_MANY_REQUESTS)
            .json_value(json_body);
        
        // Add rate limit headers
        let _ = response.add_header("X-RateLimit-Limit", &info.limit.to_string());
        let _ = response.add_header("X-RateLimit-Remaining", 
            &info.limit.saturating_sub(info.current).to_string());
        let _ = response.add_header("X-RateLimit-Reset", &info.reset_time.to_string());
        let _ = response.add_header("Retry-After", &info.reset_time.to_string());
        
        response
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let rate_limiter = self.clone();
        Box::pin(async move {
            // Check if path is exempt
            if rate_limiter.is_exempt_path(request.path()) {
                return next.run(request).await;
            }
            
            // Extract identifier
            let identifier = match rate_limiter.extract_identifier(&request) {
                Some(id) => id,
                None => {
                    // If we can't identify the request, allow it but log warning
                    tracing::warn!("Rate limiting: Could not extract identifier from request");
                    return next.run(request).await;
                }
            };
            
            // Check rate limit
            match rate_limiter.check_rate_limit(&identifier) {
                Ok(info) => {
                    if info.allowed {
                        // Continue to next middleware/handler
                        let mut response = next.run(request).await;
                        
                        // Add rate limit headers to successful responses
                        let _ = response.add_header("X-RateLimit-Limit", &info.limit.to_string());
                        let _ = response.add_header("X-RateLimit-Remaining", 
                            &info.limit.saturating_sub(info.current).to_string());
                        let _ = response.add_header("X-RateLimit-Reset", &info.reset_time.to_string());
                        
                        response
                    } else {
                        // Rate limit exceeded
                        tracing::warn!("Rate limit exceeded for identifier: {}, current: {}, limit: {}", 
                            identifier, info.current, info.limit);
                        rate_limiter.rate_limit_response(&info)
                    }
                }
                Err(err) => {
                    // Log error and allow the request
                    tracing::error!("Rate limiting check failed: {}", err);
                    next.run(request).await
                }
            }
        })
    }

    fn name(&self) -> &'static str {
        "RateLimitMiddleware"
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