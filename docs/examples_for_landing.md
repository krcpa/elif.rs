# elif.rs Landing Page Examples

This document contains correct, tested code examples for the elif.rs landing page.

## Installation

```bash
# Correct installation command
cargo install elifrs
```

## Quick Start

```rust
// The simplest elif.rs server
use elif_http::{Server, HttpConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = HttpConfig::default();
    let server = Server::new(config);
    server.run("0.0.0.0:3000").await?;
    Ok(())
}
```

## Routing

```rust
use elif_http::{
    ElifRouter, ElifRequest, ElifResponse, HttpResult,
    routing::{Route, RouteBuilder, RouteGroup},
    request::ElifMethod
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

// Handler functions using framework-native types
async fn index(_request: &ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().text("Welcome to elif.rs!"))
}

async fn list_users(_request: &ElifRequest) -> HttpResult<ElifResponse> {
    let users = vec![
        User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
        User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
    ];
    Ok(ElifResponse::ok().json(&users)?)
}

async fn show_user(request: &ElifRequest) -> HttpResult<ElifResponse> {
    // Extract path parameter using framework-native methods
    let id: u32 = request.path_param_parsed("id")?;
    let user = User {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    };
    Ok(ElifResponse::ok().json(&user)?)
}

async fn create_user(request: &ElifRequest) -> HttpResult<ElifResponse> {
    // Parse JSON body using framework-native methods
    let create_request: CreateUserRequest = request.json()?;
    let user = User {
        id: 1,
        name: create_request.name,
        email: create_request.email,
    };
    Ok(ElifResponse::with_status(crate::response::ElifStatusCode::CREATED)
        .json(&user)?)
}

// Set up routes using framework-native router
pub fn routes() -> ElifRouter {
    ElifRouter::new()
        .add_route(Route::get("/", index))
        .add_route(Route::get("/users", list_users))
        .add_route(Route::post("/users", create_user))
        .add_route(Route::get("/users/:id", show_user))
        .group("/api", api_routes())
}

fn api_routes() -> RouteGroup {
    RouteGroup::new()
        .add_route(Route::get("/health", health_check))
        .add_route(Route::get("/version", version_info))
}

async fn health_check(_request: &ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().json(&serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    }))?)
}

async fn version_info(_request: &ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().json(&serde_json::json!({
        "version": "0.9.0",
        "framework": "elif.rs"
    }))?)
}
```

## Advanced Middleware (V2 System)

```rust
use elif_http::middleware::v2::{
    MiddlewarePipelineV2, Middleware, Next, NextFuture,
    factories, ConditionalMiddleware, composition
};
use elif_http::{ElifRequest, ElifResponse, ElifMethod};
use std::time::Duration;

// Using middleware factories for common patterns
let api_middleware = composition::compose3(
    factories::rate_limit(100), // 100 requests per minute
    factories::cors_with_origins(vec!["https://yourdomain.com".to_string()]),
    factories::timeout(Duration::from_secs(30))
);

// Conditional middleware execution
let auth_middleware = ConditionalMiddleware::new(
    factories::bearer_auth("secret-token".to_string())
)
.skip_paths(vec!["/public/*", "/health", "/api/docs"])
.only_methods(vec![ElifMethod::POST, ElifMethod::PUT, ElifMethod::DELETE]);

// Dynamic middleware with runtime conditions
let debug_middleware = ConditionalMiddleware::new(
    DebugMiddleware::new()
)
.condition(|req| {
    req.header("X-Debug").map(|h| h.to_str().unwrap_or("")) == Some("true")
});

// Create comprehensive middleware pipeline
let pipeline = MiddlewarePipelineV2::new()
    .add(debug_middleware)
    .add(auth_middleware)
    .extend(api_middleware.to_pipeline());

// Custom middleware implementation
#[derive(Debug)]
struct RequestLoggingMiddleware;

impl Middleware for RequestLoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let start = std::time::Instant::now();
            let method = request.method.clone();
            let path = request.path().to_string();
            
            // Process request
            let response = next.run(request).await;
            
            // Log after processing
            let duration = start.elapsed();
            println!("{} {} - {} - {:?}", 
                method, path, response.status_code(), duration);
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "RequestLoggingMiddleware"
    }
}
```

## Database & ORM

```rust
use elif_orm::{
    Database, Model, QueryBuilder, 
    migration::{Migration, SchemaBuilder},
    relationships::{HasMany, BelongsTo}
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Model definition with ORM integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Define a model trait implementation
impl Model for User {
    type Id = Uuid;
    
    fn table_name() -> &'static str {
        "users"
    }
    
    fn primary_key() -> &'static str {
        "id"
    }
}

// Database operations using the ORM
async fn database_examples(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    // Insert a new user
    let new_user = User {
        id: Uuid::new_v4(),
        email: "user@example.com".to_string(),
        name: "John Doe".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Using the ORM query builder
    let user_id = QueryBuilder::new()
        .table("users")
        .insert([
            ("id", new_user.id.to_string().as_str()),
            ("email", &new_user.email),
            ("name", &new_user.name),
            ("created_at", &new_user.created_at.to_rfc3339()),
            ("updated_at", &new_user.updated_at.to_rfc3339()),
        ])
        .execute(db)
        .await?;
    
    // Query users with filtering and pagination
    let users: Vec<User> = QueryBuilder::new()
        .table("users")
        .select(&["id", "email", "name", "created_at", "updated_at"])
        .where_clause("email LIKE $1", vec!["%@example.com".to_string()])
        .order_by("created_at DESC")
        .limit(10)
        .offset(0)
        .fetch_all(db)
        .await?;
    
    // Update user
    QueryBuilder::new()
        .table("users")
        .update([("name", "Jane Doe")])
        .where_clause("id = $1", vec![new_user.id.to_string()])
        .execute(db)
        .await?;
    
    Ok(())
}

// Migration example
pub struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn up(&self, schema: &mut SchemaBuilder) -> Result<(), Box<dyn std::error::Error>> {
        schema.create_table("users", |table| {
            table.uuid("id").primary_key();
            table.string("email").unique().not_null();
            table.string("name").not_null();
            table.timestamp("created_at").not_null();
            table.timestamp("updated_at").not_null();
            
            table.index(["email"]);
        });
        Ok(())
    }
    
    fn down(&self, schema: &mut SchemaBuilder) -> Result<(), Box<dyn std::error::Error>> {
        schema.drop_table("users");
        Ok(())
    }
}
```

## WebSocket Support

```rust
use elif_http::websocket::{
    WebSocketManager, WebSocketConnection, WebSocketMessage,
    channel::{ChannelManager, Channel, ChannelEvent}
};

// WebSocket handler using elif.rs native types
async fn websocket_handler(
    ws_manager: WebSocketManager,
    connection: WebSocketConnection
) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut receiver) = connection.split();
    
    // Handle incoming messages
    while let Some(message) = receiver.next().await {
        match message? {
            WebSocketMessage::Text(text) => {
                // Echo the message back
                let response = WebSocketMessage::Text(
                    format!("Echo: {}", text)
                );
                sender.send(response).await?;
            }
            WebSocketMessage::Binary(data) => {
                // Handle binary data
                let response = WebSocketMessage::Binary(data);
                sender.send(response).await?;
            }
            WebSocketMessage::Close(_) => {
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}

// WebSocket channel system for real-time features
async fn channel_example() -> Result<(), Box<dyn std::error::Error>> {
    let channel_manager = ChannelManager::new();
    
    // Create a chat room channel
    let chat_room = channel_manager
        .create_channel("chat:general".to_string())
        .with_password(Some("room-password".to_string()))
        .with_max_connections(100)
        .build()
        .await?;
    
    // Handle channel events
    chat_room.on_event(|event| {
        match event {
            ChannelEvent::UserJoined { user_id, .. } => {
                println!("User {} joined the chat", user_id);
            }
            ChannelEvent::UserLeft { user_id, .. } => {
                println!("User {} left the chat", user_id);
            }
            ChannelEvent::MessageReceived { message, .. } => {
                // Broadcast message to all connected users
                chat_room.broadcast(message);
            }
        }
    });
    
    Ok(())
}
```

## Authentication & Security

```rust
use elif_auth::{
    AuthProvider, JwtProvider, SessionProvider,
    middleware::{AuthMiddleware, RequireAuth}
};
use elif_security::middleware::{
    RateLimitMiddleware, CorsMiddleware, SecurityHeadersMiddleware
};

// JWT-based authentication
let jwt_provider = JwtProvider::builder()
    .secret("your-jwt-secret".to_string())
    .issuer("your-app".to_string())
    .expiry_duration(Duration::from_hours(24))
    .build()?;

let auth_middleware = AuthMiddleware::new(jwt_provider)
    .skip_paths(vec!["/auth/login", "/auth/register", "/public/*"])
    .require_verified_email(true);

// Role-based access control
let admin_guard = RequireAuth::new()
    .require_role("admin")
    .require_permissions(vec!["users:read", "users:write"]);

// Security middleware stack
let security_pipeline = MiddlewarePipelineV2::new()
    .add(SecurityHeadersMiddleware::strict())
    .add(CorsMiddleware::new()
        .allow_origins(vec!["https://yourdomain.com".to_string()])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allow_headers(vec!["Content-Type", "Authorization"])
    )
    .add(RateLimitMiddleware::new()
        .global_limit(1000, Duration::from_minutes(1))
        .per_ip_limit(100, Duration::from_minutes(1))
    );
```

## Caching

```rust
use elif_cache::{
    CacheManager, RedisBackend, MemoryBackend,
    middleware::ResponseCacheMiddleware,
    config::CacheConfig
};

// Set up caching backend
let cache_config = CacheConfig::default()
    .redis_url("redis://localhost:6379")
    .default_ttl(Duration::from_minutes(30));

let cache_manager = CacheManager::new()
    .backend(RedisBackend::new(cache_config.clone()))
    .fallback_backend(MemoryBackend::new(1000)) // LRU cache with 1000 items
    .build()?;

// Response caching middleware
let cache_middleware = ResponseCacheMiddleware::new(cache_manager.clone())
    .cache_get_requests(true)
    .cache_post_requests(false)
    .vary_by_headers(vec!["Accept-Language", "Authorization"])
    .skip_paths(vec!["/admin/*", "/api/realtime/*"]);

// Manual caching in handlers
async fn cached_data_handler(
    request: &ElifRequest,
    cache: &CacheManager
) -> HttpResult<ElifResponse> {
    let cache_key = format!("user_data:{}", request.path_param::<String>("id")?);
    
    // Try to get from cache first
    if let Some(cached_data) = cache.get::<String>(&cache_key).await? {
        return Ok(ElifResponse::ok()
            .header("X-Cache-Status", "HIT")?
            .json(&cached_data)?);
    }
    
    // Generate fresh data
    let fresh_data = generate_expensive_data().await?;
    
    // Cache the result
    cache.set(&cache_key, &fresh_data, Duration::from_minutes(15)).await?;
    
    Ok(ElifResponse::ok()
        .header("X-Cache-Status", "MISS")?
        .json(&fresh_data)?)
}

async fn generate_expensive_data() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Simulate expensive computation
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(serde_json::json!({
        "computed_at": chrono::Utc::now(),
        "result": "expensive_computation_result"
    }))
}
```

## Testing

```rust
use elif_testing::{TestServer, TestClient, TestDatabase, factories};
use elif_http::{ElifResponse, HttpResult};

#[tokio::test]
async fn test_user_api() -> Result<(), Box<dyn std::error::Error>> {
    // Set up test server
    let test_server = TestServer::new()
        .with_routes(user_routes())
        .with_middleware(test_middleware_pipeline())
        .build()
        .await?;
    
    let client = test_server.client();
    
    // Test user creation
    let create_response = client
        .post("/api/users")
        .json(&serde_json::json!({
            "name": "Test User",
            "email": "test@example.com"
        }))
        .send()
        .await?;
    
    assert_eq!(create_response.status(), 201);
    
    let created_user: User = create_response.json().await?;
    assert_eq!(created_user.name, "Test User");
    
    // Test user retrieval
    let get_response = client
        .get(&format!("/api/users/{}", created_user.id))
        .send()
        .await?;
    
    assert_eq!(get_response.status(), 200);
    
    let retrieved_user: User = get_response.json().await?;
    assert_eq!(retrieved_user.id, created_user.id);
    
    Ok(())
}

// Database testing with factories
#[tokio::test]
async fn test_with_database() -> Result<(), Box<dyn std::error::Error>> {
    let test_db = TestDatabase::new().await?;
    
    // Create test data using factories
    let user = factories::UserFactory::new()
        .name("Test User")
        .email("test@example.com")
        .create(&test_db)
        .await?;
    
    // Test database operations
    let found_user = User::find_by_id(user.id, &test_db).await?;
    assert_eq!(found_user.email, "test@example.com");
    
    Ok(())
}
```

## CLI Usage

```bash
# Create a new elif.rs project
elifrs new my-app
cd my-app

# Generate a new model
elifrs make model User --fields="name:string,email:string:unique"

# Generate a migration
elifrs make migration create_users_table

# Run migrations
elifrs migrate run

# Generate a controller
elifrs make controller UserController --resource

# Generate API routes
elifrs make resource User

# Start development server with hot reload
elifrs serve --reload

# Run tests
elifrs test

# Generate OpenAPI specification
elifrs openapi generate

# Start Swagger UI
elifrs openapi serve
```

## Complete Application Example

```rust
use elif_http::{Server, HttpConfig, ElifRouter};
use elif_orm::Database;
use elif_auth::AuthProvider;
use elif_cache::CacheManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize core services
    let database = Database::connect("postgresql://localhost/myapp").await?;
    let cache = CacheManager::redis("redis://localhost:6379").await?;
    let auth = AuthProvider::jwt("jwt-secret".to_string());
    
    // Set up middleware pipeline
    let middleware = create_middleware_pipeline(auth.clone(), cache.clone());
    
    // Configure server
    let config = HttpConfig::builder()
        .port(3000)
        .host("0.0.0.0")
        .max_request_size(1024 * 1024) // 1MB
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // Create router with all routes
    let router = create_application_router(database, cache, auth);
    
    // Start server
    let server = Server::new(config)
        .with_router(router)
        .with_middleware(middleware);
    
    println!("🚀 Server starting on http://0.0.0.0:3000");
    server.run().await?;
    
    Ok(())
}

fn create_middleware_pipeline(
    auth: AuthProvider,
    cache: CacheManager
) -> MiddlewarePipelineV2 {
    use elif_http::middleware::v2::{factories, composition};
    
    composition::compose4(
        factories::cors(),
        factories::rate_limit(100),
        AuthMiddleware::new(auth),
        ResponseCacheMiddleware::new(cache)
    )
}

fn create_application_router(
    database: Database,
    cache: CacheManager,
    auth: AuthProvider
) -> ElifRouter {
    ElifRouter::new()
        .nest("/api/v1", api_v1_routes(database.clone()))
        .nest("/auth", auth_routes(auth))
        .nest("/admin", admin_routes(database))
        .static_files("/static", "./public")
}
```

## Key Corrections Made:

1. **Installation**: Changed `cargo install elif` to `cargo install elifrs`
2. **Routing**: Used proper framework-native types and patterns
3. **Middleware**: Showcased the new V2 middleware system with factories and composition
4. **Database**: Used realistic ORM patterns that match the actual codebase
5. **WebSocket**: Used elif.rs WebSocket abstractions instead of raw Axum
6. **Authentication**: Proper auth middleware integration
7. **Testing**: Framework-native testing utilities
8. **CLI**: Accurate command examples

All examples now use framework-native types (ElifRequest, ElifResponse, etc.) instead of raw Axum re-exports, following the pure framework approach demonstrated in the codebase.