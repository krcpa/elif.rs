//! Core Channel implementation

use super::super::types::{ConnectionId, WebSocketError, WebSocketResult};
use super::types::{ChannelId, ChannelMember, ChannelMetadata, ChannelPermissions, ChannelStats, ChannelType};
use super::message::ChannelMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// A WebSocket channel that manages members and message distribution
#[derive(Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub metadata: ChannelMetadata,
    members: Arc<RwLock<HashMap<ConnectionId, ChannelMember>>>,
    message_history: Arc<RwLock<Vec<ChannelMessage>>>,
}

impl Channel {
    /// Create a new channel
    pub fn new(name: String, channel_type: ChannelType, created_by: Option<ConnectionId>) -> Self {
        let id = ChannelId::from_name(&name);
        let metadata = ChannelMetadata {
            name,
            description: None,
            created_at: SystemTime::now(),
            created_by,
            channel_type,
            max_members: None,
            message_history_limit: Some(100), // Default to 100 messages
        };

        Self {
            id,
            metadata,
            members: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new channel with custom metadata
    pub fn with_metadata(metadata: ChannelMetadata) -> Self {
        let id = ChannelId::from_name(&metadata.name);
        
        Self {
            id,
            metadata,
            members: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a member to the channel
    pub async fn add_member(
        &self,
        connection_id: ConnectionId,
        permissions: ChannelPermissions,
        nickname: Option<String>,
    ) -> WebSocketResult<()> {
        let mut members = self.members.write().await;
        
        // Check if channel is at capacity
        if let Some(max_members) = self.metadata.max_members {
            if members.len() >= max_members {
                return Err(WebSocketError::Connection("Channel is at capacity".to_string()));
            }
        }

        // Check if member already exists
        if members.contains_key(&connection_id) {
            return Err(WebSocketError::Connection("Connection already in channel".to_string()));
        }

        let member = ChannelMember {
            connection_id,
            joined_at: SystemTime::now(),
            permissions,
            nickname,
        };

        members.insert(connection_id, member);
        info!("Added member {} to channel {}", connection_id, self.id);
        
        Ok(())
    }

    /// Remove a member from the channel
    pub async fn remove_member(&self, connection_id: ConnectionId) -> Option<ChannelMember> {
        let mut members = self.members.write().await;
        let member = members.remove(&connection_id);
        
        if member.is_some() {
            info!("Removed member {} from channel {}", connection_id, self.id);
        }
        
        member
    }

    /// Get a member by connection ID
    pub async fn get_member(&self, connection_id: ConnectionId) -> Option<ChannelMember> {
        let members = self.members.read().await;
        members.get(&connection_id).cloned()
    }

    /// Check if a connection is a member of this channel
    pub async fn has_member(&self, connection_id: ConnectionId) -> bool {
        let members = self.members.read().await;
        members.contains_key(&connection_id)
    }

    /// Get all member IDs
    pub async fn get_member_ids(&self) -> Vec<ConnectionId> {
        let members = self.members.read().await;
        members.keys().copied().collect()
    }

    /// Get all members
    pub async fn get_members(&self) -> Vec<ChannelMember> {
        let members = self.members.read().await;
        members.values().cloned().collect()
    }

    /// Get the number of members in the channel
    pub async fn member_count(&self) -> usize {
        let members = self.members.read().await;
        members.len()
    }

    /// Check if the channel is empty
    pub async fn is_empty(&self) -> bool {
        let members = self.members.read().await;
        members.is_empty()
    }

    /// Update member permissions
    pub async fn update_member_permissions(
        &self,
        connection_id: ConnectionId,
        new_permissions: ChannelPermissions,
    ) -> WebSocketResult<()> {
        let mut members = self.members.write().await;
        
        match members.get_mut(&connection_id) {
            Some(member) => {
                member.permissions = new_permissions;
                debug!("Updated permissions for member {} in channel {}", connection_id, self.id);
                Ok(())
            }
            None => Err(WebSocketError::Connection("Member not found in channel".to_string())),
        }
    }

    /// Update member nickname
    pub async fn update_member_nickname(
        &self,
        connection_id: ConnectionId,
        nickname: Option<String>,
    ) -> WebSocketResult<()> {
        let mut members = self.members.write().await;
        
        match members.get_mut(&connection_id) {
            Some(member) => {
                member.nickname = nickname;
                debug!("Updated nickname for member {} in channel {}", connection_id, self.id);
                Ok(())
            }
            None => Err(WebSocketError::Connection("Member not found in channel".to_string())),
        }
    }

    /// Add a message to the channel history
    pub async fn add_message(&self, message: ChannelMessage) {
        let mut history = self.message_history.write().await;
        
        // Add the message
        history.push(message);
        
        // Trim history if needed
        if let Some(limit) = self.metadata.message_history_limit {
            if history.len() > limit {
                history.remove(0); // Remove oldest message
            }
        }
    }

    /// Get recent messages from the channel
    pub async fn get_recent_messages(&self, count: usize) -> Vec<ChannelMessage> {
        let history = self.message_history.read().await;
        let start = history.len().saturating_sub(count);
        history[start..].to_vec()
    }

    /// Get all message history
    pub async fn get_message_history(&self) -> Vec<ChannelMessage> {
        let history = self.message_history.read().await;
        history.clone()
    }

    /// Clear message history
    pub async fn clear_message_history(&self) {
        let mut history = self.message_history.write().await;
        history.clear();
        debug!("Cleared message history for channel {}", self.id);
    }

    /// Check if a member has a specific permission
    pub async fn member_has_permission(
        &self,
        connection_id: ConnectionId,
        check: impl Fn(&ChannelPermissions) -> bool,
    ) -> bool {
        if let Some(member) = self.get_member(connection_id).await {
            check(&member.permissions)
        } else {
            false
        }
    }

    /// Validate password for protected channels
    pub fn validate_password(&self, password: &str) -> bool {
        match &self.metadata.channel_type {
            ChannelType::Protected { password_hash } => {
                // In a real implementation, you'd use proper password hashing (bcrypt, argon2, etc.)
                // For now, this is a simple hash comparison
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                
                let mut hasher = DefaultHasher::new();
                password.hash(&mut hasher);
                let hash = hasher.finish().to_string();
                
                hash == *password_hash
            }
            _ => true, // Non-protected channels don't require passwords
        }
    }

    /// Get channel statistics
    pub async fn stats(&self) -> ChannelStats {
        let members = self.members.read().await;
        let history = self.message_history.read().await;
        
        ChannelStats {
            id: self.id,
            name: self.metadata.name.clone(),
            member_count: members.len(),
            message_count: history.len(),
            channel_type: self.metadata.channel_type.clone(),
            created_at: self.metadata.created_at,
            is_empty: members.is_empty(),
        }
    }
}