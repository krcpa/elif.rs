//! Request sanitization middleware implementation
//!
//! Provides XSS prevention and input sanitization for incoming requests.

use elif_http::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::{ElifResponse, ElifStatusCode},
};
use crate::{SecurityError, SecurityResult};

pub use crate::config::SanitizationConfig;

/// Request sanitization middleware that cleans and validates input data
#[derive(Debug, Clone)]
pub struct SanitizationMiddleware {
    config: SanitizationConfig,
}

impl SanitizationMiddleware {
    /// Create new sanitization middleware with configuration
    pub fn new(config: SanitizationConfig) -> Self {
        Self { config }
    }
    
    /// Create sanitization middleware with default strict settings
    pub fn strict() -> Self {
        Self::new(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: true,
            enable_path_traversal_protection: true,
            enable_script_tag_removal: true,
            enable_html_encoding: true,
            max_request_size: Some(1024 * 1024), // 1MB
            blocked_patterns: vec![
                // XSS patterns
                r"<script[^>]*>.*?</script>".to_string(),
                r"javascript:".to_string(),
                r"on\w+\s*=".to_string(),
                // SQL injection patterns  
                r"(?i)(union|select|insert|update|delete|drop|exec|execute)".to_string(),
                // Path traversal patterns
                r"\.\./".to_string(),
                r"\.\\".to_string(),
            ],
            allowed_html_tags: vec!["b", "i", "em", "strong", "p", "br"].into_iter().map(String::from).collect(),
        })
    }
    
    /// Create sanitization middleware with permissive settings for development
    pub fn permissive() -> Self {
        Self::new(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: false,
            enable_path_traversal_protection: true,
            enable_script_tag_removal: true,
            enable_html_encoding: false,
            max_request_size: Some(10 * 1024 * 1024), // 10MB
            blocked_patterns: vec![
                r"<script[^>]*>.*?</script>".to_string(),
                r"javascript:".to_string(),
            ],
            allowed_html_tags: vec!["b", "i", "em", "strong", "p", "br", "div", "span", "a", "img"].into_iter().map(String::from).collect(),
        })
    }
    
    /// Sanitize a string value according to configuration
    fn sanitize_value(&self, value: &str) -> SecurityResult<String> {
        let mut sanitized = value.to_string();
        
        // Check blocked patterns
        for pattern in &self.config.blocked_patterns {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| SecurityError::ConfigError { 
                    message: format!("Invalid regex pattern '{}': {}", pattern, e) 
                })?;
            
            if regex.is_match(&sanitized) {
                return Err(SecurityError::PolicyViolation {
                    message: format!("Input contains blocked pattern: {}", pattern),
                });
            }
        }
        
        // Remove script tags if enabled
        if self.config.enable_script_tag_removal {
            let script_regex = regex::Regex::new(r"<script[^>]*>.*?</script>")
                .map_err(|e| SecurityError::ConfigError { 
                    message: format!("Script removal regex error: {}", e) 
                })?;
            sanitized = script_regex.replace_all(&sanitized, "").to_string();
        }
        
        // HTML encoding if enabled
        if self.config.enable_html_encoding {
            sanitized = html_escape::encode_text(&sanitized).to_string();
        }
        
        Ok(sanitized)
    }
    
    /// Check if request size is within limits
    fn check_request_size(&self, body_size: usize) -> SecurityResult<()> {
        if let Some(max_size) = self.config.max_request_size {
            if body_size > max_size {
                return Err(SecurityError::PolicyViolation {
                    message: format!("Request size {} exceeds maximum {}", body_size, max_size),
                });
            }
        }
        Ok(())
    }
}

impl Middleware for SanitizationMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Check request size
            let content_length = request.headers
                .get_str("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
                
            if let Err(e) = self.config.check_request_size(content_length) {
                return ElifResponse::with_status(ElifStatusCode::BAD_REQUEST)
                    .text(&format!("Request sanitization failed: {}", e));
            }
            
            // Check User-Agent for suspicious patterns
            if let Some(user_agent) = request.headers.get_str("user-agent") {
                if let Ok(ua_str) = user_agent.to_str() {
                    // Block known malicious user agents
                    let malicious_patterns = [
                        "sqlmap", "nikto", "nmap", "masscan", 
                        "wget", "curl", "python-requests"
                    ];
                    
                    for pattern in &malicious_patterns {
                        if ua_str.to_lowercase().contains(pattern) {
                            return ElifResponse::with_status(ElifStatusCode::FORBIDDEN)
                                .text(&format!("Suspicious User-Agent detected: {}", pattern));
                        }
                    }
                }
            }
            
            // For now, pass the request through
            // In a real implementation, we would:
            // 1. Extract and sanitize query parameters
            // 2. Extract and sanitize request body (JSON/form data)
            // 3. Reconstruct the request with sanitized data
            
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "SanitizationMiddleware"
    }
}

// Helper trait to extend SanitizationConfig with methods
trait SanitizationConfigExt {
    fn check_request_size(&self, body_size: usize) -> SecurityResult<()>;
}

impl SanitizationConfigExt for SanitizationConfig {
    fn check_request_size(&self, body_size: usize) -> SecurityResult<()> {
        if let Some(max_size) = self.max_request_size {
            if body_size > max_size {
                return Err(SecurityError::PolicyViolation {
                    message: format!("Request size {} exceeds maximum {}", body_size, max_size),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_xss_patterns() {
        let middleware = SanitizationMiddleware::strict();
        
        // Test script tag removal
        let result = middleware.sanitize_value("<script>alert('xss')</script>");
        assert!(result.is_err());
        
        // Test javascript protocol
        let result = middleware.sanitize_value("javascript:alert('xss')");
        assert!(result.is_err());
        
        // Test safe input
        let result = middleware.sanitize_value("Hello world");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_request_size_limits() {
        let middleware = SanitizationMiddleware::strict();
        
        // Test within limit
        let result = middleware.check_request_size(1024);
        assert!(result.is_ok());
        
        // Test exceeding limit
        let result = middleware.check_request_size(2 * 1024 * 1024);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_permissive_vs_strict() {
        let strict = SanitizationMiddleware::strict();
        let permissive = SanitizationMiddleware::permissive();
        
        // Both should block XSS
        assert!(strict.sanitize_value("<script>alert('xss')</script>").is_err());
        assert!(permissive.sanitize_value("<script>alert('xss')</script>").is_err());
        
        // Permissive should allow more content
        assert_eq!(permissive.config.max_request_size, Some(10 * 1024 * 1024));
        assert_eq!(strict.config.max_request_size, Some(1024 * 1024));
    }
}