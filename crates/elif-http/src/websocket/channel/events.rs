//! Channel event system

use super::super::types::ConnectionId;
use super::message::ChannelMessage;
use super::types::{ChannelId, ChannelPermissions};

/// Events that can occur in the channel system
#[derive(Debug, Clone)]
pub enum ChannelEvent {
    /// Channel was created
    ChannelCreated(ChannelId, String),
    /// Channel was deleted
    ChannelDeleted(ChannelId, String),
    /// Member joined a channel
    MemberJoined(ChannelId, ConnectionId, Option<String>),
    /// Member left a channel
    MemberLeft(ChannelId, ConnectionId, Option<String>),
    /// Message was sent to a channel
    MessageSent(ChannelId, ChannelMessage),
    /// Member permissions were updated
    PermissionsUpdated(ChannelId, ConnectionId, ChannelPermissions),
}
