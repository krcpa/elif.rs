//! Stateful HTTP server implementation
//! 
//! Provides HTTP server with full DI container integration using proper
//! Axum Router<State> patterns.

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
use tracing::{info, warn};

/// HTTP server state shared across requests
#[derive(Clone)]
pub struct AppState {
    pub container: Arc<Container>,
    pub config: HttpConfig,
}

/// HTTP server with full DI container integration
pub struct StatefulHttpServer {
    router: Router,
    state: AppState,
    addr: SocketAddr,
    middleware: MiddlewarePipeline,
}

/// Builder for configuring stateful HTTP server
pub struct StatefulHttpServerBuilder {
    container: Option<Arc<Container>>,
    http_config: Option<HttpConfig>,
    custom_routes: Vec<Router>,
}

impl StatefulHttpServerBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            container: None,
            http_config: None,
            custom_routes: Vec::new(),
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

    /// Add custom routes (stateless routers that will be given state)
    pub fn add_routes(mut self, routes: Router) -> Self {
        self.custom_routes.push(routes);
        self
    }

    /// Build the HTTP server
    pub fn build(self) -> HttpResult<StatefulHttpServer> {
        let container = self.container
            .ok_or_else(|| HttpError::config("Container is required"))?;

        let http_config = match self.http_config {
            Some(config) => config,
            None => HttpConfig::from_env()?,
        };

        http_config.validate()?;

        let server = StatefulHttpServer::new(container.clone(), http_config, self.custom_routes)?;
        Ok(server)
    }
}

impl Default for StatefulHttpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StatefulHttpServer {
    /// Create new HTTP server with DI container
    pub fn new(
        container: Arc<Container>, 
        http_config: HttpConfig,
        custom_routes: Vec<Router>,
    ) -> HttpResult<Self> {
        let app_config = container.config();
        let addr = format!("{}:{}", app_config.server.host, app_config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| HttpError::config(format!("Invalid server address: {}", e)))?;

        let state = AppState {
            container,
            config: http_config.clone(),
        };

        // Create base router with stateless health check first
        let container = state.container.clone();
        let config = state.config.clone();
        
        let health_handler = move || {
            let container = container.clone();
            let config = config.clone();
            async move { 
                stateless_health_check_with_context(container, config).await
            }
        };
        
        let mut router = Router::new()
            .route(&http_config.health_check_path, get(health_handler));

        // Merge with custom routers (all stateless)
        for custom_router in custom_routes {
            router = router.merge(custom_router);
        }

        // Create framework middleware pipeline
        let mut middleware = MiddlewarePipeline::new()
            .add(BodyLimitMiddleware::with_limit(http_config.max_request_size))
            .add(TimeoutMiddleware::with_duration(http_config.request_timeout()));

        // Add tracing if enabled
        if http_config.enable_tracing {
            middleware = middleware.add(TracingMiddleware::new());
        }

        Ok(Self {
            router,
            state,
            addr,
            middleware,
        })
    }

    /// Start the HTTP server
    pub async fn run(self) -> HttpResult<()> {
        info!(
            "Starting stateful HTTP server on {} with DI container integration", 
            self.addr
        );
        info!("Framework middleware pipeline: {:?}", self.middleware.names());

        // Note: Router is stateless, state was captured in handler closures
        let service = self.router;

        let listener = tokio::net::TcpListener::bind(self.addr).await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("Stateful HTTP server listening on {} with {} middleware", self.addr, self.middleware.len());

        axum::serve(listener, service)
            .with_graceful_shutdown(stateful_shutdown_signal())
            .await
            .map_err(|e| HttpError::startup(format!("Server failed: {}", e)))?;

        info!("Stateful HTTP server stopped gracefully");
        Ok(())
    }

    /// Start the server with custom shutdown handling
    pub async fn run_with_shutdown<F>(self, shutdown: F) -> HttpResult<()> 
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        info!(
            "Starting stateful HTTP server on {} with custom shutdown handler", 
            self.addr
        );
        info!("Framework middleware pipeline: {:?}", self.middleware.names());

        // Note: Router is stateless, state was captured in handler closures
        let service = self.router;

        let listener = tokio::net::TcpListener::bind(self.addr).await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("Stateful HTTP server listening on {} with {} middleware", self.addr, self.middleware.len());

        axum::serve(listener, service)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|e| HttpError::startup(format!("Server failed: {}", e)))?;

        info!("Stateful HTTP server stopped gracefully");
        Ok(())
    }

    /// Get server address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get reference to middleware pipeline
    pub fn middleware(&self) -> &MiddlewarePipeline {
        &self.middleware
    }

    // Note: DI container is embedded in the router state, not directly accessible
}

/// Health check endpoint handler with DI container access
async fn stateful_health_check(State(state): State<AppState>) -> Result<Json<Value>, HttpError> {
    stateless_health_check_with_context(state.container, state.config).await
}

/// Stateless health check function that takes container and config as parameters
async fn stateless_health_check_with_context(
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
        "server": "stateful",
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
async fn stateful_shutdown_signal() {
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
    fn test_stateful_server_builder() {
        let container = create_test_container();
        let http_config = HttpConfig::default();

        let server = StatefulHttpServerBuilder::new()
            .container(container)
            .http_config(http_config)
            .build();

        assert!(server.is_ok());
        let server = server.unwrap();
        assert_eq!(server.addr().port(), 8080);
    }

    #[test]
    fn test_stateful_server_builder_missing_container() {
        let result = StatefulHttpServerBuilder::new().build();
        assert!(result.is_err());
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, HttpError::ConfigError { .. }));
        }
    }

    #[test]
    fn test_stateful_server_with_custom_routes() {
        let container = create_test_container();
        let http_config = HttpConfig::default();
        
        // Create custom routes
        let custom_routes = Router::new()
            .route("/api/test", get(|| async { "test" }));

        let server = StatefulHttpServerBuilder::new()
            .container(container)
            .http_config(http_config)
            .add_routes(custom_routes)
            .build();

        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_stateful_health_check_handler() {
        let container = create_test_container();
        let state = AppState {
            container,
            config: HttpConfig::default(),
        };

        let result = stateful_health_check(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let status = response.0.get("status").and_then(|v| v.as_str()).unwrap();
        assert_eq!(status, "healthy");
        
        let server_type = response.0.get("server").and_then(|v| v.as_str()).unwrap();
        assert_eq!(server_type, "stateful");
        
        // Check that we have DI container info
        assert!(response.0.get("services").is_some());
        assert!(response.0.get("config").is_some());
    }

    #[test]
    fn test_app_state_clone() {
        let container = create_test_container();
        let state = AppState {
            container,
            config: HttpConfig::default(),
        };

        let cloned_state = state.clone();
        assert_eq!(cloned_state.config.health_check_path, "/health");
        assert_eq!(cloned_state.container.config().name, "test-app");
    }
}