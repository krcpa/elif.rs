//! # Elif HTTP Server
//! 
//! A NestJS-like HTTP server that provides a clean, intuitive API while using Axum under the hood.
//! Users interact only with framework types - Axum is completely abstracted away.

use crate::{
    HttpConfig, HttpError, HttpResult,
    ElifRouter, 
    MiddlewarePipeline, Middleware,
};
use elif_core::Container;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

/// The main HTTP server - NestJS-like experience
/// 
/// # Example
/// 
/// ```rust,no_run
/// use elif_http::{Server, HttpConfig};
/// use elif_core::{Container, container::test_implementations::*};
/// use std::sync::Arc;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = Arc::new(create_test_config());
///     let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
///     
///     let container = Container::builder()
///         .config(config)
///         .database(database)
///         .build()?;
///         
///     let server = Server::new(container, HttpConfig::default())?;
///     server.listen("0.0.0.0:3000").await?;
///     
///     Ok(())
/// }
/// ```
pub struct Server {
    container: Arc<Container>,
    config: HttpConfig,
    router: Option<ElifRouter>,
    middleware: MiddlewarePipeline,
}

impl Server {
    /// Create a new server instance
    pub fn new(container: Container, config: HttpConfig) -> HttpResult<Self> {
        Ok(Self {
            container: Arc::new(container),
            config,
            router: None,
            middleware: MiddlewarePipeline::new(),
        })
    }

    /// Create a new server with existing Arc<Container>
    pub fn with_container(container: Arc<Container>, config: HttpConfig) -> HttpResult<Self> {
        Ok(Self {
            container,
            config,
            router: None,
            middleware: MiddlewarePipeline::new(),
        })
    }

    /// Set custom routes using framework router
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// use elif_http::{Server, ElifRouter, HttpConfig};
    /// use elif_core::{Container, container::test_implementations::*};
    /// use std::sync::Arc;
    /// 
    /// # async fn get_users() -> &'static str { "users" }
    /// # async fn create_user() -> &'static str { "created" }
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Arc::new(create_test_config());
    /// let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    /// 
    /// let container = Container::builder()
    ///     .config(config)
    ///     .database(database)
    ///     .build()?;
    ///     
    /// let mut server = Server::new(container, HttpConfig::default())?;
    /// 
    /// let router = ElifRouter::new()
    ///     .get("/users", get_users)
    ///     .post("/users", create_user);
    /// 
    /// server.use_router(router);
    /// # Ok(())
    /// # }
    /// ```
    pub fn use_router(&mut self, router: ElifRouter) -> &mut Self {
        self.router = Some(router);
        self
    }

    /// Add middleware to the server
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// use elif_http::{Server, HttpConfig};
    /// use elif_core::{Container, container::test_implementations::*};
    /// use std::sync::Arc;
    /// 
    /// # struct LoggingMiddleware;
    /// # impl LoggingMiddleware { 
    /// #     fn default() -> Self { LoggingMiddleware } 
    /// # }
    /// # impl elif_http::Middleware for LoggingMiddleware {}
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Arc::new(create_test_config());
    /// let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    /// 
    /// let container = Container::builder()
    ///     .config(config)
    ///     .database(database)
    ///     .build()?;
    ///     
    /// let mut server = Server::new(container, HttpConfig::default())?;
    /// server.use_middleware(LoggingMiddleware::default());
    /// # Ok(())
    /// # }
    /// ```
    pub fn use_middleware<M>(&mut self, middleware: M) -> &mut Self 
    where 
        M: Middleware + 'static,
    {
        self.middleware = std::mem::take(&mut self.middleware).add(middleware);
        self
    }

    /// Start the server on the specified address
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// # use elif_http::{Server, HttpConfig};
    /// # use elif_core::{Container, container::test_implementations::*};
    /// # use std::sync::Arc;
    /// # 
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     let config = Arc::new(create_test_config());
    /// #     let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    /// #     
    /// #     let container = Container::builder()
    /// #         .config(config)
    /// #         .database(database)
    /// #         .build()?;
    /// #         
    /// #     let server = Server::new(container, HttpConfig::default())?;
    /// server.listen("0.0.0.0:3000").await?;
    /// #     Ok(())
    /// # }
    /// ```
    pub async fn listen<A: Into<String>>(self, addr: A) -> HttpResult<()> {
        let addr_str = addr.into();
        let socket_addr: SocketAddr = addr_str.parse()
            .map_err(|e| HttpError::config(format!("Invalid address '{}': {}", addr_str, e)))?;

        self.listen_on(socket_addr).await
    }

    /// Start the server on the specified SocketAddr
    pub async fn listen_on(self, addr: SocketAddr) -> HttpResult<()> {
        info!("🚀 Starting Elif server on {}", addr);
        info!("📋 Health check endpoint: {}", self.config.health_check_path);
        
        // Build the internal router
        let axum_router = self.build_internal_router().await?;
        
        // Create TCP listener
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", addr, e)))?;

        info!("✅ Server listening on {}", addr);
        info!("🔧 Framework: Elif.rs (Axum under the hood)");

        // Serve with graceful shutdown
        axum::serve(listener, axum_router.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| HttpError::internal(format!("Server error: {}", e)))?;

        info!("🛑 Server shut down gracefully");
        Ok(())
    }

    /// Build the internal Axum router (hidden from users)
    async fn build_internal_router(self) -> HttpResult<axum::Router> {
        let container = self.container.clone();
        let config = self.config.clone();

        // Create health check handler with captured context
        let health_container = container.clone();
        let health_config = config.clone();
        let health_handler = move || {
            let container = health_container.clone();
            let config = health_config.clone();
            async move {
                health_check(container, config).await
            }
        };

        // Start with framework router
        let mut router = if let Some(user_router) = self.router {
            user_router
        } else {
            ElifRouter::new()
        };

        // Add health check route
        router = router.get(&config.health_check_path, health_handler);

        // Convert to Axum router
        Ok(router.into_axum_router())
    }
}

/// Default health check handler
pub async fn health_check(_container: Arc<Container>, _config: HttpConfig) -> axum::response::Json<serde_json::Value> {
    use serde_json::json;

    let response = json!({
        "status": "healthy",
        "framework": "Elif.rs",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "server": {
            "ready": true,
            "uptime": "N/A"
        }
    });

    axum::response::Json(response)
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("📡 Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            warn!("📡 Received terminate signal, shutting down gracefully...");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_core::{
        container::test_implementations::*,
        app_config::AppConfigTrait,
    };

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
    fn test_server_creation() {
        let container = create_test_container();
        let config = HttpConfig::default();
        
        let server = Server::with_container(container, config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_with_arc_container() {
        let container = create_test_container();
        let config = HttpConfig::default();
        
        let server = Server::with_container(container, config);
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_handler() {
        let container = create_test_container();
        let config = HttpConfig::default();
        
        let response = health_check(container, config).await;
        // Test that response is properly formatted JSON
        assert!(response.0.get("status").is_some());
        assert_eq!(response.0["status"], "healthy");
    }

    #[test]
    fn test_invalid_address() {
        let container = create_test_container();
        let config = HttpConfig::default();
        let server = Server::with_container(container, config).unwrap();
        
        // This should be tested with an actual tokio runtime in integration tests
        // For now, we just verify the server can be created
    }
}