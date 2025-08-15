//! # Middleware
//!
//! Basic middleware system for processing requests and responses.
//! Provides async middleware trait and pipeline composition.

pub mod logging;
pub mod timing;
pub mod tracing;
pub mod timeout;
pub mod body_limit;

use std::future::Future;
use std::pin::Pin;
use axum::{
    response::{Response, IntoResponse},
    extract::Request,
};

use crate::{HttpResult, HttpError};

/// Type alias for async middleware function result
pub type MiddlewareResult = HttpResult<Response>;

/// Type alias for boxed future returned by middleware
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Core middleware trait that can process requests before handlers
/// and responses after handlers.
pub trait Middleware: Send + Sync {
    /// Process the request before it reaches the handler.
    /// Can modify the request or return early response.
    fn process_request<'a>(
        &'a self, 
        request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move { Ok(request) })
    }
    
    /// Process the response after the handler processes it.
    /// Can modify the response before returning to client.
    fn process_response<'a>(
        &'a self, 
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move { response })
    }
    
    /// Optional middleware name for debugging
    fn name(&self) -> &'static str {
        "Middleware"
    }
}

/// Middleware pipeline that composes multiple middleware in sequence
#[derive(Default)]
pub struct MiddlewarePipeline {
    middleware: Vec<Box<dyn Middleware>>,
}

impl MiddlewarePipeline {
    /// Create a new empty middleware pipeline
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }
    
    /// Add middleware to the pipeline
    pub fn add<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware.push(Box::new(middleware));
        self
    }
    
    /// Process request through all middleware in order
    pub async fn process_request(&self, mut request: Request) -> Result<Request, Response> {
        for middleware in &self.middleware {
            match middleware.process_request(request).await {
                Ok(req) => request = req,
                Err(response) => return Err(response),
            }
        }
        Ok(request)
    }
    
    /// Process response through all middleware in reverse order
    pub async fn process_response(&self, mut response: Response) -> Response {
        // Process in reverse order - last middleware added processes response first
        for middleware in self.middleware.iter().rev() {
            response = middleware.process_response(response).await;
        }
        response
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

/// Middleware wrapper that can handle errors and convert them to responses
pub struct ErrorHandlingMiddleware<M> {
    inner: M,
}

impl<M> ErrorHandlingMiddleware<M> {
    pub fn new(middleware: M) -> Self {
        Self { inner: middleware }
    }
}

impl<M: Middleware> Middleware for ErrorHandlingMiddleware<M> {
    fn process_request<'a>(
        &'a self, 
        request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            // Delegate to inner middleware with error handling
            match self.inner.process_request(request).await {
                Ok(req) => Ok(req),
                Err(response) => Err(response),
            }
        })
    }
    
    fn process_response<'a>(
        &'a self, 
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            self.inner.process_response(response).await
        })
    }
    
    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{StatusCode, Method};
    
    struct TestMiddleware {
        name: &'static str,
    }
    
    impl TestMiddleware {
        fn new(name: &'static str) -> Self {
            Self { name }
        }
    }
    
    impl Middleware for TestMiddleware {
        fn process_request<'a>(
            &'a self, 
            mut request: Request
        ) -> BoxFuture<'a, Result<Request, Response>> {
            Box::pin(async move {
                // Add a header to track middleware execution
                let headers = request.headers_mut();
                headers.insert("X-Middleware", self.name.parse().unwrap());
                Ok(request)
            })
        }
        
        fn process_response<'a>(
            &'a self, 
            mut response: Response
        ) -> BoxFuture<'a, Response> {
            Box::pin(async move {
                // Add response header
                let headers = response.headers_mut();
                headers.insert("X-Response-Middleware", self.name.parse().unwrap());
                response
            })
        }
        
        fn name(&self) -> &'static str {
            self.name
        }
    }
    
    #[tokio::test]
    async fn test_middleware_pipeline() {
        let pipeline = MiddlewarePipeline::new()
            .add(TestMiddleware::new("First"))
            .add(TestMiddleware::new("Second"));
        
        // Create test request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();
        
        // Process request
        let processed_request = pipeline.process_request(request).await.unwrap();
        
        // Should have header from last middleware (Second overwrites First)
        assert_eq!(
            processed_request.headers().get("X-Middleware").unwrap(),
            "Second"
        );
        
        // Create test response
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::empty())
            .unwrap();
        
        // Process response
        let processed_response = pipeline.process_response(response).await;
        
        // Should have header from first middleware (reverse order)
        assert_eq!(
            processed_response.headers().get("X-Response-Middleware").unwrap(),
            "First"
        );
    }
    
    #[tokio::test]
    async fn test_pipeline_info() {
        let pipeline = MiddlewarePipeline::new()
            .add(TestMiddleware::new("Test1"))
            .add(TestMiddleware::new("Test2"));
        
        assert_eq!(pipeline.len(), 2);
        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.names(), vec!["Test1", "Test2"]);
    }
    
    #[tokio::test]
    async fn test_empty_pipeline() {
        let pipeline = MiddlewarePipeline::new();
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();
        
        let processed_request = pipeline.process_request(request).await.unwrap();
        
        // Request should pass through unchanged
        assert_eq!(processed_request.method(), Method::GET);
        assert_eq!(processed_request.uri().path(), "/test");
    }
}