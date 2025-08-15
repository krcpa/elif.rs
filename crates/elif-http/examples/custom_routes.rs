//! Example: Framework-native HTTP server with custom routes
//!
//! This example shows how to add custom routes using pure framework abstractions
//! without exposing any underlying web framework implementation details.

use elif_core::{Container, container::test_implementations::*};
use elif_http::{
    Server, HttpConfig, ElifRouter, 
    ElifRequest, ElifResponse, ElifQuery,
    StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize, Serialize)]
struct UserQuery {
    name: Option<String>,
    age: Option<u32>,
}

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
    age: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    
    println!("🚀 Starting HTTP server with custom routes...");
    println!("📊 Available endpoints:");
    println!("  GET  /health  - Health check (built-in)");
    println!("  GET  /users   - List users");
    println!("  POST /users   - Create user");
    println!("  GET  /api/v1/status - API status");
    println!("🔧 Framework: Pure Elif.rs abstractions (no direct web framework usage)");
    
    // Create custom routes using framework router - pure abstractions
    let router = ElifRouter::new()
        .get("/users", list_users)
        .post("/users", create_user)
        .get("/api/v1/status", api_status);
    
    // Create and configure server - framework-native experience
    let mut server = Server::with_container(container, http_config)?;
    server.use_router(router);
    
    // Start server - framework abstractions only
    server.listen("0.0.0.0:3000").await?;

    Ok(())
}

// Clean handler functions using pure framework abstractions
async fn list_users(params: UserQuery) -> ElifResponse {
    let users = vec![
        User { id: 1, name: "Alice".to_string(), age: 30 },
        User { id: 2, name: "Bob".to_string(), age: 25 },
    ];
    
    ElifResponse::json(json!({
        "users": users,
        "query": params,
        "framework": "Elif.rs - Pure framework abstractions",
        "note": "No underlying web framework types exposed"
    })).with_status(StatusCode::OK)
}

async fn create_user() -> ElifResponse {
    ElifResponse::json(json!({
        "message": "User creation endpoint",
        "note": "Framework-native handler with no external dependencies",
        "framework": "Pure Elif.rs abstractions"
    })).with_status(StatusCode::CREATED)
}

async fn api_status() -> ElifResponse {
    ElifResponse::json(json!({
        "api_version": "v1",
        "status": "operational",
        "framework": "Elif.rs",
        "architecture": "Pure framework abstractions",
        "experience": "Framework-native development"
    })).with_status(StatusCode::OK)
}