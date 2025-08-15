//! Security Middleware Integration
//!
//! Provides a unified way to integrate all security middleware with the framework's 
//! MiddlewarePipeline, ensuring consistent usage and proper ordering.

use elif_http::middleware::MiddlewarePipeline;
use crate::{
    middleware::{cors::CorsMiddleware, csrf::CsrfMiddleware, rate_limit::RateLimitMiddleware},
    config::{CorsConfig, CsrfConfig, RateLimitConfig},
};

/// Security middleware suite builder that helps configure and integrate
/// all security middleware with the framework's MiddlewarePipeline
#[derive(Debug, Default)]
pub struct SecurityMiddlewareBuilder {
    cors_config: Option<CorsConfig>,
    csrf_config: Option<CsrfConfig>,
    rate_limit_config: Option<RateLimitConfig>,
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
    
    /// Build the security middleware pipeline
    /// 
    /// The middleware are added in the following order for optimal security:
    /// 1. CORS middleware (handles preflight requests early)
    /// 2. Rate limiting middleware (prevents abuse before processing)
    /// 3. CSRF middleware (validates tokens after rate limiting)
    pub fn build(self) -> MiddlewarePipeline {
        let mut pipeline = MiddlewarePipeline::new();
        
        // Add CORS middleware first (handles preflight requests)
        if let Some(cors_config) = self.cors_config {
            let cors_middleware = CorsMiddleware::new(cors_config);
            pipeline = pipeline.add(cors_middleware);
        }
        
        // Add rate limiting middleware second (prevents abuse early)
        if let Some(rate_limit_config) = self.rate_limit_config {
            let rate_limit_middleware = RateLimitMiddleware::new(rate_limit_config);
            pipeline = pipeline.add(rate_limit_middleware);
        }
        
        // Add CSRF middleware third (validates tokens after rate limiting)
        if let Some(csrf_config) = self.csrf_config {
            let csrf_middleware = CsrfMiddleware::new(csrf_config);
            pipeline = pipeline.add(csrf_middleware);
        }
        
        pipeline
    }
}

/// Quick setup functions for common security configurations

/// Create a basic security pipeline with permissive CORS, moderate rate limiting, and default CSRF
pub fn basic_security_pipeline() -> MiddlewarePipeline {
    SecurityMiddlewareBuilder::new()
        .with_cors_permissive()
        .with_rate_limit_default()
        .with_csrf_default()
        .build()
}

/// Create a strict security pipeline with restrictive CORS and secure CSRF
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
        .with_rate_limit(rate_limit_config)
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
        .with_rate_limit(rate_limit_config)
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
        
        // Should have CORS, Rate Limiting, and CSRF middleware
        assert_eq!(pipeline.len(), 3);
        assert_eq!(pipeline.names(), vec!["CorsMiddleware", "RateLimit", "CsrfMiddleware"]);
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
        
        assert_eq!(pipeline.len(), 3);
        
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
        
        assert_eq!(pipeline.len(), 3);
        
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
}