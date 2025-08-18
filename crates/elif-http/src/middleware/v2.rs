//! # Middleware V2
//!
//! New middleware system with handle(request, next) pattern for Laravel-style simplicity.
//! This is the new middleware API that will replace the current one.

use crate::request::ElifRequest;
use crate::response::ElifResponse;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
// use axum::extract::Request;
// use super::Middleware as OldMiddleware; // Import the old middleware trait

/// Type alias for boxed future in Next
type NextFuture = Pin<Box<dyn Future<Output = ElifResponse> + Send + 'static>>;

/// Next represents the rest of the middleware chain
pub struct Next {
    handler: Box<dyn FnOnce(ElifRequest) -> NextFuture + Send>,
}

impl Next {
    /// Create a new Next with a handler function
    pub fn new<F>(handler: F) -> Self
    where
        F: FnOnce(ElifRequest) -> NextFuture + Send + 'static,
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
pub trait Middleware: Send + Sync {
    /// Handle the request and call the next middleware in the chain
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture;
    
    /// Optional middleware name for debugging
    fn name(&self) -> &'static str {
        "Middleware"
    }
}

/// Middleware pipeline for the new system
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
    
    /// Execute the middleware pipeline with a handler
    pub async fn execute<F, Fut>(&self, request: ElifRequest, handler: F) -> ElifResponse
    where
        F: FnOnce(ElifRequest) -> Fut + Send + 'static,
        Fut: Future<Output = ElifResponse> + Send + 'static,
    {
        // Simple implementation: process each middleware sequentially
        // This is not the final implementation but will work for now
        let mut current_request = request;
        
        // For now, we'll just call the handler directly and demonstrate the concept
        // A full implementation would need more complex chaining
        if self.middleware.is_empty() {
            return handler(current_request).await;
        }
        
        // Simplified demonstration - just call first middleware with a dummy next
        let middleware = &self.middleware[0];
        let next = Next::new(move |req| {
            Box::pin(async move {
                handler(req).await
            })
        });
        
        middleware.handle(current_request, next).await
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

// TODO: Backward compatibility adapter - disabled for now due to lifetime issues
// Will be implemented in a future iteration
// 
// /// Backward compatibility adapter to wrap old middleware in the new trait
// pub struct MiddlewareAdapter<T: OldMiddleware> {
//     inner: T,
// }

/// Example logging middleware using the new pattern
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture {
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
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture {
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
    use axum::http::{HeaderMap, Method};
    
    /// Test middleware that adds a header to requests
    pub struct TestMiddleware {
        name: &'static str,
    }
    
    impl TestMiddleware {
        pub fn new(name: &'static str) -> Self {
            Self { name }
        }
    }
    
    impl Middleware for TestMiddleware {
        fn handle(&self, mut request: ElifRequest, next: Next) -> NextFuture {
            let name = self.name;
            Box::pin(async move {
                // Add a custom header to track middleware execution
                let header_name: axum::http::HeaderName = format!("x-middleware-{}", name.to_lowercase()).parse().unwrap();
                let header_value: axum::http::HeaderValue = "executed".parse().unwrap();
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
            Method::GET,
            "/test".parse().unwrap(),
            HeaderMap::new(),
        );
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().text("Hello World")
            })
        }).await;
        
        assert_eq!(response.status_code(), axum::http::StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_auth_middleware() {
        let auth_middleware = SimpleAuthMiddleware::new("secret123".to_string());
        
        // Test with valid token
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer secret123".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/protected".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async {
                ElifResponse::ok().text("Protected content")
            })
        });
        
        let response = auth_middleware.handle(request, next).await;
        assert_eq!(response.status_code(), axum::http::StatusCode::OK);
        
        // Test with invalid token
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer invalid".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/protected".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async {
                ElifResponse::ok().text("Protected content")
            })
        });
        
        let response = auth_middleware.handle(request, next).await;
        assert_eq!(response.status_code(), axum::http::StatusCode::UNAUTHORIZED);
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
}