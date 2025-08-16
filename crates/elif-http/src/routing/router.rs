//! Core routing functionality

use super::{HttpMethod, RouteInfo, RouteRegistry, params::{ParamExtractor, ParamType}};
use crate::handler::{ElifHandler, elif_handler};
use crate::request::ElifRequest;
use crate::response::{ElifResponse, IntoElifResponse};
use crate::error::HttpResult;
use service_builder::builder;
use axum::{
    Router as AxumRouter,
    routing::{get, post, put, delete, patch},
    handler::Handler,
    response::IntoResponse,
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
        }
    }

    /// Create a new router with state
    pub fn with_state(state: S) -> Self {
        Self {
            axum_router: AxumRouter::new().with_state(state),
            registry: Arc::new(Mutex::new(RouteRegistry::new())),
            route_counter: Arc::new(Mutex::new(0)),
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
        // Merge the registries
        if let (Ok(mut self_registry), Ok(other_registry)) = 
            (self.registry.lock(), other.registry.lock()) {
            for (id, route_info) in other_registry.all_routes() {
                self_registry.register(id.clone(), route_info.clone());
            }
        }
        
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
    pub fn nest(mut self, path: &str, router: Router<S>) -> Self {
        // Note: Nested routes inherit their path prefix, so we don't need to modify the registry paths
        // The registry will contain the original paths, and Axum handles the prefixing internally
        if let (Ok(mut self_registry), Ok(router_registry)) = 
            (self.registry.lock(), router.registry.lock()) {
            for (id, route_info) in router_registry.all_routes() {
                self_registry.register(id.clone(), route_info.clone());
            }
        }
        
        self.axum_router = self.axum_router.nest(path, router.axum_router);
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
        self.build_with_defaults().unwrap()
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
    use crate::error::HttpResult;

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
        let mut router = Router::<()>::new().get("/users/{id}/posts/{slug}", elif_handler);
        
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
}