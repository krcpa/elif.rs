//! Security headers middleware implementation
//!
//! Provides comprehensive security headers including CSP, HSTS, X-Frame-Options, and more.

use crate::{SecurityError, SecurityResult};
use elif_http::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::ElifResponse,
};

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
                "camera=(), microphone=(), geolocation=(), payment=(), usb=()".to_string(),
            ),
            cross_origin_embedder_policy: None,
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_resource_policy: Some("same-origin".to_string()),
            custom_headers: std::collections::HashMap::new(),
            remove_server_header: true,
            remove_x_powered_by: true,
        })
    }

    /// Apply security headers to a response (static version for async contexts)
    pub fn apply_headers_to_response(response: &mut ElifResponse, config: &SecurityHeadersConfig) {
        if let Err(e) = Self::apply_headers_impl(response, config) {
            log::warn!("Failed to apply security headers: {}", e);
            // Add error header but continue with response
            let _ = response.add_header(
                "X-Security-Error",
                format!("Header application failed: {}", e),
            );
        }
    }

    /// Apply security headers to response
    pub fn apply_headers(&self, response: &mut ElifResponse) -> SecurityResult<()> {
        Self::apply_headers_impl(response, &self.config)
    }

    /// Internal implementation for applying headers
    fn apply_headers_impl(
        response: &mut ElifResponse,
        config: &SecurityHeadersConfig,
    ) -> SecurityResult<()> {
        // Content Security Policy
        if let Some(ref csp) = config.content_security_policy {
            response
                .add_header("content-security-policy", csp)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add CSP header".to_string(),
                })?;
        }

        // HTTP Strict Transport Security
        if let Some(ref hsts) = config.strict_transport_security {
            response
                .add_header("strict-transport-security", hsts)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add HSTS header".to_string(),
                })?;
        }

        // X-Frame-Options
        if let Some(ref xfo) = config.x_frame_options {
            response.add_header("x-frame-options", xfo).map_err(|_| {
                SecurityError::ConfigError {
                    message: "Failed to add X-Frame-Options header".to_string(),
                }
            })?;
        }

        // X-Content-Type-Options
        if let Some(ref xcto) = config.x_content_type_options {
            response
                .add_header("x-content-type-options", xcto)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add X-Content-Type-Options header".to_string(),
                })?;
        }

        // X-XSS-Protection
        if let Some(ref xxp) = config.x_xss_protection {
            response.add_header("x-xss-protection", xxp).map_err(|_| {
                SecurityError::ConfigError {
                    message: "Failed to add X-XSS-Protection header".to_string(),
                }
            })?;
        }

        // Referrer-Policy
        if let Some(ref rp) = config.referrer_policy {
            response
                .add_header("referrer-policy", rp)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add Referrer-Policy header".to_string(),
                })?;
        }

        // Permissions-Policy
        if let Some(ref pp) = config.permissions_policy {
            response.add_header("permissions-policy", pp).map_err(|_| {
                SecurityError::ConfigError {
                    message: "Failed to add Permissions-Policy header".to_string(),
                }
            })?;
        }

        // Cross-Origin-Embedder-Policy
        if let Some(ref coep) = config.cross_origin_embedder_policy {
            response
                .add_header("cross-origin-embedder-policy", coep)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add Cross-Origin-Embedder-Policy header".to_string(),
                })?;
        }

        // Cross-Origin-Opener-Policy
        if let Some(ref coop) = config.cross_origin_opener_policy {
            response
                .add_header("cross-origin-opener-policy", coop)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add Cross-Origin-Opener-Policy header".to_string(),
                })?;
        }

        // Cross-Origin-Resource-Policy
        if let Some(ref corp) = config.cross_origin_resource_policy {
            response
                .add_header("cross-origin-resource-policy", corp)
                .map_err(|_| SecurityError::ConfigError {
                    message: "Failed to add Cross-Origin-Resource-Policy header".to_string(),
                })?;
        }

        // Custom headers
        for (name, value) in &config.custom_headers {
            response
                .add_header(name, value)
                .map_err(|_| SecurityError::ConfigError {
                    message: format!("Failed to add custom header '{}'", name),
                })?;
        }

        // Remove server identification headers if configured
        if config.remove_server_header {
            response.remove_header("server");
        }

        if config.remove_x_powered_by {
            response.remove_header("x-powered-by");
        }

        Ok(())
    }
}

impl Middleware for SecurityHeadersMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let config = self.config.clone();
        Box::pin(async move {
            // Continue to next middleware/handler first
            let mut response = next.run(request).await;

            // Apply security headers to response inline
            SecurityHeadersMiddleware::apply_headers_to_response(&mut response, &config);

            response
        })
    }

    fn name(&self) -> &'static str {
        "SecurityHeadersMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::response::ElifHeaderName;

    #[test]
    fn test_strict_headers() {
        let middleware = SecurityHeadersMiddleware::strict();
        let mut response = ElifResponse::ok();

        let result = middleware.apply_headers(&mut response);
        assert!(result.is_ok());

        let headers = response.headers().clone();

        // Check essential headers are present
        assert!(headers.contains_key(&ElifHeaderName::from_str("content-security-policy").unwrap()));
        assert!(
            headers.contains_key(&ElifHeaderName::from_str("strict-transport-security").unwrap())
        );
        assert!(headers.contains_key(&ElifHeaderName::from_str("x-frame-options").unwrap()));
        assert!(headers.contains_key(&ElifHeaderName::from_str("x-content-type-options").unwrap()));
        assert!(headers.contains_key(&ElifHeaderName::from_str("x-xss-protection").unwrap()));
    }

    #[test]
    fn test_development_headers() {
        let middleware = SecurityHeadersMiddleware::development();
        let mut response = ElifResponse::ok();

        let result = middleware.apply_headers(&mut response);
        assert!(result.is_ok());

        let headers = response.headers().clone();

        // Should have less restrictive CSP
        let csp = headers
            .get(&ElifHeaderName::from_str("content-security-policy").unwrap())
            .unwrap()
            .to_str()
            .unwrap();
        assert!(csp.contains("unsafe-inline"));
        assert!(csp.contains("unsafe-eval"));
    }

    #[test]
    fn test_api_focused_headers() {
        let middleware = SecurityHeadersMiddleware::api_focused();
        let mut response = ElifResponse::ok();

        let result = middleware.apply_headers(&mut response);
        assert!(result.is_ok());

        let headers = response.headers().clone();

        // Should have strict CSP for APIs
        let csp = headers
            .get(&ElifHeaderName::from_str("content-security-policy").unwrap())
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(csp, "default-src 'none'; frame-ancestors 'none'");

        // Should deny framing
        let xfo = headers
            .get(&ElifHeaderName::from_str("x-frame-options").unwrap())
            .unwrap()
            .to_str()
            .unwrap();
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
        let mut response = ElifResponse::ok();

        let result = middleware.apply_headers(&mut response);
        assert!(result.is_ok());

        // Custom headers should be applied (implementation specific check)
        let custom_header = response
            .headers()
            .get(&ElifHeaderName::from_str("X-Custom-Header").unwrap())
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(custom_header, "custom-value");
    }
}
