//! Connection registry for managing WebSocket connections

use super::connection::WebSocketConnection;
use super::types::{ConnectionId, ConnectionState, WebSocketMessage, WebSocketResult};
use super::channel::{ChannelManager, ChannelId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Events that can occur in the connection registry
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// New connection was added
    Connected(ConnectionId),
    /// Connection was removed
    Disconnected(ConnectionId, ConnectionState),
    /// Message was broadcast to all connections
    Broadcast(WebSocketMessage),
    /// Message was sent to specific connection
    MessageSent(ConnectionId, WebSocketMessage),
}

/// High-performance connection registry using Arc<RwLock<>> for concurrent access
pub struct ConnectionRegistry {
    /// Active connections
    connections: Arc<RwLock<HashMap<ConnectionId, Arc<WebSocketConnection>>>>,
    /// Channel manager for channel-based messaging
    channel_manager: Arc<ChannelManager>,
    /// Event subscribers (for future extensibility)
    event_handlers: Arc<RwLock<Vec<Box<dyn Fn(ConnectionEvent) + Send + Sync>>>>,
}

impl ConnectionRegistry {
    /// Create a new connection registry
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            channel_manager: Arc::new(ChannelManager::new()),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new connection registry with existing channel manager
    pub fn with_channel_manager(channel_manager: Arc<ChannelManager>) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            channel_manager,
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the channel manager
    pub fn channel_manager(&self) -> &Arc<ChannelManager> {
        &self.channel_manager
    }

    /// Add a connection to the registry
    pub async fn add_connection(&self, connection: WebSocketConnection) -> ConnectionId {
        let id = connection.id;
        let arc_connection = Arc::new(connection);
        
        {
            let mut connections = self.connections.write().await;
            connections.insert(id, arc_connection);
        }
        
        info!("Added connection to registry: {}", id);
        self.emit_event(ConnectionEvent::Connected(id)).await;
        
        id
    }

    /// Remove a connection from the registry
    pub async fn remove_connection(&self, id: ConnectionId) -> Option<Arc<WebSocketConnection>> {
        let connection = {
            let mut connections = self.connections.write().await;
            connections.remove(&id)
        };

        if let Some(conn) = &connection {
            let state = conn.state().await;
            
            // Clean up channel memberships
            self.channel_manager.leave_all_channels(id).await;
            
            info!("Removed connection from registry: {} (state: {:?})", id, state);
            self.emit_event(ConnectionEvent::Disconnected(id, state)).await;
        }

        connection
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: ConnectionId) -> Option<Arc<WebSocketConnection>> {
        let connections = self.connections.read().await;
        connections.get(&id).cloned()
    }

    /// Get all active connections
    pub async fn get_all_connections(&self) -> Vec<Arc<WebSocketConnection>> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Get all connection IDs
    pub async fn get_connection_ids(&self) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        connections.keys().copied().collect()
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Send a message to a specific connection
    pub async fn send_to_connection(
        &self,
        id: ConnectionId,
        message: WebSocketMessage,
    ) -> WebSocketResult<()> {
        let connection = self.get_connection(id).await
            .ok_or(WebSocketError::ConnectionNotFound(id))?;

        let result = connection.send(message.clone()).await;
        
        if result.is_ok() {
            self.emit_event(ConnectionEvent::MessageSent(id, message)).await;
        }
        
        result
    }

    /// Send a text message to a specific connection
    pub async fn send_text_to_connection<T: Into<String>>(
        &self,
        id: ConnectionId,
        text: T,
    ) -> WebSocketResult<()> {
        self.send_to_connection(id, WebSocketMessage::text(text)).await
    }

    /// Send a binary message to a specific connection
    pub async fn send_binary_to_connection<T: Into<Vec<u8>>>(
        &self,
        id: ConnectionId,
        data: T,
    ) -> WebSocketResult<()> {
        self.send_to_connection(id, WebSocketMessage::binary(data)).await
    }

    /// Broadcast a message to all active connections
    pub async fn broadcast(&self, message: WebSocketMessage) -> BroadcastResult {
        let connections = self.get_all_connections().await;
        let mut results = BroadcastResult::new();

        for connection in connections {
            if connection.is_active().await {
                match connection.send(message.clone()).await {
                    Ok(_) => results.success_count += 1,
                    Err(e) => {
                        results.failed_connections.push((connection.id, e));
                    }
                }
            } else {
                results.inactive_connections.push(connection.id);
            }
        }

        self.emit_event(ConnectionEvent::Broadcast(message)).await;
        results
    }

    /// Broadcast a text message to all active connections
    pub async fn broadcast_text<T: Into<String>>(&self, text: T) -> BroadcastResult {
        self.broadcast(WebSocketMessage::text(text)).await
    }

    /// Broadcast a binary message to all active connections
    pub async fn broadcast_binary<T: Into<Vec<u8>>>(&self, data: T) -> BroadcastResult {
        self.broadcast(WebSocketMessage::binary(data)).await
    }

    /// Send a message to a specific channel
    pub async fn send_to_channel(
        &self,
        channel_id: ChannelId,
        sender_id: ConnectionId,
        message: WebSocketMessage,
    ) -> WebSocketResult<BroadcastResult> {
        // Get the member IDs from the channel manager
        let member_ids = self.channel_manager
            .send_to_channel(channel_id, sender_id, message.clone())
            .await?;

        // Broadcast to all channel members
        let mut results = BroadcastResult::new();
        
        for member_id in member_ids {
            if let Some(connection) = self.get_connection(member_id).await {
                if connection.is_active().await {
                    match connection.send(message.clone()).await {
                        Ok(_) => results.success_count += 1,
                        Err(e) => {
                            results.failed_connections.push((member_id, e));
                        }
                    }
                } else {
                    results.inactive_connections.push(member_id);
                }
            } else {
                // Connection not in registry but still in channel - clean up
                let _ = self.channel_manager.leave_channel(channel_id, member_id).await;
            }
        }

        Ok(results)
    }

    /// Send a text message to a specific channel
    pub async fn send_text_to_channel<T: Into<String>>(
        &self,
        channel_id: ChannelId,
        sender_id: ConnectionId,
        text: T,
    ) -> WebSocketResult<BroadcastResult> {
        self.send_to_channel(channel_id, sender_id, WebSocketMessage::text(text)).await
    }

    /// Send a binary message to a specific channel
    pub async fn send_binary_to_channel<T: Into<Vec<u8>>>(
        &self,
        channel_id: ChannelId,
        sender_id: ConnectionId,
        data: T,
    ) -> WebSocketResult<BroadcastResult> {
        self.send_to_channel(channel_id, sender_id, WebSocketMessage::binary(data)).await
    }

    /// Close a specific connection
    pub async fn close_connection(&self, id: ConnectionId) -> WebSocketResult<()> {
        let connection = self.get_connection(id).await
            .ok_or(WebSocketError::ConnectionNotFound(id))?;

        connection.close().await?;
        self.remove_connection(id).await;
        
        Ok(())
    }

    /// Close all connections
    pub async fn close_all_connections(&self) -> CloseAllResult {
        let connections = self.get_all_connections().await;
        let mut results = CloseAllResult::new();
        let mut to_remove = Vec::new();

        for connection in connections {
            match connection.close().await {
                Ok(_) => {
                    to_remove.push(connection.id);
                    results.closed_count += 1;
                }
                Err(e) => {
                    results.failed_connections.push((connection.id, e));
                }
            }
        }

        // Batch removal: remove all closed connections under a single write lock
        if !to_remove.is_empty() {
            let mut connections = self.connections.write().await;
            for id in to_remove {
                if let Some(conn) = connections.remove(&id) {
                    let state = conn.state().await;
                    info!("Removed connection from registry: {} (state: {:?})", id, state);
                    // Note: We can't emit events here while holding the write lock
                    // to avoid potential deadlocks. Consider restructuring if events are critical.
                }
            }
        }

        results
    }

    /// Clean up inactive connections
    pub async fn cleanup_inactive_connections(&self) -> usize {
        let connections = self.get_all_connections().await;
        let mut to_remove = Vec::new();

        // First pass: identify inactive connections
        for connection in connections {
            if connection.is_closed().await {
                to_remove.push((connection.id, connection));
            }
        }

        let cleaned_up = to_remove.len();

        // Batch removal: remove all inactive connections under a single write lock
        if !to_remove.is_empty() {
            let mut registry_connections = self.connections.write().await;
            for (id, connection) in to_remove {
                if registry_connections.remove(&id).is_some() {
                    debug!("Cleaned up inactive connection: {}", id);
                    // Note: We can't emit Disconnected events here while holding the write lock
                    // to avoid potential deadlocks. Consider restructuring if events are critical.
                }
            }
        }

        if cleaned_up > 0 {
            info!("Cleaned up {} inactive connections", cleaned_up);
        }

        cleaned_up
    }

    /// Get registry statistics
    pub async fn stats(&self) -> RegistryStats {
        let connections = self.get_all_connections().await;
        let mut stats = RegistryStats::default();
        
        stats.total_connections = connections.len();
        
        for connection in connections {
            match connection.state().await {
                ConnectionState::Connected => stats.active_connections += 1,
                ConnectionState::Connecting => stats.connecting_connections += 1,
                ConnectionState::Closing => stats.closing_connections += 1,
                ConnectionState::Closed => stats.closed_connections += 1,
                ConnectionState::Failed(_) => stats.failed_connections += 1,
            }
            
            let conn_stats = connection.stats().await;
            stats.total_messages_sent += conn_stats.messages_sent;
            stats.total_messages_received += conn_stats.messages_received;
            stats.total_bytes_sent += conn_stats.bytes_sent;
            stats.total_bytes_received += conn_stats.bytes_received;
        }

        stats
    }

    /// Add an event handler (for future extensibility)
    pub async fn add_event_handler<F>(&self, handler: F)
    where
        F: Fn(ConnectionEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(Box::new(handler));
    }

    /// Emit an event to all handlers
    async fn emit_event(&self, event: ConnectionEvent) {
        let handlers = self.event_handlers.read().await;
        for handler in handlers.iter() {
            handler(event.clone());
        }
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of broadcasting a message to multiple connections
#[derive(Debug)]
pub struct BroadcastResult {
    pub success_count: usize,
    pub failed_connections: Vec<(ConnectionId, WebSocketError)>,
    pub inactive_connections: Vec<ConnectionId>,
}

impl BroadcastResult {
    fn new() -> Self {
        Self {
            success_count: 0,
            failed_connections: Vec::new(),
            inactive_connections: Vec::new(),
        }
    }

    pub fn total_attempted(&self) -> usize {
        self.success_count + self.failed_connections.len() + self.inactive_connections.len()
    }

    pub fn has_failures(&self) -> bool {
        !self.failed_connections.is_empty()
    }
}

/// Result of closing all connections
#[derive(Debug)]
pub struct CloseAllResult {
    pub closed_count: usize,
    pub failed_connections: Vec<(ConnectionId, WebSocketError)>,
}

impl CloseAllResult {
    fn new() -> Self {
        Self {
            closed_count: 0,
            failed_connections: Vec::new(),
        }
    }
}

/// Registry statistics
#[derive(Debug, Default)]
pub struct RegistryStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub connecting_connections: usize,
    pub closing_connections: usize,
    pub closed_connections: usize,
    pub failed_connections: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

// Re-export WebSocketError for convenience
use super::types::WebSocketError;