//! # Middleware V2
//!
//! New middleware system with handle(request, next) pattern for Laravel-style simplicity.
//! This is the new middleware API that will replace the current one.

use crate::request::{ElifRequest, ElifMethod};
use crate::response::ElifResponse;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::HashMap;
// use axum::extract::Request;
// use super::Middleware as OldMiddleware; // Import the old middleware trait

/// Type alias for boxed future in Next
pub type NextFuture<'a> = Pin<Box<dyn Future<Output = ElifResponse> + Send + 'a>>;

/// Next represents the rest of the middleware chain
pub struct Next {
    handler: Box<dyn FnOnce(ElifRequest) -> NextFuture<'static> + Send>,
}

impl Next {
    /// Create a new Next with a handler function
    pub fn new<F>(handler: F) -> Self
    where
        F: FnOnce(ElifRequest) -> NextFuture<'static> + Send + 'static,
    {
        Self {
            handler: Box::new(handler),
        }
    }

    /// Run the rest of the middleware chain with the given request
    pub async fn run(self, request: ElifRequest) -> ElifResponse {
        (self.handler)(request).await
    }
}

/// New middleware trait with Laravel-style handle(request, next) pattern
/// Uses boxed futures to be dyn-compatible
pub trait Middleware: Send + Sync + std::fmt::Debug {
    /// Handle the request and call the next middleware in the chain
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static>;
    
    /// Optional middleware name for debugging
    fn name(&self) -> &'static str {
        "Middleware"
    }
}

/// Middleware pipeline for the new system
#[derive(Debug)]
pub struct MiddlewarePipelineV2 {
    middleware: Vec<Arc<dyn Middleware>>,
}

impl Default for MiddlewarePipelineV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewarePipelineV2 {
    /// Create a new empty middleware pipeline
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }
    
    /// Add middleware to the pipeline
    pub fn add<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware.push(Arc::new(middleware));
        self
    }
    
    /// Add middleware to the pipeline (mutable version)
    pub fn add_mut<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middleware.push(Arc::new(middleware));
    }

    /// Create a pipeline from a vector of Arc<dyn Middleware>
    pub fn from_middleware_vec(middleware: Vec<Arc<dyn Middleware>>) -> Self {
        Self { middleware }
    }

    /// Add an already-boxed middleware to the pipeline
    pub fn add_boxed(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Extend this pipeline with middleware from another pipeline
    /// The middleware from this pipeline will execute before the middleware from the other pipeline
    pub fn extend(mut self, other: Self) -> Self {
        self.middleware.extend(other.middleware);
        self
    }
    
    /// Execute the middleware pipeline with a handler
    pub async fn execute<F, Fut>(&self, request: ElifRequest, handler: F) -> ElifResponse
    where
        F: FnOnce(ElifRequest) -> Fut + Send + 'static,
        Fut: Future<Output = ElifResponse> + Send + 'static,
    {
        let mut chain = Box::new(move |req: ElifRequest| {
            Box::pin(handler(req)) as NextFuture<'static>
        }) as Box<dyn FnOnce(ElifRequest) -> NextFuture<'static> + Send>;

        for middleware in self.middleware.iter().rev() {
            let middleware = middleware.clone();
            let next_handler = chain;
            chain = Box::new(move |req: ElifRequest| {
                let next = Next::new(next_handler);
                middleware.handle(req, next)
            });
        }

        chain(request).await
    }
    
    /// Get number of middleware in pipeline
    pub fn len(&self) -> usize {
        self.middleware.len()
    }
    
    /// Check if pipeline is empty
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }
    
    /// Get middleware names for debugging
    pub fn names(&self) -> Vec<&'static str> {
        self.middleware.iter().map(|m| m.name()).collect()
    }
}

impl Clone for MiddlewarePipelineV2 {
    fn clone(&self) -> Self {
        Self {
            middleware: self.middleware.clone(),
        }
    }
}

impl From<Vec<Arc<dyn Middleware>>> for MiddlewarePipelineV2 {
    fn from(middleware: Vec<Arc<dyn Middleware>>) -> Self {
        Self { middleware }
    }
}

// Legacy middleware adapter removed - all middleware should use V2 system directly

/// Conditional middleware wrapper that can skip execution based on path patterns and HTTP methods
pub struct ConditionalMiddleware<M> {
    middleware: M,
    skip_paths: Vec<String>,
    only_methods: Option<Vec<ElifMethod>>,
    condition: Option<Arc<dyn Fn(&ElifRequest) -> bool + Send + Sync>>,
}

impl<M: std::fmt::Debug> std::fmt::Debug for ConditionalMiddleware<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionalMiddleware")
            .field("middleware", &self.middleware)
            .field("skip_paths", &self.skip_paths)
            .field("only_methods", &self.only_methods)
            .field("condition", &self.condition.as_ref().map(|_| "Some(Fn)"))
            .finish()
    }
}

impl<M> ConditionalMiddleware<M> {
    pub fn new(middleware: M) -> Self {
        Self {
            middleware,
            skip_paths: Vec::new(),
            only_methods: None,
            condition: None,
        }
    }

    /// Skip middleware execution for paths matching these patterns
    /// Supports basic wildcards: "/api/*" matches "/api/users", "/api/posts", etc.
    pub fn skip_paths(mut self, paths: Vec<&str>) -> Self {
        self.skip_paths = paths.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Only execute middleware for these HTTP methods
    pub fn only_methods(mut self, methods: Vec<ElifMethod>) -> Self {
        self.only_methods = Some(methods);
        self
    }

    /// Add a custom condition function that determines whether to run the middleware
    pub fn condition<F>(mut self, condition: F) -> Self 
    where
        F: Fn(&ElifRequest) -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Arc::new(condition));
        self
    }

    /// Check if a path matches any of the skip patterns
    fn should_skip_path(&self, path: &str) -> bool {
        for pattern in &self.skip_paths {
            if Self::path_matches(path, pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob-style path matching (supports * wildcard)
    fn path_matches(path: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }
        path == pattern
    }

    /// Check if the request should be processed by this middleware
    fn should_execute(&self, request: &ElifRequest) -> bool {
        // Check skip paths
        if self.should_skip_path(request.path()) {
            return false;
        }

        // Check method restrictions
        if let Some(ref allowed_methods) = self.only_methods {
            if !allowed_methods.contains(&request.method) {
                return false;
            }
        }

        // Check custom condition
        if let Some(ref condition) = self.condition {
            if !condition(request) {
                return false;
            }
        }

        true
    }
}

impl<M: Middleware> Middleware for ConditionalMiddleware<M> {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        if self.should_execute(&request) {
            // Execute the wrapped middleware
            self.middleware.handle(request, next)
        } else {
            // Skip the middleware and go directly to next
            Box::pin(async move {
                next.run(request).await
            })
        }
    }

    fn name(&self) -> &'static str {
        "ConditionalMiddleware"
    }
}

/// Middleware factories for common patterns
pub mod factories {
    use super::*;
    use std::time::Duration;
    
    /// Rate limiting middleware factory
    pub fn rate_limit(requests_per_minute: u32) -> RateLimitMiddleware {
        RateLimitMiddleware::new()
            .limit(requests_per_minute)
            .window(Duration::from_secs(60))
    }
    
    /// Rate limiting middleware with custom window
    pub fn rate_limit_with_window(requests: u32, window: Duration) -> RateLimitMiddleware {
        RateLimitMiddleware::new()
            .limit(requests)
            .window(window)
    }
    
    /// Authentication middleware factory
    pub fn bearer_auth(token: String) -> SimpleAuthMiddleware {
        SimpleAuthMiddleware::new(token)
    }
    
    /// CORS middleware factory
    pub fn cors() -> CorsMiddleware {
        CorsMiddleware::new()
    }
    
    /// CORS middleware with specific origins
    pub fn cors_with_origins(origins: Vec<String>) -> CorsMiddleware {
        CorsMiddleware::new().allow_origins(origins)
    }
    
    /// Timeout middleware factory
    pub fn timeout(duration: Duration) -> TimeoutMiddleware {
        TimeoutMiddleware::new(duration)
    }
    
    /// Body size limit middleware factory
    pub fn body_limit(max_bytes: u64) -> BodyLimitMiddleware {
        BodyLimitMiddleware::new(max_bytes)
    }
}

/// Middleware composition utilities
pub mod composition {
    use super::*;

    /// Compose two middleware into a pipeline
    pub fn compose<M1, M2>(first: M1, second: M2) -> MiddlewarePipelineV2
    where
        M1: Middleware + 'static,
        M2: Middleware + 'static,
    {
        MiddlewarePipelineV2::new().add(first).add(second)
    }

    /// Chain multiple middleware together (alias for compose for better readability)
    pub fn chain<M1, M2>(first: M1, second: M2) -> MiddlewarePipelineV2
    where
        M1: Middleware + 'static,
        M2: Middleware + 'static,
    {
        compose(first, second)
    }

    /// Create a middleware group from multiple middleware
    pub fn group(middleware: Vec<Arc<dyn Middleware>>) -> MiddlewarePipelineV2 {
        MiddlewarePipelineV2::from(middleware)
    }
    
    /// Compose three middleware into a pipeline
    pub fn compose3<M1, M2, M3>(first: M1, second: M2, third: M3) -> MiddlewarePipelineV2
    where
        M1: Middleware + 'static,
        M2: Middleware + 'static,
        M3: Middleware + 'static,
    {
        MiddlewarePipelineV2::new().add(first).add(second).add(third)
    }

    /// Compose four middleware into a pipeline
    pub fn compose4<M1, M2, M3, M4>(first: M1, second: M2, third: M3, fourth: M4) -> MiddlewarePipelineV2
    where
        M1: Middleware + 'static,
        M2: Middleware + 'static,
        M3: Middleware + 'static,
        M4: Middleware + 'static,
    {
        MiddlewarePipelineV2::new().add(first).add(second).add(third).add(fourth)
    }
}

/// A composed middleware that executes two middleware in sequence
pub struct ComposedMiddleware<M1, M2> {
    first: M1,
    second: M2,
}

impl<M1: std::fmt::Debug, M2: std::fmt::Debug> std::fmt::Debug for ComposedMiddleware<M1, M2> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposedMiddleware")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<M1, M2> ComposedMiddleware<M1, M2> {
    pub fn new(first: M1, second: M2) -> Self {
        Self { first, second }
    }

}

// For now, let's implement composition via pipeline extension
// The composed middleware pattern is complex with Rust lifetimes in this context
impl<M1, M2> ComposedMiddleware<M1, M2>
where
    M1: Middleware + 'static,
    M2: Middleware + 'static,
{
    /// Convert to a pipeline for easier execution
    pub fn to_pipeline(self) -> MiddlewarePipelineV2 {
        MiddlewarePipelineV2::new()
            .add(self.first)
            .add(self.second)
    }
}

/// Middleware introspection and debugging utilities
pub mod introspection {
    use super::*;
    use std::time::Instant;

    /// Execution statistics for middleware
    #[derive(Debug, Clone)]
    pub struct MiddlewareStats {
        pub name: String,
        pub executions: u64,
        pub total_time: Duration,
        pub avg_time: Duration,
        pub last_execution: Option<Instant>,
    }

    impl MiddlewareStats {
        pub fn new(name: String) -> Self {
            Self {
                name,
                executions: 0,
                total_time: Duration::ZERO,
                avg_time: Duration::ZERO,
                last_execution: None,
            }
        }

        pub fn record_execution(&mut self, duration: Duration) {
            self.executions += 1;
            self.total_time += duration;
            self.avg_time = self.total_time / self.executions as u32;
            self.last_execution = Some(Instant::now());
        }
    }

    /// Debug information about a middleware pipeline
    #[derive(Debug, Clone)]
    pub struct PipelineInfo {
        pub middleware_count: usize,
        pub middleware_names: Vec<String>,
        pub execution_order: Vec<String>,
    }

    impl MiddlewarePipelineV2 {
        /// Get debug information about the pipeline
        pub fn debug_info(&self) -> PipelineInfo {
            PipelineInfo {
                middleware_count: self.len(),
                middleware_names: self.names().into_iter().map(|s| s.to_string()).collect(),
                execution_order: self.names().into_iter().map(|s| s.to_string()).collect(),
            }
        }

        /// Create a debug pipeline that wraps each middleware with timing
        pub fn with_debug(self) -> DebugPipeline {
            DebugPipeline::new(self)
        }
    }

    /// A wrapper around MiddlewarePipelineV2 that provides debugging capabilities
    #[derive(Debug)]
    pub struct DebugPipeline {
        pipeline: MiddlewarePipelineV2,
        stats: Arc<Mutex<HashMap<String, MiddlewareStats>>>,
    }

    impl DebugPipeline {
        pub fn new(pipeline: MiddlewarePipelineV2) -> Self {
            let mut stats = HashMap::new();
            for name in pipeline.names() {
                stats.insert(name.to_string(), MiddlewareStats::new(name.to_string()));
            }

            Self {
                pipeline,
                stats: Arc::new(Mutex::new(stats)),
            }
        }

        /// Get execution statistics for all middleware
        pub fn stats(&self) -> HashMap<String, MiddlewareStats> {
            self.stats.lock().unwrap().clone()
        }

        /// Get statistics for a specific middleware
        pub fn middleware_stats(&self, name: &str) -> Option<MiddlewareStats> {
            self.stats.lock().unwrap().get(name).cloned()
        }

        /// Reset all statistics
        pub fn reset_stats(&self) {
            let mut stats = self.stats.lock().unwrap();
            for (name, stat) in stats.iter_mut() {
                *stat = MiddlewareStats::new(name.clone());
            }
        }

        /// Execute the pipeline with debug tracking
        pub async fn execute_debug<F, Fut>(&self, request: ElifRequest, handler: F) -> (ElifResponse, Duration)
        where
            F: FnOnce(ElifRequest) -> Fut + Send + 'static,
            Fut: Future<Output = ElifResponse> + Send + 'static,
        {
            let start_time = Instant::now();
            let response = self.pipeline.execute(request, handler).await;
            let total_duration = start_time.elapsed();
            
            (response, total_duration)
        }
    }

    /// A middleware wrapper that tracks execution statistics
    #[derive(Debug)]
    pub struct InstrumentedMiddleware<M> {
        middleware: M,
        name: String,
        stats: Arc<Mutex<MiddlewareStats>>,
    }

    impl<M> InstrumentedMiddleware<M> {
        pub fn new(middleware: M, name: String) -> Self {
            let stats = Arc::new(Mutex::new(MiddlewareStats::new(name.clone())));
            Self {
                middleware,
                name,
                stats,
            }
        }

        /// Get the current statistics for this middleware
        pub fn stats(&self) -> MiddlewareStats {
            self.stats.lock().unwrap().clone()
        }

        /// Reset statistics for this middleware
        pub fn reset_stats(&self) {
            let mut stats = self.stats.lock().unwrap();
            *stats = MiddlewareStats::new(self.name.clone());
        }
    }

    impl<M: Middleware> Middleware for InstrumentedMiddleware<M> {
        fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
            let stats = self.stats.clone();
            let middleware_result = self.middleware.handle(request, next);

            Box::pin(async move {
                let start = Instant::now();
                let response = middleware_result.await;
                let duration = start.elapsed();
                
                stats.lock().unwrap().record_execution(duration);
                response
            })
        }

        fn name(&self) -> &'static str {
            "InstrumentedMiddleware"
        }
    }

    /// Utility function to wrap middleware with instrumentation
    pub fn instrument<M: Middleware + 'static>(middleware: M, name: String) -> InstrumentedMiddleware<M> {
        InstrumentedMiddleware::new(middleware, name)
    }
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    requests_per_window: u32,
    window: Duration,
    // Simple in-memory store - in production you'd use Redis or similar
    requests: Arc<Mutex<HashMap<String, (std::time::Instant, u32)>>>,
}

impl std::fmt::Debug for RateLimitMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimitMiddleware")
            .field("requests_per_window", &self.requests_per_window)
            .field("window", &self.window)
            .finish()
    }
}

impl RateLimitMiddleware {
    pub fn new() -> Self {
        Self {
            requests_per_window: 60, // Default: 60 requests per minute
            window: Duration::from_secs(60),
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn limit(mut self, requests: u32) -> Self {
        self.requests_per_window = requests;
        self
    }

    pub fn window(mut self, window: Duration) -> Self {
        self.window = window;
        self
    }

    fn get_client_id(&self, request: &ElifRequest) -> String {
        // Simple IP-based rate limiting - in production you might use user ID, API key, etc.
        request.header("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string()
    }

    fn is_rate_limited(&self, client_id: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = std::time::Instant::now();

        // Clean up old entries
        requests.retain(|_, (timestamp, _)| now.duration_since(*timestamp) < self.window);

        // Check current rate
        if let Some((timestamp, count)) = requests.get_mut(client_id) {
            if now.duration_since(*timestamp) < self.window {
                if *count >= self.requests_per_window {
                    return true; // Rate limited
                }
                *count += 1;
            } else {
                // Reset window
                *timestamp = now;
                *count = 1;
            }
        } else {
            // First request from this client
            requests.insert(client_id.to_string(), (now, 1));
        }

        false
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let client_id = self.get_client_id(&request);
        let is_limited = self.is_rate_limited(&client_id);

        Box::pin(async move {
            if is_limited {
                ElifResponse::with_status(crate::response::status::ElifStatusCode::TOO_MANY_REQUESTS)
                    .json_value(serde_json::json!({
                        "error": {
                            "code": "rate_limited",
                            "message": "Too many requests. Please try again later."
                        }
                    }))
            } else {
                next.run(request).await
            }
        })
    }

    fn name(&self) -> &'static str {
        "RateLimitMiddleware"
    }
}

/// CORS middleware
#[derive(Debug)]
pub struct CorsMiddleware {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
}

impl CorsMiddleware {
    pub fn new() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "OPTIONS".to_string()],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
        }
    }

    pub fn allow_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    pub fn allow_methods(mut self, methods: Vec<String>) -> Self {
        self.allowed_methods = methods;
        self
    }

    pub fn allow_headers(mut self, headers: Vec<String>) -> Self {
        self.allowed_headers = headers;
        self
    }
}

impl Middleware for CorsMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let allowed_origins = self.allowed_origins.clone();
        let allowed_methods = self.allowed_methods.clone();
        let allowed_headers = self.allowed_headers.clone();

        Box::pin(async move {
            // Handle preflight OPTIONS request
            if request.method == ElifMethod::OPTIONS {
                return ElifResponse::ok()
                    .header("Access-Control-Allow-Origin", allowed_origins.join(","))
                    .unwrap_or_else(|_| ElifResponse::ok())
                    .header("Access-Control-Allow-Methods", allowed_methods.join(","))
                    .unwrap_or_else(|_| ElifResponse::ok())
                    .header("Access-Control-Allow-Headers", allowed_headers.join(","))
                    .unwrap_or_else(|_| ElifResponse::ok());
            }

            let response = next.run(request).await;
            
            // Add CORS headers to response - chain the operations
            let response_with_origin = response
                .header("Access-Control-Allow-Origin", allowed_origins.join(","))
                .unwrap_or_else(|_| ElifResponse::ok());
                
            let response_with_methods = response_with_origin
                .header("Access-Control-Allow-Methods", allowed_methods.join(","))
                .unwrap_or_else(|_| ElifResponse::ok());
                
            response_with_methods
                .header("Access-Control-Allow-Headers", allowed_headers.join(","))
                .unwrap_or_else(|_| ElifResponse::ok())
        })
    }

    fn name(&self) -> &'static str {
        "CorsMiddleware"
    }
}

/// Timeout middleware
#[derive(Debug)]
pub struct TimeoutMiddleware {
    timeout: Duration,
}

impl TimeoutMiddleware {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Middleware for TimeoutMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let timeout = self.timeout;
        Box::pin(async move {
            match tokio::time::timeout(timeout, next.run(request)).await {
                Ok(response) => response,
                Err(_) => ElifResponse::with_status(crate::response::status::ElifStatusCode::REQUEST_TIMEOUT)
                    .json_value(serde_json::json!({
                        "error": {
                            "code": "timeout",
                            "message": "Request timed out"
                        }
                    }))
            }
        })
    }

    fn name(&self) -> &'static str {
        "TimeoutMiddleware"
    }
}

/// Body size limit middleware
#[derive(Debug)]
pub struct BodyLimitMiddleware {
    max_bytes: u64,
}

impl BodyLimitMiddleware {
    pub fn new(max_bytes: u64) -> Self {
        Self { max_bytes }
    }
}

impl Middleware for BodyLimitMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let max_bytes = self.max_bytes;
        Box::pin(async move {
            // Check if request has body and if it exceeds limit
            if let Some(body) = request.body_bytes() {
                if body.len() as u64 > max_bytes {
                    return ElifResponse::with_status(crate::response::status::ElifStatusCode::PAYLOAD_TOO_LARGE)
                        .json_value(serde_json::json!({
                            "error": {
                                "code": "payload_too_large",
                                "message": format!("Request body too large. Maximum allowed: {} bytes", max_bytes)
                            }
                        }));
                }
            }

            next.run(request).await
        })
    }

    fn name(&self) -> &'static str {
        "BodyLimitMiddleware"
    }
}

/// Example logging middleware using the new pattern
#[derive(Debug)]
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Before request
            let start = std::time::Instant::now();
            let method = request.method.clone();
            let path = request.path().to_string();
            
            // Pass to next middleware
            let response = next.run(request).await;
            
            // After response
            let duration = start.elapsed();
            println!("{} {} - {} - {:?}", method, path, response.status_code(), duration);
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}

/// Example auth middleware using the new pattern
#[derive(Debug)]
pub struct SimpleAuthMiddleware {
    required_token: String,
}

impl SimpleAuthMiddleware {
    pub fn new(token: String) -> Self {
        Self {
            required_token: token,
        }
    }
}

impl Middleware for SimpleAuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let required_token = self.required_token.clone();
        Box::pin(async move {
            // Extract token
            let token = match request.header("Authorization") {
                Some(h) => {
                    match h.to_str() {
                        Ok(header_str) if header_str.starts_with("Bearer ") => &header_str[7..],
                        _ => {
                            return ElifResponse::unauthorized()
                                .json_value(serde_json::json!({
                                    "error": {
                                        "code": "unauthorized",
                                        "message": "Missing or invalid authorization header"
                                    }
                                }));
                        }
                    }
                }
                None => {
                    return ElifResponse::unauthorized()
                        .json_value(serde_json::json!({
                            "error": {
                                "code": "unauthorized", 
                                "message": "Missing authorization header"
                            }
                        }));
                }
            };
            
            // Validate token
            if token != required_token {
                return ElifResponse::unauthorized()
                    .json_value(serde_json::json!({
                        "error": {
                            "code": "unauthorized",
                            "message": "Invalid token"
                        }
                    }));
            }
            
            // Token is valid, proceed to next middleware
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "SimpleAuthMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::ElifRequest;
    use crate::response::ElifResponse;
    
    /// Test middleware that adds a header to requests
    #[derive(Debug)]
    pub struct TestMiddleware {
        name: &'static str,
    }
    
    impl TestMiddleware {
        pub fn new(name: &'static str) -> Self {
            Self { name }
        }
    }
    
    impl Middleware for TestMiddleware {
        fn handle(&self, mut request: ElifRequest, next: Next) -> NextFuture<'static> {
            let name = self.name;
            Box::pin(async move {
                // Add a custom header to track middleware execution
                let header_name = crate::response::headers::ElifHeaderName::from_str(&format!("x-middleware-{}", name.to_lowercase())).unwrap();
                let header_value = crate::response::headers::ElifHeaderValue::from_str("executed").unwrap();
                request.headers.insert(header_name, header_value);
                
                let response = next.run(request).await;
                
                // Add response header - simplified for now
                response
            })
        }
        
        fn name(&self) -> &'static str {
            self.name
        }
    }
    
    #[tokio::test]
    async fn test_simple_middleware_execution() {
        let pipeline = MiddlewarePipelineV2::new()
            .add(TestMiddleware::new("First"))
            .add(TestMiddleware::new("Second"));
        
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |req| {
            Box::pin(async move {
                // Verify both middleware executed by checking headers they added
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-first").unwrap()), 
                    "First middleware should have added header");
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-second").unwrap()), 
                    "Second middleware should have added header");
                
                ElifResponse::ok().text("Hello World")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_middleware_chain_execution_order() {
        /// Test middleware that tracks execution order
        #[derive(Debug)]
        struct OrderTestMiddleware {
            name: &'static str,
        }
        
        impl OrderTestMiddleware {
            fn new(name: &'static str) -> Self {
                Self { name }
            }
        }
        
        impl Middleware for OrderTestMiddleware {
            fn handle(&self, mut request: ElifRequest, next: Next) -> NextFuture<'static> {
                let name = self.name;
                Box::pin(async move {
                    // Add execution order to request headers (before handler)
                    let header_name_str = format!("x-before-{}", name.to_lowercase());
                    let header_name = crate::response::headers::ElifHeaderName::from_str(&header_name_str).unwrap();
                    let header_value = crate::response::headers::ElifHeaderValue::from_str("executed").unwrap();
                    request.headers.insert(header_name, header_value);
                    
                    // Call next middleware/handler
                    let response = next.run(request).await;
                    
                    // Add execution order to response headers (after handler) 
                    let response_header = format!("x-after-{}", name.to_lowercase());
                    response.header(&response_header, "executed").unwrap_or(
                        // If header addition fails, return original response  
                        ElifResponse::ok().text("fallback")
                    )
                })
            }
            
            fn name(&self) -> &'static str {
                self.name
            }
        }
        
        // Create pipeline with multiple middleware
        let pipeline = MiddlewarePipelineV2::new()
            .add(OrderTestMiddleware::new("First"))
            .add(OrderTestMiddleware::new("Second"))
            .add(OrderTestMiddleware::new("Third"));
        
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |req| {
            Box::pin(async move {
                // Verify all middleware ran before the handler
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-before-first").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-before-second").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-before-third").unwrap()));
                
                ElifResponse::ok().text("Handler executed")
            })
        }).await;
        
        // Verify response and that all middleware ran after the handler
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        
        // Convert to axum response to check headers
        let axum_response = response.into_axum_response();
        let (parts, _body) = axum_response.into_parts();
        assert!(parts.headers.contains_key("x-after-first"));
        assert!(parts.headers.contains_key("x-after-second"));
        assert!(parts.headers.contains_key("x-after-third"));
        
        // Verify pipeline info
        assert_eq!(pipeline.len(), 3);
        assert_eq!(pipeline.names(), vec!["First", "Second", "Third"]);
    }
    
    #[tokio::test]
    async fn test_auth_middleware() {
        let auth_middleware = SimpleAuthMiddleware::new("secret123".to_string());
        
        // Test with valid token
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        headers.insert(crate::response::headers::ElifHeaderName::from_str("authorization").unwrap(), "Bearer secret123".parse().unwrap());
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/protected".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async {
                ElifResponse::ok().text("Protected content")
            })
        });
        
        let response = auth_middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        
        // Test with invalid token
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        headers.insert(crate::response::headers::ElifHeaderName::from_str("authorization").unwrap(), "Bearer invalid".parse().unwrap());
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/protected".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async {
                ElifResponse::ok().text("Protected content")
            })
        });
        
        let response = auth_middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_pipeline_info() {
        let pipeline = MiddlewarePipelineV2::new()
            .add(TestMiddleware::new("Test1"))
            .add(TestMiddleware::new("Test2"));
        
        assert_eq!(pipeline.len(), 2);
        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.names(), vec!["Test1", "Test2"]);
        
        let empty_pipeline = MiddlewarePipelineV2::new();
        assert_eq!(empty_pipeline.len(), 0);
        assert!(empty_pipeline.is_empty());
    }
    
    // Legacy compatibility test removed - all middleware use V2 system directly

    #[tokio::test]
    async fn test_conditional_middleware_skip_paths() {
        let base_middleware = TestMiddleware::new("Conditional");
        let conditional = ConditionalMiddleware::new(base_middleware)
            .skip_paths(vec!["/public/*", "/health"]);
        
        let pipeline = MiddlewarePipelineV2::new().add(conditional);

        // Test skipped path
        let request1 = ElifRequest::new(
            ElifMethod::GET,
            "/public/assets/style.css".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response1 = pipeline.execute(request1, |req| {
            Box::pin(async move {
                // Middleware should be skipped - no header added
                assert!(!req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-conditional").unwrap()));
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Test non-skipped path
        let request2 = ElifRequest::new(
            ElifMethod::GET,
            "/api/users".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = pipeline.execute(request2, |req| {
            Box::pin(async move {
                // Middleware should execute - header added
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-conditional").unwrap()));
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_conditional_middleware_only_methods() {
        let base_middleware = TestMiddleware::new("MethodConditional");
        let conditional = ConditionalMiddleware::new(base_middleware)
            .only_methods(vec![ElifMethod::POST, ElifMethod::PUT]);
        
        let pipeline = MiddlewarePipelineV2::new().add(conditional);

        // Test allowed method
        let request1 = ElifRequest::new(
            ElifMethod::POST,
            "/api/users".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response1 = pipeline.execute(request1, |req| {
            Box::pin(async move {
                // Middleware should execute for POST
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-methodconditional").unwrap()));
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Test disallowed method
        let request2 = ElifRequest::new(
            ElifMethod::GET,
            "/api/users".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = pipeline.execute(request2, |req| {
            Box::pin(async move {
                // Middleware should be skipped for GET
                assert!(!req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-methodconditional").unwrap()));
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_conditional_middleware_custom_condition() {
        let base_middleware = TestMiddleware::new("CustomConditional");
        let conditional = ConditionalMiddleware::new(base_middleware)
            .condition(|req| req.header("X-Debug").is_some());
        
        let pipeline = MiddlewarePipelineV2::new().add(conditional);

        // Test with condition met
        let mut headers1 = crate::response::headers::ElifHeaderMap::new();
        headers1.insert(crate::response::headers::ElifHeaderName::from_str("x-debug").unwrap(), "true".parse().unwrap());
        let request1 = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            headers1,
        );
        
        let response1 = pipeline.execute(request1, |req| {
            Box::pin(async move {
                // Middleware should execute when X-Debug header present
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-customconditional").unwrap()));
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Test without condition met
        let request2 = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = pipeline.execute(request2, |req| {
            Box::pin(async move {
                // Middleware should be skipped when X-Debug header not present
                assert!(!req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-customconditional").unwrap()));
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limit_factory() {
        use super::factories;
        
        let rate_limiter = factories::rate_limit(2); // 2 requests per minute
        let pipeline = MiddlewarePipelineV2::new().add(rate_limiter);

        // First request should pass
        let request1 = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response1 = pipeline.execute(request1, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Second request should also pass
        let request2 = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = pipeline.execute(request2, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::OK);

        // Third request should be rate limited
        let request3 = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response3 = pipeline.execute(request3, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response3.status_code(), crate::response::status::ElifStatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_cors_factory() {
        use super::factories;
        
        let cors = factories::cors_with_origins(vec!["https://example.com".to_string()]);
        let pipeline = MiddlewarePipelineV2::new().add(cors);

        // Test OPTIONS preflight request
        let request1 = ElifRequest::new(
            ElifMethod::OPTIONS,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response1 = pipeline.execute(request1, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should not reach here")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Test normal request with CORS headers added
        let request2 = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = pipeline.execute(request2, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("OK")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_timeout_factory() {
        use super::factories;
        use std::time::Duration;
        
        let timeout_middleware = factories::timeout(Duration::from_millis(100));
        let pipeline = MiddlewarePipelineV2::new().add(timeout_middleware);

        // Test request that completes within timeout
        let request1 = ElifRequest::new(
            ElifMethod::GET,
            "/api/fast".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response1 = pipeline.execute(request1, |_req| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                ElifResponse::ok().text("Fast response")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Test request that times out
        let request2 = ElifRequest::new(
            ElifMethod::GET,
            "/api/slow".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = pipeline.execute(request2, |_req| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                ElifResponse::ok().text("Slow response")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::REQUEST_TIMEOUT);
    }

    #[tokio::test]
    async fn test_body_limit_factory() {
        use super::factories;
        use axum::body::Bytes;
        
        let body_limit = factories::body_limit(10); // 10 bytes max
        let pipeline = MiddlewarePipelineV2::new().add(body_limit);

        // Test request with small body
        let small_body = Bytes::from("small");
        let request1 = ElifRequest::new(
            ElifMethod::POST,
            "/api/upload".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        ).with_body(small_body);
        
        let response1 = pipeline.execute(request1, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Upload successful")
            })
        }).await;
        
        assert_eq!(response1.status_code(), crate::response::status::ElifStatusCode::OK);

        // Test request with large body
        let large_body = Bytes::from("this body is way too large for the limit");
        let request2 = ElifRequest::new(
            ElifMethod::POST,
            "/api/upload".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        ).with_body(large_body);
        
        let response2 = pipeline.execute(request2, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Should not reach here")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_composition_utilities() {
        use super::composition;
        
        let middleware1 = TestMiddleware::new("First");
        let middleware2 = TestMiddleware::new("Second");
        
        // Test compose function
        let composed_pipeline = composition::compose(middleware1, middleware2);
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = composed_pipeline.execute(request, |req| {
            Box::pin(async move {
                // Both middleware should have executed
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-first").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-second").unwrap()));
                ElifResponse::ok().text("Composed response")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        assert_eq!(composed_pipeline.len(), 2);

        // Test compose3 function
        let middleware1 = TestMiddleware::new("Alpha");
        let middleware2 = TestMiddleware::new("Beta"); 
        let middleware3 = TestMiddleware::new("Gamma");
        
        let composed3_pipeline = composition::compose3(middleware1, middleware2, middleware3);
        
        let request2 = ElifRequest::new(
            ElifMethod::POST,
            "/api/composed".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response2 = composed3_pipeline.execute(request2, |req| {
            Box::pin(async move {
                // All three middleware should have executed
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-alpha").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-beta").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-gamma").unwrap()));
                ElifResponse::ok().text("Triple composed response")
            })
        }).await;
        
        assert_eq!(response2.status_code(), crate::response::status::ElifStatusCode::OK);
        assert_eq!(composed3_pipeline.len(), 3);
    }

    #[tokio::test]
    async fn test_composition_group() {
        use super::composition;
        
        let middleware_vec: Vec<Arc<dyn Middleware>> = vec![
            Arc::new(TestMiddleware::new("Group1")),
            Arc::new(TestMiddleware::new("Group2")),
            Arc::new(TestMiddleware::new("Group3")),
        ];
        
        let group_pipeline = composition::group(middleware_vec);
        
        let request = ElifRequest::new(
            ElifMethod::DELETE,
            "/api/group".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = group_pipeline.execute(request, |req| {
            Box::pin(async move {
                // All group middleware should have executed
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-group1").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-group2").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-group3").unwrap()));
                ElifResponse::ok().text("Group response")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        assert_eq!(group_pipeline.len(), 3);
        assert_eq!(group_pipeline.names(), vec!["Group1", "Group2", "Group3"]);
    }

    #[tokio::test]
    async fn test_composed_middleware_to_pipeline() {
        let middleware1 = TestMiddleware::new("ComposedA");
        let middleware2 = TestMiddleware::new("ComposedB");
        
        let composed = ComposedMiddleware::new(middleware1, middleware2);
        let pipeline = composed.to_pipeline();
        
        let request = ElifRequest::new(
            ElifMethod::PUT,
            "/api/composed".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |req| {
            Box::pin(async move {
                // Both composed middleware should have executed
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-composeda").unwrap()));
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-composedb").unwrap()));
                ElifResponse::ok().text("Composed pipeline response")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        assert_eq!(pipeline.len(), 2);
    }

    #[tokio::test]
    async fn test_introspection_debug_info() {
        let pipeline = MiddlewarePipelineV2::new()
            .add(TestMiddleware::new("Debug1"))
            .add(TestMiddleware::new("Debug2"))
            .add(TestMiddleware::new("Debug3"));
        
        let debug_info = pipeline.debug_info();
        
        assert_eq!(debug_info.middleware_count, 3);
        assert_eq!(debug_info.middleware_names, vec!["Debug1", "Debug2", "Debug3"]);
        assert_eq!(debug_info.execution_order, vec!["Debug1", "Debug2", "Debug3"]);
    }

    #[tokio::test]
    async fn test_introspection_debug_pipeline() {
        let pipeline = MiddlewarePipelineV2::new()
            .add(TestMiddleware::new("Timed1"))
            .add(TestMiddleware::new("Timed2"));
        
        let debug_pipeline = pipeline.with_debug();
        
        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/debug".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let (response, duration) = debug_pipeline.execute_debug(request, |_req| {
            Box::pin(async move {
                // Simulate some processing time
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                ElifResponse::ok().text("Debug response")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
        assert!(duration > std::time::Duration::from_millis(5));
        
        // Check that we can get stats (even if middleware aren't individually tracked yet)
        let stats = debug_pipeline.stats();
        assert_eq!(stats.len(), 2);
        assert!(stats.contains_key("Timed1"));
        assert!(stats.contains_key("Timed2"));
    }

    #[tokio::test]
    async fn test_introspection_instrumented_middleware() {
        let base_middleware = TestMiddleware::new("Base");
        let instrumented = introspection::instrument(base_middleware, "InstrumentedTest".to_string());
        
        let pipeline = MiddlewarePipelineV2::new().add(instrumented);
        
        let request = ElifRequest::new(
            ElifMethod::POST,
            "/api/instrumented".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |req| {
            Box::pin(async move {
                // Verify middleware executed
                assert!(req.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("x-middleware-base").unwrap()));
                ElifResponse::ok().text("Instrumented response")
            })
        }).await;
        
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_introspection_middleware_stats() {
        use super::introspection::{MiddlewareStats, instrument};
        
        let mut stats = MiddlewareStats::new("TestStats".to_string());
        
        // Record some executions
        stats.record_execution(std::time::Duration::from_millis(10));
        stats.record_execution(std::time::Duration::from_millis(20));
        stats.record_execution(std::time::Duration::from_millis(30));
        
        assert_eq!(stats.executions, 3);
        assert_eq!(stats.total_time, std::time::Duration::from_millis(60));
        assert_eq!(stats.avg_time, std::time::Duration::from_millis(20));
        assert!(stats.last_execution.is_some());
        
        // Test instrumented middleware stats
        let base_middleware = TestMiddleware::new("StatsTest");
        let instrumented = instrument(base_middleware, "StatsInstrumented".to_string());
        
        let initial_stats = instrumented.stats();
        assert_eq!(initial_stats.executions, 0);
        assert_eq!(initial_stats.total_time, std::time::Duration::ZERO);
        
        // Test reset functionality
        instrumented.reset_stats();
        let reset_stats = instrumented.stats();
        assert_eq!(reset_stats.executions, 0);
    }
}