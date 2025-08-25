//! Channel manager for WebSocket channel operations

use super::super::types::{ConnectionId, WebSocketError, WebSocketMessage, WebSocketResult};
use super::channel::Channel;
use super::events::ChannelEvent;
use super::message::ChannelMessage;
use super::types::{
    ChannelId, ChannelManagerStats, ChannelMetadata, ChannelPermissions, ChannelStats, ChannelType,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// High-performance channel manager for WebSocket channel operations
pub struct ChannelManager {
    /// Active channels
    channels: Arc<RwLock<HashMap<ChannelId, Arc<Channel>>>>,
    /// Connection to channel mapping for quick lookup
    connection_channels: Arc<RwLock<HashMap<ConnectionId, HashSet<ChannelId>>>>,
    /// Event handlers
    event_handlers: Arc<RwLock<Vec<Box<dyn Fn(ChannelEvent) + Send + Sync>>>>,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            connection_channels: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new channel
    pub async fn create_channel(
        &self,
        name: String,
        channel_type: ChannelType,
        created_by: Option<ConnectionId>,
    ) -> WebSocketResult<ChannelId> {
        let channel = Channel::new(name.clone(), channel_type, created_by);
        let channel_id = channel.id;

        // Add creator as admin if specified
        if let Some(creator_id) = created_by {
            channel
                .add_member(creator_id, ChannelPermissions::admin(), None)
                .await?;

            // Track the connection's channel membership
            let mut connection_channels = self.connection_channels.write().await;
            connection_channels
                .entry(creator_id)
                .or_insert_with(HashSet::new)
                .insert(channel_id);
        }

        // Store the channel
        {
            let mut channels = self.channels.write().await;
            channels.insert(channel_id, Arc::new(channel));
        }

        info!("Created channel '{}' with ID {}", name, channel_id);
        self.emit_event(ChannelEvent::ChannelCreated(channel_id, name))
            .await;

        Ok(channel_id)
    }

    /// Create a channel with custom metadata
    pub async fn create_channel_with_metadata(
        &self,
        metadata: ChannelMetadata,
    ) -> WebSocketResult<ChannelId> {
        let channel = Channel::with_metadata(metadata.clone());
        let channel_id = channel.id;

        // Add creator as admin if specified
        if let Some(creator_id) = metadata.created_by {
            channel
                .add_member(creator_id, ChannelPermissions::admin(), None)
                .await?;

            // Track the connection's channel membership
            let mut connection_channels = self.connection_channels.write().await;
            connection_channels
                .entry(creator_id)
                .or_insert_with(HashSet::new)
                .insert(channel_id);
        }

        // Store the channel
        {
            let mut channels = self.channels.write().await;
            channels.insert(channel_id, Arc::new(channel));
        }

        info!("Created channel '{}' with ID {}", metadata.name, channel_id);
        self.emit_event(ChannelEvent::ChannelCreated(channel_id, metadata.name))
            .await;

        Ok(channel_id)
    }

    /// Delete a channel
    pub async fn delete_channel(&self, channel_id: ChannelId) -> WebSocketResult<()> {
        let channel = {
            let mut channels = self.channels.write().await;
            channels.remove(&channel_id)
        };

        if let Some(channel) = channel {
            let channel_name = channel.metadata.name.clone();
            let member_ids = channel.get_member_ids().await;

            // Remove channel from all members' tracking
            if !member_ids.is_empty() {
                let mut connection_channels = self.connection_channels.write().await;
                for member_id in member_ids {
                    if let Some(member_channels) = connection_channels.get_mut(&member_id) {
                        member_channels.remove(&channel_id);
                        if member_channels.is_empty() {
                            connection_channels.remove(&member_id);
                        }
                    }
                }
            }

            info!("Deleted channel '{}' with ID {}", channel_name, channel_id);
            self.emit_event(ChannelEvent::ChannelDeleted(channel_id, channel_name))
                .await;
            Ok(())
        } else {
            Err(WebSocketError::Connection(format!(
                "Channel {} not found",
                channel_id
            )))
        }
    }

    /// Get a channel by ID
    pub async fn get_channel(&self, channel_id: ChannelId) -> Option<Arc<Channel>> {
        let channels = self.channels.read().await;
        channels.get(&channel_id).cloned()
    }

    /// Get a channel by name
    pub async fn get_channel_by_name(&self, name: &str) -> Option<Arc<Channel>> {
        let channel_id = ChannelId::from_name(name);
        self.get_channel(channel_id).await
    }

    /// Get all channels
    pub async fn get_all_channels(&self) -> Vec<Arc<Channel>> {
        let channels = self.channels.read().await;
        channels.values().cloned().collect()
    }

    /// Get channels that a connection is a member of
    pub async fn get_connection_channels(&self, connection_id: ConnectionId) -> Vec<Arc<Channel>> {
        let connection_channels = self.connection_channels.read().await;

        if let Some(channel_ids) = connection_channels.get(&connection_id) {
            let channels = self.channels.read().await;
            channel_ids
                .iter()
                .filter_map(|id| channels.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Join a connection to a channel
    pub async fn join_channel(
        &self,
        channel_id: ChannelId,
        connection_id: ConnectionId,
        password: Option<&str>,
        nickname: Option<String>,
    ) -> WebSocketResult<()> {
        let channel = self
            .get_channel(channel_id)
            .await
            .ok_or(WebSocketError::Connection(format!(
                "Channel {} not found",
                channel_id
            )))?;

        // Check access permissions
        match &channel.metadata.channel_type {
            ChannelType::Public => {
                // Anyone can join public channels
            }
            ChannelType::Private => {
                // For private channels, we'd need invitation logic
                // For now, reject all attempts
                return Err(WebSocketError::Connection(
                    "Channel is private and requires invitation".to_string(),
                ));
            }
            ChannelType::Protected { .. } => {
                let provided_password = password.ok_or(WebSocketError::Connection(
                    "Password required for protected channel".to_string(),
                ))?;

                if !channel.validate_password(provided_password) {
                    return Err(WebSocketError::Connection("Invalid password".to_string()));
                }
            }
        }

        // Add member to channel
        let permissions = if Some(connection_id) == channel.metadata.created_by {
            ChannelPermissions::admin()
        } else {
            ChannelPermissions::default()
        };

        channel
            .add_member(connection_id, permissions, nickname.clone())
            .await?;

        // Track the connection's channel membership
        {
            let mut connection_channels = self.connection_channels.write().await;
            connection_channels
                .entry(connection_id)
                .or_insert_with(HashSet::new)
                .insert(channel_id);
        }

        info!("Connection {} joined channel {}", connection_id, channel_id);
        self.emit_event(ChannelEvent::MemberJoined(
            channel_id,
            connection_id,
            nickname,
        ))
        .await;

        Ok(())
    }

    /// Remove a connection from a channel
    pub async fn leave_channel(
        &self,
        channel_id: ChannelId,
        connection_id: ConnectionId,
    ) -> WebSocketResult<()> {
        let channel = self
            .get_channel(channel_id)
            .await
            .ok_or(WebSocketError::Connection(format!(
                "Channel {} not found",
                channel_id
            )))?;

        // Get member info before removal
        let member = channel.get_member(connection_id).await;
        let nickname = member.as_ref().and_then(|m| m.nickname.clone());

        // Remove member from channel
        channel
            .remove_member(connection_id)
            .await
            .ok_or(WebSocketError::Connection(
                "Connection not a member of channel".to_string(),
            ))?;

        // Remove from connection tracking
        {
            let mut connection_channels = self.connection_channels.write().await;
            if let Some(member_channels) = connection_channels.get_mut(&connection_id) {
                member_channels.remove(&channel_id);
                if member_channels.is_empty() {
                    connection_channels.remove(&connection_id);
                }
            }
        }

        info!("Connection {} left channel {}", connection_id, channel_id);
        self.emit_event(ChannelEvent::MemberLeft(
            channel_id,
            connection_id,
            nickname,
        ))
        .await;

        // Auto-delete empty channels (except those with explicit creators)
        if channel.is_empty().await && channel.metadata.created_by.is_none() {
            self.delete_channel(channel_id).await?;
        }

        Ok(())
    }

    /// Remove a connection from all channels (useful for cleanup on disconnect)
    pub async fn leave_all_channels(&self, connection_id: ConnectionId) -> Vec<ChannelId> {
        // Acquire write lock once and remove all channel entries for this connection
        let channel_ids = {
            let mut connection_channels = self.connection_channels.write().await;
            connection_channels
                .remove(&connection_id)
                .unwrap_or_default()
        };

        let mut left_channels = Vec::new();

        // Now handle cleanup for each channel without repeated lock acquisitions
        for channel_id in channel_ids {
            if let Some(channel) = self.get_channel(channel_id).await {
                // Get member info before removal for event logging
                let member = channel.get_member(connection_id).await;
                let nickname = member.as_ref().and_then(|m| m.nickname.clone());

                // Remove member from channel
                if channel.remove_member(connection_id).await.is_some() {
                    left_channels.push(channel_id);

                    info!("Connection {} left channel {}", connection_id, channel_id);
                    self.emit_event(ChannelEvent::MemberLeft(
                        channel_id,
                        connection_id,
                        nickname,
                    ))
                    .await;

                    // Auto-delete empty channels (except those with explicit creators)
                    if channel.is_empty().await && channel.metadata.created_by.is_none() {
                        let _ = self.delete_channel(channel_id).await;
                    }
                }
            }
        }

        if !left_channels.is_empty() {
            info!(
                "Connection {} left {} channels",
                connection_id,
                left_channels.len()
            );
        }

        left_channels
    }

    /// Send a message to a channel
    pub async fn send_to_channel(
        &self,
        channel_id: ChannelId,
        sender_id: ConnectionId,
        message: WebSocketMessage,
    ) -> WebSocketResult<Vec<ConnectionId>> {
        let channel = self
            .get_channel(channel_id)
            .await
            .ok_or(WebSocketError::Connection(format!(
                "Channel {} not found",
                channel_id
            )))?;

        // Check if sender is a member and has permission to send messages
        let sender_member =
            channel
                .get_member(sender_id)
                .await
                .ok_or(WebSocketError::Connection(
                    "Sender not a member of channel".to_string(),
                ))?;

        if !sender_member.permissions.can_send_messages {
            return Err(WebSocketError::Connection(
                "No permission to send messages".to_string(),
            ));
        }

        // Create channel message
        let channel_message = ChannelMessage::new(
            channel_id,
            sender_id,
            message.clone(),
            sender_member.nickname.clone(),
        );

        // Add to channel history
        channel.add_message(channel_message.clone()).await;

        // Get all member IDs for broadcasting
        let member_ids = channel.get_member_ids().await;

        info!(
            "Message sent to channel {} by {} (broadcasting to {} members)",
            channel_id,
            sender_id,
            member_ids.len()
        );

        self.emit_event(ChannelEvent::MessageSent(channel_id, channel_message))
            .await;

        Ok(member_ids)
    }

    /// Get channel statistics for all channels
    pub async fn get_all_channel_stats(&self) -> Vec<ChannelStats> {
        let channels = self.channels.read().await;
        let mut stats = Vec::with_capacity(channels.len());

        for channel in channels.values() {
            stats.push(channel.stats().await);
        }

        stats
    }

    /// Get public channels for discovery
    pub async fn get_public_channels(&self) -> Vec<ChannelStats> {
        let channels = self.channels.read().await;
        let mut public_channels = Vec::new();

        for channel in channels.values() {
            if matches!(channel.metadata.channel_type, ChannelType::Public) {
                public_channels.push(channel.stats().await);
            }
        }

        public_channels
    }

    /// Get manager statistics
    pub async fn stats(&self) -> ChannelManagerStats {
        let channels = self.channels.read().await;
        let connection_channels = self.connection_channels.read().await;

        let mut stats = ChannelManagerStats {
            total_channels: channels.len(),
            total_connections_in_channels: connection_channels.len(),
            public_channels: 0,
            private_channels: 0,
            protected_channels: 0,
            empty_channels: 0,
        };

        for channel in channels.values() {
            match channel.metadata.channel_type {
                ChannelType::Public => stats.public_channels += 1,
                ChannelType::Private => stats.private_channels += 1,
                ChannelType::Protected { .. } => stats.protected_channels += 1,
            }

            if channel.is_empty().await {
                stats.empty_channels += 1;
            }
        }

        stats
    }

    /// Clean up empty channels
    pub async fn cleanup_empty_channels(&self) -> usize {
        let channels = self.get_all_channels().await;
        let mut cleaned_up = 0;

        for channel in channels {
            // Only auto-delete channels without explicit creators
            if channel.is_empty().await
                && channel.metadata.created_by.is_none()
                && self.delete_channel(channel.id).await.is_ok()
            {
                cleaned_up += 1;
            }
        }

        if cleaned_up > 0 {
            info!("Cleaned up {} empty channels", cleaned_up);
        }

        cleaned_up
    }

    /// Add an event handler
    pub async fn add_event_handler<F>(&self, handler: F)
    where
        F: Fn(ChannelEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(Box::new(handler));
    }

    /// Emit an event to all handlers
    async fn emit_event(&self, event: ChannelEvent) {
        let handlers = self.event_handlers.read().await;
        for handler in handlers.iter() {
            handler(event.clone());
        }
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
