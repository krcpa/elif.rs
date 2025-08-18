//! WebSocket types and abstractions for elif framework
//!
//! These types provide a clean, framework-native API while using tokio-tungstenite
//! for maximum performance under the hood.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use uuid::Uuid;

/// Unique identifier for WebSocket connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// WebSocket message types - clean API over tungstenite
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketMessage {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping frame
    Ping(Vec<u8>),
    /// Pong frame
    Pong(Vec<u8>),
    /// Close frame
    Close(Option<CloseFrame>),
}

/// Close frame information
#[derive(Debug, Clone, PartialEq)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}

/// Message type for routing and handling
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

impl WebSocketMessage {
    pub fn text<T: Into<String>>(content: T) -> Self {
        Self::Text(content.into())
    }

    pub fn binary<T: Into<Vec<u8>>>(data: T) -> Self {
        Self::Binary(data.into())
    }

    pub fn ping<T: Into<Vec<u8>>>(data: T) -> Self {
        Self::Ping(data.into())
    }

    pub fn pong<T: Into<Vec<u8>>>(data: T) -> Self {
        Self::Pong(data.into())
    }

    pub fn close() -> Self {
        Self::Close(None)
    }

    pub fn close_with_reason(code: u16, reason: String) -> Self {
        Self::Close(Some(CloseFrame { code, reason }))
    }

    pub fn message_type(&self) -> MessageType {
        match self {
            Self::Text(_) => MessageType::Text,
            Self::Binary(_) => MessageType::Binary,
            Self::Ping(_) => MessageType::Ping,
            Self::Pong(_) => MessageType::Pong,
            Self::Close(_) => MessageType::Close,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    pub fn is_control(&self) -> bool {
        matches!(self, Self::Ping(_) | Self::Pong(_) | Self::Close(_))
    }
}

// Conversion from tungstenite message to elif message
impl From<tungstenite::Message> for WebSocketMessage {
    fn from(msg: tungstenite::Message) -> Self {
        match msg {
            tungstenite::Message::Text(text) => Self::Text(text),
            tungstenite::Message::Binary(data) => Self::Binary(data),
            tungstenite::Message::Ping(data) => Self::Ping(data),
            tungstenite::Message::Pong(data) => Self::Pong(data),
            tungstenite::Message::Close(frame) => {
                Self::Close(frame.map(|f| CloseFrame {
                    code: f.code.into(),
                    reason: f.reason.into(),
                }))
            }
            tungstenite::Message::Frame(_) => {
                // Raw frames are internal to tungstenite and should never reach application code
                unreachable!("Raw frames should not be exposed by tungstenite's high-level API")
            }
        }
    }
}

// Conversion from elif message to tungstenite message
impl From<WebSocketMessage> for tungstenite::Message {
    fn from(msg: WebSocketMessage) -> Self {
        match msg {
            WebSocketMessage::Text(text) => tungstenite::Message::Text(text),
            WebSocketMessage::Binary(data) => tungstenite::Message::Binary(data),
            WebSocketMessage::Ping(data) => tungstenite::Message::Ping(data),
            WebSocketMessage::Pong(data) => tungstenite::Message::Pong(data),
            WebSocketMessage::Close(frame) => {
                tungstenite::Message::Close(frame.map(|f| tungstenite::protocol::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::from(f.code),
                    reason: f.reason.into(),
                }))
            }
        }
    }
}

/// WebSocket errors - clean API over tungstenite errors
#[derive(Debug, Error)]
pub enum WebSocketError {
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Connection closed")]
    ConnectionClosed,
    
    #[error("Invalid message type")]
    InvalidMessageType,
    
    #[error("Send queue full")]
    SendQueueFull,
    
    #[error("Connection not found: {0}")]
    ConnectionNotFound(ConnectionId),
}

impl From<tungstenite::Error> for WebSocketError {
    fn from(err: tungstenite::Error) -> Self {
        match err {
            tungstenite::Error::ConnectionClosed => Self::ConnectionClosed,
            tungstenite::Error::Protocol(msg) => Self::Protocol(msg.to_string()),
            tungstenite::Error::Io(io_err) => Self::Io(io_err),
            other => Self::Connection(other.to_string()),
        }
    }
}

/// Result type for WebSocket operations
pub type WebSocketResult<T> = Result<T, WebSocketError>;

/// Connection state tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    /// Connection is active and ready
    Connected,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Closed,
    /// Connection failed
    Failed(String),
}

impl ConnectionState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Connected)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed | Self::Failed(_))
    }
}

/// WebSocket protocol configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Maximum message size in bytes
    pub max_message_size: Option<usize>,
    /// Maximum frame size in bytes  
    pub max_frame_size: Option<usize>,
    /// Enable automatic ping/pong handling
    pub auto_pong: bool,
    /// Ping interval in seconds
    pub ping_interval: Option<u64>,
    /// Connection timeout in seconds
    pub connect_timeout: Option<u64>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: Some(64 * 1024 * 1024), // 64MB
            max_frame_size: Some(16 * 1024 * 1024),   // 16MB
            auto_pong: true,
            ping_interval: Some(30), // 30 seconds
            connect_timeout: Some(10), // 10 seconds
        }
    }
}