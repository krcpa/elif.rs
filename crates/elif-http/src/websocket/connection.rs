//! WebSocket connection management - high-performance wrapper around tokio-tungstenite

use super::types::{
    ConnectionId, ConnectionState, WebSocketMessage, WebSocketError, WebSocketResult, WebSocketConfig,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use tokio_tungstenite::{accept_async, tungstenite, WebSocketStream};
use tracing::{debug, error, info, warn};

/// WebSocket connection wrapper - clean API over tokio-tungstenite
#[derive(Clone)]
pub struct WebSocketConnection {
    /// Unique connection identifier
    pub id: ConnectionId,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Connection metadata
    metadata: Arc<RwLock<ConnectionMetadata>>,
    /// Message sender channel
    sender: mpsc::UnboundedSender<WebSocketMessage>,
    /// Configuration
    config: WebSocketConfig,
}

/// Connection metadata for tracking and debugging
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    /// When the connection was established
    pub connected_at: Instant,
    /// Remote address if available
    pub remote_addr: Option<String>,
    /// User agent if available
    pub user_agent: Option<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
    /// Message statistics
    pub stats: ConnectionStats,
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Last activity timestamp
    pub last_activity: Option<Instant>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection from a TCP stream
    pub async fn from_stream<S>(
        stream: S,
        config: WebSocketConfig,
    ) -> WebSocketResult<Self>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let id = ConnectionId::new();
        let ws_stream = accept_async(stream).await?;
        
        let (sender, receiver) = mpsc::unbounded_channel();
        let state = Arc::new(RwLock::new(ConnectionState::Connected));
        let metadata = Arc::new(RwLock::new(ConnectionMetadata {
            connected_at: Instant::now(),
            remote_addr: None,
            user_agent: None,
            custom: HashMap::new(),
            stats: ConnectionStats::default(),
        }));

        // Start the connection handler task
        let connection = Self {
            id,
            state: state.clone(),
            metadata: metadata.clone(),
            sender,
            config: config.clone(),
        };

        // Spawn the connection handler
        tokio::spawn(Self::handle_connection(
            id,
            ws_stream,
            receiver,
            state,
            metadata,
            config,
        ));

        info!("WebSocket connection established: {}", id);
        Ok(connection)
    }

    /// Send a message to the WebSocket
    pub async fn send(&self, message: WebSocketMessage) -> WebSocketResult<()> {
        if !self.is_active().await {
            return Err(WebSocketError::ConnectionClosed);
        }

        self.sender
            .send(message)
            .map_err(|_| WebSocketError::SendQueueFull)?;
        
        Ok(())
    }

    /// Send a text message
    pub async fn send_text<T: Into<String>>(&self, text: T) -> WebSocketResult<()> {
        self.send(WebSocketMessage::text(text)).await
    }

    /// Send a binary message
    pub async fn send_binary<T: Into<Vec<u8>>>(&self, data: T) -> WebSocketResult<()> {
        self.send(WebSocketMessage::binary(data)).await
    }

    /// Send a ping
    pub async fn ping<T: Into<Vec<u8>>>(&self, data: T) -> WebSocketResult<()> {
        self.send(WebSocketMessage::ping(data)).await
    }

    /// Close the connection
    pub async fn close(&self) -> WebSocketResult<()> {
        self.send(WebSocketMessage::close()).await?;
        
        let mut state = self.state.write().await;
        *state = ConnectionState::Closing;
        
        Ok(())
    }

    /// Close the connection with a reason
    pub async fn close_with_reason(&self, code: u16, reason: String) -> WebSocketResult<()> {
        self.send(WebSocketMessage::close_with_reason(code, reason)).await?;
        
        let mut state = self.state.write().await;
        *state = ConnectionState::Closing;
        
        Ok(())
    }

    /// Get the current connection state
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    /// Check if the connection is active
    pub async fn is_active(&self) -> bool {
        self.state().await.is_active()
    }

    /// Check if the connection is closed
    pub async fn is_closed(&self) -> bool {
        self.state().await.is_closed()
    }

    /// Get connection metadata
    pub async fn metadata(&self) -> ConnectionMetadata {
        self.metadata.read().await.clone()
    }

    /// Update connection metadata
    pub async fn set_metadata(&self, key: String, value: String) {
        let mut metadata = self.metadata.write().await;
        metadata.custom.insert(key, value);
    }

    /// Get connection statistics
    pub async fn stats(&self) -> ConnectionStats {
        self.metadata.read().await.stats.clone()
    }

    /// Connection handler - runs the actual WebSocket loop
    async fn handle_connection<S>(
        id: ConnectionId,
        mut ws_stream: WebSocketStream<S>,
        mut receiver: mpsc::UnboundedReceiver<WebSocketMessage>,
        state: Arc<RwLock<ConnectionState>>,
        metadata: Arc<RwLock<ConnectionMetadata>>,
        config: WebSocketConfig,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        debug!("Starting WebSocket handler for connection: {}", id);

        // Set up ping interval if configured
        let mut ping_interval = if let Some(interval) = config.ping_interval {
            Some(time::interval(Duration::from_secs(interval)))
        } else {
            None
        };

        loop {
            tokio::select! {
                // Handle incoming messages from WebSocket
                ws_msg = ws_stream.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => {
                            let elif_msg = WebSocketMessage::from(msg);
                            
                            // Update stats
                            {
                                let mut meta = metadata.write().await;
                                meta.stats.messages_received += 1;
                                meta.stats.last_activity = Some(Instant::now());
                                
                                // Estimate bytes received
                                let bytes = match &elif_msg {
                                    WebSocketMessage::Text(s) => s.len() as u64,
                                    WebSocketMessage::Binary(b) => b.len() as u64,
                                    _ => 0,
                                };
                                meta.stats.bytes_received += bytes;
                            }

                            // Handle control frames automatically
                            match &elif_msg {
                                WebSocketMessage::Ping(data) => {
                                    if config.auto_pong {
                                        let pong_msg = tungstenite::Message::Pong(data.clone());
                                        if let Err(e) = ws_stream.send(pong_msg).await {
                                            error!("Failed to send pong for {}: {}", id, e);
                                            break;
                                        }
                                    }
                                }
                                WebSocketMessage::Close(_) => {
                                    info!("Received close frame for connection: {}", id);
                                    break;
                                }
                                _ => {
                                    // For now, we just log other messages
                                    // In a full implementation, we'd route these to handlers
                                    debug!("Received message on {}: {:?}", id, elif_msg.message_type());
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error for {}: {}", id, e);
                            let mut state_lock = state.write().await;
                            *state_lock = ConnectionState::Failed(e.to_string());
                            break;
                        }
                        None => {
                            info!("WebSocket stream ended for connection: {}", id);
                            break;
                        }
                    }
                }

                // Handle outgoing messages from application
                app_msg = receiver.recv() => {
                    match app_msg {
                        Some(msg) => {
                            // Update stats
                            {
                                let mut meta = metadata.write().await;
                                meta.stats.messages_sent += 1;
                                meta.stats.last_activity = Some(Instant::now());
                                
                                // Estimate bytes sent
                                let bytes = match &msg {
                                    WebSocketMessage::Text(s) => s.len() as u64,
                                    WebSocketMessage::Binary(b) => b.len() as u64,
                                    _ => 0,
                                };
                                meta.stats.bytes_sent += bytes;
                            }

                            let tungstenite_msg = tungstenite::Message::from(msg);
                            if let Err(e) = ws_stream.send(tungstenite_msg).await {
                                error!("Failed to send message for {}: {}", id, e);
                                let mut state_lock = state.write().await;
                                *state_lock = ConnectionState::Failed(e.to_string());
                                break;
                            }
                        }
                        None => {
                            debug!("Application message channel closed for: {}", id);
                            break;
                        }
                    }
                }

                // Handle ping interval
                _ = async {
                    if let Some(ref mut interval) = ping_interval {
                        interval.tick().await;
                    } else {
                        // If no ping interval, wait indefinitely
                        std::future::pending::<()>().await;
                    }
                } => {
                    // Send ping
                    let ping_msg = tungstenite::Message::Ping(vec![]);
                    if let Err(e) = ws_stream.send(ping_msg).await {
                        error!("Failed to send ping for {}: {}", id, e);
                        break;
                    }
                    debug!("Sent ping to connection: {}", id);
                }
            }
        }

        // Connection cleanup
        let mut state_lock = state.write().await;
        if !matches!(*state_lock, ConnectionState::Failed(_)) {
            *state_lock = ConnectionState::Closed;
        }
        
        info!("WebSocket connection handler finished: {}", id);
    }
}

impl Drop for WebSocketConnection {
    fn drop(&mut self) {
        debug!("Dropping WebSocket connection: {}", self.id);
    }
}