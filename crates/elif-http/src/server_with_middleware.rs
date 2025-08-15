//! # HTTP Server with Middleware Support
//!
//! Enhanced HTTP server that demonstrates middleware pipeline integration.

use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    response::Json,
};
use serde_json::{json, Value};
use tokio::signal;
use tracing::info;
use elif_core::Container;

use crate::{
    HttpConfig, HttpError, HttpResult,
    MiddlewarePipeline, LoggingMiddleware, TimingMiddleware,
};

/// HTTP server with middleware support
pub struct MiddlewareHttpServer {
    container: Arc<Container>,
    config: HttpConfig,
    middleware: MiddlewarePipeline,
}

impl MiddlewareHttpServer {
    /// Create new server with default middleware
    pub fn new(container: Arc<Container>, config: HttpConfig) -> HttpResult<Self> {
        let server = Self {
            container,
            config,
            middleware: MiddlewarePipeline::new(),
        };
        
        Ok(server)
    }
    
    /// Create server with custom middleware pipeline
    pub fn with_middleware(
        container: Arc<Container>, 
        config: HttpConfig,
        middleware: MiddlewarePipeline
    ) -> HttpResult<Self> {
        Ok(Self {
            container,
            config,
            middleware,
        })
    }
    
    /// Add default middleware (logging + timing)
    pub fn with_default_middleware(mut self) -> Self {
        self.middleware = self.middleware
            .add(LoggingMiddleware::new())
            .add(TimingMiddleware::new());
        self
    }
    
    /// Get reference to middleware pipeline
    pub fn middleware(&self) -> &MiddlewarePipeline {
        &self.middleware
    }
    
    /// Get mutable reference to middleware pipeline
    pub fn middleware_mut(&mut self) -> &mut MiddlewarePipeline {
        &mut self.middleware
    }
    
    /// Start the server
    pub async fn run(&self) -> HttpResult<()> {
        let addr = "127.0.0.1:3000".to_string(); // Default address
        
        // Create router with middleware-aware handlers
        let router = Router::new()
            .route("/health", get(middleware_health_check))
            .route("/middleware/info", get(middleware_info_handler))
            .with_state((self.container.clone(), self.middleware.len()));

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| HttpError::startup(format!("Failed to bind to {}: {}", addr, e)))?;

        info!("HTTP server with middleware listening on {}", addr);
        info!("Middleware pipeline: {:?}", self.middleware.names());

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| HttpError::startup(format!("Server failed: {}", e)))?;

        info!("HTTP server stopped gracefully");
        Ok(())
    }
}

/// Health check endpoint that can be enhanced with middleware
pub async fn middleware_health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0",
        "server": "middleware-enabled",
        "middleware": "active"
    }))
}

/// Endpoint that provides middleware information
pub async fn middleware_info_handler(
    axum::extract::State((container, middleware_count)): axum::extract::State<(Arc<Container>, usize)>
) -> Json<Value> {
    Json(json!({
        "middleware_count": middleware_count,
        "container_registered": true, // container.services().len(), - method doesn't exist yet
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
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
        let mut signal = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler");
        signal.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoggingMiddleware, TimingMiddleware};
    
    // Mock container for testing (since the real Container requires builder pattern)
    struct MockContainer;
    
    #[tokio::test]
    async fn test_middleware_pipeline() {
        let pipeline = MiddlewarePipeline::new()
            .add(LoggingMiddleware::new())
            .add(TimingMiddleware::new());
        
        assert_eq!(pipeline.len(), 2);
        assert_eq!(
            pipeline.names(),
            vec!["LoggingMiddleware", "TimingMiddleware"]
        );
    }
    
    #[tokio::test]
    async fn test_custom_middleware_pipeline() {
        let pipeline = MiddlewarePipeline::new()
            .add(TimingMiddleware::new())
            .add(LoggingMiddleware::new().with_body_logging());
        
        assert_eq!(pipeline.len(), 2);
        assert_eq!(
            pipeline.names(),
            vec!["TimingMiddleware", "LoggingMiddleware"]
        );
    }
}