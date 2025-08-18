use super::{Router, HttpMethod, RouteInfo, RouteRegistry};
use crate::{
    handlers::elif_handler,
    request::ElifRequest,
    response::IntoElifResponse,
    errors::HttpResult,
    middleware::versioning::{VersioningConfig, VersioningMiddleware, ApiVersion},
};
use axum::{
    Router as AxumRouter,
    routing::{get, post, put, delete, patch},
    middleware::from_fn,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::future::Future;
use service_builder::builder;

/// Versioned router that handles multiple API versions
#[derive(Debug)]
#[builder]
pub struct VersionedRouter<S = ()> 
where 
    S: Clone + Send + Sync + 'static,
{
    /// Version-specific routers
    #[builder(default)]
    pub version_routers: HashMap<String, Router<S>>,
    /// Versioning configuration
    #[builder(default)]
    pub versioning_config: VersioningConfig,
    /// Global router for non-versioned routes
    #[builder(default)]
    pub global_router: Option<Router<S>>,
    /// Base API path (e.g., "/api")
    #[builder(default = "\"/api\".to_string()")]
    pub base_path: String,
}

impl<S> VersionedRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a new versioned router
    pub fn new() -> Self {
        Self {
            version_routers: HashMap::new(),
            versioning_config: VersioningConfig::build().build_with_defaults(),
            global_router: None,
            base_path: "/api".to_string(),
        }
    }

    /// Add a version with its router
    pub fn version(mut self, version: &str, router: Router<S>) -> Self {
        self.version_routers.insert(version.to_string(), router);
        
        // Add version to config if not exists
        if !self.versioning_config.versions.contains_key(version) {
            self.versioning_config.add_version(version.to_string(), ApiVersion {
                version: version.to_string(),
                deprecated: false,
                deprecation_message: None,
                sunset_date: None,
                is_default: self.version_routers.len() == 1, // First version is default
            });
        }
        
        self
    }

    /// Mark a version as deprecated
    pub fn deprecate_version(mut self, version: &str, message: Option<&str>, sunset_date: Option<&str>) -> Self {
        self.versioning_config.deprecate_version(
            version,
            message.map(|s| s.to_string()),
            sunset_date.map(|s| s.to_string())
        );
        self
    }

    /// Set default version
    pub fn default_version(mut self, version: &str) -> Self {
        // Mark all versions as non-default
        for api_version in self.versioning_config.versions.values_mut() {
            api_version.is_default = false;
        }
        
        // Mark specified version as default
        if let Some(api_version) = self.versioning_config.versions.get_mut(version) {
            api_version.is_default = true;
        }
        
        self.versioning_config.default_version = Some(version.to_string());
        self
    }

    /// Set versioning strategy
    pub fn strategy(mut self, strategy: crate::middleware::versioning::VersionStrategy) -> Self {
        self.versioning_config.strategy = strategy;
        self
    }

    /// Add global routes (not versioned)
    pub fn global(mut self, router: Router<S>) -> Self {
        self.global_router = Some(router);
        self
    }

    /// Build the final router with versioning middleware
    pub fn build(self) -> Router<S> {
        let mut final_router = Router::new();
        
        // Add global routes first (if any)
        if let Some(global_router) = self.global_router {
            final_router = final_router.merge(global_router);
        }

        // Create versioned routes
        for (version, version_router) in self.version_routers {
            let version_path = match self.versioning_config.strategy {
                crate::middleware::versioning::VersionStrategy::UrlPath => {
                    format!("{}/{}", self.base_path, version)
                },
                _ => {
                    // For non-URL strategies, all versions use the same base path
                    self.base_path.clone()
                }
            };
            
            // Nest the version router under the version path
            final_router = final_router.nest(&version_path, version_router);
        }

        // Apply versioning middleware layer - this is critical!
        // This ensures that version detection and response headers work for ALL strategies
        let versioning_layer = crate::middleware::versioning::versioning_layer(self.versioning_config);
        
        // Convert to axum router and apply the layer
        let axum_router = final_router.into_axum_router();
        let layered_router = axum_router.layer(versioning_layer);
        
        // Convert back to elif Router
        // Note: This creates a new Router with the layered axum router
        Router::new().merge_axum(layered_router)
    }

    /// Create a router builder for a specific version
    pub fn version_builder(&mut self, version: &str) -> VersionedRouteBuilder<S> {
        VersionedRouteBuilder::new(version, self)
    }
}

/// Builder for adding routes to a specific version
pub struct VersionedRouteBuilder<'a, S> 
where 
    S: Clone + Send + Sync + 'static,
{
    version: String,
    router: &'a mut VersionedRouter<S>,
    current_router: Router<S>,
}

impl<'a, S> VersionedRouteBuilder<'a, S>
where
    S: Clone + Send + Sync + 'static,
{
    fn new(version: &str, router: &'a mut VersionedRouter<S>) -> Self {
        Self {
            version: version.to_string(),
            router,
            current_router: Router::new(),
        }
    }

    /// Add a GET route for this version
    pub fn get<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.current_router = self.current_router.get(path, handler);
        self
    }

    /// Add a POST route for this version
    pub fn post<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.current_router = self.current_router.post(path, handler);
        self
    }

    /// Add a PUT route for this version
    pub fn put<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.current_router = self.current_router.put(path, handler);
        self
    }

    /// Add a DELETE route for this version
    pub fn delete<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.current_router = self.current_router.delete(path, handler);
        self
    }

    /// Add a PATCH route for this version
    pub fn patch<F, Fut, R>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = HttpResult<R>> + Send + 'static,
        R: IntoElifResponse + Send + 'static,
    {
        self.current_router = self.current_router.patch(path, handler);
        self
    }

    /// Finish building routes for this version
    pub fn finish(self) {
        self.router.version_routers.insert(self.version.clone(), self.current_router);
    }
}

/// Convenience functions for creating versioned routers
pub fn versioned_router<S>() -> VersionedRouter<S> 
where 
    S: Clone + Send + Sync + 'static,
{
    VersionedRouter::new()
}

/// Create a versioned router with URL path strategy
pub fn path_versioned_router<S>() -> VersionedRouter<S> 
where 
    S: Clone + Send + Sync + 'static,
{
    VersionedRouter::builder()
        .strategy(crate::middleware::versioning::VersionStrategy::UrlPath)
        .build().unwrap()
}

/// Create a versioned router with header strategy
pub fn header_versioned_router<S>(header_name: &str) -> VersionedRouter<S> 
where 
    S: Clone + Send + Sync + 'static,
{
    VersionedRouter::builder()
        .strategy(crate::middleware::versioning::VersionStrategy::Header(header_name.to_string()))
        .build().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifJson;
    
    #[tokio::test]
    async fn test_versioned_router_creation() {
        let router = VersionedRouter::<()>::new()
            .version("v1", Router::new())
            .version("v2", Router::new())
            .default_version("v1")
            .deprecate_version("v1", Some("Please use v2"), Some("2024-12-31"));

        assert_eq!(router.version_routers.len(), 2);
        assert!(router.version_routers.contains_key("v1"));
        assert!(router.version_routers.contains_key("v2"));
        
        let v1_version = router.versioning_config.versions.get("v1").unwrap();
        assert!(v1_version.deprecated);
        assert_eq!(v1_version.deprecation_message, Some("Please use v2".to_string()));
    }

    #[tokio::test] 
    async fn test_version_builder() {
        let mut router = VersionedRouter::<()>::new();
        
        router.version_builder("v1")
            .get("/users", |_req| async { Ok(ElifJson("users v1")) })
            .post("/users", |_req| async { Ok(ElifJson("create user v1")) })
            .finish();
        
        assert!(router.version_routers.contains_key("v1"));
    }

    #[test]
    fn test_convenience_functions() {
        let _path_router = path_versioned_router::<()>();
        let _header_router = header_versioned_router::<()>("Api-Version");
        let _versioned_router = versioned_router::<()>();
    }
}