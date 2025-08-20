use elif::prelude::*;
use elif_http::middleware::v2::{Middleware, Next, NextFuture};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;

/// Auth middleware
#[derive(Debug)]
pub struct Auth {
    // Add your configuration fields here
    // Example: 
    // pub config_value: String,
}

impl Auth {
    /// Create a new Auth
    pub fn new(/* Add your parameters here */) -> Self {
        Self {
            // Initialize your fields
        }
    }
}

impl Middleware for Auth {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        // Before request processing
        // Add your pre-processing logic here
        
        // Process the request through the rest of the middleware chain - cleaner API!
        next.call(request)
        
        // Note: For post-processing, you would still need Box::pin(async move { ... })
        // but most middleware only need pre-processing or simple pass-through
    }
    
    fn name(&self) -> &'static str {
        "Auth"
    }
}


/// Conditional wrapper for Auth
pub type ConditionalAuth = elif_http::middleware::v2::ConditionalMiddleware<Auth>;

impl Auth {
    /// Create a conditional version of this middleware
    pub fn conditional(self) -> ConditionalAuth {
        elif_http::middleware::v2::ConditionalMiddleware::new(self)
    }
}



/// Debug instrumentation for Auth
pub type InstrumentedAuth = elif_http::middleware::v2::introspection::InstrumentedMiddleware<Auth>;

impl Auth {
    /// Create an instrumented version of this middleware for debugging
    pub fn instrumented(self, name: String) -> InstrumentedAuth {
        elif_http::middleware::v2::introspection::instrument(self, name)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::request::{ElifMethod, ElifRequest};
    use elif_http::response::ElifResponse;
    use elif_http::middleware::v2::MiddlewarePipelineV2;

    #[tokio::test]
    async fn test_auth_middleware() {
        let middleware = Auth::new();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);

        let request = ElifRequest::new(
            ElifMethod::GET,
            "/test".parse().unwrap(),
            elif_http::response::headers::ElifHeaderMap::new(),
        );

        let response = pipeline.execute(request, |_req| {
            Box::pin(async move {
                ElifResponse::ok().text("Test response")
            })
        }).await;

        assert_eq!(response.status_code(), elif_http::response::status::ElifStatusCode::OK);
    }
}