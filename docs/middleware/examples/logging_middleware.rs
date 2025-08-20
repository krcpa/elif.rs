use elif::prelude::*;
use elif_http::middleware::v2::{Middleware, Next, NextFuture};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;
use std::time::Instant;
use serde_json::json;

/// Advanced Logging Middleware
/// 
/// This middleware provides comprehensive request/response logging with:
/// - Request timing
/// - Request/response size tracking  
/// - Custom log levels
/// - Structured JSON output
/// - Error tracking
#[derive(Debug)]
pub struct AdvancedLoggingMiddleware {
    log_level: LogLevel,
    include_headers: bool,
    include_body: bool,
    log_format: LogFormat,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}

impl AdvancedLoggingMiddleware {
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::Info,
            include_headers: false,
            include_body: false,
            log_format: LogFormat::Pretty,
        }
    }
    
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }
    
    pub fn include_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }
    
    pub fn include_body(mut self, include: bool) -> Self {
        self.include_body = include;
        self
    }
    
    pub fn json_format(mut self) -> Self {
        self.log_format = LogFormat::Json;
        self
    }
    
    pub fn compact_format(mut self) -> Self {
        self.log_format = LogFormat::Compact;
        self
    }
    
    fn should_log(&self, status: u16) -> bool {
        match self.log_level {
            LogLevel::Debug => true,
            LogLevel::Info => true,
            LogLevel::Warn => status >= 400,
            LogLevel::Error => status >= 500,
        }
    }
    
    fn format_log_entry(&self, entry: &LogEntry) -> String {
        match self.log_format {
            LogFormat::Pretty => self.format_pretty(entry),
            LogFormat::Json => self.format_json(entry),
            LogFormat::Compact => self.format_compact(entry),
        }
    }
    
    fn format_pretty(&self, entry: &LogEntry) -> String {
        let status_icon = match entry.status {
            200..=299 => "✅",
            300..=399 => "↗️",
            400..=499 => "❌",
            500..=599 => "💥",
            _ => "❓",
        };
        
        let mut log = format!(
            "{} {} {} → {} ({:?})",
            status_icon,
            entry.method,
            entry.path,
            entry.status,
            entry.duration
        );
        
        if let Some(size) = entry.response_size {
            log.push_str(&format!(" [{}B]", size));
        }
        
        if let Some(ref error) = entry.error {
            log.push_str(&format!(" ERROR: {}", error));
        }
        
        log
    }
    
    fn format_json(&self, entry: &LogEntry) -> String {
        let mut json_obj = json!({
            "timestamp": entry.timestamp.to_rfc3339(),
            "method": entry.method,
            "path": entry.path,
            "status": entry.status,
            "duration_ms": entry.duration.as_millis(),
        });
        
        if let Some(size) = entry.response_size {
            json_obj["response_size"] = json!(size);
        }
        
        if let Some(ref error) = entry.error {
            json_obj["error"] = json!(error);
        }
        
        if self.include_headers && !entry.headers.is_empty() {
            json_obj["headers"] = json!(entry.headers);
        }
        
        json_obj.to_string()
    }
    
    fn format_compact(&self, entry: &LogEntry) -> String {
        format!(
            "{} {} {} {}ms",
            entry.method,
            entry.path,
            entry.status,
            entry.duration.as_millis()
        )
    }
}

impl Middleware for AdvancedLoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let include_headers = self.include_headers;
        let include_body = self.include_body;
        let log_format = self.log_format.clone();
        let log_level = self.log_level.clone();
        
        Box::pin(async move {
            let start = Instant::now();
            let method = request.method.to_string();
            let path = request.path().to_string();
            
            // Collect request headers if needed
            let headers = if include_headers {
                collect_headers(&request)
            } else {
                std::collections::HashMap::new()
            };
            
            // Log incoming request
            if matches!(log_level, LogLevel::Debug) {
                println!("📥 {} {}", method, path);
            }
            
            // Process request
            let response = next.run(request).await;
            let duration = start.elapsed();
            
            // Create log entry
            let entry = LogEntry {
                timestamp: chrono::Utc::now(),
                method: method.clone(),
                path: path.clone(),
                status: response.status_code().as_u16(),
                duration,
                response_size: None, // TODO: Calculate response size
                headers,
                error: None,
            };
            
            // Log if should log based on level and status
            let should_log_entry = should_log_entry(&log_level, entry.status);
            if should_log_entry {
                let formatted = format_log_entry(&log_format, &entry);
                println!("{}", formatted);
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "AdvancedLoggingMiddleware"
    }
}

#[derive(Debug)]
struct LogEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    method: String,
    path: String,
    status: u16,
    duration: std::time::Duration,
    response_size: Option<usize>,
    headers: std::collections::HashMap<String, String>,
    error: Option<String>,
}

// Helper functions
fn collect_headers(request: &ElifRequest) -> std::collections::HashMap<String, String> {
    let mut headers = std::collections::HashMap::new();
    
    // Add common headers we care about
    if let Some(user_agent) = request.header("user-agent") {
        if let Ok(ua_str) = user_agent.to_str() {
            headers.insert("user_agent".to_string(), ua_str.to_string());
        }
    }
    
    if let Some(content_type) = request.header("content-type") {
        if let Ok(ct_str) = content_type.to_str() {
            headers.insert("content_type".to_string(), ct_str.to_string());
        }
    }
    
    headers
}

fn should_log_entry(log_level: &LogLevel, status: u16) -> bool {
    match log_level {
        LogLevel::Debug => true,
        LogLevel::Info => true,
        LogLevel::Warn => status >= 400,
        LogLevel::Error => status >= 500,
    }
}

fn format_log_entry(format: &LogFormat, entry: &LogEntry) -> String {
    match format {
        LogFormat::Pretty => format_pretty_entry(entry),
        LogFormat::Json => format_json_entry(entry),
        LogFormat::Compact => format_compact_entry(entry),
    }
}

fn format_pretty_entry(entry: &LogEntry) -> String {
    let status_icon = match entry.status {
        200..=299 => "✅",
        300..=399 => "↗️",
        400..=499 => "❌",
        500..=599 => "💥",
        _ => "❓",
    };
    
    format!(
        "{} {} {} → {} ({:?})",
        status_icon,
        entry.method,
        entry.path,
        entry.status,
        entry.duration
    )
}

fn format_json_entry(entry: &LogEntry) -> String {
    json!({
        "timestamp": entry.timestamp.to_rfc3339(),
        "method": entry.method,
        "path": entry.path,
        "status": entry.status,
        "duration_ms": entry.duration.as_millis(),
    }).to_string()
}

fn format_compact_entry(entry: &LogEntry) -> String {
    format!(
        "{} {} {} {}ms",
        entry.method,
        entry.path,
        entry.status,
        entry.duration.as_millis()
    )
}

// Preset configurations
impl AdvancedLoggingMiddleware {
    /// Development configuration with detailed logging
    pub fn development() -> Self {
        Self::new()
            .log_level(LogLevel::Debug)
            .include_headers(true)
            .include_body(false) // Usually too verbose for development
    }
    
    /// Production configuration with minimal logging
    pub fn production() -> Self {
        Self::new()
            .log_level(LogLevel::Warn)
            .json_format()
            .include_headers(false)
            .include_body(false)
    }
    
    /// Debugging configuration with maximum verbosity
    pub fn debug() -> Self {
        Self::new()
            .log_level(LogLevel::Debug)
            .include_headers(true)
            .include_body(true)
    }
}

// Usage examples
#[allow(dead_code)]
fn usage_examples() -> Result<(), Box<dyn std::error::Error>> {
    use elif_http::{Server, HttpConfig};
    use elif_core::Container;
    
    let container = Container::new();
    let mut server = Server::new(container, HttpConfig::default())?;
    
    // Basic logging
    server.use_middleware(AdvancedLoggingMiddleware::new());
    
    // Development logging with headers
    server.use_middleware(
        AdvancedLoggingMiddleware::development()
    );
    
    // Production logging (JSON format, errors only)
    server.use_middleware(
        AdvancedLoggingMiddleware::production()
    );
    
    // Custom configuration
    server.use_middleware(
        AdvancedLoggingMiddleware::new()
            .log_level(LogLevel::Info)
            .json_format()
            .include_headers(true)
    );
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::request::{ElifMethod, ElifRequest};
    use elif_http::response::headers::ElifHeaderMap;
    use elif_http::middleware::v2::MiddlewarePipelineV2;
    
    #[tokio::test]
    async fn test_logging_middleware_basic() {
        let middleware = AdvancedLoggingMiddleware::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().text("Test response")
            })
        }).await;
        
        assert_eq!(response.status_code(), elif_http::response::status::ElifStatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_logging_middleware_with_headers() {
        let middleware = AdvancedLoggingMiddleware::new().include_headers(true);
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        let mut headers = ElifHeaderMap::new();
        headers.insert("user-agent".parse().unwrap(), "test-agent".parse().unwrap());
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/api/test".parse().unwrap(),
            headers,
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().json_value(json!({"result": "success"}))
            })
        }).await;
        
        assert_eq!(response.status_code(), elif_http::response::status::ElifStatusCode::OK);
    }
}