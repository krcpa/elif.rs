//! Example: HTTP server with custom routes and middleware
//!
//! This example shows how to add custom routes, middleware, and different
//! response types to your HTTP server with DI container integration.

use elif_core::{Container, container::test_implementations::*};
use elif_http::{SimpleStatefulHttpServer, HttpConfig};
use axum::{
    Router, 
    routing::{get, post},
    response::Json,
    http::{StatusCode, HeaderMap},
    extract::Query,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Deserialize)]
struct UserQuery {
    name: Option<String>,
    age: Option<u32>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
    timestamp: String,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Custom HTTP server with additional routes
pub struct CustomRoutesServer {
    router: Router,
    addr: std::net::SocketAddr,
}

impl CustomRoutesServer {
    pub fn new(container: Arc<Container>, config: HttpConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let app_config = container.config();
        let addr = format!("{}:{}", app_config.server.host, app_config.server.port)
            .parse()?;

        // Create handlers with DI container access
        let health_container = container.clone();
        let health_config = config.clone();
        let health_handler = move || {
            let container = health_container.clone();
            let config = health_config.clone();
            async move { health_with_headers(container, config).await }
        };

        let users_container = container.clone();
        let users_handler = move |query: Query<UserQuery>| {
            let container = users_container.clone();
            async move { list_users(container, query).await }
        };

        let create_container = container.clone();
        let create_handler = move || {
            let container = create_container.clone();
            async move { create_user(container).await }
        };

        let stats_container = container.clone();
        let stats_handler = move || {
            let container = stats_container.clone();
            async move { server_stats(container).await }
        };

        // Build router with custom routes
        let router = Router::new()
            .route(&config.health_check_path, get(health_handler))
            .route("/api/users", get(users_handler))
            .route("/api/users", post(create_handler))
            .route("/api/stats", get(stats_handler))
            .route("/api/version", get(version_info))
            .route("/api/time", get(current_time));

        Ok(Self { router, addr })
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting custom routes HTTP server on {}", self.addr);
        println!("📋 Available endpoints:");
        println!("   GET  /health     - Health check with custom headers");
        println!("   GET  /api/users  - List users (supports ?name=X&age=Y)");
        println!("   POST /api/users  - Create user");
        println!("   GET  /api/stats  - Server statistics");
        println!("   GET  /api/version - Version information");
        println!("   GET  /api/time   - Current server time");

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

/// Health check with custom HTTP headers
async fn health_with_headers(
    container: Arc<Container>, 
    config: HttpConfig
) -> (StatusCode, HeaderMap, Json<ApiResponse<serde_json::Value>>) {
    let mut headers = HeaderMap::new();
    headers.insert("X-Service", "elif-rs".parse().unwrap());
    headers.insert("X-Version", "0.1.0".parse().unwrap());
    
    let database = container.database();
    let db_healthy = database.is_connected();
    
    let status = if db_healthy { 
        StatusCode::OK 
    } else { 
        StatusCode::SERVICE_UNAVAILABLE 
    };

    let app_config = container.config();
    let data = json!({
        "status": if db_healthy { "healthy" } else { "degraded" },
        "environment": format!("{:?}", app_config.environment),
        "services": {
            "database": if db_healthy { "healthy" } else { "unhealthy" },
            "container": "healthy"
        },
        "config": {
            "request_timeout": config.request_timeout_secs,
            "tracing_enabled": config.enable_tracing
        }
    });

    (status, headers, Json(ApiResponse::success(data)))
}

/// List users with query parameters
async fn list_users(
    container: Arc<Container>,
    Query(params): Query<UserQuery>
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let app_config = container.config();
    
    // Simulate user data based on query parameters
    let mut users = vec![
        json!({
            "id": 1,
            "name": "Alice",
            "age": 30,
            "environment": format!("{:?}", app_config.environment)
        }),
        json!({
            "id": 2,
            "name": "Bob", 
            "age": 25,
            "environment": format!("{:?}", app_config.environment)
        }),
    ];

    // Filter by name if provided
    if let Some(name) = params.name {
        users = users.into_iter()
            .filter(|user| {
                user.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.to_lowercase().contains(&name.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect();
    }

    // Filter by age if provided
    if let Some(age) = params.age {
        users = users.into_iter()
            .filter(|user| {
                user.get("age")
                    .and_then(|a| a.as_u64())
                    .map(|a| a as u32 == age)
                    .unwrap_or(false)
            })
            .collect();
    }

    Json(ApiResponse::success(users))
}

/// Create a new user (POST endpoint)
async fn create_user(container: Arc<Container>) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let database = container.database();
    
    if !database.is_connected() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::error("Database not available".to_string()))
        );
    }

    // Simulate user creation
    let new_user = json!({
        "id": 3,
        "name": "Charlie",
        "age": 28,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "status": "created"
    });

    (StatusCode::CREATED, Json(ApiResponse::success(new_user)))
}

/// Server statistics endpoint
async fn server_stats(container: Arc<Container>) -> Json<ApiResponse<serde_json::Value>> {
    let app_config = container.config();
    let database = container.database();
    
    let stats = json!({
        "server": {
            "name": app_config.name,
            "environment": format!("{:?}", app_config.environment),
            "host": app_config.server.host,
            "port": app_config.server.port,
            "workers": app_config.server.workers
        },
        "services": {
            "database": {
                "connected": database.is_connected(),
                "url": app_config.database_url
            }
        },
        "runtime": {
            "uptime_seconds": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() % 86400, // Simplified uptime calculation
            "memory_usage": "N/A" // Would use actual memory stats in real app
        }
    });

    Json(ApiResponse::success(stats))
}

/// Simple version endpoint (no DI needed)
async fn version_info() -> Json<ApiResponse<serde_json::Value>> {
    let version = json!({
        "framework": "elif.rs",
        "version": "0.1.0",
        "build": "development",
        "features": [
            "dependency-injection",
            "http-server", 
            "database-integration",
            "middleware-support"
        ]
    });

    Json(ApiResponse::success(version))
}

/// Current time endpoint (no DI needed)
async fn current_time() -> Json<ApiResponse<serde_json::Value>> {
    let time_info = json!({
        "utc": chrono::Utc::now().to_rfc3339(),
        "unix_timestamp": chrono::Utc::now().timestamp(),
        "timezone": "UTC",
        "format": "RFC3339"
    });

    Json(ApiResponse::success(time_info))
}

/// Graceful shutdown handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create DI container
    let config = Arc::new(create_test_config());
    let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    
    let container = Container::builder()
        .config(config)
        .database(database)
        .build()?
        .into();

    // Create and run server
    let server = CustomRoutesServer::new(container, HttpConfig::default())?;
    server.run().await?;

    Ok(())
}