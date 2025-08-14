//! Example: Simple HTTP server with DI container integration
//!
//! This example demonstrates how to create an HTTP server with proper
//! dependency injection integration without Router<State> issues.

use elif_core::{Container, container::test_implementations::*};
use elif_http::{SimpleStatefulHttpServer, HttpConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize basic tracing (in a real app, use tracing-subscriber)
    // tracing_subscriber::init();

    // Create test container with DI services
    let config = Arc::new(create_test_config());
    let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    
    let container = Container::builder()
        .config(config)
        .database(database)
        .build()?
        .into();

    // Create HTTP configuration
    let http_config = HttpConfig::default();
    
    println!("🚀 Starting HTTP server with DI container integration...");
    println!("📊 Health check available at: http://127.0.0.1:8080{}", http_config.health_check_path);
    println!("🔧 Container services: database, config");
    
    // Create and run the server
    let server = SimpleStatefulHttpServer::new(container, http_config)?;
    server.run().await?;

    Ok(())
}