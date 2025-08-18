//! Core types for the WebSocket channel system

use super::super::types::ConnectionId;
use super::password::SecurePasswordHash;
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
    /// 
    /// Uses UUID v5 with SHA-1 hashing to generate a stable, deterministic UUID
    /// that will be the same across Rust versions and platforms.
    pub fn from_name(name: &str) -> Self {
        // Use UUID v5 with a namespace for channel names
        // Using OID namespace as it's appropriate for application-specific identifiers
        const CHANNEL_NAMESPACE: Uuid = Uuid::from_bytes([
            0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1,
            0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
        ]); // This is the standard OID namespace UUID
        
        Self(Uuid::new_v5(&CHANNEL_NAMESPACE, name.as_bytes()))
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
    /// Protected channel - password required with secure Argon2 hashing
    Protected { 
        #[serde(serialize_with = "serialize_password_hash")]
        #[serde(deserialize_with = "deserialize_password_hash")]
        password_hash: SecurePasswordHash 
    },
}

// Custom serialization for SecurePasswordHash
fn serialize_password_hash<S>(hash: &SecurePasswordHash, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(hash.as_str())
}

// Custom deserialization for SecurePasswordHash  
fn deserialize_password_hash<'de, D>(deserializer: D) -> Result<SecurePasswordHash, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hash_string = String::deserialize(deserializer)?;
    SecurePasswordHash::from_hash_string(hash_string)
        .map_err(serde::de::Error::custom)
}

impl ChannelType {
    /// Create a protected channel type with a securely hashed password
    pub fn protected_with_password(password: &str) -> Result<Self, super::password::PasswordError> {
        let password_hash = SecurePasswordHash::hash_password(password)?;
        Ok(Self::Protected { password_hash })
    }
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