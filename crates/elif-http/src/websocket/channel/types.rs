//! Core types for the WebSocket channel system

use super::super::types::ConnectionId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;
use uuid::Uuid;

/// Unique identifier for channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub Uuid);

impl ChannelId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a channel ID from a string name (deterministic)
    pub fn from_name(name: &str) -> Self {
        // Use a deterministic approach with hashing for now
        // In production, you might want to enable UUID v5 feature
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Convert hash to UUID bytes (simplified approach)
        let bytes = [
            ((hash >> 56) & 0xFF) as u8,
            ((hash >> 48) & 0xFF) as u8,
            ((hash >> 40) & 0xFF) as u8,
            ((hash >> 32) & 0xFF) as u8,
            ((hash >> 24) & 0xFF) as u8,
            ((hash >> 16) & 0xFF) as u8,
            ((hash >> 8) & 0xFF) as u8,
            (hash & 0xFF) as u8,
            0, 0, 0, 0, // padding
            ((hash >> 28) & 0xFF) as u8,
            ((hash >> 20) & 0xFF) as u8,
            ((hash >> 12) & 0xFF) as u8,
            ((hash >> 4) & 0xFF) as u8,
        ];
        
        Self(Uuid::from_bytes(bytes))
    }
}

impl Default for ChannelId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Channel visibility and access control
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelType {
    /// Public channel - anyone can join
    Public,
    /// Private channel - invitation required
    Private,
    /// Protected channel - password required
    Protected { password_hash: String },
}

/// Channel permissions for members
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelPermissions {
    /// Can send messages to the channel
    pub can_send_messages: bool,
    /// Can invite other users to the channel
    pub can_invite: bool,
    /// Can kick members from the channel (moderator)
    pub can_kick: bool,
    /// Can modify channel settings (admin)
    pub can_modify: bool,
}

impl Default for ChannelPermissions {
    fn default() -> Self {
        Self {
            can_send_messages: true,
            can_invite: false,
            can_kick: false,
            can_modify: false,
        }
    }
}

impl ChannelPermissions {
    pub fn moderator() -> Self {
        Self {
            can_send_messages: true,
            can_invite: true,
            can_kick: true,
            can_modify: false,
        }
    }

    pub fn admin() -> Self {
        Self {
            can_send_messages: true,
            can_invite: true,
            can_kick: true,
            can_modify: true,
        }
    }
}

/// Channel member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMember {
    pub connection_id: ConnectionId,
    pub joined_at: SystemTime,
    pub permissions: ChannelPermissions,
    pub nickname: Option<String>,
}

/// Channel metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMetadata {
    pub name: String,
    pub description: Option<String>,
    pub created_at: SystemTime,
    pub created_by: Option<ConnectionId>,
    pub channel_type: ChannelType,
    pub max_members: Option<usize>,
    pub message_history_limit: Option<usize>,
}

/// Channel statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub id: ChannelId,
    pub name: String,
    pub member_count: usize,
    pub message_count: usize,
    pub channel_type: ChannelType,
    pub created_at: SystemTime,
    pub is_empty: bool,
}

/// Channel manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelManagerStats {
    pub total_channels: usize,
    pub total_connections_in_channels: usize,
    pub public_channels: usize,
    pub private_channels: usize,
    pub protected_channels: usize,
    pub empty_channels: usize,
}