//! WebSocket server foundation for elif framework
//!
//! This module provides WebSocket server capabilities integrated with the HTTP server,
//! including connection management, lifecycle handling, and message routing.

pub mod channel;
pub mod connection;
pub mod handler;
pub mod registry;
pub mod server;
pub mod types;

// Re-export main types
pub use channel::{
    Channel, ChannelEvent, ChannelId, ChannelManager, ChannelManagerStats, ChannelMember,
    ChannelMessage, ChannelMetadata, ChannelPermissions, ChannelStats, ChannelType,
};
pub use connection::WebSocketConnection;
pub use handler::{SimpleWebSocketHandler, WebSocketHandler, WebSocketUpgrade};
pub use registry::{ConnectionEvent, ConnectionRegistry};
pub use server::WebSocketServer;
pub use types::{
    ConnectionId, ConnectionState, MessageType, WebSocketConfig, WebSocketError, WebSocketMessage,
    WebSocketResult,
};
