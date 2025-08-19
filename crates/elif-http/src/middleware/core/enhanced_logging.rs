//! # Enhanced Logging Middleware
//!
//! Production-ready logging middleware with structured logging, correlation IDs, 
//! and request tracing using V2 middleware system.

use std::time::{Instant, Duration};
use std::collections::HashMap;
use uuid::Uuid;

use tracing::{info, warn, error};
use serde_json::{json, Value};

use crate::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
};

/// Enhanced logging middleware with structured logging and request tracing
#[derive(Debug, Clone)]
pub struct EnhancedLoggingMiddleware {
    config: LoggingConfig,
}

/// Configuration for enhanced logging middleware  
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Enable structured JSON logging
    pub structured: bool,
    /// Log request body (be careful with sensitive data)
    pub log_request_body: bool,
    /// Log response body (be careful with sensitive data) 
    pub log_response_body: bool,
    /// Log request headers (sensitive headers are always filtered)
    pub log_request_headers: bool,
    /// Log response headers
    pub log_response_headers: bool,
    /// Enable request correlation ID tracking
    pub correlation_ids: bool,
    /// Enable request tracing spans
    pub tracing_spans: bool,
    /// Slow request threshold (requests slower than this will be logged as warnings)
    pub slow_request_threshold: Duration,
    /// Custom header name for correlation ID (defaults to "X-Correlation-ID")
    pub correlation_header: String,
    /// Additional custom fields to include in structured logs
    pub custom_fields: HashMap<String, String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            structured: true,
            log_request_body: false,
            log_response_body: false,
            log_request_headers: true,
            log_response_headers: false,
            correlation_ids: true,
            tracing_spans: true,
            slow_request_threshold: Duration::from_millis(1000),
            correlation_header: "X-Correlation-ID".to_string(),
            custom_fields: HashMap::new(),
        }
    }
}

/// Request context for enhanced logging
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub correlation_id: String,
    pub start_time: Instant,
    pub method: String,
    pub path: String,
    pub user_agent: Option<String>,
    pub remote_addr: Option<String>,
}

impl EnhancedLoggingMiddleware {
    /// Create new enhanced logging middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: LoggingConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: LoggingConfig) -> Self {
        Self { config }
    }
    
    /// Builder pattern: Enable structured JSON logging
    pub fn structured(mut self, enabled: bool) -> Self {
        self.config.structured = enabled;
        self
    }
    
    /// Builder pattern: Enable request body logging
    pub fn log_request_body(mut self, enabled: bool) -> Self {
        self.config.log_request_body = enabled;
        self
    }
    
    /// Builder pattern: Enable response body logging
    pub fn log_response_body(mut self, enabled: bool) -> Self {
        self.config.log_response_body = enabled;
        self
    }
    
    /// Builder pattern: Enable correlation ID tracking
    pub fn correlation_ids(mut self, enabled: bool) -> Self {
        self.config.correlation_ids = enabled;
        self
    }
    
    /// Builder pattern: Set slow request threshold
    pub fn slow_request_threshold(mut self, threshold: Duration) -> Self {
        self.config.slow_request_threshold = threshold;
        self
    }
    
    /// Builder pattern: Enable request header logging
    pub fn log_request_headers(mut self, enabled: bool) -> Self {
        self.config.log_request_headers = enabled;
        self
    }
    
    /// Builder pattern: Add custom field for structured logging
    pub fn with_custom_field<K, V>(mut self, key: K, value: V) -> Self 
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.config.custom_fields.insert(key.into(), value.into());
        self
    }
}

impl Default for EnhancedLoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for EnhancedLoggingMiddleware {
    fn handle(&self, mut request: ElifRequest, next: Next) -> NextFuture<'static> {
        let config = self.config.clone();
        Box::pin(async move {
            let context = {
                let correlation_id = if !config.correlation_ids {
                    "disabled".to_string()
                } else {
                    // Try to get existing correlation ID from headers
                    if let Some(header_value) = request.headers.get_str(&config.correlation_header) {
                        if let Ok(correlation_id) = header_value.to_str() {
                            if !correlation_id.is_empty() && correlation_id.len() <= 64 {
                                correlation_id.to_string()
                            } else {
                                Uuid::new_v4().to_string()
                            }
                        } else {
                            Uuid::new_v4().to_string()
                        }
                    } else {
                        Uuid::new_v4().to_string()
                    }
                };
                
                let method = request.method.to_string();
                let path = request.uri.path().to_string();
                
                let user_agent = request.headers
                    .get_str("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .map(String::from);
                
                let remote_addr = request.headers
                    .get_str("x-forwarded-for")
                    .or_else(|| request.headers.get_str("x-real-ip"))
                    .and_then(|h| h.to_str().ok())
                    .map(String::from);
                
                RequestContext {
                    correlation_id,
                    start_time: Instant::now(),
                    method,
                    path,
                    user_agent,
                    remote_addr,
                }
            };
            
            // Log the incoming request
            if config.structured {
                let mut log_data = json!({
                    "event": "request_start",
                    "correlation_id": context.correlation_id,
                    "method": context.method,
                    "path": context.path,
                    "user_agent": context.user_agent,
                    "remote_addr": context.remote_addr,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                
                // Add custom fields
                for (key, value) in &config.custom_fields {
                    log_data[key] = Value::String(value.clone());
                }
                
                // Add headers if enabled
                if config.log_request_headers {
                    let mut headers = HashMap::new();
                    for name in request.headers.keys() {
                        if let Some(value) = request.headers.get_str(name.as_str()) {
                            if let Ok(value_str) = value.to_str() {
                                if !is_sensitive_header(name.as_str()) {
                                    headers.insert(name.as_str().to_string(), value_str.to_string());
                                } else {
                                    headers.insert(name.as_str().to_string(), "[REDACTED]".to_string());
                                }
                            }
                        }
                    }
                    log_data["headers"] = json!(headers);
                }
                
                info!(target: "elif::http::request", "{}", log_data);
            } else {
                info!(
                    "→ {} {} [{}] from {}",
                    context.method,
                    context.path,
                    context.correlation_id,
                    context.remote_addr.as_deref().unwrap_or("unknown")
                );
            }
            
            // Add correlation ID header for response processing
            if config.correlation_ids && context.correlation_id != "disabled" {
                if let Err(e) = request.headers.add_header("x-elif-correlation-id", &context.correlation_id) {
                    warn!("Failed to add correlation ID header: {}", e);
                }
            }
            
            // Continue to next middleware/handler
            let mut response = next.run(request).await;
            
            // Calculate duration and log response
            let duration = context.start_time.elapsed();
            let status = response.status_code();
            let duration_ms = duration.as_millis();
            
            let is_slow = duration > config.slow_request_threshold;
            let is_error = status.is_client_error() || status.is_server_error();
            
            if config.structured {
                let mut log_data = json!({
                    "event": "request_complete",
                    "correlation_id": context.correlation_id,
                    "method": context.method,
                    "path": context.path,
                    "status_code": status.as_u16(),
                    "duration_ms": duration_ms,
                    "is_slow": is_slow,
                    "is_error": is_error,
                    "user_agent": context.user_agent,
                    "remote_addr": context.remote_addr,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                
                // Add custom fields
                for (key, value) in &config.custom_fields {
                    log_data[key] = Value::String(value.clone());
                }
                
                // Add response headers if enabled
                if config.log_response_headers {
                    let mut headers = HashMap::new();
                    for (name, value) in response.headers().iter() {
                        if let Ok(value_str) = value.to_str() {
                            headers.insert(name.as_str().to_string(), value_str.to_string());
                        }
                    }
                    log_data["response_headers"] = json!(headers);
                }
                
                // Log at appropriate level
                if is_error {
                    error!(target: "elif::http::response", "{}", log_data);
                } else if is_slow {
                    warn!(target: "elif::http::response", "Slow request: {}", log_data);
                } else {
                    info!(target: "elif::http::response", "{}", log_data);
                }
            } else {
                let log_msg = format!(
                    "← {:?} {} [{}] {}ms",
                    status,
                    context.path,
                    context.correlation_id,
                    duration_ms
                );
                
                if is_error {
                    error!("{}", log_msg);
                } else if is_slow {
                    warn!("SLOW: {}", log_msg);
                } else {
                    info!("{}", log_msg);
                }
            }
            
            // Add correlation ID to response headers if enabled
            if config.correlation_ids && context.correlation_id != "disabled" {
                if let Err(e) = response.add_header(&config.correlation_header, &context.correlation_id) {
                    warn!("Failed to add correlation ID to response: {}", e);
                }
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "EnhancedLoggingMiddleware"
    }
}

/// Check if a header name contains sensitive information
fn is_sensitive_header(name: &str) -> bool {
    let sensitive_headers = [
        "authorization",
        "cookie",
        "set-cookie",
        "x-api-key",
        "x-auth-token",
        "x-csrf-token", 
        "x-access-token",
        "bearer",
        "basic",
        "digest",
        "negotiate",
        "oauth",
        "jwt",
        "session",
        "password",
        "secret",
        "key",
        "token",
    ];
    
    let name_lower = name.to_lowercase();
    sensitive_headers.iter().any(|&sensitive| {
        name_lower.contains(sensitive)
    })
}

/// Convenience builder for common logging configurations
impl EnhancedLoggingMiddleware {
    /// Development configuration: verbose logging, correlation IDs, no body logging
    pub fn development() -> Self {
        Self::new()
            .structured(false)
            .correlation_ids(true)
            .slow_request_threshold(Duration::from_millis(500))
            .with_custom_field("env", "development")
    }
    
    /// Production configuration: structured logging, correlation IDs, minimal verbosity
    pub fn production() -> Self {
        Self::new()
            .structured(true)
            .log_request_body(false)
            .log_response_body(false)
            .correlation_ids(true)
            .slow_request_threshold(Duration::from_millis(2000))
            .with_custom_field("env", "production")
    }
    
    /// Debug configuration: maximum verbosity for troubleshooting
    pub fn debug() -> Self {
        Self::new()
            .structured(true)
            .log_request_body(true)
            .log_response_body(true)
            .log_request_headers(true)
            .correlation_ids(true)
            .slow_request_threshold(Duration::from_millis(100))
            .with_custom_field("env", "debug")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::v2::MiddlewarePipelineV2;
    use crate::request::{ElifRequest, ElifMethod};
    use crate::response::{ElifResponse, ElifStatusCode, ElifHeaderMap};
    
    #[test]
    fn test_sensitive_header_detection() {
        assert!(is_sensitive_header("Authorization"));
        assert!(is_sensitive_header("AUTHORIZATION"));
        assert!(is_sensitive_header("x-api-key"));
        assert!(is_sensitive_header("Cookie"));
        assert!(is_sensitive_header("Bearer-Token"));
        assert!(is_sensitive_header("JWT-Token"));
        
        assert!(!is_sensitive_header("Content-Type"));
        assert!(!is_sensitive_header("User-Agent"));
        assert!(!is_sensitive_header("Accept"));
        assert!(!is_sensitive_header("X-Forwarded-For"));
    }
    
    #[test]
    fn test_logging_config_builder() {
        let config = LoggingConfig::default();
        assert!(config.structured);
        assert!(config.correlation_ids);
        assert!(!config.log_request_body);
        assert_eq!(config.correlation_header, "X-Correlation-ID");
    }
    
    #[tokio::test]
    async fn test_enhanced_logging_middleware_v2() {
        let middleware = EnhancedLoggingMiddleware::development();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("content-type".parse().unwrap(), "application/json".parse().unwrap());
        headers.insert("user-agent".parse().unwrap(), "test-client/1.0".parse().unwrap());
        headers.insert("authorization".parse().unwrap(), "Bearer secret-token".parse().unwrap());
        headers.insert("x-correlation-id".parse().unwrap(), "existing-correlation-123".parse().unwrap());
        
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/api/users".parse().unwrap(),
            headers,
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Created user")
            })
        }).await;
        
        // Should complete successfully
        assert_eq!(response.status_code(), ElifStatusCode::OK);
        
        // Should have correlation ID in response headers
        assert!(response.has_header("X-Correlation-ID"));
    }
    
    #[test]
    fn test_preset_configurations() {
        let dev = EnhancedLoggingMiddleware::development();
        assert!(!dev.config.structured);
        assert_eq!(dev.config.custom_fields.get("env").unwrap(), "development");
        
        let prod = EnhancedLoggingMiddleware::production();
        assert!(prod.config.structured);
        assert!(!prod.config.log_request_body);
        assert_eq!(prod.config.custom_fields.get("env").unwrap(), "production");
        
        let debug = EnhancedLoggingMiddleware::debug();
        assert!(debug.config.structured);
        assert!(debug.config.log_request_body);
        assert!(debug.config.log_response_body);
        assert_eq!(debug.config.custom_fields.get("env").unwrap(), "debug");
    }
}