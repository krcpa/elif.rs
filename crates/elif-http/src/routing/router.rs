//! Core routing functionality

use super::{HttpMethod, RouteInfo, RouteRegistry};
use crate::handlers::elif_handler;
use crate::request::ElifRequest;
use crate::response::{IntoElifResponse, ElifResponse};
use crate::errors::HttpResult;
use crate::middleware::v2::{Middleware, MiddlewarePipelineV2};
use crate::controller::{ElifController, factory::IocControllable};
use std::pin::Pin;
use axum::{
    Router as AxumRouter,
    routing::{get, post, put, delete, patch},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::future::Future;
use elif_core::container::IocContainer;

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
    route_middleware: HashMap<String, Vec<String>>, // route_id -> middleware group names
    controller_registry: Arc<Mutex<ControllerRegistry>>,
    ioc_container: Option<Arc<IocContainer>>,
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
            route_middleware: HashMap::new(),
            controller_registry: Arc::new(Mutex::new(ControllerRegistry::new())),
            ioc_container: None,
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
            route_middleware: HashMap::new(),
            controller_registry: Arc::new(Mutex::new(ControllerRegistry::new())),
            ioc_container: None,
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

    /// Create a named middleware group for use with route-specific middleware
    pub fn middleware_group(mut self, name: &str, middleware: Vec<Arc<dyn Middleware>>) -> Self {
        let pipeline = MiddlewarePipelineV2::from(middleware);
        self.middleware_groups.insert(name.to_string(), pipeline);
        self
    }

    /// Create a route builder for defining routes with middleware groups
    pub fn route(self, path: &str) -> RouteBuilder<S> {
        RouteBuilder::new(self, path.to_string())
    }


    /// Private helper method to add routes with less duplication
    fn add_route<F, Fut, R, M>(mut self, method: HttpMethod, path: &str, handler: F, method_router_fn: M) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
        M: FnOnce(crate::handlers::handler::ElifHandlerWrapper<F, Fut, R>) -> axum::routing::MethodRouter<S>,
    {
        self.register_route(method, path, None);
        let method_router = method_router_fn(elif_handler(handler));
        self.axum_router = self.axum_router.route(path, method_router);
        self
    }

    /// Add a GET route with elif handler
    pub fn get<F, Fut, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_route(HttpMethod::GET, path, handler, get)
    }

    /// Add a POST route with elif handler
    pub fn post<F, Fut, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_route(HttpMethod::POST, path, handler, post)
    }

    /// Add a PUT route with elif handler
    pub fn put<F, Fut, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_route(HttpMethod::PUT, path, handler, put)
    }

    /// Add a DELETE route with elif handler
    pub fn delete<F, Fut, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_route(HttpMethod::DELETE, path, handler, delete)
    }

    /// Add a PATCH route with elif handler
    pub fn patch<F, Fut, R>(self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_route(HttpMethod::PATCH, path, handler, patch)
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

    /// Set the IoC container for controller dependency injection
    pub fn with_ioc_container(mut self, container: Arc<IocContainer>) -> Self {
        self.ioc_container = Some(container);
        self
    }

    /// Register a controller type that will be resolved from the IoC container
    /// The controller type must implement both ElifController and IocControllable
    pub fn controller_from_container<C>(self) -> Self
    where
        C: ElifController + IocControllable + 'static,
    {
        let container = self.ioc_container.as_ref()
            .expect("IoC container must be set before registering IoC controllers. Use .with_ioc_container() first");
        let container_arc = Arc::clone(container);

        // Create a temporary controller instance to get its metadata
        let temp_controller = match C::from_ioc_container(container, None) {
            Ok(controller) => controller,
            Err(err) => {
                eprintln!("Warning: Failed to create controller for route registration: {}", err);
                return self;
            }
        };

        let base_path = temp_controller.base_path().to_string();
        let controller_name = temp_controller.name().to_string();

        // Register all controller routes with IoC-resolved handlers
        let mut router = self;
        for route in temp_controller.routes() {
            let full_path = router.combine_paths(&base_path, &route.path);
            let handler = ioc_controller_handler::<C>(
                Arc::clone(&container_arc),
                route.handler_name.clone(),
            );
            
            router = match route.method {
                HttpMethod::GET => router.get(&full_path, handler),
                HttpMethod::POST => router.post(&full_path, handler),
                HttpMethod::PUT => router.put(&full_path, handler),
                HttpMethod::DELETE => router.delete(&full_path, handler),
                HttpMethod::PATCH => router.patch(&full_path, handler),
                _ => {
                    // For unsupported HTTP methods, we'll skip for now
                    continue;
                }
            };
            
            // TODO: Apply route-specific middleware
        }
        
        // Store controller type info for introspection (using a dummy instance)
        if let Ok(mut registry) = router.controller_registry.lock() {
            let controller_arc = Arc::new(temp_controller);
            registry.register(controller_name, controller_arc as Arc<dyn ElifController>);
        }
        
        router
    }

    /// Register a controller type with request-scoped dependency injection
    /// This creates controllers per request with scoped dependencies
    pub fn scoped_controller_from_container<C>(self) -> Self
    where
        C: ElifController + IocControllable + 'static,
    {
        let container = self.ioc_container.as_ref()
            .expect("IoC container must be set before registering IoC controllers. Use .with_ioc_container() first");
        let container_arc = Arc::clone(container);

        // Create a temporary controller instance to get its metadata
        let temp_controller = match C::from_ioc_container(container, None) {
            Ok(controller) => controller,
            Err(err) => {
                eprintln!("Warning: Failed to create controller for route registration: {}", err);
                return self;
            }
        };

        let base_path = temp_controller.base_path().to_string();
        let controller_name = temp_controller.name().to_string();

        // Register all controller routes with scoped IoC-resolved handlers
        let mut router = self;
        for route in temp_controller.routes() {
            let full_path = router.combine_paths(&base_path, &route.path);
            let handler = scoped_ioc_controller_handler::<C>(
                Arc::clone(&container_arc),
                route.handler_name.clone(),
            );
            
            router = match route.method {
                HttpMethod::GET => router.get(&full_path, handler),
                HttpMethod::POST => router.post(&full_path, handler),
                HttpMethod::PUT => router.put(&full_path, handler),
                HttpMethod::DELETE => router.delete(&full_path, handler),
                HttpMethod::PATCH => router.patch(&full_path, handler),
                _ => {
                    continue;
                }
            };
        }
        
        // Store controller type info
        if let Ok(mut registry) = router.controller_registry.lock() {
            let controller_arc = Arc::new(temp_controller);
            registry.register(controller_name, controller_arc as Arc<dyn ElifController>);
        }
        
        router
    }
    
    /// Helper method to combine base path and route path
    fn combine_paths(&self, base: &str, route: &str) -> String {
        let base = base.trim_end_matches('/');
        let route = route.trim_start_matches('/');
        
        let path = if route.is_empty() {
            base.to_string()
        } else if base.is_empty() {
            format!("/{}", route)
        } else {
            format!("{}/{}", base, route)
        };

        // Ensure path is never empty to prevent Axum panics
        if path.is_empty() {
            "/".to_string()
        } else {
            path
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
        
        // Merge middleware groups and route-specific middleware mappings
        self.middleware_groups.extend(other.middleware_groups);
        self.route_middleware.extend(other.route_middleware);
        
        // Merge controller registries - avoid deadlock by not holding both locks simultaneously
        if let Ok(other_controller_registry) = other.controller_registry.lock() {
            let controllers_to_merge: Vec<_> = other_controller_registry
                .all_controllers()
                .map(|(name, controller)| (name.clone(), Arc::clone(controller)))
                .collect();
            
            // Release the other lock before acquiring self lock
            drop(other_controller_registry);
            
            if let Ok(mut self_controller_registry) = self.controller_registry.lock() {
                for (name, controller) in controllers_to_merge {
                    self_controller_registry.register(name, controller);
                }
            }
        }
        
        // Merge IoC containers (prefer self's container if both exist)
        if self.ioc_container.is_none() && other.ioc_container.is_some() {
            self.ioc_container = other.ioc_container;
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
        
        // Merge middleware groups and route-specific middleware mappings from nested router
        self.middleware_groups.extend(router.middleware_groups);
        self.route_middleware.extend(router.route_middleware);
        
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

    /// Get route-specific middleware mappings
    pub fn route_middleware(&self) -> &HashMap<String, Vec<String>> {
        &self.route_middleware
    }

    /// Get the controller registry for introspection
    pub fn controller_registry(&self) -> Arc<Mutex<ControllerRegistry>> {
        Arc::clone(&self.controller_registry)
    }

    /// Get the IoC container if set
    pub fn ioc_container(&self) -> Option<&Arc<IocContainer>> {
        self.ioc_container.as_ref()
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

/// Builder for creating routes with middleware groups and additional metadata
pub struct RouteBuilder<S = ()>
where
    S: Clone + Send + Sync + 'static,
{
    router: Router<S>,
    path: String,
    middleware_groups: Vec<String>,
    name: Option<String>,
}

impl<S> RouteBuilder<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(router: Router<S>, path: String) -> Self {
        Self {
            router,
            path,
            middleware_groups: Vec::new(),
            name: None,
        }
    }

    /// Apply a middleware group to this route
    pub fn use_group(mut self, group_name: &str) -> Self {
        self.middleware_groups.push(group_name.to_string());
        self
    }

    /// Set route name for URL generation
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Private helper method for RouteBuilder route registration
    fn add_method_route<F, Fut, R, M>(mut self, method: HttpMethod, handler: F, method_router_fn: M) -> Router<S>
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
        M: FnOnce(crate::handlers::handler::ElifHandlerWrapper<F, Fut, R>) -> axum::routing::MethodRouter<S>,
    {
        let route_id = self.router.register_route(method, &self.path, self.name.clone());
        self.router.route_middleware.insert(route_id, self.middleware_groups);
        let method_router = method_router_fn(elif_handler(handler));
        self.router.axum_router = self.router.axum_router.route(&self.path, method_router);
        self.router
    }

    /// Add a GET route with elif handler
    pub fn get<F, Fut, R>(self, handler: F) -> Router<S>
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_method_route(HttpMethod::GET, handler, get)
    }

    /// Add a POST route with elif handler
    pub fn post<F, Fut, R>(self, handler: F) -> Router<S>
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_method_route(HttpMethod::POST, handler, post)
    }

    /// Add a PUT route with elif handler
    pub fn put<F, Fut, R>(self, handler: F) -> Router<S>
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_method_route(HttpMethod::PUT, handler, put)
    }

    /// Add a DELETE route with elif handler
    pub fn delete<F, Fut, R>(self, handler: F) -> Router<S>
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_method_route(HttpMethod::DELETE, handler, delete)
    }

    /// Add a PATCH route with elif handler
    pub fn patch<F, Fut, R>(self, handler: F) -> Router<S>
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.add_method_route(HttpMethod::PATCH, handler, patch)
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

        // Create a test controller
        struct TestController;

        #[async_trait::async_trait]
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
            
            async fn handle_request(
                self: std::sync::Arc<Self>,
                method_name: String,
                _request: ElifRequest,
            ) -> HttpResult<ElifResponse> {
                match method_name.as_str() {
                    "list" => Ok(ElifResponse::ok().text("List method")),
                    "show" => Ok(ElifResponse::ok().text("Show method")),
                    "create" => Ok(ElifResponse::ok().text("Create method")),
                    _ => Ok(ElifResponse::not_found().text("Method not found")),
                }
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
    fn test_combine_paths_edge_cases() {
        let router = Router::<()>::new();
        
        // Test normal cases
        assert_eq!(router.combine_paths("/users", "/posts"), "/users/posts");
        assert_eq!(router.combine_paths("/users", "posts"), "/users/posts");
        assert_eq!(router.combine_paths("users", "/posts"), "users/posts");
        assert_eq!(router.combine_paths("users", "posts"), "users/posts");
        
        // Test edge cases that could produce empty paths
        assert_eq!(router.combine_paths("", ""), "/");  // Both empty -> root
        assert_eq!(router.combine_paths("/", ""), "/"); // Base is root, route empty
        assert_eq!(router.combine_paths("", "/"), "/"); // Base empty, route is root
        assert_eq!(router.combine_paths("/", "/"), "/"); // Both are root
        
        // Test with trailing/leading slashes
        assert_eq!(router.combine_paths("/users/", ""), "/users");
        assert_eq!(router.combine_paths("/users/", "/posts"), "/users/posts");
        assert_eq!(router.combine_paths("/", "posts"), "/posts");
        assert_eq!(router.combine_paths("users", "/"), "users");
    }

    #[test]
    fn test_controller_with_root_base_path() {
        use crate::controller::{ElifController, ControllerRoute};

        // Create a controller with root base path
        struct RootController;

        #[async_trait::async_trait]
        impl ElifController for RootController {
            fn name(&self) -> &str { "RootController" }
            fn base_path(&self) -> &str { "/" } // Root base path
            
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![
                    ControllerRoute::new(HttpMethod::GET, "", "home"), // Should become "/"
                    ControllerRoute::new(HttpMethod::GET, "/health", "health"), // Should become "/health"
                ]
            }
            
            async fn handle_request(
                self: std::sync::Arc<Self>,
                method_name: String,
                _request: ElifRequest,
            ) -> HttpResult<ElifResponse> {
                match method_name.as_str() {
                    "home" => Ok(ElifResponse::ok().text("Home")),
                    "health" => Ok(ElifResponse::ok().text("Health")),
                    _ => Ok(ElifResponse::not_found().text("Not found")),
                }
            }
        }

        let controller = RootController;
        let router = Router::<()>::new().controller(controller);
        
        // Check that routes were registered correctly
        let registry = router.registry();
        let reg = registry.lock().unwrap();
        let routes = reg.all_routes();
        
        // Should have 2 routes
        assert_eq!(routes.len(), 2);
        
        // Verify paths are correct (one should be "/" and one should be "/health")
        let paths: Vec<&String> = routes.values().map(|route| &route.path).collect();
        assert!(paths.contains(&&"/".to_string()));
        assert!(paths.contains(&&"/health".to_string()));
    }

    #[test]
    fn test_controller_with_empty_base_path() {
        use crate::controller::{ElifController, ControllerRoute};

        // Create a controller with empty base path
        struct EmptyBaseController;

        #[async_trait::async_trait]
        impl ElifController for EmptyBaseController {
            fn name(&self) -> &str { "EmptyBaseController" }
            fn base_path(&self) -> &str { "" } // Empty base path
            
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![
                    ControllerRoute::new(HttpMethod::GET, "", "root"), // Should become "/"
                    ControllerRoute::new(HttpMethod::GET, "/api", "api"), // Should become "/api"
                ]
            }
            
            async fn handle_request(
                self: std::sync::Arc<Self>,
                method_name: String,
                _request: ElifRequest,
            ) -> HttpResult<ElifResponse> {
                match method_name.as_str() {
                    "root" => Ok(ElifResponse::ok().text("Root")),
                    "api" => Ok(ElifResponse::ok().text("API")),
                    _ => Ok(ElifResponse::not_found().text("Not found")),
                }
            }
        }

        let controller = EmptyBaseController;
        let router = Router::<()>::new().controller(controller);
        
        // Check that routes were registered correctly
        let registry = router.registry();
        let reg = registry.lock().unwrap();
        let routes = reg.all_routes();
        
        // Should have 2 routes  
        assert_eq!(routes.len(), 2);
        
        // Verify paths are correct
        let paths: Vec<&String> = routes.values().map(|route| &route.path).collect();
        assert!(paths.contains(&&"/".to_string()));
        assert!(paths.contains(&&"/api".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_router_merge_no_deadlock() {
        use crate::controller::{ElifController, ControllerRoute};
        use tokio::time::{timeout, Duration};

        // Create test controllers
        struct TestControllerA;
        #[async_trait::async_trait]
        impl ElifController for TestControllerA {
            fn name(&self) -> &str { "TestControllerA" }
            fn base_path(&self) -> &str { "/a" }
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![ControllerRoute::new(HttpMethod::GET, "", "test")]
            }
            async fn handle_request(self: std::sync::Arc<Self>, _method_name: String, _request: ElifRequest) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok().text("A"))
            }
        }

        struct TestControllerB;
        #[async_trait::async_trait]
        impl ElifController for TestControllerB {
            fn name(&self) -> &str { "TestControllerB" }
            fn base_path(&self) -> &str { "/b" }
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![ControllerRoute::new(HttpMethod::GET, "", "test")]
            }
            async fn handle_request(self: std::sync::Arc<Self>, _method_name: String, _request: ElifRequest) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok().text("B"))
            }
        }

        // Test concurrent merging that could cause deadlock
        let task1 = tokio::spawn(async move {
            // Create fresh routers for concurrent merging test
            let a_copy = Router::<()>::new().controller(TestControllerA);
            let b_copy = Router::<()>::new().controller(TestControllerB);
            let _merged = a_copy.merge(b_copy);
        });
        
        let task2 = tokio::spawn(async move {
            // This creates the reverse merge scenario
            let a_copy = Router::<()>::new().controller(TestControllerA);
            let b_copy = Router::<()>::new().controller(TestControllerB);
            let _merged = b_copy.merge(a_copy);
        });

        // If there's a deadlock, this will timeout
        let result = timeout(Duration::from_millis(100), async {
            let _ = tokio::try_join!(task1, task2);
        }).await;

        // Should complete without timing out (no deadlock)
        assert!(result.is_ok(), "Router merge operations should not deadlock");
    }

    #[test]
    fn test_controller_merge_preserves_all_controllers() {
        use crate::controller::{ElifController, ControllerRoute};

        struct ControllerA;
        #[async_trait::async_trait]
        impl ElifController for ControllerA {
            fn name(&self) -> &str { "ControllerA" }
            fn base_path(&self) -> &str { "/a" }
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![ControllerRoute::new(HttpMethod::GET, "", "test")]
            }
            async fn handle_request(self: std::sync::Arc<Self>, _method_name: String, _request: ElifRequest) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok().text("A"))
            }
        }

        struct ControllerB;
        #[async_trait::async_trait]
        impl ElifController for ControllerB {
            fn name(&self) -> &str { "ControllerB" }
            fn base_path(&self) -> &str { "/b" }
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![ControllerRoute::new(HttpMethod::GET, "", "test")]
            }
            async fn handle_request(self: std::sync::Arc<Self>, _method_name: String, _request: ElifRequest) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok().text("B"))
            }
        }

        let router_a = Router::<()>::new().controller(ControllerA);
        let router_b = Router::<()>::new().controller(ControllerB);
        
        let merged_router = router_a.merge(router_b);
        
        // Verify both controllers are present in the merged registry
        let controller_registry = merged_router.controller_registry();
        let registry = controller_registry.lock().unwrap();
        
        assert!(registry.get_controller("ControllerA").is_some());
        assert!(registry.get_controller("ControllerB").is_some());
        
        // Verify controller names
        let names = registry.controller_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&"ControllerA".to_string()));
        assert!(names.contains(&&"ControllerB".to_string()));
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

    #[test]
    fn test_route_builder_with_middleware_groups() {
        use crate::middleware::v2::LoggingMiddleware;
        use std::sync::Arc;
        
        // Create middleware groups
        let router = Router::<()>::new()
            .middleware_group("api", vec![Arc::new(LoggingMiddleware)])
            .middleware_group("auth", vec![Arc::new(LoggingMiddleware)])
            .route("/api/users")
                .use_group("api")
                .use_group("auth")
                .get(elif_handler);
        
        // Verify middleware groups exist
        assert!(router.middleware_groups().contains_key("api"));
        assert!(router.middleware_groups().contains_key("auth"));
        
        // Verify route middleware mappings exist
        assert_eq!(router.route_middleware().len(), 1);
        
        // Check that the route was assigned the correct middleware groups
        let route_middleware = router.route_middleware().values().next().unwrap();
        assert_eq!(route_middleware.len(), 2);
        assert!(route_middleware.contains(&"api".to_string()));
        assert!(route_middleware.contains(&"auth".to_string()));
    }

    #[test]
    fn test_route_builder_chaining() {
        use crate::middleware::v2::LoggingMiddleware;
        use std::sync::Arc;
        
        let router = Router::<()>::new()
            .middleware_group("api", vec![Arc::new(LoggingMiddleware)])
            .route("/api/users")
                .use_group("api")
                .name("users.index")
                .get(elif_handler);
        
        // Verify route was registered with correct name and middleware
        let binding = router.registry();
        let registry = binding.lock().unwrap();
        let routes: Vec<_> = registry.all_routes().values().collect();
        assert_eq!(routes.len(), 1);
        
        let route = routes[0];
        assert_eq!(route.path, "/api/users");
        assert_eq!(route.name, Some("users.index".to_string()));
        assert_eq!(route.method, HttpMethod::GET);
    }

    #[test] 
    fn test_multiple_routes_with_different_middleware() {
        use crate::middleware::v2::{LoggingMiddleware, SimpleAuthMiddleware};
        use std::sync::Arc;
        
        let router = Router::<()>::new()
            .middleware_group("api", vec![Arc::new(LoggingMiddleware)])
            .middleware_group("auth", vec![Arc::new(SimpleAuthMiddleware::new("secret".to_string()))])
            .route("/api/public")
                .use_group("api")
                .get(elif_handler)
            .route("/api/protected")
                .use_group("api")
                .use_group("auth")
                .get(elif_handler)
            .route("/api/admin")
                .use_group("auth")
                .get(elif_handler);
        
        // Verify all routes were created
        assert_eq!(router.registry().lock().unwrap().all_routes().len(), 3);
        
        // Verify route middleware mappings
        let route_middleware = router.route_middleware();
        assert_eq!(route_middleware.len(), 3);
        
        // Check each route has correct middleware
        let middleware_counts: Vec<usize> = route_middleware.values().map(|v| v.len()).collect();
        middleware_counts.iter().for_each(|&count| assert!(count > 0));
        
        // Verify we have the right mix of middleware assignments
        let total_middleware_assignments: usize = middleware_counts.iter().sum();
        assert_eq!(total_middleware_assignments, 4); // 1 + 2 + 1 = 4 middleware assignments
    }

    #[test]
    fn test_ioc_controller_registration() {
        use crate::controller::{ElifController, ControllerRoute, factory::IocControllable};
        use elif_core::container::IocContainer;
        use elif_core::ServiceBinder;
        use std::sync::Arc;

        // Test service for dependency injection
        #[derive(Default)]
        struct TestService {
            #[allow(dead_code)]
            name: String,
        }

        unsafe impl Send for TestService {}
        unsafe impl Sync for TestService {}

        // Test controller with dependency injection
        struct IocTestController {
            #[allow(dead_code)]
            service: Arc<TestService>,
        }

        #[async_trait::async_trait]
        impl ElifController for IocTestController {
            fn name(&self) -> &str { "IocTestController" }
            fn base_path(&self) -> &str { "/ioc-test" }
            
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![
                    ControllerRoute::new(HttpMethod::GET, "", "list"),
                    ControllerRoute::new(HttpMethod::GET, "/{id}", "show"),
                ]
            }
            
            async fn handle_request(
                self: Arc<Self>,
                method_name: String,
                _request: ElifRequest,
            ) -> HttpResult<ElifResponse> {
                match method_name.as_str() {
                    "list" => Ok(ElifResponse::ok().text("IoC List")),
                    "show" => Ok(ElifResponse::ok().text("IoC Show")),
                    _ => Ok(ElifResponse::not_found().text("Method not found")),
                }
            }
        }

        impl IocControllable for IocTestController {
            fn from_ioc_container(
                container: &IocContainer,
                _scope: Option<&elif_core::container::ScopeId>,
            ) -> Result<Self, String> {
                let service = container.resolve::<TestService>()
                    .map_err(|e| format!("Failed to resolve TestService: {}", e))?;
                
                Ok(Self { service })
            }
        }

        // Set up IoC container
        let mut container = IocContainer::new();
        container.bind::<TestService, TestService>();
        container.build().expect("Container build failed");

        let container_arc = Arc::new(container);

        // Create router with IoC container and register controller
        let router = Router::<()>::new()
            .with_ioc_container(Arc::clone(&container_arc))
            .controller_from_container::<IocTestController>();

        // Verify routes were registered
        let registry = router.registry();
        let reg = registry.lock().unwrap();
        assert_eq!(reg.all_routes().len(), 2);

        // Verify controller was registered in controller registry
        let controller_registry = router.controller_registry();
        let ctrl_reg = controller_registry.lock().unwrap();
        assert!(ctrl_reg.get_controller("IocTestController").is_some());
    }

    #[test]
    #[should_panic(expected = "IoC container must be set before registering IoC controllers")]
    fn test_ioc_controller_without_container() {
        use crate::controller::{ElifController, ControllerRoute, factory::IocControllable};
        use elif_core::container::IocContainer;

        struct TestController;

        #[async_trait::async_trait]
        impl ElifController for TestController {
            fn name(&self) -> &str { "TestController" }
            fn base_path(&self) -> &str { "/test" }
            fn routes(&self) -> Vec<ControllerRoute> { vec![] }
            
            async fn handle_request(
                self: std::sync::Arc<Self>,
                _method_name: String,
                _request: ElifRequest,
            ) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok().text("test"))
            }
        }

        impl IocControllable for TestController {
            fn from_ioc_container(
                _container: &IocContainer,
                _scope: Option<&elif_core::container::ScopeId>,
            ) -> Result<Self, String> {
                Ok(Self)
            }
        }

        // This should panic because no IoC container is set
        Router::<()>::new().controller_from_container::<TestController>();
    }

    #[test]
    fn test_router_with_ioc_container() {
        use elif_core::container::IocContainer;
        use std::sync::Arc;

        let container = Arc::new(IocContainer::new());
        let router = Router::<()>::new()
            .with_ioc_container(Arc::clone(&container));

        assert!(router.ioc_container().is_some());
        assert!(Arc::ptr_eq(router.ioc_container().unwrap(), &container));
    }

    #[test]
    fn test_merge_preserves_ioc_container() {
        use elif_core::container::IocContainer;
        use std::sync::Arc;

        let container = Arc::new(IocContainer::new());
        
        let router1 = Router::<()>::new()
            .with_ioc_container(Arc::clone(&container));
        
        let router2 = Router::<()>::new();
        
        let merged = router1.merge(router2);
        
        // Should preserve router1's container
        assert!(merged.ioc_container().is_some());
        assert!(Arc::ptr_eq(merged.ioc_container().unwrap(), &container));
    }

    #[test]
    fn test_merge_prefers_first_container() {
        use elif_core::container::IocContainer;
        use std::sync::Arc;

        let container1 = Arc::new(IocContainer::new());
        let container2 = Arc::new(IocContainer::new());
        
        let router1 = Router::<()>::new()
            .with_ioc_container(Arc::clone(&container1));
        
        let router2 = Router::<()>::new()
            .with_ioc_container(Arc::clone(&container2));
        
        let merged = router1.merge(router2);
        
        // Should prefer router1's container
        assert!(merged.ioc_container().is_some());
        assert!(Arc::ptr_eq(merged.ioc_container().unwrap(), &container1));
    }

    #[test]
    fn test_scoped_controller_registration() {
        use crate::controller::{ElifController, ControllerRoute, factory::IocControllable};
        use elif_core::container::IocContainer;
        use elif_core::ServiceBinder;
        use std::sync::Arc;

        #[derive(Default)]
        struct ScopedService {
            #[allow(dead_code)]
            id: String,
        }

        unsafe impl Send for ScopedService {}
        unsafe impl Sync for ScopedService {}

        struct ScopedController {
            #[allow(dead_code)]
            service: Arc<ScopedService>,
        }

        #[async_trait::async_trait]
        impl ElifController for ScopedController {
            fn name(&self) -> &str { "ScopedController" }
            fn base_path(&self) -> &str { "/scoped" }
            
            fn routes(&self) -> Vec<ControllerRoute> {
                vec![ControllerRoute::new(HttpMethod::GET, "/test", "test")]
            }
            
            async fn handle_request(
                self: Arc<Self>,
                _method_name: String,
                _request: ElifRequest,
            ) -> HttpResult<ElifResponse> {
                Ok(ElifResponse::ok().text("Scoped"))
            }
        }

        impl IocControllable for ScopedController {
            fn from_ioc_container(
                container: &IocContainer,
                _scope: Option<&elif_core::container::ScopeId>,
            ) -> Result<Self, String> {
                let service = container.resolve::<ScopedService>()
                    .map_err(|e| format!("Failed to resolve ScopedService: {}", e))?;
                
                Ok(Self { service })
            }
        }

        // Set up IoC container
        let mut container = IocContainer::new();
        container.bind::<ScopedService, ScopedService>();
        container.build().expect("Container build failed");

        let container_arc = Arc::new(container);

        // Create router with scoped controller
        let router = Router::<()>::new()
            .with_ioc_container(Arc::clone(&container_arc))
            .scoped_controller_from_container::<ScopedController>();

        // Verify routes were registered
        let registry = router.registry();
        let reg = registry.lock().unwrap();
        assert_eq!(reg.all_routes().len(), 1);

        // Verify controller was registered
        let controller_registry = router.controller_registry();
        let ctrl_reg = controller_registry.lock().unwrap();
        assert!(ctrl_reg.get_controller("ScopedController").is_some());
    }

    #[test]
    fn test_route_without_middleware_groups() {
        let router = Router::<()>::new()
            .route("/simple")
                .get(elif_handler);
        
        // Verify route was created without middleware groups
        let binding = router.registry();
        let registry = binding.lock().unwrap();
        assert_eq!(registry.all_routes().len(), 1);
        
        let route_middleware = router.route_middleware();
        assert_eq!(route_middleware.len(), 1);
        
        // The route should have empty middleware groups list
        let middleware_groups = route_middleware.values().next().unwrap();
        assert_eq!(middleware_groups.len(), 0);
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

/// Create an IoC controller handler function that resolves controllers from the container
pub fn ioc_controller_handler<C>(
    container: Arc<IocContainer>, 
    method_name: String,
) -> impl Fn(ElifRequest) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> + Clone + Send + Sync + 'static
where
    C: ElifController + IocControllable + 'static,
{
    move |request: ElifRequest| {
        let container = Arc::clone(&container);
        let method_name = method_name.clone();
        
        Box::pin(async move {
            // Resolve controller from IoC container
            let controller = C::from_ioc_container(&container, None)
                .map_err(|e| crate::errors::HttpError::InternalError {
                    message: format!("Failed to resolve controller: {}", e),
                })?;
            
            let controller_arc = Arc::new(controller);
            controller_arc.handle_request(method_name, request).await
        })
    }
}

/// Create a scoped IoC controller handler that creates a new scope per request
pub fn scoped_ioc_controller_handler<C>(
    container: Arc<IocContainer>, 
    method_name: String,
) -> impl Fn(ElifRequest) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> + Clone + Send + Sync + 'static
where
    C: ElifController + IocControllable + 'static,
{
    move |request: ElifRequest| {
        let container = Arc::clone(&container);
        let method_name = method_name.clone();
        
        Box::pin(async move {
            // Create a new scope for this request
            let scope_id = container.create_scope()
                .map_err(|e| crate::errors::HttpError::InternalError {
                    message: format!("Failed to create request scope: {}", e),
                })?;
            
            // Resolve controller with scoped dependencies
            let controller = C::from_ioc_container(&container, Some(&scope_id))
                .map_err(|e| crate::errors::HttpError::InternalError {
                    message: format!("Failed to resolve scoped controller: {}", e),
                })?;
            
            let controller_arc = Arc::new(controller);
            let result = controller_arc.handle_request(method_name, request).await;
            
            // Clean up the scope (fire and forget - don't block response)
            let container_clone = Arc::clone(&container);
            tokio::spawn(async move {
                let _ = container_clone.dispose_scope(&scope_id).await;
            });
            
            result
        })
    }
}