//! IoC-enabled middleware system
//! 
//! Provides middleware creation and dependency injection using the IoC container.

use std::sync::Arc;
use std::collections::HashMap;

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::ElifRequest;
use crate::errors::HttpError;
use elif_core::container::{IocContainer, ScopeId};

/// Trait for middleware that can be created from IoC container
pub trait IocMiddleware: Middleware {
    /// Create middleware instance with dependencies resolved from IoC container
    fn from_ioc_container(
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<Self, String>
    where
        Self: Sized;
}

/// Middleware factory for IoC-enabled middleware
pub struct IocMiddlewareFactory<M> {
    _phantom: std::marker::PhantomData<M>,
}

impl<M> IocMiddlewareFactory<M> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<M> Default for IocMiddlewareFactory<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> IocMiddlewareFactory<M>
where
    M: IocMiddleware + 'static,
{
    /// Create middleware instance from IoC container
    pub fn create(
        &self,
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<M, HttpError> {
        M::from_ioc_container(container, scope)
            .map_err(|e| HttpError::InternalError {
                message: format!("Failed to create middleware: {}", e),
            })
    }
}

/// Registry for managing IoC-enabled middleware
pub struct MiddlewareRegistry {
    factories: HashMap<String, Box<dyn MiddlewareFactory>>,
    container: Arc<IocContainer>,
}

/// Trait for creating middleware instances
pub trait MiddlewareFactory: Send + Sync {
    /// Create middleware instance from IoC container
    fn create_middleware(
        &self,
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<Arc<dyn Middleware>, HttpError>;
}

impl<M> MiddlewareFactory for IocMiddlewareFactory<M>
where
    M: IocMiddleware + 'static,
{
    fn create_middleware(
        &self,
        container: &IocContainer,
        scope: Option<&ScopeId>,
    ) -> Result<Arc<dyn Middleware>, HttpError> {
        let middleware = self.create(container, scope)?;
        Ok(Arc::new(middleware))
    }
}

impl MiddlewareRegistry {
    /// Create new middleware registry with IoC container
    pub fn new(container: Arc<IocContainer>) -> Self {
        Self {
            factories: HashMap::new(),
            container,
        }
    }

    /// Register an IoC-enabled middleware type
    pub fn register<M>(&mut self, name: &str) -> Result<(), HttpError>
    where
        M: IocMiddleware + 'static,
    {
        let factory = Box::new(IocMiddlewareFactory::<M>::new());
        self.factories.insert(name.to_string(), factory);
        Ok(())
    }

    /// Register a custom middleware factory
    pub fn register_factory(
        &mut self,
        name: &str,
        factory: Box<dyn MiddlewareFactory>,
    ) {
        self.factories.insert(name.to_string(), factory);
    }

    /// Create middleware instance by name
    pub fn create_middleware(
        &self,
        name: &str,
        scope: Option<&ScopeId>,
    ) -> Result<Arc<dyn Middleware>, HttpError> {
        let factory = self.factories.get(name)
            .ok_or_else(|| HttpError::InternalError {
                message: format!("Middleware '{}' not registered", name),
            })?;

        factory.create_middleware(&self.container, scope)
    }

    /// Create multiple middleware instances by names
    pub fn create_middleware_pipeline(
        &self,
        names: &[&str],
        scope: Option<&ScopeId>,
    ) -> Result<Vec<Arc<dyn Middleware>>, HttpError> {
        names
            .iter()
            .map(|name| self.create_middleware(name, scope))
            .collect()
    }

    /// Get list of registered middleware names
    pub fn registered_middleware(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}

/// Builder for middleware registry
pub struct MiddlewareRegistryBuilder {
    container: Option<Arc<IocContainer>>,
    middleware: Vec<(String, Box<dyn MiddlewareFactory>)>,
}

impl MiddlewareRegistryBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            container: None,
            middleware: Vec::new(),
        }
    }

    /// Set the IoC container
    pub fn container(mut self, container: Arc<IocContainer>) -> Self {
        self.container = Some(container);
        self
    }

    /// Register an IoC-enabled middleware
    pub fn register<M>(mut self, name: &str) -> Self
    where
        M: IocMiddleware + 'static,
    {
        let factory = Box::new(IocMiddlewareFactory::<M>::new());
        self.middleware.push((name.to_string(), factory));
        self
    }

    /// Register a custom middleware factory
    pub fn register_factory(
        mut self,
        name: &str,
        factory: Box<dyn MiddlewareFactory>,
    ) -> Self {
        self.middleware.push((name.to_string(), factory));
        self
    }

    /// Build the middleware registry
    pub fn build(self) -> Result<MiddlewareRegistry, HttpError> {
        let container = self.container
            .ok_or_else(|| HttpError::InternalError {
                message: "IoC container is required for middleware registry".to_string(),
            })?;

        let mut registry = MiddlewareRegistry::new(container);

        for (name, factory) in self.middleware {
            registry.register_factory(&name, factory);
        }

        Ok(registry)
    }
}

impl Default for MiddlewareRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper middleware that lazily creates IoC-enabled middleware per request
pub struct LazyIocMiddleware {
    middleware_name: String,
    registry: Arc<MiddlewareRegistry>,
}

impl std::fmt::Debug for LazyIocMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyIocMiddleware")
            .field("middleware_name", &self.middleware_name)
            .finish()
    }
}

impl LazyIocMiddleware {
    /// Create new lazy IoC middleware
    pub fn new(middleware_name: String, registry: Arc<MiddlewareRegistry>) -> Self {
        Self {
            middleware_name,
            registry,
        }
    }
}

impl Middleware for LazyIocMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let middleware_name = self.middleware_name.clone();
        let registry = self.registry.clone();

        Box::pin(async move {
            // Create a new scope for this request if needed
            let scope_result = registry.container.create_scope();
            let scope = scope_result.ok();

            // Create the actual middleware instance
            match registry.create_middleware(&middleware_name, scope.as_ref()) {
                Ok(middleware) => {
                    // Delegate to the actual middleware
                    let result = middleware.handle(request, next).await;
                    
                    // Clean up scope
                    if let Some(scope_id) = scope {
                        let _ = registry.container.dispose_scope(&scope_id).await;
                    }
                    
                    result
                }
                Err(_) => {
                    // If middleware creation fails, continue to next middleware
                    next.run(request).await
                }
            }
        })
    }

    fn name(&self) -> &'static str {
        "LazyIocMiddleware"
    }
}

/// Request-scoped middleware context for dependency injection
#[derive(Clone, Debug)]
pub struct MiddlewareContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub correlation_id: Option<String>,
    pub custom_data: HashMap<String, String>,
}

impl MiddlewareContext {
    /// Create middleware context from request
    pub fn from_request(request: &ElifRequest) -> Self {
        Self {
            request_id: request
                .header("x-request-id")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
            user_id: request
                .header("x-user-id")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            session_id: request
                .header("x-session-id")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            correlation_id: request
                .header("x-correlation-id")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            custom_data: HashMap::new(),
        }
    }

    /// Add custom data to the context
    pub fn with_data(mut self, key: String, value: String) -> Self {
        self.custom_data.insert(key, value);
        self
    }
}

/// Middleware group for organizing related middleware
pub struct MiddlewareGroup {
    name: String,
    middleware_names: Vec<String>,
    registry: Arc<MiddlewareRegistry>,
}

impl MiddlewareGroup {
    /// Create new middleware group
    pub fn new(name: String, middleware_names: Vec<String>, registry: Arc<MiddlewareRegistry>) -> Self {
        Self {
            name,
            middleware_names,
            registry,
        }
    }

    /// Create all middleware in the group
    pub fn create_middleware(
        &self,
        scope: Option<&ScopeId>,
    ) -> Result<Vec<Arc<dyn Middleware>>, HttpError> {
        self.middleware_names
            .iter()
            .map(|name| self.registry.create_middleware(name, scope))
            .collect()
    }

    /// Get group name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get middleware names in the group
    pub fn middleware_names(&self) -> &[String] {
        &self.middleware_names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::v2::Middleware;
    use elif_core::container::{IocContainer, ServiceBinder};

    // Test service for middleware injection
    #[derive(Default, Clone)]
    pub struct TestLoggerService {
        pub name: String,
    }

    unsafe impl Send for TestLoggerService {}
    unsafe impl Sync for TestLoggerService {}

    // Test middleware with IoC dependencies
    #[derive(Debug)]
    pub struct TestIocMiddleware {
        logger: Arc<TestLoggerService>,
    }

    impl IocMiddleware for TestIocMiddleware {
        fn from_ioc_container(
            container: &IocContainer,
            _scope: Option<&ScopeId>,
        ) -> Result<Self, String> {
            let logger = container.resolve::<TestLoggerService>()
                .map_err(|e| format!("Failed to resolve TestLoggerService: {}", e))?;

            Ok(Self { logger })
        }
    }

    impl Middleware for TestIocMiddleware {
        fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
            Box::pin(async move {
                // Use the injected logger service
                println!("TestIocMiddleware: Using logger: {}", self.logger.name);
                next.run(request).await
            })
        }

        fn name(&self) -> &'static str {
            "TestIocMiddleware"
        }
    }

    #[tokio::test]
    async fn test_ioc_middleware_creation() {
        let mut container = IocContainer::new();
        
        // Register the logger service
        let logger_service = TestLoggerService {
            name: "TestLogger".to_string(),
        };
        container.bind_instance::<TestLoggerService, TestLoggerService>(logger_service);
        container.build().expect("Container build failed");

        let container_arc = Arc::new(container);
        let mut registry = MiddlewareRegistry::new(container_arc);

        // Register the IoC middleware
        registry.register::<TestIocMiddleware>("test_middleware")
            .expect("Failed to register middleware");

        // Create middleware instance
        let middleware = registry.create_middleware("test_middleware", None)
            .expect("Failed to create middleware");

        assert_eq!(middleware.name(), "TestIocMiddleware");
    }

    #[tokio::test]
    async fn test_middleware_registry_builder() {
        let mut container = IocContainer::new();
        container.bind::<TestLoggerService, TestLoggerService>();
        container.build().expect("Container build failed");

        let registry = MiddlewareRegistryBuilder::new()
            .container(Arc::new(container))
            .register::<TestIocMiddleware>("test_ioc")
            .build()
            .expect("Failed to build middleware registry");

        let registered = registry.registered_middleware();
        assert!(registered.contains(&"test_ioc".to_string()));

        let middleware = registry.create_middleware("test_ioc", None)
            .expect("Failed to create middleware");

        assert_eq!(middleware.name(), "TestIocMiddleware");
    }

    #[tokio::test]
    async fn test_middleware_pipeline_creation() {
        let mut container = IocContainer::new();
        container.bind::<TestLoggerService, TestLoggerService>();
        container.build().expect("Container build failed");

        let registry = MiddlewareRegistryBuilder::new()
            .container(Arc::new(container))
            .register::<TestIocMiddleware>("ioc1")
            .register::<TestIocMiddleware>("ioc2")
            .build()
            .expect("Failed to build middleware registry");

        let middleware_pipeline = registry
            .create_middleware_pipeline(&["ioc1", "ioc2"], None)
            .expect("Failed to create middleware pipeline");

        assert_eq!(middleware_pipeline.len(), 2);
    }

    #[tokio::test]
    async fn test_lazy_ioc_middleware() {
        use crate::request::method::HttpMethod;
        use crate::response::headers::ElifHeaderMap;

        let mut container = IocContainer::new();
        container.bind::<TestLoggerService, TestLoggerService>();
        container.build().expect("Container build failed");

        let registry = Arc::new(
            MiddlewareRegistryBuilder::new()
                .container(Arc::new(container))
                .register::<TestIocMiddleware>("lazy_test")
                .build()
                .expect("Failed to build middleware registry")
        );

        let lazy_middleware = LazyIocMiddleware::new("lazy_test".to_string(), registry);

        let request = ElifRequest::new(
            HttpMethod::GET,
            "/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );

        let next = Next::new(|_req| {
            Box::pin(async {
                ElifResponse::ok().text("Success")
            })
        });

        let response = lazy_middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::status::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_context_from_request() {
        use crate::request::method::HttpMethod;
        use crate::response::headers::{ElifHeaderMap, ElifHeaderName, ElifHeaderValue};

        let mut headers = ElifHeaderMap::new();
        headers.insert(
            ElifHeaderName::from_str("x-request-id").unwrap(),
            ElifHeaderValue::from_str("req-123").unwrap(),
        );
        headers.insert(
            ElifHeaderName::from_str("x-user-id").unwrap(),
            ElifHeaderValue::from_str("user-456").unwrap(),
        );

        let request = ElifRequest::new(
            HttpMethod::POST,
            "/api/test".parse().unwrap(),
            headers,
        );

        let context = MiddlewareContext::from_request(&request);

        assert_eq!(context.request_id, "req-123");
        assert_eq!(context.user_id, Some("user-456".to_string()));
        assert!(context.session_id.is_none());
    }

    #[tokio::test]
    async fn test_middleware_group() {
        let mut container = IocContainer::new();
        container.bind::<TestLoggerService, TestLoggerService>();
        container.build().expect("Container build failed");

        let registry = Arc::new(
            MiddlewareRegistryBuilder::new()
                .container(Arc::new(container))
                .register::<TestIocMiddleware>("group1")
                .register::<TestIocMiddleware>("group2")
                .build()
                .expect("Failed to build middleware registry")
        );

        let group = MiddlewareGroup::new(
            "test_group".to_string(),
            vec!["group1".to_string(), "group2".to_string()],
            registry,
        );

        assert_eq!(group.name(), "test_group");
        assert_eq!(group.middleware_names().len(), 2);

        let middleware = group.create_middleware(None)
            .expect("Failed to create group middleware");

        assert_eq!(middleware.len(), 2);
    }
}