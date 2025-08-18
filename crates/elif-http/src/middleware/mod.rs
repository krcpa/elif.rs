//! # Middleware
//!
//! Comprehensive middleware system for processing requests and responses.
//! Provides async middleware trait, pipeline composition, and built-in middleware.

pub mod pipeline;
pub mod core;
pub mod utils;
pub mod v2;

// Re-export core middleware functionality
pub use pipeline::*;

// Re-export all core middleware
pub use core::*;

// Re-export utility middleware
pub use utils::*;

use axum::{extract::Request, response::Response};
use std::future::Future;
use std::pin::Pin;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifStatusCode as StatusCode;
    use axum::http::Method;
    
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