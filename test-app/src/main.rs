mod controllers;
mod middleware;
mod models;
mod routes;

use axum::{
    extract::Query,
    http::{header::CONTENT_TYPE, Method, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let app = create_app();
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    println!("🚀 Server running on http://0.0.0.0:3000");
    println!("📖 Add routes with: elif route add GET /path controller_name");
    println!("🔍 Introspection: /_map.json, /_openapi.json, /_health");
    
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE])
        .allow_origin(Any);
    
    Router::new()
        .merge(routes::router())
        // Introspection endpoints
        .route("/_map.json", get(introspection_map))
        .route("/_openapi.json", get(introspection_openapi))
        .route("/_health", get(health_check))
        .layer(cors)
}

// <<<ELIF:BEGIN agent-editable:introspection_map>>>
async fn introspection_map() -> Result<Json<Value>, StatusCode> {
    // TODO: Implement dynamic route discovery
    let map = json!({
        "routes": [
            {
                "method": "GET",
                "path": "/_health",
                "handler": "health_check",
                "file": "src/main.rs"
            },
            {
                "method": "GET", 
                "path": "/_map.json",
                "handler": "introspection_map",
                "file": "src/main.rs"
            },
            {
                "method": "GET",
                "path": "/_openapi.json", 
                "handler": "introspection_openapi",
                "file": "src/main.rs"
            }
        ],
        "models": [],
        "resources": []
    });
    
    Ok(Json(map))
}
// <<<ELIF:END agent-editable:introspection_map>>>

// <<<ELIF:BEGIN agent-editable:introspection_openapi>>>
async fn introspection_openapi() -> Result<Json<Value>, StatusCode> {
    // TODO: Generate OpenAPI spec from routes
    let openapi = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "elif.rs API",
            "version": "0.1.0",
            "description": "Generated with elif.rs framework"
        },
        "servers": [
            {
                "url": "http://localhost:3000",
                "description": "Development server"
            }
        ],
        "paths": {
            "/_health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "status": {"type": "string"},
                                            "timestamp": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    
    Ok(Json(openapi))
}
// <<<ELIF:END agent-editable:introspection_openapi>>>

async fn health_check(_query: Query<HashMap<String, String>>) -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0"
    }))
}
