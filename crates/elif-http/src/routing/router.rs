//! Core routing functionality

use super::{HttpMethod, RouteInfo, RouteRegistry, params::{ParamExtractor, ParamType}};
use crate::handlers::elif_handler;
use crate::request::ElifRequest;
use crate::response::IntoElifResponse;
use crate::errors::HttpResult;
use crate::middleware::v2::{Middleware, MiddlewarePipelineV2};
use service_builder::builder;
use axum::{
    Router as AxumRouter,
    routing::{get, post, put, delete, patch},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::future::Future;

/// Main router for the elif.rs framework
#[derive(Debug)]
pub struct Router<S = ()> 
where 
    S: Clone + Send + Sync + 'static,
{
    axum_router: AxumRouter<S>,
    registry: Arc<Mutex<RouteRegistry>>,
    route_counter: Arc<Mutex<usize>>,
    middleware_stack: MiddlewarePipelineV2,
    middleware_groups: HashMap<String, MiddlewarePipelineV2>,
}

impl<S> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a new router
    pub fn new() -> Self {
        Self {
            axum_router: AxumRouter::new(),
            registry: Arc::new(Mutex::new(RouteRegistry::new())),
            route_counter: Arc::new(Mutex::new(0)),
            middleware_stack: MiddlewarePipelineV2::new(),
            middleware_groups: HashMap::new(),
        }
    }

    /// Create a new router with state
    pub fn with_state(state: S) -> Self {
        Self {
            axum_router: AxumRouter::new().with_state(state),
            registry: Arc::new(Mutex::new(RouteRegistry::new())),
            route_counter: Arc::new(Mutex::new(0)),
            middleware_stack: MiddlewarePipelineV2::new(),
            middleware_groups: HashMap::new(),
        }
    }

    /// Generate a unique route ID
    fn next_route_id(&self) -> String {
        let mut counter = self.route_counter.lock().unwrap();
        *counter += 1;
        format!("route_{}", counter)
    }

    /// Register a route with the registry
    fn register_route(&self, method: HttpMethod, path: &str, name: Option<String>) -> String {
        let route_id = self.next_route_id();
        let params = self.extract_param_names(path);
        
        let route_info = RouteInfo {
            name: name.clone(),
            path: path.to_string(),
            method,
            params,
            group: None, // TODO: Support groups
        };
        
        self.registry.lock().unwrap().register(route_id.clone(), route_info);
        route_id
    }

    /// Extract parameter names from a route path
    fn extract_param_names(&self, path: &str) -> Vec<String> {
        path.split('/')
            .filter_map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    Some(segment[1..segment.len()-1].to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Add global middleware to the router
    pub fn use_middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware_stack = self.middleware_stack.add(middleware);
        self
    }

    /// Create a named middleware group for future use with route-specific middleware
    /// 
    /// Currently, middleware groups are stored but not actively used for request processing.
    /// This method prepares middleware groups for future route-specific middleware functionality.
    /// For now, use `use_middleware()` to add global middleware that will be applied to all routes.
    pub fn middleware_group(mut self, name: &str, middleware: Vec<Arc<dyn Middleware>>) -> Self {
        let pipeline = MiddlewarePipelineV2::from(middleware);
        self.middleware_groups.insert(name.to_string(), pipeline);
        self
    }


    /// Add a GET route with elif handler
    pub fn get<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.register_route(HttpMethod::GET, path, None);
        let method_router = get(elif_handler(handler));
        self.axum_router = self.axum_router.route(path, method_router);
        self
    }

    /// Add a POST route with elif handler
    pub fn post<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.register_route(HttpMethod::POST, path, None);
        self.axum_router = self.axum_router.route(path, post(elif_handler(handler)));
        self
    }

    /// Add a PUT route with elif handler
    pub fn put<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.register_route(HttpMethod::PUT, path, None);
        self.axum_router = self.axum_router.route(path, put(elif_handler(handler)));
        self
    }

    /// Add a DELETE route with elif handler
    pub fn delete<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.register_route(HttpMethod::DELETE, path, None);
        self.axum_router = self.axum_router.route(path, delete(elif_handler(handler)));
        self
    }

    /// Add a PATCH route with elif handler
    pub fn patch<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.register_route(HttpMethod::PATCH, path, None);
        self.axum_router = self.axum_router.route(path, patch(elif_handler(handler)));
        self
    }

    /// Merge another ElifRouter - the primary method for composing routers
    pub fn merge(mut self, other: Router<S>) -> Self {
        // Merge the registries with unique IDs to avoid conflicts
        if let (Ok(mut self_registry), Ok(other_registry)) = 
            (self.registry.lock(), other.registry.lock()) {
            for (_old_id, route_info) in other_registry.all_routes() {
                // Generate a new unique ID for the merged route to avoid ID conflicts
                let new_id = self.next_route_id();
                self_registry.register(new_id, route_info.clone());
            }
        }
        
        // Merge middleware groups
        self.middleware_groups.extend(other.middleware_groups);
        
        // Merge global middleware stacks. The middleware from `self` will run first.
        self.middleware_stack = self.middleware_stack.extend(other.middleware_stack);
        
        // Merge the underlying Axum routers
        self.axum_router = self.axum_router.merge(other.axum_router);
        self
    }

    /// Internal method to merge with Axum router (for framework internals only)
    pub(crate) fn merge_axum(mut self, other: AxumRouter<S>) -> Self {
        self.axum_router = self.axum_router.merge(other);
        self
    }

    /// Nest routes under a path prefix
    /// 
    /// The nested router's global middleware will be applied only to the nested routes,
    /// not to the parent router's routes. This is achieved by converting the nested
    /// router's middleware pipeline into an Axum Layer before nesting.
    pub fn nest(mut self, path: &str, router: Router<S>) -> Self {
        // Note: Nested routes inherit their path prefix, so we don't need to modify the registry paths
        // The registry will contain the original paths, and Axum handles the prefixing internally
        // Merge nested router's routes into parent registry with unique IDs
        if let (Ok(mut self_registry), Ok(router_registry)) = 
            (self.registry.lock(), router.registry.lock()) {
            for (_old_id, route_info) in router_registry.all_routes() {
                // Generate a new unique ID for the nested route to avoid ID conflicts
                let new_id = self.next_route_id();
                self_registry.register(new_id, route_info.clone());
            }
        }
        
        // Merge middleware groups from nested router
        self.middleware_groups.extend(router.middleware_groups);
        
        // Apply nested router's global middleware as a layer before nesting
        // This ensures the middleware only applies to the nested routes
        let has_nested_middleware = !router.middleware_stack.is_empty();
        let nested_middleware = router.middleware_stack.clone();
        let nested_axum_router = router.axum_router;
        
        let nested_axum_router = if has_nested_middleware {
            use axum::middleware::from_fn;
            use axum::extract::Request;
            use axum::response::Response;
            use axum::middleware::Next;
            
            // Create a layer from the nested router's middleware pipeline
            nested_axum_router.layer(from_fn(move |req: Request, next: Next| {
                let pipeline = nested_middleware.clone();
                async move {
                    // Convert axum request to ElifRequest
                    let elif_req = crate::request::ElifRequest::from_axum_request(req).await;
                    
                    // Execute middleware pipeline
                    let response = pipeline.execute(elif_req, |req| {
                        Box::pin(async move {
                            // Convert back to axum request for next handler
                            let axum_req = req.into_axum_request();
                            let axum_response = next.run(axum_req).await;
                            // Convert axum response to ElifResponse
                            crate::response::ElifResponse::from_axum_response(axum_response).await
                        })
                    }).await;
                    
                    // Convert ElifResponse back to axum response
                    response.into_axum_response()
                }
            }))
        } else {
            nested_axum_router
        };
        
        self.axum_router = self.axum_router.nest(path, nested_axum_router);
        self
    }

    /// Internal method to nest with Axum router (for framework internals only)
    pub(crate) fn nest_axum(mut self, path: &str, router: AxumRouter<S>) -> Self {
        self.axum_router = self.axum_router.nest(path, router);
        self
    }

    /// Get the underlying Axum router
    pub fn into_axum_router(self) -> AxumRouter<S> {
        self.axum_router
    }

    /// Get route registry for introspection
    pub fn registry(&self) -> Arc<Mutex<RouteRegistry>> {
        Arc::clone(&self.registry)
    }

    /// Generate URL for a named route
    pub fn url_for(&self, name: &str, params: &HashMap<String, String>) -> Option<String> {
        let registry = self.registry.lock().unwrap();
        if let Some(route) = registry.get_by_name(name) {
            let mut url = route.path.clone();
            for (key, value) in params {
                url = url.replace(&format!("{{{}}}", key), value);
            }
            Some(url)
        } else {
            None
        }
    }

    /// Get the global middleware pipeline
    pub fn middleware_pipeline(&self) -> &MiddlewarePipelineV2 {
        &self.middleware_stack
    }

    /// Get available middleware groups
    pub fn middleware_groups(&self) -> &HashMap<String, MiddlewarePipelineV2> {
        &self.middleware_groups
    }

    /// Add a raw Axum route while preserving router state (for internal use)
    pub(crate) fn add_axum_route(mut self, path: &str, method_router: axum::routing::MethodRouter<S>) -> Self {
        self.axum_router = self.axum_router.route(path, method_router);
        self
    }
}

impl<S> Default for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for RouteBuilder
#[derive(Debug, Clone)]
#[builder]
pub struct RouteBuilderConfig {
    #[builder(optional)]
    pub name: Option<String>,
    
    #[builder(default)]
    pub param_types: HashMap<String, ParamType>,
    
    #[builder(default)]
    pub middleware: Vec<String>, // Placeholder for future middleware support
}

impl RouteBuilderConfig {
    /// Build a Route from the config
    pub fn build_route(self) -> Route {
        Route {
            name: self.name,
            param_types: self.param_types,
            middleware: self.middleware,
        }
    }
}

// Add convenience methods to the generated builder
impl RouteBuilderConfigBuilder {
    /// Add parameter type specification
    pub fn add_param(self, name: &str, param_type: ParamType) -> Self {
        let mut param_types_map = self.param_types.unwrap_or_default();
        param_types_map.insert(name.to_string(), param_type);
        RouteBuilderConfigBuilder {
            name: self.name,
            param_types: Some(param_types_map),
            middleware: self.middleware,
        }
    }
    
    /// Add multiple parameter type specifications
    pub fn add_params(self, params: HashMap<String, ParamType>) -> Self {
        let mut param_types_map = self.param_types.unwrap_or_default();
        param_types_map.extend(params);
        RouteBuilderConfigBuilder {
            name: self.name,
            param_types: Some(param_types_map),
            middleware: self.middleware,
        }
    }
    
    /// Add middleware
    pub fn add_middleware(self, middleware: &str) -> Self {
        let mut middlewares_vec = self.middleware.unwrap_or_default();
        middlewares_vec.push(middleware.to_string());
        RouteBuilderConfigBuilder {
            name: self.name,
            param_types: self.param_types,
            middleware: Some(middlewares_vec),
        }
    }
    
    pub fn build_config(self) -> RouteBuilderConfig {
        self.build_with_defaults().expect("Building RouteBuilderConfig should not fail as all fields have defaults")
    }
}

/// Builder for creating routes with additional metadata
pub struct RouteBuilder {
    builder_config: RouteBuilderConfigBuilder,
}

impl RouteBuilder {
    pub fn new() -> Self {
        Self {
            builder_config: RouteBuilderConfig::builder(),
        }
    }

    /// Set route name for URL generation
    pub fn name(self, name: &str) -> Self {
        Self {
            builder_config: self.builder_config.name(Some(name.to_string())),
        }
    }

    /// Add parameter type specification
    pub fn param(self, name: &str, param_type: ParamType) -> Self {
        Self {
            builder_config: self.builder_config.add_param(name, param_type),
        }
    }

    /// Add multiple parameter type specifications
    pub fn params(self, params: HashMap<String, ParamType>) -> Self {
        Self {
            builder_config: self.builder_config.add_params(params),
        }
    }

    /// Add middleware
    pub fn middleware(self, middleware: &str) -> Self {
        Self {
            builder_config: self.builder_config.add_middleware(middleware),
        }
    }

    /// Build the route configuration
    pub fn build(self) -> Route {
        self.builder_config.build_config().build_route()
    }
}

impl Default for RouteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Route configuration
#[derive(Debug)]
pub struct Route {
    pub name: Option<String>,
    pub param_types: HashMap<String, ParamType>,
    pub middleware: Vec<String>,
}

impl Route {
    pub fn builder() -> RouteBuilder {
        RouteBuilder::new()
    }

    /// Create parameter extractor for this route
    pub fn param_extractor(&self) -> ParamExtractor {
        let mut extractor = ParamExtractor::new();
        for (name, param_type) in &self.param_types {
            extractor = extractor.param(name, param_type.clone());
        }
        extractor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::ElifRequest;
    use crate::response::ElifResponse;
    use crate::errors::HttpResult;

    async fn elif_handler(_req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("Hello, World!"))
    }

    #[test]
    fn test_router_creation() {
        let router = Router::<()>::new()
            .get("/", elif_handler)
            .post("/users", elif_handler)
            .get("/users/{id}", elif_handler);
        
        let registry = router.registry();
        let reg = registry.lock().unwrap();
        assert_eq!(reg.all_routes().len(), 3);
    }

    #[test]
    fn test_param_extraction() {
        let router = Router::<()>::new();
        let params = router.extract_param_names("/users/{id}/posts/{slug}");
        assert_eq!(params, vec!["id", "slug"]);
    }

    #[test]
    fn test_url_generation() {
        let router = Router::<()>::new().get("/users/{id}/posts/{slug}", elif_handler);
        
        // Manually add a named route to registry for testing
        {
            let mut registry = router.registry.lock().unwrap();
            let route_info = RouteInfo {
                name: Some("user.posts.show".to_string()),
                path: "/users/{id}/posts/{slug}".to_string(),
                method: HttpMethod::GET,
                params: vec!["id".to_string(), "slug".to_string()],
                group: None,
            };
            registry.register("test_route".to_string(), route_info);
        }
        
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("slug".to_string(), "hello-world".to_string());
        
        let url = router.url_for("user.posts.show", &params);
        assert_eq!(url, Some("/users/123/posts/hello-world".to_string()));
    }

    #[test]
    fn test_middleware_integration() {
        use crate::middleware::v2::LoggingMiddleware;
        
        let router = Router::<()>::new()
            .use_middleware(LoggingMiddleware)
            .get("/", elif_handler);
        
        // Verify middleware was added to the pipeline
        assert_eq!(router.middleware_pipeline().len(), 1);
        assert_eq!(router.middleware_pipeline().names(), vec!["LoggingMiddleware"]);
    }

    #[test]
    fn test_middleware_groups() {
        use crate::middleware::v2::LoggingMiddleware;
        use std::sync::Arc;
        
        let router = Router::<()>::new()
            .middleware_group("api", vec![Arc::new(LoggingMiddleware)])
            .get("/", elif_handler);
        
        // Verify middleware group was created
        assert!(router.middleware_groups().contains_key("api"));
        
        // Verify the middleware group contains the actual middleware
        let api_group = router.middleware_groups().get("api").unwrap();
        assert_eq!(api_group.len(), 1);
        assert_eq!(api_group.names(), vec!["LoggingMiddleware"]);
    }

    #[test]
    fn test_router_merge_with_middleware() {
        use crate::middleware::v2::{LoggingMiddleware, SimpleAuthMiddleware};
        use std::sync::Arc;
        
        let router1 = Router::<()>::new()
            .use_middleware(LoggingMiddleware)
            .middleware_group("auth", vec![Arc::new(SimpleAuthMiddleware::new("secret".to_string()))]);
        
        let router2 = Router::<()>::new()
            .use_middleware(SimpleAuthMiddleware::new("token".to_string()))
            .middleware_group("api", vec![Arc::new(LoggingMiddleware)]);
        
        let merged = router1.merge(router2);
        
        // Verify that middleware groups were merged
        assert!(merged.middleware_groups().contains_key("auth"));
        assert!(merged.middleware_groups().contains_key("api"));
        
        // Verify middleware groups contain the correct middleware
        let auth_group = merged.middleware_groups().get("auth").unwrap();
        assert_eq!(auth_group.len(), 1);
        assert_eq!(auth_group.names(), vec!["SimpleAuthMiddleware"]);
        
        let api_group = merged.middleware_groups().get("api").unwrap();
        assert_eq!(api_group.len(), 1);
        assert_eq!(api_group.names(), vec!["LoggingMiddleware"]);
        
        // Verify global middleware from both routers is preserved and merged
        assert_eq!(merged.middleware_pipeline().len(), 2);
        assert_eq!(merged.middleware_pipeline().names(), vec!["LoggingMiddleware", "SimpleAuthMiddleware"]);
    }

    #[test]
    fn test_middleware_group_with_multiple_middleware() {
        use crate::middleware::v2::{LoggingMiddleware, SimpleAuthMiddleware};
        use std::sync::Arc;
        
        let router = Router::<()>::new()
            .middleware_group("api", vec![
                Arc::new(LoggingMiddleware),
                Arc::new(SimpleAuthMiddleware::new("secret".to_string()))
            ])
            .get("/", elif_handler);
        
        // Verify middleware group contains both middleware
        let api_group = router.middleware_groups().get("api").unwrap();
        assert_eq!(api_group.len(), 2);
        assert_eq!(api_group.names(), vec!["LoggingMiddleware", "SimpleAuthMiddleware"]);
    }

    #[test]
    fn test_middleware_merge_preserves_global_middleware() {
        use crate::middleware::v2::{LoggingMiddleware, SimpleAuthMiddleware};
        
        // Test that global middleware from both routers is preserved during merge
        let router1 = Router::<()>::new()
            .use_middleware(LoggingMiddleware)
            .use_middleware(SimpleAuthMiddleware::new("router1".to_string()));
        
        let router2 = Router::<()>::new()
            .use_middleware(SimpleAuthMiddleware::new("router2".to_string()))
            .use_middleware(LoggingMiddleware);
        
        let merged = router1.merge(router2);
        
        // Should have all 4 middleware instances: 2 from router1 + 2 from router2
        assert_eq!(merged.middleware_pipeline().len(), 4);
        
        // Verify execution order: router1's middleware first, then router2's
        assert_eq!(
            merged.middleware_pipeline().names(), 
            vec!["LoggingMiddleware", "SimpleAuthMiddleware", "SimpleAuthMiddleware", "LoggingMiddleware"]
        );
    }
    
    #[test]
    fn test_nested_router_middleware_scoping() {
        use crate::middleware::v2::{LoggingMiddleware, SimpleAuthMiddleware};
        use std::sync::Arc;
        
        // Create parent router with its own middleware
        let parent_router = Router::<()>::new()
            .use_middleware(LoggingMiddleware)
            .get("/parent", elif_handler);
        
        // Create nested router with different middleware
        let nested_router = Router::<()>::new()
            .use_middleware(SimpleAuthMiddleware::new("nested_secret".to_string()))
            .get("/nested", elif_handler);
        
        // Nest the router
        let composed_router = parent_router.nest("/api", nested_router);
        
        // Verify parent router's middleware is preserved
        assert_eq!(composed_router.middleware_pipeline().len(), 1);
        assert_eq!(composed_router.middleware_pipeline().names(), vec!["LoggingMiddleware"]);
        
        // Verify route registry contains both routes from both routers
        // The nested router's routes should be merged into the parent registry
        let registry = composed_router.registry();
        let reg = registry.lock().unwrap();
        let route_count = reg.all_routes().len();
        
        // Should have both parent route (/parent) and nested route (/nested)
        // Note: If this fails, the issue is likely in the nest() registry merging logic
        assert_eq!(route_count, 2, "Expected 2 routes after nesting (parent + nested)");
        
        // The nested router's middleware should be applied as a layer to the nested routes
        // This test verifies the structure is correct - actual middleware execution
        // would need integration tests with a running server
    }
    
    #[test]
    fn test_nested_router_middleware_groups_merged() {
        use crate::middleware::v2::LoggingMiddleware;
        use std::sync::Arc;
        
        // Create parent router with middleware group
        let parent_router = Router::<()>::new()
            .middleware_group("parent_group", vec![Arc::new(LoggingMiddleware)])
            .get("/parent", elif_handler);
        
        // Create nested router with different middleware group
        let nested_router = Router::<()>::new()
            .middleware_group("nested_group", vec![Arc::new(LoggingMiddleware)])
            .get("/nested", elif_handler);
        
        // Nest the router
        let composed_router = parent_router.nest("/api", nested_router);
        
        // Verify both middleware groups are preserved
        assert!(composed_router.middleware_groups().contains_key("parent_group"));
        assert!(composed_router.middleware_groups().contains_key("nested_group"));
        
        // Verify the groups contain the correct middleware
        let parent_group = composed_router.middleware_groups().get("parent_group").unwrap();
        assert_eq!(parent_group.len(), 1);
        assert_eq!(parent_group.names(), vec!["LoggingMiddleware"]);
        
        let nested_group = composed_router.middleware_groups().get("nested_group").unwrap();
        assert_eq!(nested_group.len(), 1);
        assert_eq!(nested_group.names(), vec!["LoggingMiddleware"]);
    }
    
    #[test]
    fn test_nested_router_empty_middleware_optimization() {
        use crate::middleware::v2::LoggingMiddleware;
        
        // Create parent router with middleware
        let parent_router = Router::<()>::new()
            .use_middleware(LoggingMiddleware)
            .get("/parent", elif_handler);
        
        // Create nested router WITHOUT middleware (empty pipeline)
        let nested_router = Router::<()>::new()
            .get("/nested", elif_handler);
        
        // Verify nested router has empty middleware
        assert_eq!(nested_router.middleware_pipeline().len(), 0);
        assert!(nested_router.middleware_pipeline().is_empty());
        
        // Nest the router
        let composed_router = parent_router.nest("/api", nested_router);
        
        // Parent middleware should still be there
        assert_eq!(composed_router.middleware_pipeline().len(), 1);
        assert_eq!(composed_router.middleware_pipeline().names(), vec!["LoggingMiddleware"]);
        
        // The implementation should optimize for empty nested middleware
        // (no unnecessary layer application)
    }
    
    #[test]
    fn test_simple_nest_route_registration() {
        // Test basic nesting without middleware to understand route registration behavior
        let parent_router = Router::<()>::new()
            .get("/parent", elif_handler);
        
        let nested_router = Router::<()>::new()
            .get("/nested", elif_handler);
        
        // Check initial route counts
        assert_eq!(parent_router.registry().lock().unwrap().all_routes().len(), 1);
        assert_eq!(nested_router.registry().lock().unwrap().all_routes().len(), 1);
        
        // Nest the routers
        let composed_router = parent_router.nest("/api", nested_router);
        
        // Check final route count
        let route_count = composed_router.registry().lock().unwrap().all_routes().len();
        
        // Verify that nesting merges route registries correctly
        // We expect the nested router's routes to be merged into the parent registry
        assert_eq!(route_count, 2, "Expected both parent and nested routes to be registered, got: {}", route_count);
    }
}