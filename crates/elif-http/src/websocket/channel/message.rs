//! Channel message types and functionality

use super::super::types::{ConnectionId, WebSocketMessage};
use super::types::ChannelId;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// A message within a channel context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: Uuid,
    pub channel_id: ChannelId,
    pub sender_id: ConnectionId,
    pub content: WebSocketMessage,
    pub timestamp: SystemTime,
    pub sender_nickname: Option<String>,
}

impl ChannelMessage {
    /// Create a new channel message
    pub fn new(
        channel_id: ChannelId,
        sender_id: ConnectionId,
        content: WebSocketMessage,
        sender_nickname: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel_id,
            sender_id,
            content,
            timestamp: SystemTime::now(),
            sender_nickname,
        }
    }
}
