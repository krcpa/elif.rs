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
    use axum::http::{HeaderMap, Method};
    
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
}