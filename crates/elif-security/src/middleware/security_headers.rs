//! Security headers middleware implementation
//!
//! Provides comprehensive security headers including CSP, HSTS, X-Frame-Options, and more.

use axum::{
    http::{HeaderName, HeaderValue, StatusCode},
    response::Response,
    body::Body,
};
use elif_http::middleware::{Middleware, BoxFuture};
use crate::{SecurityError, SecurityResult};

pub use crate::config::SecurityHeadersConfig;

/// Security headers middleware that adds comprehensive security headers to responses
#[derive(Debug, Clone)]
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersMiddleware {
    /// Create new security headers middleware with configuration
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }
    
    /// Create security headers middleware with strict production settings
    pub fn strict() -> Self {
        Self::new(SecurityHeadersConfig {
            // Content Security Policy - Very restrictive
            content_security_policy: Some(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; child-src 'none'; frame-src 'none'; worker-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'; manifest-src 'self'"
                .to_string()
            ),
            // HTTP Strict Transport Security - 2 years, include subdomains
            strict_transport_security: Some("max-age=63072000; includeSubDomains; preload".to_string()),
            // X-Frame-Options - Deny all framing
            x_frame_options: Some("DENY".to_string()),
            // X-Content-Type-Options - Prevent MIME sniffing
            x_content_type_options: Some("nosniff".to_string()),
            // X-XSS-Protection - Enable XSS filtering
            x_xss_protection: Some("1; mode=block".to_string()),
            // Referrer Policy - Strict same-origin
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            // Permissions Policy - Restrict dangerous features
            permissions_policy: Some(
                "camera=(), microphone=(), geolocation=(), interest-cohort=()"
                .to_string()
            ),
            // Cross-Origin Embedder Policy
            cross_origin_embedder_policy: Some("require-corp".to_string()),
            // Cross-Origin Opener Policy  
            cross_origin_opener_policy: Some("same-origin".to_string()),
            // Cross-Origin Resource Policy
            cross_origin_resource_policy: Some("same-origin".to_string()),
            // Custom headers
            custom_headers: std::collections::HashMap::new(),
            // Server header removal
            remove_server_header: true,
            remove_x_powered_by: true,
        })
    }
    
    /// Create security headers middleware with development-friendly settings
    pub fn development() -> Self {
        Self::new(SecurityHeadersConfig {
            // More permissive CSP for development
            content_security_policy: Some(
                "default-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src 'self' data: blob: https:; connect-src 'self' ws: wss: http: https:"
                .to_string()
            ),
            // Shorter HSTS for development
            strict_transport_security: Some("max-age=31536000".to_string()),
            // Less restrictive frame options
            x_frame_options: Some("SAMEORIGIN".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("origin-when-cross-origin".to_string()),
            // More permissive permissions policy
            permissions_policy: Some("camera=(), microphone=(), geolocation=()".to_string()),
            cross_origin_embedder_policy: None,
            cross_origin_opener_policy: Some("same-origin-allow-popups".to_string()),
            cross_origin_resource_policy: Some("cross-origin".to_string()),
            custom_headers: std::collections::HashMap::new(),
            remove_server_header: false,
            remove_x_powered_by: true,
        })
    }
    
    /// Create security headers middleware for API endpoints
    pub fn api_focused() -> Self {
        Self::new(SecurityHeadersConfig {
            // API-focused CSP
            content_security_policy: Some("default-src 'none'; frame-ancestors 'none'".to_string()),
            strict_transport_security: Some("max-age=63072000; includeSubDomains".to_string()),
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("no-referrer".to_string()),
            permissions_policy: Some(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
                .to_string()
            ),
            cross_origin_embedder_policy: None,
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_resource_policy: Some("same-origin".to_string()),
            custom_headers: std::collections::HashMap::new(),
            remove_server_header: true,
            remove_x_powered_by: true,
        })
    }
    
    /// Apply security headers to response
    fn apply_headers(&self, mut response: Response) -> SecurityResult<Response> {
        let headers = response.headers_mut();
        
        // Content Security Policy
        if let Some(ref csp) = self.config.content_security_policy {
            headers.insert(
                HeaderName::from_static("content-security-policy"),
                HeaderValue::from_str(csp)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid CSP header: {}", e) 
                    })?
            );
        }
        
        // HTTP Strict Transport Security
        if let Some(ref hsts) = self.config.strict_transport_security {
            headers.insert(
                HeaderName::from_static("strict-transport-security"),
                HeaderValue::from_str(hsts)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid HSTS header: {}", e) 
                    })?
            );
        }
        
        // X-Frame-Options
        if let Some(ref xfo) = self.config.x_frame_options {
            headers.insert(
                HeaderName::from_static("x-frame-options"),
                HeaderValue::from_str(xfo)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid X-Frame-Options header: {}", e) 
                    })?
            );
        }
        
        // X-Content-Type-Options
        if let Some(ref xcto) = self.config.x_content_type_options {
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_str(xcto)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid X-Content-Type-Options header: {}", e) 
                    })?
            );
        }
        
        // X-XSS-Protection  
        if let Some(ref xxp) = self.config.x_xss_protection {
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_str(xxp)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid X-XSS-Protection header: {}", e) 
                    })?
            );
        }
        
        // Referrer-Policy
        if let Some(ref rp) = self.config.referrer_policy {
            headers.insert(
                HeaderName::from_static("referrer-policy"),
                HeaderValue::from_str(rp)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid Referrer-Policy header: {}", e) 
                    })?
            );
        }
        
        // Permissions-Policy
        if let Some(ref pp) = self.config.permissions_policy {
            headers.insert(
                HeaderName::from_static("permissions-policy"),
                HeaderValue::from_str(pp)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid Permissions-Policy header: {}", e) 
                    })?
            );
        }
        
        // Cross-Origin-Embedder-Policy
        if let Some(ref coep) = self.config.cross_origin_embedder_policy {
            headers.insert(
                HeaderName::from_static("cross-origin-embedder-policy"),
                HeaderValue::from_str(coep)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid Cross-Origin-Embedder-Policy header: {}", e) 
                    })?
            );
        }
        
        // Cross-Origin-Opener-Policy
        if let Some(ref coop) = self.config.cross_origin_opener_policy {
            headers.insert(
                HeaderName::from_static("cross-origin-opener-policy"),
                HeaderValue::from_str(coop)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid Cross-Origin-Opener-Policy header: {}", e) 
                    })?
            );
        }
        
        // Cross-Origin-Resource-Policy
        if let Some(ref corp) = self.config.cross_origin_resource_policy {
            headers.insert(
                HeaderName::from_static("cross-origin-resource-policy"),
                HeaderValue::from_str(corp)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid Cross-Origin-Resource-Policy header: {}", e) 
                    })?
            );
        }
        
        // Custom headers
        for (name, value) in &self.config.custom_headers {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes())
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid custom header name '{}': {}", name, e) 
                    })?,
                HeaderValue::from_str(value)
                    .map_err(|e| SecurityError::ConfigError { 
                        message: format!("Invalid custom header value for '{}': {}", name, e) 
                    })?
            );
        }
        
        // Remove server identification headers if configured
        if self.config.remove_server_header {
            headers.remove("server");
        }
        
        if self.config.remove_x_powered_by {
            headers.remove("x-powered-by");
        }
        
        Ok(response)
    }
}

impl Middleware for SecurityHeadersMiddleware {
    fn process_response<'a>(
        &'a self, 
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Apply security headers to response
            match self.apply_headers(response) {
                Ok(response) => response,
                Err(e) => {
                    // If header application fails, return original response with error header
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header("X-Security-Error", format!("Header application failed: {}", e))
                        .body(Body::empty())
                        .unwrap_or_else(|_| Response::new(Body::empty()))
                }
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "SecurityHeadersMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_headers() {
        let middleware = SecurityHeadersMiddleware::strict();
        let response = Response::new(Body::empty());
        
        let result = middleware.apply_headers(response);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        let headers = response.headers();
        
        // Check essential headers are present
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("strict-transport-security"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-xss-protection"));
    }
    
    #[test]
    fn test_development_headers() {
        let middleware = SecurityHeadersMiddleware::development();
        let response = Response::new(Body::empty());
        
        let result = middleware.apply_headers(response);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        let headers = response.headers();
        
        // Should have less restrictive CSP
        let csp = headers.get("content-security-policy").unwrap().to_str().unwrap();
        assert!(csp.contains("unsafe-inline"));
        assert!(csp.contains("unsafe-eval"));
    }
    
    #[test]
    fn test_api_focused_headers() {
        let middleware = SecurityHeadersMiddleware::api_focused();
        let response = Response::new(Body::empty());
        
        let result = middleware.apply_headers(response);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        let headers = response.headers();
        
        // Should have strict CSP for APIs
        let csp = headers.get("content-security-policy").unwrap().to_str().unwrap();
        assert_eq!(csp, "default-src 'none'; frame-ancestors 'none'");
        
        // Should deny framing
        let xfo = headers.get("x-frame-options").unwrap().to_str().unwrap();
        assert_eq!(xfo, "DENY");
    }
    
    #[test]
    fn test_custom_headers() {
        let mut custom_headers = std::collections::HashMap::new();
        custom_headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());
        
        let config = SecurityHeadersConfig {
            custom_headers,
            ..SecurityHeadersConfig::default()
        };
        
        let middleware = SecurityHeadersMiddleware::new(config);
        let response = Response::new(Body::empty());
        
        let result = middleware.apply_headers(response);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        let headers = response.headers();
        
        assert_eq!(
            headers.get("x-custom-header").unwrap().to_str().unwrap(),
            "custom-value"
        );
    }
    
    #[test]
    fn test_header_removal() {
        let config = SecurityHeadersConfig {
            remove_server_header: true,
            remove_x_powered_by: true,
            ..SecurityHeadersConfig::default()
        };
        
        let middleware = SecurityHeadersMiddleware::new(config);
        
        let mut response = Response::new(Body::empty());
        response.headers_mut().insert("server", HeaderValue::from_static("nginx/1.20"));
        response.headers_mut().insert("x-powered-by", HeaderValue::from_static("PHP/8.0"));
        
        let result = middleware.apply_headers(response);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        let headers = response.headers();
        
        // Headers should be removed
        assert!(!headers.contains_key("server"));
        assert!(!headers.contains_key("x-powered-by"));
    }
}