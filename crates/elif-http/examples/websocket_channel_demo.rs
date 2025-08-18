//! WebSocket Channel System Demo
//!
//! This example demonstrates the channel abstraction system for WebSocket messaging
//! including join/leave functionality, room concepts, and message broadcasting.

use elif_http::{
    websocket::{
        WebSocketMessage, ConnectionId, ChannelType, ChannelPermissions,
        ConnectionRegistry,
    },
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize basic logging
    tracing_subscriber::fmt::init();

    println!("🚀 WebSocket Channel System Demo");
    println!("This demonstrates the channel abstraction and messaging system.");
    println!();

    // Create connection registry and channel manager
    let registry = Arc::new(ConnectionRegistry::new());
    let channel_manager = registry.channel_manager();

    // Simulate some connections
    let alice_id = ConnectionId::new();
    let bob_id = ConnectionId::new();
    let charlie_id = ConnectionId::new();
    
    println!("👥 Simulated Connections:");
    println!("   • Alice: {}", alice_id);
    println!("   • Bob: {}", bob_id);
    println!("   • Charlie: {}", charlie_id);
    println!();

    // Demo 1: Create and manage channels
    println!("📺 Demo 1: Channel Creation and Management");
    
    // Create a public channel
    let general_id = channel_manager.create_channel(
        "general".to_string(),
        ChannelType::Public,
        None,
    ).await.unwrap();
    println!("   ✅ Created public channel 'general': {}", general_id);

    // Create a private channel with Alice as admin
    let private_id = channel_manager.create_channel(
        "team-leads".to_string(),
        ChannelType::Private,
        Some(alice_id),
    ).await.unwrap();
    println!("   ✅ Created private channel 'team-leads': {}", private_id);

    // Create a protected channel with secure password hashing
    let protected_channel_type = ChannelType::protected_with_password("secret123").unwrap();
    let protected_id = channel_manager.create_channel(
        "secret-project".to_string(),
        protected_channel_type,
        Some(alice_id),
    ).await.unwrap();
    println!("   ✅ Created protected channel 'secret-project': {}", protected_id);
    println!();

    // Demo 2: Join channels
    println!("🚪 Demo 2: Joining Channels");
    
    // Alice joins general channel
    channel_manager.join_channel(
        general_id,
        alice_id,
        None,
        Some("Alice".to_string()),
    ).await.unwrap();
    println!("   ✅ Alice joined 'general' channel");

    // Bob joins general channel
    channel_manager.join_channel(
        general_id,
        bob_id,
        None,
        Some("Bob".to_string()),
    ).await.unwrap();
    println!("   ✅ Bob joined 'general' channel");

    // Charlie tries to join private channel (should fail)
    match channel_manager.join_channel(
        private_id,
        charlie_id,
        None,
        Some("Charlie".to_string()),
    ).await {
        Ok(_) => println!("   ❌ Charlie joined private channel (unexpected)"),
        Err(_) => println!("   ✅ Charlie blocked from private channel (expected)"),
    }

    // Alice joins protected channel with password
    channel_manager.join_channel(
        protected_id,
        alice_id,
        Some("secret123"),
        Some("Alice".to_string()),
    ).await.unwrap();
    println!("   ✅ Alice joined protected channel with password");

    // Bob tries to join protected channel without password
    match channel_manager.join_channel(
        protected_id,
        bob_id,
        None,
        Some("Bob".to_string()),
    ).await {
        Ok(_) => println!("   ❌ Bob joined protected channel without password (unexpected)"),
        Err(_) => println!("   ✅ Bob blocked from protected channel (no password)"),
    }
    println!();

    // Demo 3: Channel discovery
    println!("🔍 Demo 3: Channel Discovery");
    
    let public_channels = channel_manager.get_public_channels().await;
    println!("   📋 Public channels:");
    for channel in &public_channels {
        println!("      • {}: {} members", channel.name, channel.member_count);
    }

    let all_stats = channel_manager.get_all_channel_stats().await;
    println!("   📋 All channels: {} total", all_stats.len());
    println!();

    // Demo 4: Message broadcasting
    println!("💬 Demo 4: Message Broadcasting");
    
    // Alice sends message to general channel
    let message_result = channel_manager.send_to_channel(
        general_id,
        alice_id,
        WebSocketMessage::text("Hello everyone in general!"),
    ).await.unwrap();
    println!("   ✅ Alice sent message to {} members in general channel", message_result.len());

    // Bob sends message to general channel
    let message_result = channel_manager.send_to_channel(
        general_id,
        bob_id,
        WebSocketMessage::text("Hi Alice! Great to be here."),
    ).await.unwrap();
    println!("   ✅ Bob sent message to {} members in general channel", message_result.len());

    // Check message history
    if let Some(general_channel) = channel_manager.get_channel(general_id).await {
        let history = general_channel.get_message_history().await;
        println!("   📜 General channel has {} messages in history", history.len());
    }
    println!();

    // Demo 5: Permission management
    println!("🔐 Demo 5: Permission Management");
    
    if let Some(general_channel) = channel_manager.get_channel(general_id).await {
        // Update Bob's permissions to remove message sending
        let mut restricted_perms = ChannelPermissions::default();
        restricted_perms.can_send_messages = false;
        
        general_channel.update_member_permissions(bob_id, restricted_perms).await.unwrap();
        println!("   ✅ Removed Bob's message sending permission");

        // Bob tries to send a message (should fail)
        match channel_manager.send_to_channel(
            general_id,
            bob_id,
            WebSocketMessage::text("This should fail"),
        ).await {
            Ok(_) => println!("   ❌ Bob sent message without permission (unexpected)"),
            Err(_) => println!("   ✅ Bob blocked from sending message (no permission)"),
        }

        // Restore Bob's permissions
        general_channel.update_member_permissions(
            bob_id, 
            ChannelPermissions::default()
        ).await.unwrap();
        println!("   ✅ Restored Bob's message sending permission");
    }
    println!();

    // Demo 6: Channel statistics
    println!("📊 Demo 6: Channel Statistics");
    
    let manager_stats = channel_manager.stats().await;
    println!("   📈 Manager Statistics:");
    println!("      • Total channels: {}", manager_stats.total_channels);
    println!("      • Public channels: {}", manager_stats.public_channels);
    println!("      • Private channels: {}", manager_stats.private_channels);
    println!("      • Protected channels: {}", manager_stats.protected_channels);
    println!("      • Total connections in channels: {}", manager_stats.total_connections_in_channels);

    // Display individual channel stats
    let all_stats = channel_manager.get_all_channel_stats().await;
    println!("   🏠 Individual Channel Stats:");
    for stats in &all_stats {
        println!("      • '{}': {} members, {} messages", 
                 stats.name, stats.member_count, stats.message_count);
    }
    println!();

    // Demo 7: Leave channels and cleanup
    println!("🚪 Demo 7: Leave Channels and Cleanup");
    
    // Bob leaves general channel
    channel_manager.leave_channel(general_id, bob_id).await.unwrap();
    println!("   ✅ Bob left general channel");

    // Alice leaves all channels
    let left_channels = channel_manager.leave_all_channels(alice_id).await;
    println!("   ✅ Alice left {} channels", left_channels.len());

    // Show updated stats
    let updated_stats = channel_manager.stats().await;
    println!("   📈 Updated stats: {} total channels, {} connections", 
             updated_stats.total_channels, 
             updated_stats.total_connections_in_channels);

    // Clean up empty channels
    let cleaned = channel_manager.cleanup_empty_channels().await;
    println!("   🧹 Cleaned up {} empty channels", cleaned);
    println!();

    // Final summary
    println!("✨ Channel System Features Demonstrated:");
    println!("   ✅ Channel creation (public/private/protected)");
    println!("   ✅ Join/leave functionality with access control");
    println!("   ✅ Password-protected channels");
    println!("   ✅ Message broadcasting to channel members");
    println!("   ✅ Permission system (send messages, invite, kick, admin)");
    println!("   ✅ Channel discovery and listing");
    println!("   ✅ Message history tracking");
    println!("   ✅ Channel statistics and monitoring");
    println!("   ✅ Automatic cleanup of empty channels");
    println!("   ✅ Member management and tracking");
    
    println!();
    println!("🎯 Channel System Ready for Production Integration!");
    println!("   The channel abstraction provides a solid foundation for");
    println!("   building real-time messaging applications with rooms,");
    println!("   broadcasting, and access control.");
}