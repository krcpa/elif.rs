use elif::prelude::*;
use elif_http::middleware::v2::{Middleware, Next, NextFuture, MiddlewarePipelineV2};
use elif_http::middleware::v2::{composition, factories, ConditionalMiddleware};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;
use serde_json::json;
use std::sync::Arc;

/// This file demonstrates various middleware composition patterns
/// and how to build complex middleware pipelines

// Example: Security-focused middleware stack
pub fn security_stack() -> MiddlewarePipelineV2 {
    composition::compose4(
        factories::cors_with_origins(vec!["https://example.com".to_string()]),
        factories::rate_limit(100),
        SecurityHeadersMiddleware::new(),
        CSRFMiddleware::new("csrf-secret".to_string()),
    )
}

// Example: API middleware stack with authentication and logging
pub fn api_stack() -> MiddlewarePipelineV2 {
    MiddlewarePipelineV2::new()
        .add(factories::profiler()) // Request timing
        .add(LoggingMiddleware::json_format()) // Structured logging
        .add(ConditionalMiddleware::new(AuthMiddleware::new("api-key".to_string()))
            .skip_paths(vec!["/health".to_string(), "/metrics".to_string()]))
        .add(factories::body_limit(1024 * 1024)) // 1MB body limit
        .add(RateLimitingMiddleware::new(1000)) // API rate limiting
}

// Example: Development middleware stack
pub fn development_stack() -> MiddlewarePipelineV2 {
    MiddlewarePipelineV2::new()
        .add(CorsMiddleware::permissive()) // Allow all origins for dev
        .add(LoggingMiddleware::development()) // Verbose logging
        .add(DevErrorMiddleware::new()) // Detailed error responses
        .add(HotReloadMiddleware::new()) // Auto-reload on changes
}

// Example: Production middleware stack
pub fn production_stack() -> MiddlewarePipelineV2 {
    MiddlewarePipelineV2::new()
        .add(SecurityHeadersMiddleware::strict())
        .add(factories::cors_with_origins(vec!["https://yourdomain.com".to_string()]))
        .add(RateLimitingMiddleware::new(60)) // Stricter rate limiting
        .add(LoggingMiddleware::production()) // JSON logging, errors only
        .add(CompressionMiddleware::new())
        .add(CachingMiddleware::new(std::time::Duration::from_secs(300)))
}

// Example: Custom middleware composition utility
pub struct MiddlewareBuilder {
    pipeline: MiddlewarePipelineV2,
}

impl MiddlewareBuilder {
    pub fn new() -> Self {
        Self {
            pipeline: MiddlewarePipelineV2::new(),
        }
    }
    
    pub fn security(mut self) -> Self {
        self.pipeline = self.pipeline
            .add(SecurityHeadersMiddleware::new())
            .add(CSRFMiddleware::new("default-secret".to_string()));
        self
    }
    
    pub fn cors(mut self, origins: Vec<String>) -> Self {
        self.pipeline = self.pipeline.add(factories::cors_with_origins(origins));
        self
    }
    
    pub fn auth(mut self, secret: String) -> Self {
        self.pipeline = self.pipeline.add(AuthMiddleware::new(secret));
        self
    }
    
    pub fn logging(mut self, format: LogFormat) -> Self {
        let middleware = match format {
            LogFormat::Json => LoggingMiddleware::json_format(),
            LogFormat::Pretty => LoggingMiddleware::development(),
            LogFormat::Compact => LoggingMiddleware::compact(),
        };
        self.pipeline = self.pipeline.add(middleware);
        self
    }
    
    pub fn rate_limit(mut self, requests_per_minute: u32) -> Self {
        self.pipeline = self.pipeline.add(factories::rate_limit(requests_per_minute));
        self
    }
    
    pub fn build(self) -> MiddlewarePipelineV2 {
        self.pipeline
    }
}

// Example usage of the builder pattern
#[allow(dead_code)]
fn builder_example() -> MiddlewarePipelineV2 {
    MiddlewareBuilder::new()
        .security()
        .cors(vec!["https://example.com".to_string()])
        .auth("api-secret".to_string())
        .logging(LogFormat::Json)
        .rate_limit(100)
        .build()
}

// Example: Conditional middleware composition
pub fn conditional_composition_example() -> MiddlewarePipelineV2 {
    MiddlewarePipelineV2::new()
        // Always apply CORS
        .add(factories::cors())
        
        // Only apply auth to protected paths
        .add(ConditionalMiddleware::new(AuthMiddleware::new("secret".to_string()))
            .skip_paths(vec![
                "/health".to_string(),
                "/public/*".to_string(),
                "/auth/login".to_string(),
            ]))
        
        // Only apply rate limiting to API endpoints  
        .add(ConditionalMiddleware::new(RateLimitingMiddleware::new(60))
            .skip_paths(vec!["/health".to_string()])
            .only_methods(vec![
                elif_http::request::ElifMethod::POST,
                elif_http::request::ElifMethod::PUT,
                elif_http::request::ElifMethod::DELETE,
            ]))
}

// Example: Environment-specific composition
pub fn environment_specific_stack(env: &str) -> MiddlewarePipelineV2 {
    match env {
        "development" => development_stack(),
        "production" => production_stack(),
        "testing" => testing_stack(),
        _ => default_stack(),
    }
}

fn testing_stack() -> MiddlewarePipelineV2 {
    MiddlewarePipelineV2::new()
        .add(LoggingMiddleware::compact()) // Minimal logging for tests
        .add(TestingMiddleware::new()) // Special middleware for tests
}

fn default_stack() -> MiddlewarePipelineV2 {
    MiddlewarePipelineV2::new()
        .add(factories::cors())
        .add(LoggingMiddleware::development())
}

// Example middleware implementations for composition examples
#[derive(Debug)]
pub struct SecurityHeadersMiddleware {
    strict_mode: bool,
}

impl SecurityHeadersMiddleware {
    pub fn new() -> Self {
        Self { strict_mode: false }
    }
    
    pub fn strict() -> Self {
        Self { strict_mode: true }
    }
}

impl Middleware for SecurityHeadersMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let strict = self.strict_mode;
        Box::pin(async move {
            let mut response = next.run(request).await;
            
            // Add security headers
            let _ = response.add_header("X-Frame-Options", "DENY");
            let _ = response.add_header("X-Content-Type-Options", "nosniff");
            let _ = response.add_header("X-XSS-Protection", "1; mode=block");
            
            if strict {
                let _ = response.add_header(
                    "Content-Security-Policy", 
                    "default-src 'self'; script-src 'self' 'unsafe-inline'"
                );
                let _ = response.add_header(
                    "Strict-Transport-Security", 
                    "max-age=31536000; includeSubDomains"
                );
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "SecurityHeadersMiddleware"
    }
}

#[derive(Debug)]
pub struct CSRFMiddleware {
    secret: String,
}

impl CSRFMiddleware {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl Middleware for CSRFMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Skip CSRF for GET requests
            if matches!(request.method, elif_http::request::ElifMethod::GET) {
                return next.run(request).await;
            }
            
            // Validate CSRF token for state-changing requests
            let csrf_token = request.header("X-CSRF-Token")
                .and_then(|h| h.to_str().ok());
            
            match csrf_token {
                Some(_token) => {
                    // In real implementation, validate the token
                    next.run(request).await
                }
                None => {
                    ElifResponse::with_status(elif_http::response::status::ElifStatusCode::FORBIDDEN)
                        .json_value(json!({
                            "error": {
                                "code": "missing_csrf_token",
                                "message": "CSRF token required"
                            }
                        }))
                }
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "CSRFMiddleware"
    }
}

#[derive(Debug)]
pub struct AuthMiddleware {
    secret: String,
}

impl AuthMiddleware {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl Middleware for AuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let secret = self.secret.clone();
        Box::pin(async move {
            // Simple token validation
            let token = request.header("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.strip_prefix("Bearer "));
            
            match token {
                Some(t) if t == secret => next.run(request).await,
                _ => ElifResponse::unauthorized()
                    .json_value(json!({
                        "error": {
                            "code": "unauthorized",
                            "message": "Invalid or missing token"
                        }
                    }))
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "AuthMiddleware"
    }
}

#[derive(Debug)]
pub struct RateLimitingMiddleware {
    requests_per_minute: u32,
}

impl RateLimitingMiddleware {
    pub fn new(requests_per_minute: u32) -> Self {
        Self { requests_per_minute }
    }
}

impl Middleware for RateLimitingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Simplified rate limiting - in production use Redis or similar
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "RateLimitingMiddleware"
    }
}

#[derive(Debug)]
pub struct LoggingMiddleware {
    format: LogFormat,
}

#[derive(Debug, Clone)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

impl LoggingMiddleware {
    pub fn json_format() -> Self {
        Self { format: LogFormat::Json }
    }
    
    pub fn development() -> Self {
        Self { format: LogFormat::Pretty }
    }
    
    pub fn production() -> Self {
        Self { format: LogFormat::Json }
    }
    
    pub fn compact() -> Self {
        Self { format: LogFormat::Compact }
    }
}

impl Middleware for LoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let start = std::time::Instant::now();
            let method = request.method.to_string();
            let path = request.path().to_string();
            
            let response = next.run(request).await;
            
            let duration = start.elapsed();
            let status = response.status_code().as_u16();
            
            // Log based on format
            match LogFormat::Json {
                LogFormat::Json => {
                    println!("{}", json!({
                        "method": method,
                        "path": path,
                        "status": status,
                        "duration_ms": duration.as_millis()
                    }));
                }
                LogFormat::Pretty => {
                    println!("📝 {} {} → {} ({:?})", method, path, status, duration);
                }
                LogFormat::Compact => {
                    println!("{} {} {} {:?}", method, path, status, duration);
                }
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}

#[derive(Debug)]
pub struct CorsMiddleware;

impl CorsMiddleware {
    pub fn permissive() -> Self {
        Self
    }
}

impl Middleware for CorsMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let mut response = next.run(request).await;
            let _ = response.add_header("Access-Control-Allow-Origin", "*");
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "CorsMiddleware"
    }
}

#[derive(Debug)]
pub struct DevErrorMiddleware;

impl DevErrorMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for DevErrorMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // In real implementation, catch panics and provide detailed error info
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "DevErrorMiddleware"
    }
}

#[derive(Debug)]
pub struct HotReloadMiddleware;

impl HotReloadMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for HotReloadMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // In real implementation, watch for file changes and trigger reloads
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "HotReloadMiddleware"
    }
}

#[derive(Debug)]
pub struct CompressionMiddleware;

impl CompressionMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for CompressionMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let mut response = next.run(request).await;
            let _ = response.add_header("Content-Encoding", "gzip");
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "CompressionMiddleware"
    }
}

#[derive(Debug)]
pub struct CachingMiddleware {
    cache_duration: std::time::Duration,
}

impl CachingMiddleware {
    pub fn new(cache_duration: std::time::Duration) -> Self {
        Self { cache_duration }
    }
}

impl Middleware for CachingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let cache_duration = self.cache_duration;
        Box::pin(async move {
            let mut response = next.run(request).await;
            let _ = response.add_header("Cache-Control", &format!("max-age={}", cache_duration.as_secs()));
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "CachingMiddleware"
    }
}

#[derive(Debug)]
pub struct TestingMiddleware;

impl TestingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for TestingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let mut response = next.run(request).await;
            let _ = response.add_header("X-Test-Mode", "true");
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "TestingMiddleware"
    }
}

// Complete usage example
#[allow(dead_code)]
fn complete_usage_example() -> Result<(), Box<dyn std::error::Error>> {
    use elif_http::{Server, HttpConfig};
    use elif_core::Container;
    
    let container = Container::new();
    let mut server = Server::new(container, HttpConfig::default())?;
    
    // Use environment-specific middleware stack
    let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    let middleware_stack = environment_specific_stack(&env);
    
    // Apply the entire middleware stack at once
    for middleware in middleware_stack.middleware {
        // Note: This is conceptual - in practice you'd need to extract and apply each middleware
    }
    
    // Or use builder pattern
    let custom_stack = MiddlewareBuilder::new()
        .security()
        .cors(vec!["https://myapp.com".to_string()])
        .auth("my-secret-key".to_string())
        .logging(LogFormat::Json)
        .rate_limit(120)
        .build();
    
    Ok(())
}