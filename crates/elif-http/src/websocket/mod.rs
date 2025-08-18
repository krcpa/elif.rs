//! WebSocket server foundation for elif framework
//!
//! This module provides WebSocket server capabilities integrated with the HTTP server,
//! including connection management, lifecycle handling, and message routing.

pub mod connection;
pub mod registry;
pub mod server;
pub mod types;
pub mod handler;

// Re-export main types
pub use connection::WebSocketConnection;
pub use registry::{ConnectionRegistry, ConnectionEvent};
pub use server::WebSocketServer;
pub use types::{
    WebSocketMessage, WebSocketError, WebSocketResult, MessageType, WebSocketConfig,
    ConnectionId, ConnectionState,
};
pub use handler::{WebSocketHandler, WebSocketUpgrade, SimpleWebSocketHandler};