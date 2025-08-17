//! Middleware pipeline for composing multiple middleware

use super::Middleware;
use axum::{extract::Request, response::Response};
use std::future::Future;
use std::pin::Pin;

/// Type alias for boxed future returned by middleware
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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