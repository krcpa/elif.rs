//! Security Middleware Integration
//!
//! Provides a unified way to integrate all security middleware with the framework's 
//! MiddlewarePipeline, ensuring consistent usage and proper ordering.

use elif_http::middleware::MiddlewarePipeline;
use service_builder::builder;
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

/// Security middleware configuration
#[derive(Debug, Clone)]
#[builder]
pub struct SecurityMiddlewareConfig {
    #[builder(optional)]
    pub cors_config: Option<CorsConfig>,
    #[builder(optional)]
    pub csrf_config: Option<CsrfConfig>,
    #[builder(optional)]
    pub rate_limit_config: Option<RateLimitConfig>,
    #[builder(optional)]
    pub sanitization_config: Option<SanitizationConfig>,
    #[builder(optional)]
    pub security_headers_config: Option<SecurityHeadersConfig>,
}

impl SecurityMiddlewareConfig {
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

// Add convenience methods to the generated builder
impl SecurityMiddlewareConfigBuilder {
    /// Add CORS middleware with permissive settings (not recommended for production)
    pub fn with_cors_permissive(self) -> Self {
        self.cors_config(Some(CorsConfig::default()))
    }
    
    /// Add CSRF middleware with default configuration
    pub fn with_csrf_default(self) -> Self {
        self.csrf_config(Some(CsrfConfig::default()))
    }
    
    /// Add rate limiting middleware with default configuration (100 req/min by IP)
    pub fn with_rate_limit_default(self) -> Self {
        self.rate_limit_config(Some(RateLimitConfig::default()))
    }
    
    /// Add rate limiting middleware with strict configuration (10 req/min by IP)
    pub fn with_rate_limit_strict(self) -> Self {
        self.rate_limit_config(Some(RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
            identifier: crate::config::RateLimitIdentifier::IpAddress,
            exempt_paths: std::collections::HashSet::new(),
        }))
    }
    
    /// Add request sanitization middleware with strict configuration
    pub fn with_sanitization_strict(self) -> Self {
        self.sanitization_config(Some(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: true,
            enable_path_traversal_protection: true,
            enable_script_tag_removal: true,
            enable_html_encoding: true,
            max_request_size: Some(1024 * 1024), // 1MB
            ..SanitizationConfig::default()
        }))
    }
    
    /// Add request sanitization middleware with permissive configuration
    pub fn with_sanitization_permissive(self) -> Self {
        self.sanitization_config(Some(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: false,
            enable_path_traversal_protection: true,
            enable_script_tag_removal: true,
            enable_html_encoding: false,
            max_request_size: Some(10 * 1024 * 1024), // 10MB
            ..SanitizationConfig::default()
        }))
    }
    
    /// Add security headers middleware with strict production configuration
    pub fn with_security_headers_strict(self) -> Self {
        use std::collections::HashMap;
        
        self.security_headers_config(Some(SecurityHeadersConfig {
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
        }))
    }
    
    /// Add security headers middleware with development-friendly configuration
    pub fn with_security_headers_development(self) -> Self {
        use std::collections::HashMap;
        
        self.security_headers_config(Some(SecurityHeadersConfig {
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
        }))
    }
    
    pub fn build_config(self) -> SecurityMiddlewareConfig {
        self.build_with_defaults().expect("Building SecurityMiddlewareConfig should not fail as all fields are optional")
    }
}

/// Quick setup functions for common security configurations

/// Create a basic security pipeline with permissive CORS, moderate rate limiting, basic sanitization, and default CSRF
pub fn basic_security_pipeline() -> MiddlewarePipeline {
    SecurityMiddlewareConfig::builder()
        .with_cors_permissive()
        .with_security_headers_development()
        .with_rate_limit_default()
        .with_sanitization_permissive()
        .with_csrf_default()
        .build_config()
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
    
    SecurityMiddlewareConfig::builder()
        .cors_config(Some(cors_config))
        .with_security_headers_strict()
        .rate_limit_config(Some(rate_limit_config))
        .with_sanitization_strict()
        .csrf_config(Some(csrf_config))
        .build_config()
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
    
    SecurityMiddlewareConfig::builder()
        .cors_config(Some(cors_config))
        .with_security_headers_development()
        .rate_limit_config(Some(rate_limit_config))
        .with_sanitization_permissive()
        .csrf_config(Some(csrf_config))
        .build_config()
        .build()
}
