//! Minimal working HTTP server
//! 
//! This is the simplest possible implementation to test the basic concepts.

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
use tracing::info;

/// Minimal HTTP server that works
pub struct MinimalHttpServer {
    addr: SocketAddr,
}

impl MinimalHttpServer {
    /// Create new minimal HTTP server
    pub fn new(container: Arc<Container>, config: HttpConfig) -> HttpResult<Self> {
        let app_config = container.config();
        let addr = format!("{}:{}", app_config.server.host, app_config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| HttpError::config(format!("Invalid server address: {}", e)))?;

        Ok(Self { addr })
    }

    /// Start the server
    pub async fn run(self) -> HttpResult<()> {
        info!("Starting minimal HTTP server on {}", self.addr);

        // Create a basic router without state
        let router = Router::new()
            .route("/health", get(minimal_health_check));

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", self.addr, e)))?;

        info!("Minimal HTTP server listening on {}", self.addr);

        axum::serve(listener, router)
            .with_graceful_shutdown(minimal_shutdown_signal())
            .await
            .map_err(|e| HttpError::startup(format!("Server failed: {}", e)))?;

        info!("Minimal HTTP server stopped gracefully");
        Ok(())
    }
}

/// Basic health check without DI container access
pub async fn minimal_health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0",
        "server": "minimal"
    }))
}

/// Minimal graceful shutdown signal handler
async fn minimal_shutdown_signal() {
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