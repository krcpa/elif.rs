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
//!     let creator_id = ConnectionId::new();
//!     let joiner_id = ConnectionId::new();
//!     
//!     // Create a public channel
//!     let channel_id = manager.create_channel(
//!         "general".to_string(),
//!         ChannelType::Public,
//!         Some(creator_id),
//!     ).await?;
//!     
//!     // Join the channel with a different connection
//!     manager.join_channel(
//!         channel_id,
//!         joiner_id,
//!         None, // No password needed for public channels
//!         Some("Alice".to_string()),
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod channel;
pub mod events;
pub mod manager;
pub mod message;
pub mod password;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use channel::Channel;
pub use events::ChannelEvent;
pub use manager::ChannelManager;
pub use message::ChannelMessage;
pub use types::{
    ChannelId, ChannelManagerStats, ChannelMember, ChannelMetadata, ChannelPermissions,
    ChannelStats, ChannelType,
};
