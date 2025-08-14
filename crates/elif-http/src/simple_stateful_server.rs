//! Simple HTTP server with DI integration using closure approach
//! 
//! This approach avoids Router<State> issues by using closures to capture
//! DI container context in handlers.

use crate::{HttpConfig, HttpError, HttpResult};
use elif_core::Container;
use axum::{
    Router,
    routing::get,
    response::Json,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

/// Simple HTTP server with DI container integration
pub struct SimpleStatefulHttpServer {
    router: Router,
    addr: SocketAddr,
}

impl SimpleStatefulHttpServer {
    /// Create new HTTP server with DI container
    pub fn new(container: Arc<Container>, config: HttpConfig) -> HttpResult<Self> {
        let app_config = container.config();
        let addr = format!("{}:{}", app_config.server.host, app_config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| HttpError::config(format!("Invalid server address: {}", e)))?;

        // Create health check handler with captured container
        let health_container = container.clone();
        let health_config = config.clone();
        let health_handler = move || {
            let container = health_container.clone();
            let config = health_config.clone();
            async move {
                health_check_with_di(container, config).await
            }
        };

        // Create router with captured DI container
        let router = Router::new()
            .route(&config.health_check_path, get(health_handler));

        Ok(Self { router, addr })
    }

    /// Start the server
    pub async fn run(self) -> HttpResult<()> {
        info!("Starting simple stateful HTTP server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("Simple stateful HTTP server listening on {}", self.addr);

        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| HttpError::startup(format!("Server failed: {}", e)))?;

        info!("Simple stateful HTTP server stopped gracefully");
        Ok(())
    }
}

/// Health check handler with DI container access via closure capture
async fn health_check_with_di(container: Arc<Container>, config: HttpConfig) -> Json<Value> {
    // Check database connection
    let database = container.database();
    let db_healthy = database.is_connected();
    
    let app_config = container.config();
    let response = json!({
        "status": if db_healthy { "healthy" } else { "degraded" },
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0",
        "environment": format!("{:?}", app_config.environment),
        "server": "simple-stateful",
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

    if !db_healthy {
        warn!("Health check degraded: database not connected");
    }

    Json(response)
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
    fn test_simple_stateful_server_creation() {
        let container = create_test_container();
        let config = HttpConfig::default();

        let server = SimpleStatefulHttpServer::new(container, config);
        assert!(server.is_ok());

        let server = server.unwrap();
        assert_eq!(server.addr.port(), 8080);
    }

    #[tokio::test]
    async fn test_health_check_with_di() {
        let container = create_test_container();
        let config = HttpConfig::default();

        let result = health_check_with_di(container, config).await;
        let value = result.0;
        
        assert_eq!(value.get("status").and_then(|v| v.as_str()).unwrap(), "healthy");
        assert_eq!(value.get("server").and_then(|v| v.as_str()).unwrap(), "simple-stateful");
        
        // Check that we have DI container info
        assert!(value.get("services").is_some());
        assert!(value.get("config").is_some());
    }
}