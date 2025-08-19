//! Route groups for organizing related routes

use super::{HttpMethod, RouteRegistry, RouteInfo};
use service_builder::builder;
use axum::{
    Router as AxumRouter,
    routing::{get, post, put, delete, patch},
    handler::Handler,
};
use std::sync::{Arc, Mutex};

/// Route group for organizing related routes with shared configuration
#[derive(Debug)]
pub struct RouteGroup<S = ()>
where
    S: Clone + Send + Sync + 'static,
{
    prefix: String,
    name: String,
    router: AxumRouter<S>,
    registry: Arc<Mutex<RouteRegistry>>,
    #[allow(dead_code)]
    middleware: Vec<String>, // Placeholder for future middleware
}

impl<S> RouteGroup<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a new route group
    pub fn new(name: &str, prefix: &str, registry: Arc<Mutex<RouteRegistry>>) -> Self {
        Self {
            prefix: prefix.trim_end_matches('/').to_string(),
            name: name.to_string(),
            router: AxumRouter::new(),
            registry,
            middleware: Vec::new(),
        }
    }

    /// Create full path with group prefix
    fn full_path(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        if self.prefix.is_empty() {
            format!("/{}", path)
        } else {
            format!("{}/{}", self.prefix, path)
        }
    }

    /// Register a route in the group
    fn register_route(&self, method: HttpMethod, path: &str, route_name: Option<String>) {
        let full_path = self.full_path(path);
        let params = self.extract_param_names(&full_path);
        
        let final_name = route_name.or_else(|| {
            // Generate default name: group.method.path_segments
            let path_segments: Vec<&str> = path.trim_matches('/')
                .split('/')
                .filter(|s| !s.is_empty() && !s.starts_with('{'))
                .collect();
            
            if path_segments.is_empty() {
                Some(format!("{}.{}", self.name, method_to_string(&method).to_lowercase()))
            } else {
                Some(format!("{}.{}.{}", 
                    self.name, 
                    method_to_string(&method).to_lowercase(),
                    path_segments.join("_")
                ))
            }
        });
        
        let route_info = RouteInfo {
            name: final_name,
            path: full_path,
            method,
            params,
            group: Some(self.name.clone()),
        };
        
        let route_id = format!("{}_{}", self.name, uuid::Uuid::new_v4());
        self.registry.lock().unwrap().register(route_id, route_info);
    }

    /// Extract parameter names from path
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

    /// Add a GET route to the group
    pub fn get<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.register_route(HttpMethod::GET, path, None);
        self.router = self.router.route(path, get(handler));
        self
    }

    /// Add a POST route to the group
    pub fn post<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.register_route(HttpMethod::POST, path, None);
        self.router = self.router.route(path, post(handler));
        self
    }

    /// Add a PUT route to the group
    pub fn put<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.register_route(HttpMethod::PUT, path, None);
        self.router = self.router.route(path, put(handler));
        self
    }

    /// Add a DELETE route to the group
    pub fn delete<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.register_route(HttpMethod::DELETE, path, None);
        self.router = self.router.route(path, delete(handler));
        self
    }

    /// Add a PATCH route to the group
    pub fn patch<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.register_route(HttpMethod::PATCH, path, None);
        self.router = self.router.route(path, patch(handler));
        self
    }

    /// Add a named route
    pub fn route<H, T>(mut self, method: HttpMethod, path: &str, name: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.register_route(method.clone(), path, Some(name.to_string()));
        
        let axum_method = axum::http::Method::from(method);
        match axum_method {
            axum::http::Method::GET => self.router = self.router.route(path, get(handler)),
            axum::http::Method::POST => self.router = self.router.route(path, post(handler)),
            axum::http::Method::PUT => self.router = self.router.route(path, put(handler)),
            axum::http::Method::DELETE => self.router = self.router.route(path, delete(handler)),
            axum::http::Method::PATCH => self.router = self.router.route(path, patch(handler)),
            _ => {} // TODO: Handle other methods
        }
        
        self
    }

    /// Get the prefix for this group
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Get the name of this group
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Convert to Axum router for mounting
    pub fn into_router(self) -> AxumRouter<S> {
        self.router
    }
}

/// Configuration for GroupBuilder
#[derive(Debug, Clone)]
#[builder]
pub struct GroupBuilderConfig {
    #[builder(getter)]
    pub name: String,
    
    #[builder(default, getter)]
    pub prefix: String,
    
    #[builder(default, getter)]
    pub middleware: Vec<String>,
}

impl GroupBuilderConfig {
    /// Build a RouteGroup from the config
    pub fn build_group<S>(self, registry: Arc<Mutex<RouteRegistry>>) -> RouteGroup<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        RouteGroup::new(&self.name, &self.prefix, registry)
    }
}

// Add convenience methods to the generated builder
impl GroupBuilderConfigBuilder {
    /// Add middleware to the group
    pub fn add_middleware(self, middleware_name: &str) -> Self {
        let mut middlewares_vec = self.middleware.unwrap_or_default();
        middlewares_vec.push(middleware_name.to_string());
        GroupBuilderConfigBuilder {
            name: self.name,
            prefix: self.prefix,
            middleware: Some(middlewares_vec),
        }
    }
    
    /// Add multiple middlewares
    pub fn add_middlewares(self, new_middlewares: Vec<String>) -> Self {
        let mut middlewares_vec = self.middleware.unwrap_or_default();
        middlewares_vec.extend(new_middlewares);
        GroupBuilderConfigBuilder {
            name: self.name,
            prefix: self.prefix,
            middleware: Some(middlewares_vec),
        }
    }
    
    pub fn build_config(self) -> GroupBuilderConfig {
        self.build_with_defaults().unwrap()
    }
}

/// Builder for creating route groups with configuration
pub struct GroupBuilder {
    builder_config: GroupBuilderConfigBuilder,
}

impl GroupBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            builder_config: GroupBuilderConfig::builder().name(name.to_string()),
        }
    }

    /// Set the URL prefix for the group
    pub fn prefix(self, prefix: &str) -> Self {
        Self {
            builder_config: self.builder_config.prefix(prefix.to_string()),
        }
    }

    /// Add middleware to the group (placeholder)
    pub fn middleware(self, middleware_name: &str) -> Self {
        Self {
            builder_config: self.builder_config.add_middleware(middleware_name),
        }
    }

    /// Add multiple middlewares
    pub fn middlewares(self, middlewares: Vec<String>) -> Self {
        Self {
            builder_config: self.builder_config.add_middlewares(middlewares),
        }
    }

    /// Build the route group
    pub fn build<S>(self, registry: Arc<Mutex<RouteRegistry>>) -> RouteGroup<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        self.builder_config.build_config().build_group(registry)
    }
}

/// Convert HttpMethod to string for naming
fn method_to_string(method: &HttpMethod) -> &'static str {
    match method {
        HttpMethod::GET => "GET",
        HttpMethod::POST => "POST",
        HttpMethod::PUT => "PUT",
        HttpMethod::DELETE => "DELETE",
        HttpMethod::PATCH => "PATCH",
        HttpMethod::HEAD => "HEAD",
        HttpMethod::OPTIONS => "OPTIONS",
        HttpMethod::TRACE => "TRACE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::Html;

    async fn handler() -> Html<&'static str> {
        Html("<h1>Handler</h1>")
    }

    #[test]
    fn test_group_creation() {
        let registry = Arc::new(Mutex::new(RouteRegistry::new()));
        let group = RouteGroup::<()>::new("api", "/api/v1", Arc::clone(&registry));
        
        assert_eq!(group.name(), "api");
        assert_eq!(group.prefix(), "/api/v1");
    }

    #[test]
    fn test_group_path_generation() {
        let registry = Arc::new(Mutex::new(RouteRegistry::new()));
        let group = RouteGroup::<()>::new("api", "/api/v1", registry);
        
        assert_eq!(group.full_path("users"), "/api/v1/users");
        assert_eq!(group.full_path("/users/{id}"), "/api/v1/users/{id}");
    }

    #[test]
    fn test_group_builder() {
        let registry = Arc::new(Mutex::new(RouteRegistry::new()));
        let group = GroupBuilder::new("api")
            .prefix("/api/v1")
            .middleware("auth")
            .build::<()>(registry)
            .get("/users", handler)
            .post("/users", handler);
        
        assert_eq!(group.name(), "api");
        assert_eq!(group.prefix(), "/api/v1");
    }
}