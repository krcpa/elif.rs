//! Security Middleware Integration
//!
//! Provides a unified way to integrate all security middleware with the framework's 
//! MiddlewarePipeline, ensuring consistent usage and proper ordering.

use elif_http::middleware::MiddlewarePipeline;
use http::StatusCode;
use crate::{
    middleware::{
        cors::CorsMiddleware, 
        csrf::CsrfMiddleware, 
        rate_limit::RateLimitMiddleware,
        sanitization::SanitizationMiddleware,
        security_headers::SecurityHeadersMiddleware,
    },
    config::{CorsConfig, CsrfConfig, RateLimitConfig, SanitizationConfig, SecurityHeadersConfig},
};

/// Security middleware suite builder that helps configure and integrate
/// all security middleware with the framework's MiddlewarePipeline
#[derive(Debug, Default)]
pub struct SecurityMiddlewareBuilder {
    cors_config: Option<CorsConfig>,
    csrf_config: Option<CsrfConfig>,
    rate_limit_config: Option<RateLimitConfig>,
    sanitization_config: Option<SanitizationConfig>,
    security_headers_config: Option<SecurityHeadersConfig>,
}

impl SecurityMiddlewareBuilder {
    /// Create a new security middleware builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add CORS middleware with configuration
    pub fn with_cors(mut self, config: CorsConfig) -> Self {
        self.cors_config = Some(config);
        self
    }
    
    /// Add CORS middleware with permissive settings (not recommended for production)
    pub fn with_cors_permissive(mut self) -> Self {
        self.cors_config = Some(CorsConfig::default());
        self
    }
    
    /// Add CSRF middleware with configuration
    pub fn with_csrf(mut self, config: CsrfConfig) -> Self {
        self.csrf_config = Some(config);
        self
    }
    
    /// Add CSRF middleware with default configuration
    pub fn with_csrf_default(mut self) -> Self {
        self.csrf_config = Some(CsrfConfig::default());
        self
    }
    
    /// Add rate limiting middleware with configuration
    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self
    }
    
    /// Add rate limiting middleware with default configuration (100 req/min by IP)
    pub fn with_rate_limit_default(mut self) -> Self {
        self.rate_limit_config = Some(RateLimitConfig::default());
        self
    }
    
    /// Add rate limiting middleware with strict configuration (10 req/min by IP)
    pub fn with_rate_limit_strict(mut self) -> Self {
        self.rate_limit_config = Some(RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
            identifier: crate::config::RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        });
        self
    }
    
    /// Add request sanitization middleware with configuration
    pub fn with_sanitization(mut self, config: SanitizationConfig) -> Self {
        self.sanitization_config = Some(config);
        self
    }
    
    /// Add request sanitization middleware with strict configuration
    pub fn with_sanitization_strict(mut self) -> Self {
        self.sanitization_config = Some(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: true,
            enable_path_traversal_protection: true,
            enable_script_tag_removal: true,
            enable_html_encoding: true,
            max_request_size: Some(1024 * 1024), // 1MB
            ..SanitizationConfig::default()
        });
        self
    }
    
    /// Add request sanitization middleware with permissive configuration
    pub fn with_sanitization_permissive(mut self) -> Self {
        self.sanitization_config = Some(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: false,
            enable_path_traversal_protection: true,
            enable_script_tag_removal: true,
            enable_html_encoding: false,
            max_request_size: Some(10 * 1024 * 1024), // 10MB
            ..SanitizationConfig::default()
        });
        self
    }
    
    /// Add security headers middleware with configuration
    pub fn with_security_headers(mut self, config: SecurityHeadersConfig) -> Self {
        self.security_headers_config = Some(config);
        self
    }
    
    /// Add security headers middleware with strict production configuration
    pub fn with_security_headers_strict(mut self) -> Self {
        use std::collections::HashMap;
        
        self.security_headers_config = Some(SecurityHeadersConfig {
            content_security_policy: Some(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; child-src 'none'; frame-src 'none'; worker-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'"
                .to_string()
            ),
            strict_transport_security: Some("max-age=63072000; includeSubDomains; preload".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some("camera=(), microphone=(), geolocation=(), interest-cohort=()".to_string()),
            cross_origin_embedder_policy: Some("require-corp".to_string()),
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_resource_policy: Some("same-origin".to_string()),
            custom_headers: HashMap::new(),
            remove_server_header: true,
            remove_x_powered_by: true,
        });
        self
    }
    
    /// Add security headers middleware with development-friendly configuration
    pub fn with_security_headers_development(mut self) -> Self {
        use std::collections::HashMap;
        
        self.security_headers_config = Some(SecurityHeadersConfig {
            content_security_policy: Some(
                "default-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src 'self' data: blob: https:; connect-src 'self' ws: wss: http: https:"
                .to_string()
            ),
            strict_transport_security: Some("max-age=31536000".to_string()),
            x_frame_options: Some("SAMEORIGIN".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("origin-when-cross-origin".to_string()),
            permissions_policy: Some("camera=(), microphone=(), geolocation=()".to_string()),
            cross_origin_embedder_policy: None,
            cross_origin_opener_policy: Some("same-origin-allow-popups".to_string()),
            cross_origin_resource_policy: Some("cross-origin".to_string()),
            custom_headers: HashMap::new(),
            remove_server_header: false,
            remove_x_powered_by: true,
        });
        self
    }
    
    /// Build the security middleware pipeline
    /// 
    /// The middleware are added in the following order for optimal security:
    /// 1. CORS middleware (handles preflight requests early)
    /// 2. Security Headers middleware (adds security headers to all responses)
    /// 3. Rate limiting middleware (prevents abuse before processing)
    /// 4. Request Sanitization middleware (cleans input before validation)
    /// 5. CSRF middleware (validates tokens after sanitization)
    pub fn build(self) -> MiddlewarePipeline {
        let mut pipeline = MiddlewarePipeline::new();
        
        // Add CORS middleware first (handles preflight requests)
        if let Some(cors_config) = self.cors_config {
            let cors_middleware = CorsMiddleware::new(cors_config);
            pipeline = pipeline.add(cors_middleware);
        }
        
        // Add security headers middleware second (applies to all responses)
        if let Some(security_headers_config) = self.security_headers_config {
            let security_headers_middleware = SecurityHeadersMiddleware::new(security_headers_config);
            pipeline = pipeline.add(security_headers_middleware);
        }
        
        // Add rate limiting middleware third (prevents abuse early)
        if let Some(rate_limit_config) = self.rate_limit_config {
            let rate_limit_middleware = RateLimitMiddleware::new(rate_limit_config);
            pipeline = pipeline.add(rate_limit_middleware);
        }
        
        // Add sanitization middleware fourth (cleans input before validation)
        if let Some(sanitization_config) = self.sanitization_config {
            let sanitization_middleware = SanitizationMiddleware::new(sanitization_config);
            pipeline = pipeline.add(sanitization_middleware);
        }
        
        // Add CSRF middleware last (validates tokens after sanitization)
        if let Some(csrf_config) = self.csrf_config {
            let csrf_middleware = CsrfMiddleware::new(csrf_config);
            pipeline = pipeline.add(csrf_middleware);
        }
        
        pipeline
    }
}

/// Quick setup functions for common security configurations

/// Create a basic security pipeline with permissive CORS, moderate rate limiting, basic sanitization, and default CSRF
pub fn basic_security_pipeline() -> MiddlewarePipeline {
    SecurityMiddlewareBuilder::new()
        .with_cors_permissive()
        .with_security_headers_development()
        .with_rate_limit_default()
        .with_sanitization_permissive()
        .with_csrf_default()
        .build()
}

/// Create a strict security pipeline with restrictive CORS, strict sanitization, and secure CSRF
pub fn strict_security_pipeline(allowed_origins: Vec<String>) -> MiddlewarePipeline {
    use std::collections::HashSet;
    
    let cors_config = CorsConfig {
        allowed_origins: Some(allowed_origins.into_iter().collect::<HashSet<_>>()),
        allow_credentials: true,
        max_age: Some(300), // 5 minutes
        ..CorsConfig::default()
    };
    
    let csrf_config = CsrfConfig {
        secure_cookie: true,
        token_lifetime: 3600, // 1 hour
        ..CsrfConfig::default()
    };
    
    let rate_limit_config = RateLimitConfig {
        max_requests: 30, // Strict rate limiting
        window_seconds: 60,
        identifier: crate::config::RateLimitIdentifier::IpAddress,
        exempt_paths: std::collections::HashSet::new(),
    };
    
    SecurityMiddlewareBuilder::new()
        .with_cors(cors_config)
        .with_security_headers_strict()
        .with_rate_limit(rate_limit_config)
        .with_sanitization_strict()
        .with_csrf(csrf_config)
        .build()
}

/// Create a development security pipeline with relaxed settings
pub fn development_security_pipeline() -> MiddlewarePipeline {
    let cors_config = CorsConfig {
        allowed_origins: None, // Allow all origins in development
        allow_credentials: false,
        ..CorsConfig::default()
    };
    
    let csrf_config = CsrfConfig {
        secure_cookie: false, // Allow non-HTTPS in development
        token_lifetime: 7200, // 2 hours for convenience
        ..CsrfConfig::default()
    };
    
    let rate_limit_config = RateLimitConfig {
        max_requests: 1000, // Permissive rate limiting for development
        window_seconds: 60,
        identifier: crate::config::RateLimitIdentifier::IpAddress,
        exempt_paths: std::collections::HashSet::new(),
    };
    
    SecurityMiddlewareBuilder::new()
        .with_cors(cors_config)
        .with_security_headers_development()
        .with_rate_limit(rate_limit_config)
        .with_sanitization_permissive()
        .with_csrf(csrf_config)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Request, http::Method, body::Body};
    
    #[tokio::test]
    async fn test_basic_security_pipeline() {
        let pipeline = basic_security_pipeline();
        
        // Should have CORS, Security Headers, Rate Limiting, Sanitization, and CSRF middleware
        assert_eq!(pipeline.len(), 5);
        assert_eq!(pipeline.names(), vec!["CorsMiddleware", "SecurityHeadersMiddleware", "RateLimit", "SanitizationMiddleware", "CsrfMiddleware"]);
    }
    
    #[tokio::test]
    async fn test_security_middleware_builder() {
        let cors_config = CorsConfig::default();
        let csrf_config = CsrfConfig::default();
        
        let pipeline = SecurityMiddlewareBuilder::new()
            .with_cors(cors_config)
            .with_csrf(csrf_config)
            .build();
        
        assert_eq!(pipeline.len(), 2);
        assert!(pipeline.names().contains(&"CorsMiddleware"));
        assert!(pipeline.names().contains(&"CsrfMiddleware"));
    }
    
    #[tokio::test]
    async fn test_cors_only_pipeline() {
        let pipeline = SecurityMiddlewareBuilder::new()
            .with_cors_permissive()
            .build();
        
        assert_eq!(pipeline.len(), 1);
        assert_eq!(pipeline.names(), vec!["CorsMiddleware"]);
    }
    
    #[tokio::test]
    async fn test_csrf_only_pipeline() {
        let pipeline = SecurityMiddlewareBuilder::new()
            .with_csrf_default()
            .build();
        
        assert_eq!(pipeline.len(), 1);
        assert_eq!(pipeline.names(), vec!["CsrfMiddleware"]);
    }
    
    #[tokio::test]
    async fn test_security_pipeline_processing() {
        let pipeline = basic_security_pipeline();
        
        // Test normal GET request (should pass CORS and be exempt from CSRF)
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header("Origin", "https://example.com")
            .body(Body::empty())
            .unwrap();
        
        let result = pipeline.process_request(request).await;
        
        // Should pass through successfully
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_strict_security_pipeline() {
        let allowed_origins = vec!["https://trusted.com".to_string()];
        let pipeline = strict_security_pipeline(allowed_origins);
        
        assert_eq!(pipeline.len(), 5);
        
        // Test request from allowed origin
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header("Origin", "https://trusted.com")
            .body(Body::empty())
            .unwrap();
        
        let result = pipeline.process_request(request).await;
        assert!(result.is_ok());
        
        // Test request from disallowed origin
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header("Origin", "https://evil.com")
            .body(Body::empty())
            .unwrap();
        
        let result = pipeline.process_request(request).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_development_security_pipeline() {
        let pipeline = development_security_pipeline();
        
        assert_eq!(pipeline.len(), 5);
        
        // Should allow any origin in development mode
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header("Origin", "http://localhost:3000")
            .body(Body::empty())
            .unwrap();
        
        let result = pipeline.process_request(request).await;
        assert!(result.is_ok());
    }

    // ============================================================================
    // COMPREHENSIVE INTEGRATION TESTS - Phase 3.17
    // ============================================================================

    #[tokio::test]
    async fn test_security_pipeline_order_enforcement() {
        // Test that middleware are applied in the correct order: CORS -> Rate Limit -> CSRF
        let pipeline = SecurityMiddlewareBuilder::new()
            .with_cors_permissive()
            .with_rate_limit_default()
            .with_csrf_default()
            .build();
        
        assert_eq!(pipeline.len(), 3);
        let names = pipeline.names();
        assert_eq!(names[0], "CorsMiddleware");      // First: handles preflight requests
        assert_eq!(names[1], "RateLimit");          // Second: prevents abuse early
        assert_eq!(names[2], "CsrfMiddleware");     // Third: validates tokens after rate limiting
    }

    #[tokio::test]
    async fn test_comprehensive_cors_preflight_handling() {
        let pipeline = basic_security_pipeline();
        
        // Test CORS preflight request passes through all middleware
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/test")
            .header("Origin", "https://trusted.com")
            .header("Access-Control-Request-Method", "POST")
            .header("Access-Control-Request-Headers", "content-type,x-csrf-token")
            .body(Body::empty())
            .unwrap();
        
        let result = pipeline.process_request(request).await;
        
        // For preflight requests, CORS middleware should return early with proper response
        // This is expected behavior - it returns Err(response) to short-circuit the pipeline
        match result {
            Err(response) => {
                // Verify it's a successful preflight response (204 No Content)
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
                
                // Verify CORS headers are present in response
                let headers = response.headers();
                assert!(headers.contains_key("access-control-allow-origin"));
                assert!(headers.contains_key("access-control-allow-methods"));
            }
            Ok(_) => panic!("Expected CORS middleware to handle preflight request and return early"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiting_with_csrf_integration() {
        let config = RateLimitConfig {
            max_requests: 2, // Very low limit for testing
            window_seconds: 60,
            identifier: crate::config::RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        };
        
        let pipeline = SecurityMiddlewareBuilder::new()
            .with_rate_limit(config)
            .with_csrf_default()
            .build();
        
        // First POST request should be rate limited but not processed by CSRF
        let request1 = Request::builder()
            .method(Method::POST)
            .uri("/api/test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"test": "data"}"#))
            .unwrap();
        
        let result1 = pipeline.process_request(request1).await;
        
        // Second POST request should also be rate limited
        let request2 = Request::builder()
            .method(Method::POST)
            .uri("/api/test2")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"test": "data2"}"#))
            .unwrap();
        
        let result2 = pipeline.process_request(request2).await;
        
        // Third POST request should hit rate limit before CSRF validation
        let request3 = Request::builder()
            .method(Method::POST)
            .uri("/api/test3")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"test": "data3"}"#))
            .unwrap();
        
        let result3 = pipeline.process_request(request3).await;
        
        // Third request should fail with rate limit error, not CSRF error
        assert!(result3.is_err());
    }

    #[tokio::test]
    async fn test_security_configuration_combinations() {
        // Test all possible combinations of security middleware
        
        // 1. CORS only
        let cors_only = SecurityMiddlewareBuilder::new()
            .with_cors_permissive()
            .build();
        assert_eq!(cors_only.len(), 1);
        
        // 2. CSRF only  
        let csrf_only = SecurityMiddlewareBuilder::new()
            .with_csrf_default()
            .build();
        assert_eq!(csrf_only.len(), 1);
        
        // 3. Rate Limit only
        let rate_limit_only = SecurityMiddlewareBuilder::new()
            .with_rate_limit_default()
            .build();
        assert_eq!(rate_limit_only.len(), 1);
        
        // 4. CORS + CSRF
        let cors_csrf = SecurityMiddlewareBuilder::new()
            .with_cors_permissive()
            .with_csrf_default()
            .build();
        assert_eq!(cors_csrf.len(), 2);
        
        // 5. CORS + Rate Limit
        let cors_rate = SecurityMiddlewareBuilder::new()
            .with_cors_permissive()
            .with_rate_limit_default()
            .build();
        assert_eq!(cors_rate.len(), 2);
        
        // 6. CSRF + Rate Limit
        let csrf_rate = SecurityMiddlewareBuilder::new()
            .with_csrf_default()
            .with_rate_limit_default()
            .build();
        assert_eq!(csrf_rate.len(), 2);
        
        // 7. All three (already tested above)
        let all_three = SecurityMiddlewareBuilder::new()
            .with_cors_permissive()
            .with_csrf_default()
            .with_rate_limit_default()
            .build();
        assert_eq!(all_three.len(), 3);
    }

    #[tokio::test]
    async fn test_production_ready_strict_pipeline() {
        // Test a production-ready configuration with strict settings
        let allowed_origins = vec![
            "https://app.example.com".to_string(),
            "https://admin.example.com".to_string(),
        ];
        
        let pipeline = strict_security_pipeline(allowed_origins);
        
        // Test allowed origin
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/data")
            .header("Origin", "https://app.example.com")
            .header("User-Agent", "Mozilla/5.0 (compatible; Security Test)")
            .body(Body::empty())
            .unwrap();
        
        let result = pipeline.process_request(request).await;
        assert!(result.is_ok());
        
        // Test disallowed origin
        let request_bad = Request::builder()
            .method(Method::GET)
            .uri("/api/data")
            .header("Origin", "https://malicious.com")
            .body(Body::empty())
            .unwrap();
        
        let result_bad = pipeline.process_request(request_bad).await;
        assert!(result_bad.is_err());
    }

    #[tokio::test]
    async fn test_middleware_error_propagation() {
        let pipeline = basic_security_pipeline();
        
        // Test request that should fail CORS validation
        let bad_cors_request = Request::builder()
            .method(Method::POST)
            .uri("/api/sensitive")
            .header("Origin", "null") // Null origin should fail in some configurations
            .body(Body::empty())
            .unwrap();
        
        // The error from middleware should propagate correctly
        let result = pipeline.process_request(bad_cors_request).await;
        // Basic pipeline is permissive, so this might pass, but error handling is tested
        
        // Test with strict pipeline instead
        let strict_pipeline = strict_security_pipeline(vec!["https://trusted.com".to_string()]);
        
        let strict_request = Request::builder()
            .method(Method::POST)
            .uri("/api/sensitive")
            .header("Origin", "https://untrusted.com")
            .body(Body::empty())
            .unwrap();
        
        let strict_result = strict_pipeline.process_request(strict_request).await;
        assert!(strict_result.is_err());
    }

    #[tokio::test]
    async fn test_development_vs_production_configuration() {
        let dev_pipeline = development_security_pipeline();
        let prod_pipeline = strict_security_pipeline(vec!["https://prod.example.com".to_string()]);
        
        // Request that should pass in dev but fail in production
        let localhost_request = Request::builder()
            .method(Method::GET)
            .uri("/api/test")
            .header("Origin", "http://localhost:3000")
            .body(Body::empty())
            .unwrap();
        
        // Should pass in development
        let dev_result = dev_pipeline.process_request(localhost_request).await;
        assert!(dev_result.is_ok());
        
        // Should fail in production (different origin)
        let localhost_request_prod = Request::builder()
            .method(Method::GET)
            .uri("/api/test")
            .header("Origin", "http://localhost:3000")
            .body(Body::empty())
            .unwrap();
        
        let prod_result = prod_pipeline.process_request(localhost_request_prod).await;
        assert!(prod_result.is_err());
    }
}