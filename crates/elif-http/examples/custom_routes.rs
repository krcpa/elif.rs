//! Example: NestJS-like HTTP server with custom routes
//!
//! This example shows how to add custom routes using the clean Server API
//! that completely abstracts Axum complexity away from users.

use elif_core::{Container, container::test_implementations::*};
use elif_http::{Server, HttpConfig, ElifRouter};
use axum::{
    routing::{get, post},
    response::Json,
    extract::Query,
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
    println!("  GET  /health  - Health check");
    println!("  GET  /users   - List users");
    println!("  POST /users   - Create user");
    println!("  GET  /api/v1/status - API status");
    
    // Create custom routes using framework router
    let router = ElifRouter::new()
        .get("/users", list_users)
        .post("/users", create_user)
        .get("/api/v1/status", api_status);
    
    // Create and configure server - NestJS-like experience
    let mut server = Server::with_container(container, http_config)?;
    server.use_router(router);
    
    // Start server - clean and simple
    server.listen("0.0.0.0:3000").await?;

    Ok(())
}

// Clean handler functions - no Axum complexity exposed to users
async fn list_users(Query(params): Query<UserQuery>) -> Json<serde_json::Value> {
    let users = vec![
        User { id: 1, name: "Alice".to_string(), age: 30 },
        User { id: 2, name: "Bob".to_string(), age: 25 },
    ];
    
    Json(json!({
        "users": users,
        "query": params,
        "framework": "Elif.rs - NestJS-like experience"
    }))
}

async fn create_user() -> Json<serde_json::Value> {
    Json(json!({
        "message": "User creation endpoint",
        "note": "This would normally parse request body and create user",
        "framework": "Clean API abstraction"
    }))
}

async fn api_status() -> Json<serde_json::Value> {
    Json(json!({
        "api_version": "v1",
        "status": "operational",
        "framework": "Elif.rs",
        "underlying": "Axum (completely hidden)",
        "experience": "NestJS-like"
    }))
}