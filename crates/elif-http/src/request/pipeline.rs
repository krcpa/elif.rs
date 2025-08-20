//! Request/Response pipeline that integrates routing and middleware
//!
//! This module provides the unified request processing pipeline that:
//! - Integrates the new framework-native routing engine
//! - Handles parameter injection from route matches
//! - Executes middleware pipelines with route-specific configurations  
//! - Provides clean error handling throughout the pipeline
//! - Supports route-specific middleware execution

use crate::routing::{RouteMatcher, RouteMatch, HttpMethod};
use crate::middleware::v2::{MiddlewarePipelineV2, NextFuture};
use crate::request::ElifRequest;
use crate::response::ElifResponse;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during request pipeline processing
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Route not found: {method} {path}")]
    RouteNotFound { method: HttpMethod, path: String },
    
    #[error("Parameter error: {0}")]
    Parameter(#[from] ParamError),
    
    #[error("Middleware error: {0}")]
    Middleware(String),
    
    #[error("Handler error: {0}")]
    Handler(String),
    
    #[error("Internal pipeline error: {0}")]
    Internal(String),
}

/// Errors that can occur during parameter extraction and parsing
#[derive(Error, Debug)]
pub enum ParamError {
    #[error("Missing parameter: {0}")]
    Missing(String),
    
    #[error("Failed to parse parameter '{param}' with value '{value}': {error}")]
    ParseError {
        param: String,
        value: String,
        error: String,
    },
}

/// Handler function type for request processing
pub type HandlerFn = dyn Fn(ElifRequest) -> NextFuture<'static> + Send + Sync;

/// Configuration for route-specific middleware groups
#[derive(Debug, Clone)]
pub struct MiddlewareGroup {
    pub name: String,
    pub pipeline: MiddlewarePipelineV2,
}

/// The main request processing pipeline
pub struct RequestPipeline {
    /// Route matcher for resolving incoming requests
    matcher: Arc<RouteMatcher>,
    /// Global middleware that runs for all requests
    global_middleware: MiddlewarePipelineV2,
    /// Named middleware groups for route-specific execution
    middleware_groups: HashMap<String, MiddlewarePipelineV2>,
    /// Route handlers mapped by route ID
    handlers: HashMap<String, Arc<HandlerFn>>,
}

impl RequestPipeline {
    /// Create a new request pipeline with a route matcher
    pub fn new(matcher: RouteMatcher) -> Self {
        Self {
            matcher: Arc::new(matcher),
            global_middleware: MiddlewarePipelineV2::new(),
            middleware_groups: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    /// Add global middleware that runs for all requests
    pub fn add_global_middleware<M>(mut self, middleware: M) -> Self 
    where 
        M: crate::middleware::v2::Middleware + 'static,
    {
        self.global_middleware = self.global_middleware.add(middleware);
        self
    }

    /// Add a named middleware group for route-specific execution
    pub fn add_middleware_group<S: Into<String>>(mut self, name: S, pipeline: MiddlewarePipelineV2) -> Self {
        self.middleware_groups.insert(name.into(), pipeline);
        self
    }

    /// Register a handler for a specific route ID
    pub fn add_handler<S: Into<String>, F>(mut self, route_id: S, handler: F) -> Self 
    where 
        F: Fn(ElifRequest) -> NextFuture<'static> + Send + Sync + 'static,
    {
        self.handlers.insert(route_id.into(), Arc::new(handler));
        self
    }

    /// Process an incoming request through the complete pipeline
    pub async fn process(&self, request: ElifRequest) -> ElifResponse {
        match self.process_internal(request).await {
            Ok(response) => response,
            Err(error) => self.handle_pipeline_error(error),
        }
    }

    /// Internal request processing with error handling
    async fn process_internal(&self, request: ElifRequest) -> Result<ElifResponse, PipelineError> {
        // 1. Route resolution
        let route_match = self.resolve_route(&request)?;
        
        // 2. Parameter injection
        let request_with_params = self.inject_params(request, &route_match);
        
        // 3. Build complete middleware pipeline for this route
        let complete_pipeline = self.build_route_pipeline(&route_match)?;
        
        // 4. Execute middleware + handler
        let route_id = route_match.route_id.clone();
        let handlers = self.handlers.clone();
        let response = complete_pipeline.execute(request_with_params, move |req| {
            let route_id = route_id.clone();
            let handlers = handlers.clone();
            Box::pin(async move {
                match handlers.get(&route_id) {
                    Some(handler) => handler(req).await,
                    None => {
                        ElifResponse::internal_server_error()
                            .with_json(&serde_json::json!({
                                "error": {
                                    "code": "handler_not_found",
                                    "message": format!("No handler registered for route: {}", route_id)
                                }
                            }))
                    }
                }
            })
        }).await;
        
        Ok(response)
    }

    /// Resolve incoming request to a matching route
    fn resolve_route(&self, request: &ElifRequest) -> Result<RouteMatch, PipelineError> {
        let http_method = match request.method.to_axum() {
            &axum::http::Method::GET => HttpMethod::GET,
            &axum::http::Method::POST => HttpMethod::POST,
            &axum::http::Method::PUT => HttpMethod::PUT,
            &axum::http::Method::DELETE => HttpMethod::DELETE,
            &axum::http::Method::PATCH => HttpMethod::PATCH,
            &axum::http::Method::HEAD => HttpMethod::HEAD,
            &axum::http::Method::OPTIONS => HttpMethod::OPTIONS,
            &axum::http::Method::TRACE => HttpMethod::TRACE,
            _ => HttpMethod::GET, // Fallback for any unknown method
        };
        
        self.matcher
            .resolve(&http_method, request.path())
            .ok_or_else(|| PipelineError::RouteNotFound {
                method: http_method,
                path: request.path().to_string(),
            })
    }

    /// Inject route parameters into the request
    fn inject_params(&self, mut request: ElifRequest, route_match: &RouteMatch) -> ElifRequest {
        for (key, value) in &route_match.params {
            request.add_path_param(key, value);
        }
        request
    }

    /// Build the complete middleware pipeline for a specific route
    fn build_route_pipeline(&self, _route_match: &RouteMatch) -> Result<MiddlewarePipelineV2, PipelineError> {
        let pipeline = self.global_middleware.clone();
        
        // Add route-specific middleware groups
        // For now, we'll look for middleware group names in route metadata
        // This can be extended to support route definition with middleware specifications
        
        Ok(pipeline)
    }


    /// Handle pipeline errors by converting them to appropriate HTTP responses
    fn handle_pipeline_error(&self, error: PipelineError) -> ElifResponse {
        match error {
            PipelineError::RouteNotFound { .. } => {
                ElifResponse::not_found()
                    .with_json(&serde_json::json!({
                        "error": {
                            "code": "not_found",
                            "message": "The requested resource was not found"
                        }
                    }))
            }
            PipelineError::Parameter(param_error) => {
                ElifResponse::bad_request()
                    .with_json(&serde_json::json!({
                        "error": {
                            "code": "parameter_error",
                            "message": param_error.to_string()
                        }
                    }))
            }
            PipelineError::Middleware(msg) | PipelineError::Handler(msg) | PipelineError::Internal(msg) => {
                ElifResponse::internal_server_error()
                    .with_json(&serde_json::json!({
                        "error": {
                            "code": "internal_error",
                            "message": msg
                        }
                    }))
            }
        }
    }

    /// Get statistics about the pipeline
    pub fn stats(&self) -> PipelineStats {
        PipelineStats {
            total_routes: self.matcher.all_routes().len(),
            global_middleware_count: self.global_middleware.len(),
            middleware_groups: self.middleware_groups.len(),
            registered_handlers: self.handlers.len(),
        }
    }

    /// Get the route matcher for introspection
    pub fn matcher(&self) -> &RouteMatcher {
        &self.matcher
    }

    /// Get global middleware pipeline for introspection
    pub fn global_middleware(&self) -> &MiddlewarePipelineV2 {
        &self.global_middleware
    }

    /// Get middleware groups for introspection
    pub fn middleware_groups(&self) -> &HashMap<String, MiddlewarePipelineV2> {
        &self.middleware_groups
    }
}

impl std::fmt::Debug for RequestPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestPipeline")
            .field("matcher", &self.matcher)
            .field("global_middleware", &self.global_middleware)
            .field("middleware_groups", &self.middleware_groups)
            .field("handlers", &self.handlers.len())
            .finish()
    }
}

/// Statistics about the request pipeline
#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub total_routes: usize,
    pub global_middleware_count: usize,
    pub middleware_groups: usize,
    pub registered_handlers: usize,
}

/// Builder for creating request pipelines
pub struct RequestPipelineBuilder {
    matcher: Option<RouteMatcher>,
    global_middleware: MiddlewarePipelineV2,
    middleware_groups: HashMap<String, MiddlewarePipelineV2>,
    handlers: HashMap<String, Arc<HandlerFn>>,
}

impl RequestPipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self {
            matcher: None,
            global_middleware: MiddlewarePipelineV2::new(),
            middleware_groups: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    /// Set the route matcher
    pub fn matcher(mut self, matcher: RouteMatcher) -> Self {
        self.matcher = Some(matcher);
        self
    }

    /// Add global middleware
    pub fn global_middleware<M>(mut self, middleware: M) -> Self 
    where 
        M: crate::middleware::v2::Middleware + 'static,
    {
        self.global_middleware = self.global_middleware.add(middleware);
        self
    }

    /// Add a middleware group
    pub fn middleware_group<S: Into<String>>(mut self, name: S, pipeline: MiddlewarePipelineV2) -> Self {
        self.middleware_groups.insert(name.into(), pipeline);
        self
    }

    /// Add a route handler
    pub fn handler<S: Into<String>, F>(mut self, route_id: S, handler: F) -> Self 
    where 
        F: Fn(ElifRequest) -> NextFuture<'static> + Send + Sync + 'static,
    {
        self.handlers.insert(route_id.into(), Arc::new(handler));
        self
    }

    /// Build the request pipeline
    pub fn build(self) -> Result<RequestPipeline, PipelineError> {
        let matcher = self.matcher.ok_or_else(|| {
            PipelineError::Internal("Route matcher is required".to_string())
        })?;

        Ok(RequestPipeline {
            matcher: Arc::new(matcher),
            global_middleware: self.global_middleware,
            middleware_groups: self.middleware_groups,
            handlers: self.handlers,
        })
    }
}

impl Default for RequestPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for parameter extraction with better error handling
pub mod parameter_extraction {
    use super::{ParamError, ElifRequest};
    use std::str::FromStr;
    use std::fmt::{Debug, Display};

    /// Extract and parse a path parameter with type conversion
    pub fn extract_path_param<T>(request: &ElifRequest, name: &str) -> Result<T, ParamError>
    where
        T: FromStr,
        T::Err: Debug + Display,
    {
        let param_value = request.path_param(name)
            .ok_or_else(|| ParamError::Missing(name.to_string()))?;

        param_value.parse()
            .map_err(|e| ParamError::ParseError {
                param: name.to_string(),
                value: param_value.clone(),
                error: format!("{}", e),
            })
    }

    /// Extract and parse a query parameter with type conversion
    pub fn extract_query_param<T>(request: &ElifRequest, name: &str) -> Result<Option<T>, ParamError>
    where
        T: FromStr,
        T::Err: Debug + Display,
    {
        if let Some(param_value) = request.query_param(name) {
            let parsed = param_value.parse()
                .map_err(|e| ParamError::ParseError {
                    param: name.to_string(),
                    value: param_value.clone(),
                    error: format!("{}", e),
                })?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Extract required query parameter with type conversion
    pub fn extract_required_query_param<T>(request: &ElifRequest, name: &str) -> Result<T, ParamError>
    where
        T: FromStr,
        T::Err: Debug + Display,
    {
        extract_query_param(request, name)?
            .ok_or_else(|| ParamError::Missing(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::{RouteMatcherBuilder};
    use crate::middleware::v2::{MiddlewarePipelineV2, LoggingMiddleware};
    use crate::response::{ElifResponse, ElifStatusCode};

    #[tokio::test]
    async fn test_basic_pipeline_processing() {
        // Create a simple route matcher
        let matcher = RouteMatcherBuilder::new()
            .get("home".to_string(), "/".to_string())
            .get("user_show".to_string(), "/users/{id}".to_string())
            .build()
            .unwrap();

        // Create pipeline with a simple handler
        let pipeline = RequestPipelineBuilder::new()
            .matcher(matcher)
            .handler("home", |_req| {
                Box::pin(async move {
                    ElifResponse::ok().with_text("Welcome home!")
                })
            })
            .handler("user_show", |req| {
                Box::pin(async move {
                    let user_id: u32 = match parameter_extraction::extract_path_param(&req, "id") {
                        Ok(id) => id,
                        Err(_) => return ElifResponse::bad_request().with_text("Invalid user ID"),
                    };
                    ElifResponse::ok().with_json(&serde_json::json!({
                        "user_id": user_id,
                        "message": "User details"
                    }))
                })
            })
            .build()
            .unwrap();

        // Test home route
        let home_request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );

        let home_response = pipeline.process(home_request).await;
        assert_eq!(home_response.status_code(), ElifStatusCode::OK);

        // Test user route with parameter
        let user_request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/users/123".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );

        let user_response = pipeline.process(user_request).await;
        assert_eq!(user_response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_pipeline_with_middleware() {
        let matcher = RouteMatcherBuilder::new()
            .get("test".to_string(), "/test".to_string())
            .build()
            .unwrap();

        let pipeline = RequestPipelineBuilder::new()
            .matcher(matcher)
            .global_middleware(LoggingMiddleware)
            .handler("test", |_req| {
                Box::pin(async move {
                    ElifResponse::ok().with_text("Test response")
                })
            })
            .build()
            .unwrap();

        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );

        let response = pipeline.process(request).await;
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_pipeline_route_not_found() {
        let matcher = RouteMatcherBuilder::new()
            .get("home".to_string(), "/".to_string())
            .build()
            .unwrap();

        let pipeline = RequestPipelineBuilder::new()
            .matcher(matcher)
            .build()
            .unwrap();

        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/nonexistent".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );

        let response = pipeline.process(request).await;
        assert_eq!(response.status_code(), ElifStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_pipeline_handler_not_found() {
        let matcher = RouteMatcherBuilder::new()
            .get("test".to_string(), "/test".to_string())
            .build()
            .unwrap();

        // Create pipeline without registering a handler
        let pipeline = RequestPipelineBuilder::new()
            .matcher(matcher)
            .build()
            .unwrap();

        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );

        let response = pipeline.process(request).await;
        assert_eq!(response.status_code(), ElifStatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_parameter_extraction_helpers() {
        let mut request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/users/123?page=2&limit=10".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );

        // Simulate parameter injection
        request.add_path_param("id", "123");
        request.add_query_param("page", "2");
        request.add_query_param("limit", "10");

        // Test path parameter extraction
        let user_id: u32 = parameter_extraction::extract_path_param(&request, "id").unwrap();
        assert_eq!(user_id, 123);

        // Test query parameter extraction
        let page: Option<u32> = parameter_extraction::extract_query_param(&request, "page").unwrap();
        assert_eq!(page, Some(2));

        // Test required query parameter extraction
        let limit: u32 = parameter_extraction::extract_required_query_param(&request, "limit").unwrap();
        assert_eq!(limit, 10);

        // Test missing parameter
        let result = parameter_extraction::extract_path_param::<u32>(&request, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParamError::Missing(_)));

        // Test invalid parameter parsing
        request.add_path_param("invalid", "not_a_number");
        let result = parameter_extraction::extract_path_param::<u32>(&request, "invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParamError::ParseError { .. }));
    }

    #[tokio::test]
    async fn test_pipeline_stats() {
        let matcher = RouteMatcherBuilder::new()
            .get("route1".to_string(), "/route1".to_string())
            .get("route2".to_string(), "/route2".to_string())
            .build()
            .unwrap();

        let middleware_group = MiddlewarePipelineV2::new().add(LoggingMiddleware);

        let pipeline = RequestPipelineBuilder::new()
            .matcher(matcher)
            .global_middleware(LoggingMiddleware)
            .middleware_group("auth", middleware_group)
            .handler("route1", |_req| {
                Box::pin(async move { ElifResponse::ok() })
            })
            .handler("route2", |_req| {
                Box::pin(async move { ElifResponse::ok() })
            })
            .build()
            .unwrap();

        let stats = pipeline.stats();
        assert_eq!(stats.total_routes, 2);
        assert_eq!(stats.global_middleware_count, 1);
        assert_eq!(stats.middleware_groups, 1);
        assert_eq!(stats.registered_handlers, 2);
    }
}