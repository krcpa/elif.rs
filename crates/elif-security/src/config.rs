//! Security configuration types and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Global security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// CORS configuration
    pub cors: Option<CorsConfig>,
    
    /// CSRF protection configuration  
    pub csrf: Option<CsrfConfig>,
    
    /// Rate limiting configuration
    pub rate_limiting: Option<RateLimitConfig>,
    
    /// Security headers configuration
    pub security_headers: Option<SecurityHeadersConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors: Some(CorsConfig::default()),
            csrf: Some(CsrfConfig::default()),
            rate_limiting: Some(RateLimitConfig::default()),
            security_headers: Some(SecurityHeadersConfig::default()),
        }
    }
}

/// CORS (Cross-Origin Resource Sharing) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins - None means allow all origins (*)
    pub allowed_origins: Option<HashSet<String>>,
    
    /// Allowed HTTP methods
    pub allowed_methods: HashSet<String>,
    
    /// Allowed request headers
    pub allowed_headers: HashSet<String>,
    
    /// Headers exposed to the client
    pub exposed_headers: HashSet<String>,
    
    /// Whether to allow credentials (cookies, authorization headers)
    pub allow_credentials: bool,
    
    /// Maximum age for preflight cache (seconds)
    pub max_age: Option<u32>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert("GET".to_string());
        allowed_methods.insert("POST".to_string());
        allowed_methods.insert("PUT".to_string());
        allowed_methods.insert("DELETE".to_string());
        allowed_methods.insert("OPTIONS".to_string());
        
        let mut allowed_headers = HashSet::new();
        allowed_headers.insert("content-type".to_string());
        allowed_headers.insert("authorization".to_string());
        allowed_headers.insert("x-requested-with".to_string());
        allowed_headers.insert("x-csrf-token".to_string()); // Support for CSRF protection
        
        Self {
            allowed_origins: None, // Allow all by default (not recommended for production)
            allowed_methods,
            allowed_headers,
            exposed_headers: HashSet::new(),
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
        }
    }
}

/// CSRF (Cross-Site Request Forgery) protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfConfig {
    /// Token header name
    pub token_header: String,
    
    /// Cookie name for CSRF token
    pub cookie_name: String,
    
    /// Token lifetime in seconds
    pub token_lifetime: u64,
    
    /// Whether to use secure cookies (HTTPS only)
    pub secure_cookie: bool,
    
    /// Paths that are exempt from CSRF protection
    pub exempt_paths: HashSet<String>,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            token_header: "X-CSRF-Token".to_string(),
            cookie_name: "_csrf_token".to_string(),
            token_lifetime: 3600, // 1 hour
            secure_cookie: true,
            exempt_paths: HashSet::new(),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    
    /// Time window duration
    pub window_seconds: u32,
    
    /// Identifier strategy (IP, user ID, etc.)
    pub identifier: RateLimitIdentifier,
    
    /// Paths exempt from rate limiting
    pub exempt_paths: HashSet<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 60, // 1 minute
            identifier: RateLimitIdentifier::IpAddress,
            exempt_paths: HashSet::new(),
        }
    }
}

/// Rate limit identifier strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitIdentifier {
    /// Use client IP address
    IpAddress,
    /// Use authenticated user ID
    UserId,
    /// Use API key
    ApiKey,
    /// Custom identifier from header
    CustomHeader(String),
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Enable X-Frame-Options header
    pub x_frame_options: Option<String>,
    
    /// Enable X-Content-Type-Options header
    pub x_content_type_options: bool,
    
    /// Enable X-XSS-Protection header
    pub x_xss_protection: bool,
    
    /// Strict-Transport-Security header (HSTS)
    pub hsts: Option<HstsConfig>,
    
    /// Content Security Policy header
    pub csp: Option<String>,
    
    /// Referrer Policy header
    pub referrer_policy: Option<String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: true,
            x_xss_protection: true,
            hsts: Some(HstsConfig::default()),
            csp: Some("default-src 'self'".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
        }
    }
}

/// HTTP Strict Transport Security (HSTS) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HstsConfig {
    /// Maximum age in seconds
    pub max_age: u32,
    
    /// Include subdomains
    pub include_subdomains: bool,
    
    /// Preload directive
    pub preload: bool,
}

impl Default for HstsConfig {
    fn default() -> Self {
        Self {
            max_age: 31536000, // 1 year
            include_subdomains: true,
            preload: false,
        }
    }
}