//! Simple HTTP server implementation for testing
//! 
//! This is a simplified version to get the basic patterns working.

use crate::{HttpConfig, HttpError, HttpResult};
use elif_core::Container;
use axum::{
    Router,
    routing::get,
    extract::State,
    response::Json,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn, error};

/// Simple HTTP server state
#[derive(Clone)]
pub struct SimpleServerState {
    pub container: Arc<Container>,
}

/// Simple HTTP server for testing
pub struct SimpleHttpServer {
    router: Router<SimpleServerState>,
    addr: SocketAddr,
    config: HttpConfig,
}

impl SimpleHttpServer {
    /// Create new simple HTTP server
    pub fn new(container: Arc<Container>, config: HttpConfig) -> HttpResult<Self> {
        let app_config = container.config();
        let addr = format!("{}:{}", app_config.server.host, app_config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| HttpError::config(format!("Invalid server address: {}", e)))?;

        let state = SimpleServerState { container };

        let router = Router::new()
            .route(&config.health_check_path, get(simple_health_check))
            .with_state(state);

        Ok(Self {
            router,
            addr,
            config,
        })
    }

    /// Start the server
    pub async fn run(self) -> HttpResult<()> {
        info!("Starting simple HTTP server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("HTTP server listening on {}", self.addr);

        axum::serve(listener, self.router.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(simple_shutdown_signal())
            .await
            .map_err(|e| HttpError::startup(format!("Server failed: {}", e)))?;

        info!("HTTP server stopped gracefully");
        Ok(())
    }
}

/// Simple health check handler
async fn simple_health_check(State(state): State<SimpleServerState>) -> Json<Value> {
    let container = &state.container;
    let database = container.database();
    let db_healthy = database.is_connected();
    let app_config = container.config();

    Json(json!({
        "status": if db_healthy { "healthy" } else { "unhealthy" },
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0",
        "environment": format!("{:?}", app_config.environment),
        "services": {
            "database": if db_healthy { "healthy" } else { "unhealthy" }
        }
    }))
}

/// Simple graceful shutdown signal handler
async fn simple_shutdown_signal() {
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