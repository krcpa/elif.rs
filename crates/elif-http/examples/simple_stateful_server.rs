//! Example: NestJS-like HTTP server with DI container integration
//!
//! This example demonstrates how to create an HTTP server using the new
//! clean Server API that abstracts away Axum complexity.

use elif_core::{Container, container::test_implementations::*};
use elif_http::{Server, HttpConfig, ElifRouter};
use std::sync::Arc;
use axum::response::Json;
use serde_json::{json, Value};

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
    
    println!("🚀 Starting HTTP server with clean NestJS-like API...");
    println!("📊 Health check available at: http://127.0.0.1:3000{}", http_config.health_check_path);
    println!("🔧 Container services: database, config");
    
    // Create custom routes (optional)
    let router = ElifRouter::new()
        .get("/hello", hello_handler)
        .get("/status", status_handler);
    
    // Create and configure the server with NestJS-like API
    let mut server = Server::with_container(container, http_config)?;
    server.use_router(router);
    
    // Start the server
    server.listen("0.0.0.0:3000").await?;

    Ok(())
}

// Example handlers - clean and simple
async fn hello_handler() -> Json<Value> {
    Json(json!({
        "message": "Hello from Elif.rs!",
        "framework": "NestJS-like API",
        "powered_by": "Axum (hidden)"
    }))
}

async fn status_handler() -> Json<Value> {
    Json(json!({
        "server": "running",
        "api": "clean",
        "complexity": "abstracted"
    }))
}