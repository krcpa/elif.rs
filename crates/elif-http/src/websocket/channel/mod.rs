//! Channel abstraction system for WebSocket messaging
//!
//! This module provides channel-based messaging capabilities, allowing connections
//! to join/leave channels and enabling targeted message broadcasting to channel members.
//!
//! ## Architecture
//!
//! The channel system is organized into several logical modules:
//!
//! - [`types`] - Core types and data structures
//! - [`channel`] - Individual channel implementation
//! - [`manager`] - Channel lifecycle and management
//! - [`message`] - Channel message types
//! - [`events`] - Event system for channel operations
//!
//! ## Quick Start
//!
//! ```rust
//! use elif_http::websocket::{ChannelManager, ChannelType, ConnectionId};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let manager = ChannelManager::new();
//!     let connection_id = ConnectionId::new();
//!     
//!     // Create a public channel
//!     let channel_id = manager.create_channel(
//!         "general".to_string(),
//!         ChannelType::Public,
//!         Some(connection_id),
//!     ).await?;
//!     
//!     // Join the channel
//!     manager.join_channel(
//!         channel_id,
//!         connection_id,
//!         None, // No password needed for public channels
//!         Some("Alice".to_string()),
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod types;
pub mod channel;
pub mod manager;
pub mod message;
pub mod events;
pub mod password;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use types::{
    ChannelId, ChannelType, ChannelPermissions, ChannelMember, ChannelMetadata,
    ChannelStats, ChannelManagerStats,
};
pub use channel::Channel;
pub use manager::ChannelManager;
pub use message::ChannelMessage;
pub use events::ChannelEvent;