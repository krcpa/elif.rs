//! WebSocket server integration with elif HTTP server

use super::registry::{ConnectionRegistry, RegistryStats};
use super::types::{ConnectionId, WebSocketConfig, WebSocketMessage, WebSocketResult};
use super::connection::WebSocketConnection;
use crate::routing::ElifRouter;
use axum::{
    extract::ws::WebSocketUpgrade as AxumWebSocketUpgrade,
    routing::get,
};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, info};

/// WebSocket server - integrates with elif HTTP server
pub struct WebSocketServer {
    /// Connection registry
    registry: Arc<ConnectionRegistry>,
    /// WebSocket configuration
    config: WebSocketConfig,
    /// Cleanup task handle
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ConnectionRegistry::new()),
            config: WebSocketConfig::default(),
            cleanup_handle: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: WebSocketConfig) -> Self {
        Self {
            registry: Arc::new(ConnectionRegistry::new()),
            config,
            cleanup_handle: None,
        }
    }

    /// Get the connection registry
    pub fn registry(&self) -> Arc<ConnectionRegistry> {
        self.registry.clone()
    }

    /// Get server statistics
    pub async fn stats(&self) -> RegistryStats {
        self.registry.stats().await
    }

    /// Add a WebSocket route to the router using a simple closure
    /// For now, this is a placeholder that will be improved in later iterations
    pub fn add_websocket_route<F, Fut>(
        &self,
        router: ElifRouter,
        path: &str,
        _handler: F,
    ) -> ElifRouter
    where
        F: Fn(ConnectionId, Arc<WebSocketConnection>) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // For the foundation, we'll add a simple WebSocket endpoint
        // Full message handling will be added in later iterations
        
        // Create a placeholder WebSocket handler
        let ws_handler = move |ws: AxumWebSocketUpgrade| async move {
            ws.on_upgrade(|mut socket| async move {
                tracing::info!("WebSocket connection established");
                // For now, just keep the connection alive
                while let Some(_msg) = socket.recv().await {
                    // Echo back for testing
                    if let Ok(_) = socket.send(axum::extract::ws::Message::Text("pong".to_string())).await {
                        continue;
                    }
                    break;
                }
                tracing::info!("WebSocket connection closed");
            })
        };
        
        // Add the route using the router's internal mechanism
        // This is a temporary solution for the foundation
        let axum_router = router.into_axum_router();
        let updated_router = axum_router.route(path, get(ws_handler));
        
        // Create a new ElifRouter from the updated axum router
        // For now, we lose the route registry information
        ElifRouter::new().merge_axum(updated_router)
    }

    /// Add a simple WebSocket handler function (alias for add_websocket_route)
    pub fn add_handler<F, Fut>(
        &self,
        router: ElifRouter,
        path: &str,
        handler: F,
    ) -> ElifRouter
    where
        F: Fn(ConnectionId, Arc<WebSocketConnection>) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.add_websocket_route(router, path, handler)
    }

    /// Broadcast a message to all connections
    pub async fn broadcast(&self, message: WebSocketMessage) -> super::registry::BroadcastResult {
        self.registry.broadcast(message).await
    }

    /// Broadcast text to all connections
    pub async fn broadcast_text<T: Into<String>>(&self, text: T) -> super::registry::BroadcastResult {
        self.registry.broadcast_text(text).await
    }

    /// Broadcast binary data to all connections
    pub async fn broadcast_binary<T: Into<Vec<u8>>>(&self, data: T) -> super::registry::BroadcastResult {
        self.registry.broadcast_binary(data).await
    }

    /// Send a message to a specific connection
    pub async fn send_to_connection(
        &self,
        id: ConnectionId,
        message: WebSocketMessage,
    ) -> WebSocketResult<()> {
        self.registry.send_to_connection(id, message).await
    }

    /// Send text to a specific connection
    pub async fn send_text_to_connection<T: Into<String>>(
        &self,
        id: ConnectionId,
        text: T,
    ) -> WebSocketResult<()> {
        self.registry.send_text_to_connection(id, text).await
    }

    /// Send binary data to a specific connection
    pub async fn send_binary_to_connection<T: Into<Vec<u8>>>(
        &self,
        id: ConnectionId,
        data: T,
    ) -> WebSocketResult<()> {
        self.registry.send_binary_to_connection(id, data).await
    }

    /// Get all active connection IDs
    pub async fn get_connection_ids(&self) -> Vec<ConnectionId> {
        self.registry.get_connection_ids().await
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        self.registry.connection_count().await
    }

    /// Close a specific connection
    pub async fn close_connection(&self, id: ConnectionId) -> WebSocketResult<()> {
        self.registry.close_connection(id).await
    }

    /// Close all connections
    pub async fn close_all_connections(&self) -> super::registry::CloseAllResult {
        self.registry.close_all_connections().await
    }

    /// Start the cleanup task for inactive connections
    pub fn start_cleanup_task(&mut self, interval_seconds: u64) {
        if self.cleanup_handle.is_some() {
            debug!("Cleanup task already running");
            return;
        }

        let registry = self.registry.clone();
        let handle = tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(interval_seconds));
            
            loop {
                cleanup_interval.tick().await;
                let cleaned = registry.cleanup_inactive_connections().await;
                if cleaned > 0 {
                    debug!("Cleanup task removed {} inactive connections", cleaned);
                }
            }
        });

        self.cleanup_handle = Some(handle);
        info!("Started WebSocket cleanup task with {}s interval", interval_seconds);
    }

    /// Stop the cleanup task
    pub fn stop_cleanup_task(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
            info!("Stopped WebSocket cleanup task");
        }
    }
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WebSocketServer {
    fn drop(&mut self) {
        self.stop_cleanup_task();
    }
}

/// Builder for WebSocket server configuration
#[derive(Debug)]
pub struct WebSocketServerBuilder {
    config: WebSocketConfig,
    cleanup_interval: Option<u64>,
}

impl WebSocketServerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: WebSocketConfig::default(),
            cleanup_interval: Some(300), // 5 minutes default
        }
    }

    /// Set maximum message size
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config.max_message_size = Some(size);
        self
    }

    /// Set maximum frame size
    pub fn max_frame_size(mut self, size: usize) -> Self {
        self.config.max_frame_size = Some(size);
        self
    }

    /// Enable/disable automatic pong responses
    pub fn auto_pong(mut self, enabled: bool) -> Self {
        self.config.auto_pong = enabled;
        self
    }

    /// Set ping interval in seconds
    pub fn ping_interval(mut self, seconds: u64) -> Self {
        self.config.ping_interval = Some(seconds);
        self
    }

    /// Set connection timeout in seconds
    pub fn connect_timeout(mut self, seconds: u64) -> Self {
        self.config.connect_timeout = Some(seconds);
        self
    }

    /// Set cleanup interval in seconds
    pub fn cleanup_interval(mut self, seconds: u64) -> Self {
        self.cleanup_interval = Some(seconds);
        self
    }

    /// Disable cleanup task
    pub fn no_cleanup(mut self) -> Self {
        self.cleanup_interval = None;
        self
    }

    /// Build the WebSocket server
    pub fn build(self) -> WebSocketServer {
        let mut server = WebSocketServer::with_config(self.config);
        
        if let Some(interval) = self.cleanup_interval {
            server.start_cleanup_task(interval);
        }
        
        server
    }
}

impl Default for WebSocketServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}