//! HTTP server implementation
//! 
//! Provides the main HTTP server using Axum with integration to the elif-core
//! dependency injection container and configuration system.

use crate::{
    HttpConfig, HttpError, HttpResult,
    MiddlewarePipeline, TracingMiddleware, TimeoutMiddleware, BodyLimitMiddleware,
};
use elif_core::{Container, app_config::AppConfigTrait};
use axum::{
    Router,
    routing::{get, IntoMakeService},
    extract::State,
    response::Json,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use std::time::Duration;
use tracing::{info, warn, error};

/// HTTP server state shared across requests
#[derive(Clone)]
pub struct ServerState {
    pub container: Arc<Container>,
    pub http_config: HttpConfig,
}

/// HTTP server with DI container integration
pub struct HttpServer {
    app: IntoMakeService<Router>,
    addr: SocketAddr,
    config: HttpConfig,
    container: Arc<Container>,
    middleware: MiddlewarePipeline,
}

/// Builder for configuring HTTP server
pub struct HttpServerBuilder {
    container: Option<Arc<Container>>,
    http_config: Option<HttpConfig>,
    router: Option<Router>,
}

impl HttpServerBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            container: None,
            http_config: None,
            router: None,
        }
    }

    /// Set the DI container
    pub fn container(mut self, container: Arc<Container>) -> Self {
        self.container = Some(container);
        self
    }

    /// Set HTTP configuration (loads from environment if not provided)
    pub fn http_config(mut self, config: HttpConfig) -> Self {
        self.http_config = Some(config);
        self
    }

    /// Set custom router (will be merged with default routes)
    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Build the HTTP server
    pub fn build(self) -> HttpResult<HttpServer> {
        let container = self.container
            .ok_or_else(|| HttpError::config("Container is required"))?;

        let http_config = match self.http_config {
            Some(config) => config,
            None => HttpConfig::from_env()?,
        };

        http_config.validate()?;

        let server = HttpServer::new(container, http_config, self.router)?;
        Ok(server)
    }
}

impl Default for HttpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpServer {
    /// Create new HTTP server with DI container
    pub fn new(
        container: Arc<Container>, 
        http_config: HttpConfig,
        custom_router: Option<Router>,
    ) -> HttpResult<Self> {
        let app_config = container.config();
        let addr = format!("{}:{}", app_config.server.host, app_config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| HttpError::config(format!("Invalid server address: {}", e)))?;

        let state = ServerState {
            container: container.clone(),
            http_config: http_config.clone(),
        };

        // Create health check handler with captured state
        let health_container = container.clone();
        let health_config = http_config.clone();
        let health_handler = move || {
            let container = health_container.clone();
            let config = health_config.clone();
            async move {
                stateless_health_check(container, config).await
            }
        };

        // Create base router with health check (no state)
        let mut app = Router::new()
            .route(&http_config.health_check_path, get(health_handler));

        // Merge with custom router if provided
        if let Some(custom_router) = custom_router {
            app = app.merge(custom_router);
        }

        // Create framework middleware pipeline
        let mut middleware = MiddlewarePipeline::new()
            .add(BodyLimitMiddleware::with_limit(http_config.max_request_size))
            .add(TimeoutMiddleware::with_duration(http_config.request_timeout()));

        // Add tracing if enabled
        if http_config.enable_tracing {
            middleware = middleware.add(TracingMiddleware::new());
        }

        // Convert to make service for server
        let app = app.into_make_service();

        Ok(Self {
            app,
            addr,
            config: http_config,
            container,
            middleware,
        })
    }

    /// Start the HTTP server
    pub async fn run(self) -> HttpResult<()> {
        info!(
            "Starting HTTP server on {} with config: {:?}", 
            self.addr, self.config
        );
        info!("Framework middleware pipeline: {:?}", self.middleware.names());

        // Validate container before starting
        self.container.validate()
            .map_err(|e| HttpError::startup(format!("Container validation failed: {}", e)))?;

        let listener = tokio::net::TcpListener::bind(self.addr).await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("HTTP server listening on {} with {} middleware", self.addr, self.middleware.len());

        let server = axum::serve(listener, self.app)
            .with_graceful_shutdown(shutdown_signal());

        if let Err(e) = server.await {
            error!("Server error: {}", e);
            return Err(HttpError::startup(format!("Server failed: {}", e)));
        }

        info!("HTTP server stopped gracefully");
        Ok(())
    }

    /// Start the server with custom shutdown handling
    pub async fn run_with_shutdown<F>(self, shutdown: F) -> HttpResult<()> 
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        info!(
            "Starting HTTP server on {} with custom shutdown handler", 
            self.addr
        );
        info!("Framework middleware pipeline: {:?}", self.middleware.names());

        // Validate container before starting
        self.container.validate()
            .map_err(|e| HttpError::startup(format!("Container validation failed: {}", e)))?;

        let listener = tokio::net::TcpListener::bind(self.addr).await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("HTTP server listening on {} with {} middleware", self.addr, self.middleware.len());

        let server = axum::serve(listener, self.app)
            .with_graceful_shutdown(shutdown);

        if let Err(e) = server.await {
            error!("Server error: {}", e);
            return Err(HttpError::startup(format!("Server failed: {}", e)));
        }

        info!("HTTP server stopped gracefully");
        Ok(())
    }

    /// Get server configuration
    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    /// Get server address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get DI container
    pub fn container(&self) -> Arc<Container> {
        self.container.clone()
    }

    /// Get reference to middleware pipeline
    pub fn middleware(&self) -> &MiddlewarePipeline {
        &self.middleware
    }
}

/// Stateless health check function that takes container and config as parameters
async fn stateless_health_check(
    container: Arc<Container>,
    config: HttpConfig
) -> Result<Json<Value>, HttpError> {
    // Check database connection
    let database = container.database();
    let db_healthy = database.is_connected();
    
    if !db_healthy {
        warn!("Health check failed: database not connected");
        return Err(HttpError::health_check("Database connection unavailable"));
    }

    let app_config = container.config();
    let response = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0",
        "environment": format!("{:?}", app_config.environment),
        "server": "stateless",
        "services": {
            "database": if db_healthy { "healthy" } else { "unhealthy" },
            "container": "healthy"
        },
        "config": {
            "request_timeout": config.request_timeout_secs,
            "health_check_path": config.health_check_path,
            "tracing_enabled": config.enable_tracing
        }
    });

    Ok(Json(response))
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            info!("Received terminate signal, initiating graceful shutdown");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_core::container::test_implementations::*;
    use std::sync::Arc;
    use tokio_test;

    fn create_test_container() -> Arc<Container> {
        let config = Arc::new(create_test_config());
        let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
        
        Container::builder()
            .config(config)
            .database(database)
            .build()
            .unwrap()
            .into()
    }

    #[test]
    fn test_http_server_builder() {
        let container = create_test_container();
        let http_config = HttpConfig::default();

        let server = HttpServerBuilder::new()
            .container(container)
            .http_config(http_config)
            .build();

        assert!(server.is_ok());
        let server = server.unwrap();
        assert_eq!(server.addr().port(), 8080);
    }

    #[test]
    fn test_http_server_builder_missing_container() {
        let result = HttpServerBuilder::new().build();
        assert!(result.is_err());
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, HttpError::ConfigError { .. }));
        }
    }

    #[test]
    fn test_server_config_access() {
        let container = create_test_container();
        let http_config = HttpConfig::default();

        let server = HttpServerBuilder::new()
            .container(container.clone())
            .http_config(http_config.clone())
            .build()
            .unwrap();

        assert_eq!(server.config().request_timeout_secs, http_config.request_timeout_secs);
        assert_eq!(server.container().config().name, "test-app");
    }

    #[tokio::test]
    async fn test_health_check_handler() {
        let container = create_test_container();
        let http_config = HttpConfig::default();
        let _state = ServerState {
            container: container.clone(),
            http_config: http_config.clone(),
        };

        let result = stateless_health_check(container, http_config).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let status = response.0.get("status").and_then(|v| v.as_str()).unwrap();
        assert_eq!(status, "healthy");
    }

    #[test]
    fn test_invalid_server_address() {
        let container = create_test_container();
        
        // Modify config to have invalid address
        let mut app_config = create_test_config();
        app_config.server.host = "invalid-host".to_string();
        let config_arc = Arc::new(app_config);
        
        // Create container with invalid config
        let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
        let invalid_container = Container::builder()
            .config(config_arc)
            .database(database)
            .build()
            .unwrap();

        let result = HttpServer::new(
            Arc::new(invalid_container), 
            HttpConfig::default(),
            None
        );

        assert!(result.is_err());
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, HttpError::ConfigError { .. }));
        }
    }
}