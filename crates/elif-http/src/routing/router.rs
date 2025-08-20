//! Core routing functionality

use super::{HttpMethod, RouteInfo, RouteRegistry, params::{ParamExtractor, ParamType}};
use crate::handlers::elif_handler;
use crate::request::ElifRequest;
use crate::response::{IntoElifResponse, ElifResponse};
use crate::errors::HttpResult;
use crate::middleware::v2::{Middleware, MiddlewarePipelineV2};
use crate::controller::ElifController;
use service_builder::builder;
use std::pin::Pin;
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
    controller_registry: Arc<Mutex<ControllerRegistry>>,
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
            controller_registry: Arc::new(Mutex::new(ControllerRegistry::new())),
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
            controller_registry: Arc::new(Mutex::new(ControllerRegistry::new())),
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
    
    /// Extend router middleware with external middleware pipeline
    /// External middleware will be executed before the router's own middleware
    pub fn extend_middleware(mut self, external_middleware: MiddlewarePipelineV2) -> Self {
        self.middleware_stack = external_middleware.extend(self.middleware_stack);
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

    /// Register a controller with automatic route registration
    pub fn controller<C>(mut self, controller: C) -> Self
    where
        C: ElifController + 'static,
    {
        let base_path = controller.base_path().to_string();
        let controller_name = controller.name().to_string();
        let controller_arc = Arc::new(controller);
        
        // Register all controller routes
        for route in controller_arc.routes() {
            let full_path = self.combine_paths(&base_path, &route.path);
            let handler = controller_handler(Arc::clone(&controller_arc), route.handler_name.clone());
            
            self = match route.method {
                HttpMethod::GET => self.get(&full_path, handler),
                HttpMethod::POST => self.post(&full_path, handler),
                HttpMethod::PUT => self.put(&full_path, handler),
                HttpMethod::DELETE => self.delete(&full_path, handler),
                HttpMethod::PATCH => self.patch(&full_path, handler),
                _ => {
                    // For unsupported HTTP methods, we'll skip for now
                    // This can be extended to support more methods
                    continue;
                }
            };
            
            // TODO: Apply route-specific middleware
            // This will be implemented when middleware system is enhanced
        }
        
        // Store controller reference for introspection
        if let Ok(mut registry) = self.controller_registry.lock() {
            registry.register(controller_name, controller_arc as Arc<dyn ElifController>);
        }
        
        self
    }
    
    /// Helper method to combine base path and route path
    fn combine_paths(&self, base: &str, route: &str) -> String {
        let base = base.trim_end_matches('/');
        let route = route.trim_start_matches('/');
        
        if route.is_empty() {
            base.to_string()
        } else if base.is_empty() {
            format!("/{}", route)
        } else {
            format!("{}/{}", base, route)
        }
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
        
        // Merge controller registries
        if let (Ok(mut self_controller_registry), Ok(other_controller_registry)) = 
            (self.controller_registry.lock(), other.controller_registry.lock()) {
            for (name, controller) in other_controller_registry.all_controllers() {
                self_controller_registry.register(name.clone(), Arc::clone(controller));
            }
        }
        
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

    /// Get the controller registry for introspection
    pub fn controller_registry(&self) -> Arc<Mutex<ControllerRegistry>> {
        Arc::clone(&self.controller_registry)
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
    fn test_controller_registration() {
        use crate::controller::{ElifController, ControllerRoute, RouteParam};
        use crate::routing::params::ParamType;
        use std::pin::Pin;
        use std::future::Future;

        // Create a test controller
        struct TestController;

        impl ElifController for TestController {
            fn name(&self) -> &str { "TestController" }
            fn base_path(&self) -> &str { "/test" }
            
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![
                    ControllerRoute::new(HttpMethod::GET, "", "list"),
                    ControllerRoute::new(HttpMethod::GET, "/{id}", "show")
                        .add_param(RouteParam::new("id", ParamType::Integer)),
                    ControllerRoute::new(HttpMethod::POST, "", "create"),
                ]
            }
            
            fn handle_request(
                &self,
                method_name: String,
                _request: ElifRequest,
            ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> {
                Box::pin(async move {
                    match method_name.as_str() {
                        "list" => Ok(ElifResponse::ok().text("List method")),
                        "show" => Ok(ElifResponse::ok().text("Show method")),
                        "create" => Ok(ElifResponse::ok().text("Create method")),
                        _ => Ok(ElifResponse::not_found().text("Method not found")),
                    }
                })
            }
        }

        let controller = TestController;
        let router = Router::<()>::new().controller(controller);
        
        // Check that routes were registered
        let registry = router.registry();
        let reg = registry.lock().unwrap();
        assert_eq!(reg.all_routes().len(), 3);
        
        // Check that controller was registered
        let controller_registry = router.controller_registry();
        let ctrl_reg = controller_registry.lock().unwrap();
        assert!(ctrl_reg.get_controller("TestController").is_some());
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

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::middleware::v2::{Middleware, Next, NextFuture};
    use crate::response::ElifResponse;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;
    use serde_json::json;

    /// Test middleware that adds headers to verify execution
    #[derive(Debug, Clone)]
    struct HeaderTestMiddleware {
        name: String,
        counter: Arc<Mutex<usize>>,
    }

    impl HeaderTestMiddleware {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                counter: Arc::new(Mutex::new(0)),
            }
        }

    }

    impl Middleware for HeaderTestMiddleware {
        fn handle(&self, mut request: ElifRequest, next: Next) -> NextFuture<'static> {
            let name = self.name.clone();
            let counter = self.counter.clone();
            
            Box::pin(async move {
                // Increment counter to track execution
                {
                    let mut count = counter.lock().unwrap();
                    *count += 1;
                }

                // Add request header to track middleware execution
                let header_name = crate::response::headers::ElifHeaderName::from_str(&format!("x-middleware-{}", name.to_lowercase())).unwrap();
                let header_value = crate::response::headers::ElifHeaderValue::from_str("executed").unwrap();
                request.headers.insert(header_name, header_value);

                // Call next in chain
                let response = next.run(request).await;

                // Add response header to verify middleware ran after handler  
                let response_header = format!("x-response-{}", name.to_lowercase());
                response.header(&response_header, "executed").unwrap_or_else(|_| {
                    // Return original response if header addition fails
                    ElifResponse::ok().text("Middleware executed")
                })
            })
        }

        fn name(&self) -> &'static str {
            // Return static string for consistency in tests - we need to leak the string to make it 'static
            match self.name.as_str() {
                "Parent" => "Parent",
                "Nested" => "Nested", 
                "Global" => "Global",
                "First" => "First",
                "Second" => "Second",
                "Third" => "Third",
                "Router1" => "Router1",
                "Router2" => "Router2",
                _ => "TestMiddleware",
            }
        }
    }

    // Simple test handlers
    async fn test_handler(request: ElifRequest) -> HttpResult<ElifResponse> {
        // Return headers from request so we can verify middleware execution
        let mut response_headers = HashMap::new();
        
        for (key, value) in request.headers.iter() {
            let key_str = key.as_str().to_string();
            if let Ok(value_str) = value.to_str() {
                response_headers.insert(key_str, value_str.to_string());
            }
        }
        
        Ok(ElifResponse::ok().json(&json!({
            "message": "Hello from handler",
            "request_headers": response_headers,
            "path": request.path()
        }))?)
    }

    async fn nested_handler(request: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().json(&json!({
            "message": "Hello from nested handler", 
            "path": request.path()
        }))?)
    }


    #[tokio::test]
    async fn test_global_middleware_execution() {
        // Create middleware that we can verify execution for
        let test_middleware = HeaderTestMiddleware::new("Global");
        let middleware_counter = test_middleware.counter.clone();
        
        // Create router with global middleware
        let router = Router::<()>::new()
            .use_middleware(test_middleware)
            .get("/test", test_handler);
        
        // Verify middleware is in the pipeline
        assert_eq!(router.middleware_pipeline().len(), 1);
        assert_eq!(router.middleware_pipeline().names(), vec!["Global"]);
        
        // Verify counter starts at 0
        assert_eq!(middleware_counter.lock().unwrap().clone(), 0);
        
        // Test middleware execution through pipeline (unit test style)
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = router.middleware_pipeline().execute(request, |req| {
            Box::pin(async move {
                // Verify middleware added the header
                let header_name = crate::response::headers::ElifHeaderName::from_str("x-middleware-global").unwrap();
                assert!(req.headers.contains_key(&header_name));
                ElifResponse::ok().text("Pipeline test response")
            })
        }).await;
        
        // Verify middleware was executed once
        assert_eq!(middleware_counter.lock().unwrap().clone(), 1);
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_nested_router_middleware_isolation() {
        // Create different middleware for parent and nested routers
        let parent_middleware = HeaderTestMiddleware::new("Parent");
        let nested_middleware = HeaderTestMiddleware::new("Nested");
        
        let parent_counter = parent_middleware.counter.clone();
        let nested_counter = nested_middleware.counter.clone();
        
        // Create parent router with its middleware
        let parent_router = Router::<()>::new()
            .use_middleware(parent_middleware)
            .get("/parent", test_handler);
        
        // Create nested router with different middleware  
        let nested_router = Router::<()>::new()
            .use_middleware(nested_middleware)
            .get("/nested", nested_handler);
        
        // Compose the routers
        let composed_router = parent_router.nest("/api", nested_router);
        
        // Verify structure
        assert_eq!(composed_router.middleware_pipeline().len(), 1); // Only parent middleware in global
        assert_eq!(composed_router.middleware_pipeline().names(), vec!["Parent"]);
        
        // Test that both middleware start with 0 executions
        assert_eq!(parent_counter.lock().unwrap().clone(), 0);
        assert_eq!(nested_counter.lock().unwrap().clone(), 0);
        
        // Test middleware isolation by manually executing pipelines
        
        // 1. Test parent route - should only execute parent middleware
        let parent_request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/parent".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let parent_response = composed_router.middleware_pipeline().execute(parent_request, |req| {
            Box::pin(async move {
                // Should have parent middleware header, not nested
                let parent_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-parent").unwrap();
                let nested_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-nested").unwrap();
                assert!(req.headers.contains_key(&parent_header));
                assert!(!req.headers.contains_key(&nested_header));
                ElifResponse::ok().text("Parent response")
            })
        }).await;
        
        assert_eq!(parent_response.status_code(), crate::response::status::ElifStatusCode::OK);
        assert_eq!(parent_counter.lock().unwrap().clone(), 1);
        assert_eq!(nested_counter.lock().unwrap().clone(), 0); // Nested middleware should not execute
        
        // Note: Testing nested route middleware execution would require integration with Axum
        // The structural test above verifies that the middleware scoping is set up correctly
        // Runtime testing would require a full HTTP server setup
    }

    #[tokio::test]
    async fn test_middleware_execution_order() {
        // Create middleware with execution tracking
        let first_middleware = HeaderTestMiddleware::new("First");
        let second_middleware = HeaderTestMiddleware::new("Second");
        let third_middleware = HeaderTestMiddleware::new("Third");
        
        let first_counter = first_middleware.counter.clone();
        let second_counter = second_middleware.counter.clone();
        let third_counter = third_middleware.counter.clone();
        
        // Create router with multiple middleware
        let router = Router::<()>::new()
            .use_middleware(first_middleware)
            .use_middleware(second_middleware)
            .use_middleware(third_middleware)
            .get("/test", test_handler);
        
        // Verify all middleware are in pipeline
        assert_eq!(router.middleware_pipeline().len(), 3);
        assert_eq!(router.middleware_pipeline().names(), vec!["First", "Second", "Third"]);
        
        // Execute through pipeline
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = router.middleware_pipeline().execute(request, |req| {
            Box::pin(async move {
                // All middleware should have executed and added headers
                let first_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-first").unwrap();
                let second_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-second").unwrap();
                let third_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-third").unwrap();
                assert!(req.headers.contains_key(&first_header));
                assert!(req.headers.contains_key(&second_header));
                assert!(req.headers.contains_key(&third_header));
                
                ElifResponse::ok().text("Handler executed after all middleware")
            })
        }).await;
        
        // Verify all middleware executed exactly once
        assert_eq!(first_counter.lock().unwrap().clone(), 1);
        assert_eq!(second_counter.lock().unwrap().clone(), 1);
        assert_eq!(third_counter.lock().unwrap().clone(), 1);
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test] 
    async fn test_router_merge_middleware_execution() {
        // Create different middleware for each router
        let router1_middleware = HeaderTestMiddleware::new("Router1");
        let router2_middleware = HeaderTestMiddleware::new("Router2");
        
        let router1_counter = router1_middleware.counter.clone();
        let router2_counter = router2_middleware.counter.clone();
        
        // Create two routers with different middleware
        let router1 = Router::<()>::new()
            .use_middleware(router1_middleware)
            .get("/router1", test_handler);
            
        let router2 = Router::<()>::new()
            .use_middleware(router2_middleware)
            .get("/router2", test_handler);
        
        // Merge the routers
        let merged_router = router1.merge(router2);
        
        // Verify merged middleware pipeline contains both middleware
        assert_eq!(merged_router.middleware_pipeline().len(), 2);
        assert_eq!(merged_router.middleware_pipeline().names(), vec!["Router1", "Router2"]);
        
        // Test execution through merged pipeline
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response = merged_router.middleware_pipeline().execute(request, |req| {
            Box::pin(async move {
                // Both router middleware should have executed
                let router1_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-router1").unwrap();
                let router2_header = crate::response::headers::ElifHeaderName::from_str("x-middleware-router2").unwrap();
                assert!(req.headers.contains_key(&router1_header));
                assert!(req.headers.contains_key(&router2_header));
                
                ElifResponse::ok().text("Merged router response")
            })
        }).await;
        
        // Verify both middleware executed
        assert_eq!(router1_counter.lock().unwrap().clone(), 1);
        assert_eq!(router2_counter.lock().unwrap().clone(), 1);
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_with_early_return() {
        /// Middleware that returns early based on a condition
        #[derive(Debug)]
        struct AuthMiddleware {
            required_token: String,
        }
        
        impl AuthMiddleware {
            fn new(token: String) -> Self {
                Self { required_token: token }
            }
        }
        
        impl Middleware for AuthMiddleware {
            fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
                let required_token = self.required_token.clone();
                Box::pin(async move {
                    // Check authorization header
                    let auth_header = request.header("authorization")
                        .and_then(|h| h.to_str().ok());
                    
                    match auth_header {
                        Some(header) if header.starts_with("Bearer ") => {
                            let token = &header[7..];
                            if token == required_token {
                                // Token is valid, proceed
                                next.run(request).await
                            } else {
                                // Invalid token, return early
                                ElifResponse::unauthorized()
                                    .json_value(json!({
                                        "error": {
                                            "code": "invalid_token",
                                            "message": "Invalid authorization token"
                                        }
                                    }))
                            }
                        }
                        _ => {
                            // Missing or malformed auth header
                            ElifResponse::unauthorized()
                                .json_value(json!({
                                    "error": {
                                        "code": "missing_token",
                                        "message": "Authorization header required"
                                    }
                                }))
                        }
                    }
                })
            }
            
            fn name(&self) -> &'static str {
                "AuthMiddleware"
            }
        }
        
        // Create router with auth middleware
        let router = Router::<()>::new()
            .use_middleware(AuthMiddleware::new("secret123".to_string()))
            .get("/protected", test_handler);
        
        // Test request without auth header (should return early)
        let request_no_auth = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/protected".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );
        
        let response_no_auth = router.middleware_pipeline().execute(request_no_auth, |_req| {
            Box::pin(async move {
                // This handler should NOT be called due to early return
                panic!("Handler should not be called when auth fails");
            })
        }).await;
        
        assert_eq!(response_no_auth.status_code(), crate::response::status::ElifStatusCode::UNAUTHORIZED);
        
        // Test request with valid auth header (should proceed to handler)
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let auth_header = crate::response::headers::ElifHeaderName::from_str("authorization").unwrap();
        let auth_value = crate::response::headers::ElifHeaderValue::from_str("Bearer secret123").unwrap();
        headers.insert(auth_header, auth_value);
        let request_valid_auth = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/protected".parse().unwrap(),
            headers,
        );
        
        let response_valid_auth = router.middleware_pipeline().execute(request_valid_auth, |req| {
            Box::pin(async move {
                // Handler should be called with valid auth
                assert!(req.header("authorization").is_some());
                ElifResponse::ok().text("Protected content accessed")
            })
        }).await;
        
        assert_eq!(response_valid_auth.status_code(), crate::response::status::ElifStatusCode::OK);
        
        // Test request with invalid auth token (should return early) 
        let mut headers = crate::response::headers::ElifHeaderMap::new();
        let auth_header = crate::response::headers::ElifHeaderName::from_str("authorization").unwrap();
        let auth_value = crate::response::headers::ElifHeaderValue::from_str("Bearer invalid").unwrap();
        headers.insert(auth_header, auth_value);
        let request_invalid_auth = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/protected".parse().unwrap(),
            headers,
        );
        
        let response_invalid_auth = router.middleware_pipeline().execute(request_invalid_auth, |_req| {
            Box::pin(async move {
                // Handler should NOT be called with invalid token
                panic!("Handler should not be called when auth token is invalid");
            })
        }).await;
        
        assert_eq!(response_invalid_auth.status_code(), crate::response::status::ElifStatusCode::UNAUTHORIZED);
    }
}

/// Create a controller handler function
pub fn controller_handler<C>(controller: Arc<C>, method_name: String) -> impl Fn(ElifRequest) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> + Clone + Send + Sync + 'static
where
    C: ElifController + 'static,
{
    move |request: ElifRequest| {
        let controller = Arc::clone(&controller);
        let method_name = method_name.clone();
        
        Box::pin(async move {
            controller.handle_request(method_name, request).await
        })
    }
}

/// Registry for managing registered controllers
pub struct ControllerRegistry {
    controllers: HashMap<String, Arc<dyn ElifController>>,
}

impl std::fmt::Debug for ControllerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControllerRegistry")
            .field("controllers", &format!("{} controllers", self.controllers.len()))
            .finish()
    }
}

impl ControllerRegistry {
    pub fn new() -> Self {
        Self {
            controllers: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, name: String, controller: Arc<dyn ElifController>) {
        self.controllers.insert(name, controller);
    }
    
    pub fn get_controller(&self, name: &str) -> Option<&Arc<dyn ElifController>> {
        self.controllers.get(name)
    }
    
    pub fn all_controllers(&self) -> impl Iterator<Item = (&String, &Arc<dyn ElifController>)> {
        self.controllers.iter()
    }
    
    pub fn controller_names(&self) -> Vec<&String> {
        self.controllers.keys().collect()
    }
}