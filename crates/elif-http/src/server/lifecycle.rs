//! Server lifecycle management - startup, shutdown, and signal handling

use crate::{
    config::HttpConfig,
    errors::{HttpError, HttpResult},
    routing::ElifRouter,
    server::health::health_check_handler,
};
use elif_core::Container;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

/// Build the internal Axum router (hidden from users)
pub async fn build_internal_router(
    container: Arc<Container>,
    config: HttpConfig,
    user_router: Option<ElifRouter>,
) -> HttpResult<axum::Router> {
    // Create health check handler with captured context
    let health_container = container.clone();
    let health_config = config.clone();
    let health_handler = move |_req: crate::request::ElifRequest| {
        let container = health_container.clone();
        let config = health_config.clone();
        async move {
            Ok(crate::response::ElifResponse::ok().json(&health_check_handler(container, config).await.0)?)
        }
    };

    // Start with framework router
    let mut router = if let Some(user_router) = user_router {
        user_router
    } else {
        ElifRouter::new()
    };

    // Add health check route
    router = router.get(&config.health_check_path, health_handler);

    // Convert to Axum router
    Ok(router.into_axum_router())
}

/// Start the server with graceful shutdown
pub async fn start_server(addr: SocketAddr, router: axum::Router) -> HttpResult<()> {
    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", addr, e)))?;

    info!("✅ Server listening on {}", addr);
    info!("🔧 Framework: Elif.rs (Axum under the hood)");

    // Serve with graceful shutdown
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| HttpError::internal(format!("Server error: {}", e)))?;

    Ok(())
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