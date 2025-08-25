#[cfg(test)]
mod tests {
    use crate::websocket::{
        Channel, ChannelId, ChannelManager, ChannelMessage, ChannelMetadata, ChannelPermissions,
        ChannelType, ConnectionId, WebSocketMessage,
    };
    use std::collections::HashSet;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_channel_creation() {
        let channel = Channel::new("test-channel".to_string(), ChannelType::Public, None);

        assert_eq!(channel.metadata.name, "test-channel");
        assert!(matches!(channel.metadata.channel_type, ChannelType::Public));
        assert_eq!(channel.member_count().await, 0);
        assert!(channel.is_empty().await);
    }

    #[tokio::test]
    async fn test_channel_id_deterministic() {
        let id1 = ChannelId::from_name("test-channel");
        let id2 = ChannelId::from_name("test-channel");
        assert_eq!(id1, id2);

        let id3 = ChannelId::from_name("different-channel");
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn test_add_remove_members() {
        let channel = Channel::new("test-channel".to_string(), ChannelType::Public, None);

        let connection_id = ConnectionId::new();

        // Add member
        let result = channel
            .add_member(
                connection_id,
                ChannelPermissions::default(),
                Some("test-user".to_string()),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(channel.member_count().await, 1);
        assert!(channel.has_member(connection_id).await);

        // Get member
        let member = channel.get_member(connection_id).await.unwrap();
        assert_eq!(member.connection_id, connection_id);
        assert_eq!(member.nickname, Some("test-user".to_string()));

        // Remove member
        let removed = channel.remove_member(connection_id).await;
        assert!(removed.is_some());
        assert_eq!(channel.member_count().await, 0);
        assert!(!channel.has_member(connection_id).await);
    }

    #[tokio::test]
    async fn test_channel_capacity() {
        let metadata = ChannelMetadata {
            name: "limited-channel".to_string(),
            description: None,
            created_at: SystemTime::now(),
            created_by: None,
            channel_type: ChannelType::Public,
            max_members: Some(2),
            message_history_limit: None,
        };

        let channel = Channel::with_metadata(metadata);

        // Add two members (should succeed)
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let id3 = ConnectionId::new();

        assert!(channel
            .add_member(id1, ChannelPermissions::default(), None)
            .await
            .is_ok());
        assert!(channel
            .add_member(id2, ChannelPermissions::default(), None)
            .await
            .is_ok());

        // Try to add third member (should fail)
        assert!(channel
            .add_member(id3, ChannelPermissions::default(), None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_message_history() {
        let channel = Channel::new("test-channel".to_string(), ChannelType::Public, None);

        let sender_id = ConnectionId::new();
        let message1 = ChannelMessage {
            id: Uuid::new_v4(),
            channel_id: channel.id,
            sender_id,
            content: WebSocketMessage::text("Hello"),
            timestamp: SystemTime::now(),
            sender_nickname: None,
        };

        let message2 = ChannelMessage {
            id: Uuid::new_v4(),
            channel_id: channel.id,
            sender_id,
            content: WebSocketMessage::text("World"),
            timestamp: SystemTime::now(),
            sender_nickname: None,
        };

        channel.add_message(message1.clone()).await;
        channel.add_message(message2.clone()).await;

        let history = channel.get_message_history().await;
        assert_eq!(history.len(), 2);

        let recent = channel.get_recent_messages(1).await;
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, message2.id);
    }

    #[tokio::test]
    async fn test_channel_manager_creation() {
        let manager = ChannelManager::new();
        let stats = manager.stats().await;
        assert_eq!(stats.total_channels, 0);

        // Create a channel
        let channel_id = manager
            .create_channel("test-channel".to_string(), ChannelType::Public, None)
            .await
            .unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.total_channels, 1);
        assert_eq!(stats.public_channels, 1);

        // Get channel
        let channel = manager.get_channel(channel_id).await.unwrap();
        assert_eq!(channel.metadata.name, "test-channel");
    }

    #[tokio::test]
    async fn test_channel_manager_join_leave() {
        let manager = ChannelManager::new();
        let connection_id = ConnectionId::new();

        // Create a public channel
        let channel_id = manager
            .create_channel("public-test".to_string(), ChannelType::Public, None)
            .await
            .unwrap();

        // Join channel
        let result = manager
            .join_channel(
                channel_id,
                connection_id,
                None,
                Some("test-user".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // Verify membership
        let channel = manager.get_channel(channel_id).await.unwrap();
        assert!(channel.has_member(connection_id).await);
        assert_eq!(channel.member_count().await, 1);

        // Leave channel
        let result = manager.leave_channel(channel_id, connection_id).await;
        assert!(result.is_ok());

        // Channel should be auto-deleted when empty since it has no explicit creator
        let channel = manager.get_channel(channel_id).await;
        assert!(channel.is_none());
    }

    #[tokio::test]
    async fn test_protected_channel() {
        let manager = ChannelManager::new();
        let connection_id = ConnectionId::new();

        // Create protected channel with secure password hashing
        let channel_type = ChannelType::protected_with_password("secret123").unwrap();
        let channel_id = manager
            .create_channel("protected-test".to_string(), channel_type, None)
            .await
            .unwrap();

        // Try to join without password (should fail)
        let result = manager
            .join_channel(channel_id, connection_id, None, None)
            .await;
        assert!(result.is_err());

        // Try with wrong password (should fail)
        let result = manager
            .join_channel(channel_id, connection_id, Some("wrong-password"), None)
            .await;
        assert!(result.is_err());

        // Try with correct password (should succeed)
        let result = manager
            .join_channel(channel_id, connection_id, Some("secret123"), None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_channel_message_sending() {
        let manager = ChannelManager::new();
        let sender_id = ConnectionId::new();
        let receiver_id = ConnectionId::new();

        // Create channel and add members
        let channel_id = manager
            .create_channel(
                "message-test".to_string(),
                ChannelType::Public,
                Some(sender_id),
            )
            .await
            .unwrap();

        // Join second member
        manager
            .join_channel(channel_id, receiver_id, None, None)
            .await
            .unwrap();

        // Send message
        let message = WebSocketMessage::text("Hello channel!");
        let result = manager
            .send_to_channel(channel_id, sender_id, message)
            .await;
        assert!(result.is_ok());

        let member_ids = result.unwrap();
        assert_eq!(member_ids.len(), 2); // sender and receiver
        assert!(member_ids.contains(&sender_id));
        assert!(member_ids.contains(&receiver_id));

        // Verify message in history
        let channel = manager.get_channel(channel_id).await.unwrap();
        let history = channel.get_message_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].sender_id, sender_id);
    }

    #[tokio::test]
    async fn test_channel_permissions() {
        let manager = ChannelManager::new();
        let admin_id = ConnectionId::new();
        let regular_id = ConnectionId::new();

        // Create channel with admin
        let channel_id = manager
            .create_channel(
                "permission-test".to_string(),
                ChannelType::Public,
                Some(admin_id),
            )
            .await
            .unwrap();

        // Join regular user
        manager
            .join_channel(channel_id, regular_id, None, None)
            .await
            .unwrap();

        // Get channel and update regular user permissions (remove send permission)
        let channel = manager.get_channel(channel_id).await.unwrap();
        let mut permissions = ChannelPermissions::default();
        permissions.can_send_messages = false;
        channel
            .update_member_permissions(regular_id, permissions)
            .await
            .unwrap();

        // Try to send message as regular user (should fail)
        let message = WebSocketMessage::text("This should fail");
        let result = manager
            .send_to_channel(channel_id, regular_id, message)
            .await;
        assert!(result.is_err());

        // Send as admin (should succeed)
        let message = WebSocketMessage::text("Admin message");
        let result = manager.send_to_channel(channel_id, admin_id, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_channel_cleanup() {
        let manager = ChannelManager::new();
        let connection_id = ConnectionId::new();

        // Create channel without explicit creator (should be auto-deleted when empty)
        let channel_id = manager
            .create_channel("cleanup-test".to_string(), ChannelType::Public, None)
            .await
            .unwrap();

        // Join and then leave
        manager
            .join_channel(channel_id, connection_id, None, None)
            .await
            .unwrap();
        manager
            .leave_channel(channel_id, connection_id)
            .await
            .unwrap();

        // Channel should be automatically deleted
        let channel = manager.get_channel(channel_id).await;
        assert!(channel.is_none());
    }

    #[tokio::test]
    async fn test_leave_all_channels() {
        let manager = ChannelManager::new();
        let connection_id = ConnectionId::new();

        // Create multiple channels
        let channel1_id = manager
            .create_channel("channel1".to_string(), ChannelType::Public, None)
            .await
            .unwrap();
        let channel2_id = manager
            .create_channel("channel2".to_string(), ChannelType::Public, None)
            .await
            .unwrap();
        let channel3_id = manager
            .create_channel("channel3".to_string(), ChannelType::Public, None)
            .await
            .unwrap();

        // Join all channels
        manager
            .join_channel(channel1_id, connection_id, None, None)
            .await
            .unwrap();
        manager
            .join_channel(channel2_id, connection_id, None, None)
            .await
            .unwrap();
        manager
            .join_channel(channel3_id, connection_id, None, None)
            .await
            .unwrap();

        // Verify membership
        let user_channels = manager.get_connection_channels(connection_id).await;
        assert_eq!(user_channels.len(), 3);

        // Leave all channels
        let left_channels = manager.leave_all_channels(connection_id).await;
        assert_eq!(left_channels.len(), 3);

        // Verify no membership
        let user_channels = manager.get_connection_channels(connection_id).await;
        assert_eq!(user_channels.len(), 0);
    }

    #[tokio::test]
    async fn test_channel_discovery() {
        let manager = ChannelManager::new();

        // Create mix of channel types
        manager
            .create_channel("public1".to_string(), ChannelType::Public, None)
            .await
            .unwrap();
        manager
            .create_channel("public2".to_string(), ChannelType::Public, None)
            .await
            .unwrap();
        manager
            .create_channel("private1".to_string(), ChannelType::Private, None)
            .await
            .unwrap();

        // Get public channels
        let public_channels = manager.get_public_channels().await;
        assert_eq!(public_channels.len(), 2);

        let public_names: HashSet<String> =
            public_channels.iter().map(|c| c.name.clone()).collect();
        assert!(public_names.contains("public1"));
        assert!(public_names.contains("public2"));
        assert!(!public_names.contains("private1"));
    }

    #[tokio::test]
    async fn test_channel_stats() {
        let manager = ChannelManager::new();
        let connection_id = ConnectionId::new();

        let channel_id = manager
            .create_channel(
                "stats-test".to_string(),
                ChannelType::Public,
                Some(connection_id),
            )
            .await
            .unwrap();

        // Send some messages
        let message1 = WebSocketMessage::text("Message 1");
        let message2 = WebSocketMessage::text("Message 2");
        manager
            .send_to_channel(channel_id, connection_id, message1)
            .await
            .unwrap();
        manager
            .send_to_channel(channel_id, connection_id, message2)
            .await
            .unwrap();

        // Check channel stats
        let channel = manager.get_channel(channel_id).await.unwrap();
        let stats = channel.stats().await;

        assert_eq!(stats.name, "stats-test");
        assert_eq!(stats.member_count, 1);
        assert_eq!(stats.message_count, 2);
        assert!(matches!(stats.channel_type, ChannelType::Public));
        assert!(!stats.is_empty);

        // Check manager stats
        let manager_stats = manager.stats().await;
        assert_eq!(manager_stats.total_channels, 1);
        assert_eq!(manager_stats.total_connections_in_channels, 1);
        assert_eq!(manager_stats.public_channels, 1);
        assert_eq!(manager_stats.private_channels, 0);
        assert_eq!(manager_stats.protected_channels, 0);
    }
}
