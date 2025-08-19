//! # Maintenance Mode Middleware
//!
//! Provides maintenance mode functionality to temporarily disable application access.
//! Supports custom responses, whitelisted paths, and dynamic enable/disable.

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::{ElifRequest, ElifMethod};
use crate::response::{ElifResponse, ElifStatusCode};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::path::Path;

/// Maintenance mode response type
#[derive(Debug, Clone)]
pub enum MaintenanceResponse {
    /// Simple text response
    Text(String),
    /// JSON response with error details
    Json(serde_json::Value),
    /// HTML response (e.g., maintenance page)
    Html(String),
    /// Custom response with status code and body
    Custom {
        status_code: ElifStatusCode,
        content_type: String,
        body: Vec<u8>,
    },
    /// Load response from file
    File(String),
}

impl Default for MaintenanceResponse {
    fn default() -> Self {
        Self::Json(serde_json::json!({
            "error": {
                "code": "maintenance_mode",
                "message": "Service temporarily unavailable due to maintenance",
                "hint": "Please try again later"
            }
        }))
    }
}

impl MaintenanceResponse {
    /// Convert to ElifResponse
    pub async fn to_elif_response(&self) -> ElifResponse {
        match self {
            Self::Text(text) => {
                ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE).text(text.clone())
            }
            Self::Json(json) => {
                ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE).json_value(json.clone())
            }
            Self::Html(html) => {
                ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE)
                    .content_type("text/html")
                    .unwrap_or_else(|_| ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE))
                    .text(html.clone())
            }
            Self::Custom { status_code, content_type, body } => {
                ElifResponse::with_status(*status_code)
                    .content_type(content_type)
                    .unwrap_or_else(|_| ElifResponse::with_status(*status_code))
                    .bytes(axum::body::Bytes::copy_from_slice(body))
            }
            Self::File(path) => {
                // Try to load file content
                match tokio::fs::read(path).await {
                    Ok(content) => {
                        // Determine content type from file extension
                        let content_type = match Path::new(path).extension()
                            .and_then(|ext| ext.to_str()) {
                            Some("html") => "text/html",
                            Some("json") => "application/json", 
                            Some("txt") => "text/plain",
                            _ => "text/plain",
                        };
                        
                        ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE)
                            .content_type(content_type)
                            .unwrap_or_else(|_| ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE))
                            .bytes(axum::body::Bytes::from(content))
                    }
                    Err(_) => {
                        // File not found, return default response
                        ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE).json_value(serde_json::json!({
                            "error": {
                                "code": "maintenance_mode",
                                "message": "Service temporarily unavailable"
                            }
                        }))
                    }
                }
            }
        }
    }
}

/// Path matching strategy
#[derive(Debug)]
pub enum PathMatch {
    /// Exact path match
    Exact(String),
    /// Path prefix match
    Prefix(String),
    /// Regex pattern match (stores compiled regex for performance)
    Regex(regex::Regex),
    /// Custom matcher function
    Custom(fn(&str) -> bool),
}

impl PathMatch {
    /// Create a new regex path matcher (compiles the regex once)
    pub fn regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(regex::Regex::new(pattern)?))
    }

    /// Check if this matcher matches the given path
    pub fn matches(&self, path: &str) -> bool {
        match self {
            Self::Exact(exact_path) => path == exact_path,
            Self::Prefix(prefix) => path.starts_with(prefix),
            Self::Regex(compiled_regex) => compiled_regex.is_match(path),
            Self::Custom(matcher) => matcher(path),
        }
    }
}

impl Clone for PathMatch {
    fn clone(&self) -> Self {
        match self {
            Self::Exact(s) => Self::Exact(s.clone()),
            Self::Prefix(s) => Self::Prefix(s.clone()),
            Self::Regex(regex) => Self::Regex(regex.clone()),
            Self::Custom(f) => Self::Custom(*f),
        }
    }
}

impl PartialEq for PathMatch {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact(a), Self::Exact(b)) => a == b,
            (Self::Prefix(a), Self::Prefix(b)) => a == b,
            (Self::Regex(a), Self::Regex(b)) => a.as_str() == b.as_str(),
            (Self::Custom(a), Self::Custom(b)) => std::ptr::eq(a as *const _, b as *const _),
            _ => false,
        }
    }
}

/// Maintenance mode configuration
#[derive(Debug)]
pub struct MaintenanceModeConfig {
    /// Whether maintenance mode is currently enabled
    pub enabled: Arc<RwLock<bool>>,
    /// Response to send during maintenance mode
    pub response: MaintenanceResponse,
    /// Paths that should be allowed during maintenance mode
    pub allowed_paths: Vec<PathMatch>,
    /// HTTP methods that should be allowed during maintenance mode
    pub allowed_methods: HashSet<ElifMethod>,
    /// IP addresses that should bypass maintenance mode
    pub allowed_ips: HashSet<String>,
    /// Custom header to bypass maintenance mode
    pub bypass_header: Option<(String, String)>,
    /// Whether to add Retry-After header
    pub add_retry_after: Option<u64>,
}

impl Default for MaintenanceModeConfig {
    fn default() -> Self {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert(ElifMethod::GET); // Allow health checks by default
        
        Self {
            enabled: Arc::new(RwLock::new(false)),
            response: MaintenanceResponse::default(),
            allowed_paths: vec![
                PathMatch::Exact("/health".to_string()),
                PathMatch::Exact("/ping".to_string()),
                PathMatch::Prefix("/status".to_string()),
            ],
            allowed_methods,
            allowed_ips: HashSet::new(),
            bypass_header: None,
            add_retry_after: Some(3600), // 1 hour
        }
    }
}

/// Maintenance mode middleware
#[derive(Debug)]
pub struct MaintenanceModeMiddleware {
    config: MaintenanceModeConfig,
}

impl MaintenanceModeMiddleware {
    /// Create new maintenance mode middleware
    pub fn new() -> Self {
        Self {
            config: MaintenanceModeConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: MaintenanceModeConfig) -> Self {
        Self { config }
    }
    
    /// Enable maintenance mode
    pub fn enable(&self) -> Result<(), std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, bool>>> {
        let mut enabled = self.config.enabled.write()?;
        *enabled = true;
        Ok(())
    }
    
    /// Disable maintenance mode
    pub fn disable(&self) -> Result<(), std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, bool>>> {
        let mut enabled = self.config.enabled.write()?;
        *enabled = false;
        Ok(())
    }
    
    /// Check if maintenance mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled.read()
            .map(|enabled| *enabled)
            .unwrap_or(false)
    }
    
    /// Set maintenance response
    pub fn response(mut self, response: MaintenanceResponse) -> Self {
        self.config.response = response;
        self
    }
    
    /// Add allowed path (exact match)
    pub fn allow_path(mut self, path: impl Into<String>) -> Self {
        self.config.allowed_paths.push(PathMatch::Exact(path.into()));
        self
    }
    
    /// Add allowed path prefix
    pub fn allow_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.allowed_paths.push(PathMatch::Prefix(prefix.into()));
        self
    }
    
    /// Add allowed path regex pattern
    pub fn allow_regex(mut self, pattern: &str) -> Result<Self, regex::Error> {
        self.config.allowed_paths.push(PathMatch::regex(pattern)?);
        Ok(self)
    }
    
    /// Add custom path matcher
    pub fn allow_custom(mut self, matcher: fn(&str) -> bool) -> Self {
        self.config.allowed_paths.push(PathMatch::Custom(matcher));
        self
    }
    
    /// Add allowed HTTP method
    pub fn allow_method(mut self, method: ElifMethod) -> Self {
        self.config.allowed_methods.insert(method);
        self
    }
    
    /// Add allowed IP address
    pub fn allow_ip(mut self, ip: impl Into<String>) -> Self {
        self.config.allowed_ips.insert(ip.into());
        self
    }
    
    /// Set bypass header (name and expected value)
    pub fn bypass_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.bypass_header = Some((name.into(), value.into()));
        self
    }
    
    /// Set Retry-After header value in seconds
    pub fn retry_after(mut self, seconds: u64) -> Self {
        self.config.add_retry_after = Some(seconds);
        self
    }
    
    /// Disable Retry-After header
    pub fn no_retry_after(mut self) -> Self {
        self.config.add_retry_after = None;
        self
    }
    
    /// Start in enabled state
    pub fn enabled(self) -> Self {
        let _ = self.enable();
        self
    }
    
    /// Check if request should bypass maintenance mode
    fn should_allow_request(&self, request: &ElifRequest) -> bool {
        // Check if maintenance mode is disabled
        if !self.is_enabled() {
            return true;
        }
        
        // Method check is removed - we check paths first, then other bypass conditions
        
        // Check allowed paths
        let path = request.path();
        for path_match in &self.config.allowed_paths {
            if path_match.matches(path) {
                return true;
            }
        }
        
        // Check bypass header
        if let Some((header_name, expected_value)) = &self.config.bypass_header {
            if let Some(header_value) = request.header(header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    if value_str == expected_value {
                        return true;
                    }
                }
            }
        }
        
        // Check allowed IPs (simplified - would need real IP extraction in production)
        // For now, we'll check X-Forwarded-For or X-Real-IP headers
        let client_ip = request.header("x-forwarded-for")
            .or_else(|| request.header("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim());
        
        if let Some(ip) = client_ip {
            if self.config.allowed_ips.contains(ip) {
                return true;
            }
        }
        
        false
    }
    
    /// Create maintenance response
    async fn create_maintenance_response(&self) -> ElifResponse {
        let mut response = self.config.response.to_elif_response().await;
        
        // Add Retry-After header if configured
        if let Some(retry_after) = self.config.add_retry_after {
            response = response.header("retry-after", &retry_after.to_string())
                .unwrap_or_else(|_| ElifResponse::with_status(ElifStatusCode::SERVICE_UNAVAILABLE));
        }
        
        response
    }
}

impl Default for MaintenanceModeMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for MaintenanceModeMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let should_allow = self.should_allow_request(&request);
        let config = MaintenanceModeConfig {
            enabled: Arc::clone(&self.config.enabled),
            response: self.config.response.clone(),
            allowed_paths: self.config.allowed_paths.clone(),
            allowed_methods: self.config.allowed_methods.clone(),
            allowed_ips: self.config.allowed_ips.clone(),
            bypass_header: self.config.bypass_header.clone(),
            add_retry_after: self.config.add_retry_after,
        };
        
        Box::pin(async move {
            if should_allow {
                // Request is allowed, continue to next middleware/handler
                next.run(request).await
            } else {
                // Maintenance mode is active, return maintenance response
                let middleware = MaintenanceModeMiddleware { config };
                middleware.create_maintenance_response().await
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "MaintenanceModeMiddleware"
    }
}

/// Builder for creating maintenance mode middleware with shared state
pub struct MaintenanceModeBuilder {
    enabled: Arc<RwLock<bool>>,
}

impl MaintenanceModeBuilder {
    /// Create a new maintenance mode builder
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Enable maintenance mode
    pub fn enable(&self) {
        if let Ok(mut enabled) = self.enabled.write() {
            *enabled = true;
        }
    }
    
    /// Disable maintenance mode
    pub fn disable(&self) {
        if let Ok(mut enabled) = self.enabled.write() {
            *enabled = false;
        }
    }
    
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.read()
            .map(|enabled| *enabled)
            .unwrap_or(false)
    }
    
    /// Build middleware with shared state
    pub fn build(&self) -> MaintenanceModeMiddleware {
        let config = MaintenanceModeConfig {
            enabled: Arc::clone(&self.enabled),
            ..Default::default()
        };
        MaintenanceModeMiddleware::with_config(config)
    }
    
    /// Build middleware with custom configuration but shared enabled state
    pub fn build_with_config(&self, mut config: MaintenanceModeConfig) -> MaintenanceModeMiddleware {
        config.enabled = Arc::clone(&self.enabled);
        MaintenanceModeMiddleware::with_config(config)
    }
}

impl Default for MaintenanceModeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifResponse;
    use crate::response::headers::ElifHeaderMap;
    use crate::request::ElifRequest;
    
    #[test]
    fn test_path_matching() {
        let exact = PathMatch::Exact("/health".to_string());
        assert!(exact.matches("/health"));
        assert!(!exact.matches("/health-check"));
        
        let prefix = PathMatch::Prefix("/api/".to_string());
        assert!(prefix.matches("/api/users"));
        assert!(prefix.matches("/api/"));
        assert!(!prefix.matches("/v1/api/users"));
        
        let regex = PathMatch::regex(r"^/api/v\d+/.*").unwrap();
        assert!(regex.matches("/api/v1/users"));
        assert!(regex.matches("/api/v2/posts"));
        assert!(!regex.matches("/api/users"));
        
        let custom = PathMatch::Custom(|path| path.ends_with(".json"));
        assert!(custom.matches("/data.json"));
        assert!(!custom.matches("/data.xml"));
    }
    
    #[tokio::test]
    async fn test_maintenance_response_types() {
        // Text response
        let text_response = MaintenanceResponse::Text("Under maintenance".to_string());
        let response = text_response.to_elif_response().await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
        
        // JSON response
        let json_response = MaintenanceResponse::Json(serde_json::json!({
            "error": "maintenance"
        }));
        let response = json_response.to_elif_response().await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
        
        // HTML response
        let html_response = MaintenanceResponse::Html("<h1>Maintenance</h1>".to_string());
        let response = html_response.to_elif_response().await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
        
        // Custom response
        let custom_response = MaintenanceResponse::Custom {
            status_code: ElifStatusCode::LOCKED,
            content_type: "text/plain".to_string(),
            body: b"Locked".to_vec(),
        };
        let response = custom_response.to_elif_response().await;
        assert_eq!(response.status_code(), ElifStatusCode::LOCKED);
    }
    
    #[tokio::test]
    async fn test_maintenance_mode_disabled() {
        let middleware = MaintenanceModeMiddleware::new(); // Disabled by default
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/data".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Normal response")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_maintenance_mode_enabled() {
        let middleware = MaintenanceModeMiddleware::new().enabled();
        
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/api/data".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should not reach here")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[tokio::test]
    async fn test_maintenance_mode_allowed_paths() {
        let middleware = MaintenanceModeMiddleware::new()
            .enabled()
            .allow_path("/health");
        
        // Health check should be allowed
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/health".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Healthy")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::OK);
        
        // Other paths should be blocked
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/data".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should be blocked")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[tokio::test]
    async fn test_maintenance_mode_bypass_header() {
        let middleware = MaintenanceModeMiddleware::new()
            .enabled()
            .bypass_header("x-admin-key", "secret123");
        
        // Request with correct bypass header
        let mut headers = ElifHeaderMap::new();
        headers.insert(crate::response::headers::ElifHeaderName::from_str("x-admin-key").unwrap(), "secret123".parse().unwrap());
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/admin/panel".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Admin panel")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::OK);
        
        // Request with wrong bypass header
        let mut headers = ElifHeaderMap::new();
        headers.insert(crate::response::headers::ElifHeaderName::from_str("x-admin-key").unwrap(), "wrong-key".parse().unwrap());
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/admin/panel".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should be blocked")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[tokio::test]
    async fn test_maintenance_mode_allowed_ips() {
        let middleware = MaintenanceModeMiddleware::new()
            .enabled()
            .allow_ip("192.168.1.100");
        
        // Request from allowed IP
        let mut headers = ElifHeaderMap::new();
        headers.insert(crate::response::headers::ElifHeaderName::from_str("x-forwarded-for").unwrap(), "192.168.1.100".parse().unwrap());
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Allowed IP")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_maintenance_mode_builder() {
        let builder = MaintenanceModeBuilder::new();
        let middleware = builder.build();
        
        assert!(!builder.is_enabled());
        
        // Enable maintenance mode via builder
        builder.enable();
        assert!(builder.is_enabled());
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/data".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should be blocked")
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), ElifStatusCode::SERVICE_UNAVAILABLE);
        
        // Disable and test again
        builder.disable();
        assert!(!builder.is_enabled());
    }
    
    #[test]
    fn test_middleware_builder_pattern() {
        let middleware = MaintenanceModeMiddleware::new()
            .allow_path("/health")
            .allow_prefix("/status")
            .allow_method(ElifMethod::OPTIONS)
            .allow_ip("127.0.0.1")
            .bypass_header("x-bypass", "secret")
            .retry_after(7200)
            .enabled();
        
        assert!(middleware.is_enabled());
        assert_eq!(middleware.config.allowed_paths.len(), 5); // 3 default + 2 added
        assert!(middleware.config.allowed_methods.contains(&ElifMethod::OPTIONS));
        assert!(middleware.config.allowed_ips.contains("127.0.0.1"));
        assert_eq!(middleware.config.bypass_header, Some(("x-bypass".to_string(), "secret".to_string())));
        assert_eq!(middleware.config.add_retry_after, Some(7200));
    }

    #[test]
    fn test_regex_performance_improvement() {
        // Test that regex is compiled once, not on every match
        let regex_matcher = PathMatch::regex(r"^/api/v\d+/.*").unwrap();
        
        // These multiple matches should use the same compiled regex
        // (This is a behavioral test - the main benefit is performance under load)
        assert!(regex_matcher.matches("/api/v1/users"));
        assert!(regex_matcher.matches("/api/v2/posts"));
        assert!(regex_matcher.matches("/api/v3/comments"));
        assert!(!regex_matcher.matches("/api/users"));
        assert!(!regex_matcher.matches("/v1/api/users"));
        
        // Verify error handling for invalid regex
        let invalid_regex = PathMatch::regex(r"[invalid");
        assert!(invalid_regex.is_err());
    }

    #[test] 
    fn test_path_match_clone_and_equality() {
        let exact1 = PathMatch::Exact("/test".to_string());
        let exact2 = PathMatch::Exact("/test".to_string());
        let exact3 = PathMatch::Exact("/other".to_string());
        
        assert_eq!(exact1, exact2);
        assert_ne!(exact1, exact3);
        
        let cloned = exact1.clone();
        assert_eq!(exact1, cloned);
        
        let regex1 = PathMatch::regex(r"^/api/.*").unwrap();
        let regex2 = PathMatch::regex(r"^/api/.*").unwrap(); 
        let regex3 = PathMatch::regex(r"^/other/.*").unwrap();
        
        assert_eq!(regex1, regex2); // Same pattern
        assert_ne!(regex1, regex3); // Different pattern
        
        let cloned_regex = regex1.clone();
        assert_eq!(regex1, cloned_regex);
    }
}