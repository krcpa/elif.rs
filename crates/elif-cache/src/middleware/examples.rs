//! Usage examples for HTTP Response Caching Middleware
//!
//! This module provides examples and integration patterns for using the
//! HTTP response caching middleware with the elif.rs framework.

#[cfg(feature = "http-cache")]
pub mod usage_examples {
    use crate::{
        Cache, MemoryBackend, CacheConfig,
        middleware::response_cache::{HttpResponseCacheMiddleware, HttpCacheBuilder},
    };
    use elif_http::middleware::MiddlewarePipeline;
    use std::time::Duration;

    /// Example 1: Basic Response Caching
    /// 
    /// This example shows how to set up basic HTTP response caching
    /// with default configuration.
    #[cfg(feature = "http-cache")]
    pub fn example_basic_caching() -> HttpResponseCacheMiddleware<MemoryBackend> {
        // Create cache backend
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        // Create middleware with defaults
        HttpResponseCacheMiddleware::with_defaults(cache)
    }
    
    /// Example 2: Advanced Response Caching Configuration
    /// 
    /// This example demonstrates advanced configuration options
    /// for content negotiation and performance optimization.
    #[cfg(feature = "http-cache")]
    pub fn example_advanced_caching() -> HttpResponseCacheMiddleware<MemoryBackend> {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        HttpCacheBuilder::new()
            .default_ttl(Duration::from_secs(600)) // 10 minutes
            .max_response_size(5 * 1024 * 1024) // 5MB
            .vary_by_headers(vec![
                "Accept".to_string(),
                "Accept-Encoding".to_string(),
                "Accept-Language".to_string(),
                "Authorization".to_string(), // For per-user caching
                "User-Agent".to_string(), // For device-specific responses
            ])
            .exclude_paths(vec![
                "/admin/*".to_string(),
                "/api/auth/*".to_string(),
                "/api/user/*".to_string(), // User-specific endpoints
            ])
            .exclude_path("/health") // Health check endpoint
            .cache_private_responses() // Enable private response caching
            .build(cache)
    }
    
    /// Example 3: E-commerce Content Negotiation Caching
    /// 
    /// This example shows caching configuration optimized for
    /// e-commerce applications with heavy content negotiation.
    #[cfg(feature = "http-cache")]
    pub fn example_ecommerce_caching() -> HttpResponseCacheMiddleware<MemoryBackend> {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        HttpCacheBuilder::new()
            .default_ttl(Duration::from_secs(1800)) // 30 minutes
            .vary_by_headers(vec![
                "Accept".to_string(),
                "Accept-Language".to_string(),
                "Accept-Encoding".to_string(),
                "Accept-Currency".to_string(), // Custom header for currency
                "X-Country-Code".to_string(), // Custom header for localization
                "X-User-Segment".to_string(), // Custom header for user segmentation
            ])
            .exclude_paths(vec![
                "/checkout/*".to_string(),
                "/cart/*".to_string(),
                "/account/*".to_string(),
                "/admin/*".to_string(),
            ])
            .cache_all_responses() // Cache 404s for product pages
            .key_prefix("ecommerce:".to_string())
            .build(cache)
    }
    
    /// Example 4: API Response Caching
    /// 
    /// This example shows caching configuration for REST APIs
    /// with focus on performance and content negotiation.
    #[cfg(feature = "http-cache")]
    pub fn example_api_caching() -> HttpResponseCacheMiddleware<MemoryBackend> {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        HttpCacheBuilder::new()
            .default_ttl(Duration::from_secs(300)) // 5 minutes
            .vary_by_headers(vec![
                "Accept".to_string(), // JSON vs XML
                "Accept-Version".to_string(), // API versioning
                "Accept-Encoding".to_string(), // Compression
            ])
            .exclude_paths(vec![
                "/api/auth/*".to_string(),
                "/api/user/profile".to_string(),
                "/api/real-time/*".to_string(),
            ])
            .exclude_methods(vec![
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
            ])
            .key_prefix("api:".to_string())
            .build(cache)
    }
    
    /// Example 5: Multi-language Website Caching
    /// 
    /// This example demonstrates caching for internationalized websites
    /// with language-specific content.
    #[cfg(feature = "http-cache")]
    pub fn example_i18n_caching() -> HttpResponseCacheMiddleware<MemoryBackend> {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        HttpCacheBuilder::new()
            .default_ttl(Duration::from_secs(3600)) // 1 hour
            .vary_by_headers(vec![
                "Accept-Language".to_string(),
                "Accept-Encoding".to_string(),
                "X-Locale".to_string(), // Custom locale header
                "X-Timezone".to_string(), // Timezone for date formatting
            ])
            .exclude_paths(vec![
                "/admin/*".to_string(),
                "/user/*".to_string(),
                "/api/auth/*".to_string(),
            ])
            .key_prefix("i18n:".to_string())
            .build(cache)
    }
    
    /// Example Integration with Middleware Pipeline
    /// 
    /// This example shows how to integrate the response cache middleware
    /// into the elif-http middleware pipeline.
    #[cfg(feature = "http-cache")]
    pub fn example_middleware_integration() -> MiddlewarePipeline {
        use elif_http::middleware::{
            MiddlewarePipeline,
            logging::LoggingMiddleware,
            timing::TimingMiddleware,
        };
        
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let cache_middleware = HttpResponseCacheMiddleware::with_defaults(cache);
        
        MiddlewarePipeline::new()
            .add(LoggingMiddleware::new()) // First: log all requests
            .add(TimingMiddleware::new()) // Second: time requests
            .add(cache_middleware) // Third: check cache (before processing)
            // Response processing happens in reverse order:
            // 1. Cache middleware processes response (stores if cacheable)
            // 2. Timing middleware adds timing headers
            // 3. Logging middleware logs response
    }
    
    /// Performance Recommendations
    /// 
    /// This module contains performance recommendations and best practices
    /// for using HTTP response caching effectively.
    pub mod performance_tips {
        use super::*;
        
        /// Recommended configuration for high-traffic applications
        #[cfg(feature = "http-cache")]
        pub fn high_traffic_config() -> HttpCacheBuilder {
            HttpCacheBuilder::new()
                // Longer cache times for static content
                .default_ttl(Duration::from_secs(1800)) // 30 minutes
                
                // Larger response size limit for better compression
                .max_response_size(10 * 1024 * 1024) // 10MB
                
                // Minimal vary headers for better cache hit rates
                .vary_by_headers(vec![
                    "Accept-Encoding".to_string(), // Only compression
                ])
                
                // Exclude dynamic endpoints
                .exclude_paths(vec![
                    "/api/real-time/*".to_string(),
                    "/api/user/*".to_string(),
                    "/search".to_string(), // Search results change frequently
                ])
                
                // Custom prefix for organized cache keys
                .key_prefix("high_traffic:".to_string())
        }
        
        /// Recommended configuration for content-heavy applications
        #[cfg(feature = "http-cache")]
        pub fn content_heavy_config() -> HttpCacheBuilder {
            HttpCacheBuilder::new()
                // Very long cache times for content
                .default_ttl(Duration::from_secs(7200)) // 2 hours
                
                // Large response size for media content
                .max_response_size(50 * 1024 * 1024) // 50MB
                
                // Content negotiation headers
                .vary_by_headers(vec![
                    "Accept".to_string(),
                    "Accept-Encoding".to_string(),
                    "Accept-Language".to_string(),
                ])
                
                // Cache error responses for content that might be temporarily unavailable
                .cache_all_responses()
                
                .key_prefix("content:".to_string())
        }
    }
    
    /// Testing utilities for HTTP response caching
    pub mod testing {
        use super::*;
        use axum::{extract::Request, response::Response, http::Method};
        
        /// Create a test request with specific headers for testing caching behavior
        pub fn create_test_request(method: Method, uri: &str, headers: Vec<(&str, &str)>) -> Request {
            let mut builder = Request::builder()
                .method(method)
                .uri(uri);
                
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            
            builder.body(axum::body::Body::empty()).unwrap()
        }
        
        /// Create a test response with specific headers and body
        pub fn create_test_response(status: u16, headers: Vec<(&str, &str)>, body: &str) -> Response {
            let mut builder = Response::builder().status(status);
            
            for (name, value) in headers {
                builder = builder.header(name, value);
            }
            
            builder.body(axum::body::Body::from(body.to_string())).unwrap()
        }
        
        /// Test different content negotiation scenarios
        pub fn content_negotiation_test_cases() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
            vec![
                ("JSON API", vec![("accept", "application/json")]),
                ("XML API", vec![("accept", "application/xml")]),
                ("HTML Page", vec![("accept", "text/html")]),
                ("Compressed JSON", vec![
                    ("accept", "application/json"),
                    ("accept-encoding", "gzip, deflate")
                ]),
                ("Localized Content", vec![
                    ("accept", "text/html"),
                    ("accept-language", "en-US,en;q=0.9")
                ]),
                ("Multi-format Negotiation", vec![
                    ("accept", "application/json,application/xml;q=0.9,*/*;q=0.8"),
                    ("accept-encoding", "gzip, deflate, br"),
                    ("accept-language", "en-US,en;q=0.9,es;q=0.8")
                ]),
            ]
        }
    }
}

#[cfg(all(test, feature = "http-cache"))]
mod tests {
    use super::usage_examples::*;
    use super::usage_examples::testing::*;
    use axum::http::Method;
    
    #[test]
    fn test_basic_caching_middleware_creation() {
        let middleware = example_basic_caching();
        assert_eq!(middleware.name(), "ResponseCache");
    }
    
    #[test]
    fn test_advanced_caching_configuration() {
        let middleware = example_advanced_caching();
        assert!(middleware.config.cache_private_responses);
        assert!(!middleware.config.enable_etag); // Disabled in example
        assert_eq!(middleware.config.max_response_size, 5 * 1024 * 1024);
    }
    
    #[test]
    fn test_content_negotiation_test_cases() {
        let test_cases = content_negotiation_test_cases();
        assert!(!test_cases.is_empty());
        
        // Verify JSON API case
        let json_case = test_cases.iter().find(|(name, _)| *name == "JSON API").unwrap();
        assert_eq!(json_case.1, vec![("accept", "application/json")]);
    }
    
    #[test]
    fn test_request_creation() {
        let request = create_test_request(
            Method::GET, 
            "/api/users", 
            vec![("accept", "application/json"), ("accept-encoding", "gzip")]
        );
        
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().path(), "/api/users");
        assert!(request.headers().get("accept").is_some());
        assert!(request.headers().get("accept-encoding").is_some());
    }
    
    #[test]
    fn test_response_creation() {
        let response = create_test_response(
            200, 
            vec![("content-type", "application/json"), ("cache-control", "max-age=300")],
            r#"{"data": "test"}"#
        );
        
        assert_eq!(response.status().as_u16(), 200);
        assert!(response.headers().get("content-type").is_some());
        assert!(response.headers().get("cache-control").is_some());
    }
}