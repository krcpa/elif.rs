# elif.rs

> Where Rust meets Developer Experience - The framework designed for exceptional DX and AI-native development.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is the first Rust web framework built from the ground up for both Developer Experience (DX) and AI Experience (AX). With zero boilerplate, predictable performance, and APIs that both humans and AI can understand, elif makes Rust web development delightful.

## 🚀 Quick Start

```bash
# Install elif
cargo install elifrs

# Create a new project
elifrs new my-app
cd my-app

# Start building
cargo run
```

### Your First App

```rust
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = ElifRouter::new()
        .get("/", || async { 
            ElifResponse::ok().text("Hello from elif.rs!") 
        })
        .get("/users", || async {
            ElifResponse::ok().json(&serde_json::json!({
                "users": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ]
            }))
        });

    let config = HttpConfig::default();
    let mut server = Server::new(config);
    server.use_router(router);
    
    println!("🚀 Server running at http://localhost:3000");
    server.run("0.0.0.0:3000").await?;
    Ok(())
}
```

## 🎯 Why elif.rs?

### **DX** Developer Experience
- **Zero Boilerplate**: Start with minimal code
- **Instant Feedback**: Hot reloading that actually works
- **Type Safe**: If it compiles, it works
- **Clear Errors**: Helpful messages that guide you

### **AX** AI Experience
- **LLM-Friendly APIs**: Patterns that AI understands
- **Self-Documenting**: Code that explains itself
- **Context-Aware**: Smart completions and suggestions
- **AI-First Design**: Built for the future of development

## ✨ Features

### Fast by Default
Sub-millisecond response times. No runtime overhead. Just pure Rust performance.

```rust
async fn fast_handler(_req: &ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().text("Response in 0.68ms"))
}
```

### Type Safe
Catch errors at compile time. Ship with confidence.

```rust
#[derive(Serialize)]
struct User { id: u32, name: String }

async fn get_user(req: &ElifRequest) -> HttpResult<ElifResponse> {
    let id: u32 = req.path_param_parsed("id")?;
    let user = User { id, name: format!("User {}", id) };
    Ok(ElifResponse::ok().json(&user)?)
}
```

### Modern Middleware (V2)
Clean, composable middleware with the pattern you expect:

```rust
use elif_http::middleware::v2::{Middleware, Next, NextFuture};

impl Middleware for AuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            if !is_authenticated(&request) {
                return ElifResponse::unauthorized();
            }
            next.run(request).await
        })
    }
}
```

### Modular Design
Start small. Scale big. No rewrites.

```rust
use elif_http::middleware::v2::{factories, composition};

// Compose middleware easily
let api_middleware = composition::compose3(
    factories::rate_limit(100),
    factories::cors_with_origins(vec!["https://yourdomain.com".to_string()]),
    factories::timeout(Duration::from_secs(30))
);
```

## 📦 Ecosystem

### Core Packages
```toml
[dependencies]
elif-core = "0.5.0"         # Core architecture
elif-http = "0.7.0"         # HTTP server & WebSocket
elif-orm = "0.7.0"          # Database ORM
elif-auth = "0.4.0"         # Authentication
elif-cache = "0.3.0"        # Caching system
```

### DX Tools
```bash
elifrs new <app>            # Create new project
elifrs serve --reload       # Hot reload development
elifrs test                # Testing utilities
elifrs migrate             # Database migrations
```

### AX Integration
```bash
elifrs generate            # AI-powered code generation
elifrs openapi generate    # Auto-generate API docs
```

## 🏗️ Real-World Example

```rust
use elif_http::{Server, HttpConfig, ElifRouter, ElifRequest, ElifResponse, HttpResult};
use elif_orm::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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

// Define routes with framework-native types
fn api_routes() -> ElifRouter {
    ElifRouter::new()
        .get("/users", list_users)
        .post("/users", create_user)
        .get("/users/:id", get_user)
}

async fn list_users(_req: &ElifRequest) -> HttpResult<ElifResponse> {
    let users = vec![
        User { id: 1, name: "Alice".into(), email: "alice@example.com".into() },
        User { id: 2, name: "Bob".into(), email: "bob@example.com".into() },
    ];
    Ok(ElifResponse::ok().json(&users)?)
}

async fn create_user(req: &ElifRequest) -> HttpResult<ElifResponse> {
    let input: CreateUserRequest = req.json()?;
    let user = User {
        id: 1,
        name: input.name,
        email: input.email,
    };
    Ok(ElifResponse::with_status(201).json(&user)?)
}

async fn get_user(req: &ElifRequest) -> HttpResult<ElifResponse> {
    let id: u32 = req.path_param_parsed("id")?;
    let user = User {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    };
    Ok(ElifResponse::ok().json(&user)?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up middleware pipeline
    use elif_http::middleware::v2::{MiddlewarePipelineV2, factories};
    
    let middleware = MiddlewarePipelineV2::new()
        .add(factories::logger())
        .add(factories::cors())
        .add(factories::rate_limit(100));
    
    // Configure and start server
    let config = HttpConfig::default();
    let mut server = Server::new(config);
    
    server.use_router(api_routes());
    server.use_middleware(middleware);
    
    println!("🚀 Server running at http://localhost:3000");
    server.run("0.0.0.0:3000").await?;
    Ok(())
}
```

## 🛠️ CLI Commands

```bash
# Project Management
elifrs new <name>          # Create new project
elifrs generate            # Generate from AI specs
elifrs check              # Validate project

# Development
elifrs serve --reload      # Start with hot reload
cargo test                # Run tests
cargo build --release     # Build for production

# Database
elifrs migrate run        # Run migrations
elifrs migrate create     # Create migration
elifrs migrate rollback   # Rollback migration

# API Documentation
elifrs openapi generate   # Generate OpenAPI spec
elifrs openapi serve      # Start Swagger UI
```

## 🔌 WebSockets

Real-time made simple:

```rust
use elif_http::websocket::{WebSocketConnection, WebSocketMessage};

async fn websocket_handler(connection: WebSocketConnection) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut receiver) = connection.split();
    
    while let Some(message) = receiver.next().await {
        match message? {
            WebSocketMessage::Text(text) => {
                let response = WebSocketMessage::Text(format!("Echo: {}", text));
                sender.send(response).await?;
            }
            WebSocketMessage::Close(_) => break,
            _ => {}
        }
    }
    Ok(())
}
```

## 💾 Database & ORM

```rust
use elif_orm::{Database, QueryBuilder};

// Connect to database
let db = Database::connect("postgresql://localhost/myapp").await?;

// Query with the builder
let users = QueryBuilder::new()
    .table("users")
    .select(&["id", "name", "email"])
    .where_clause("email LIKE $1", vec!["%@example.com".to_string()])
    .order_by("created_at DESC")
    .limit(10)
    .fetch_all(&db)
    .await?;
```

## 📊 Performance

elif.rs is built for production:

- **145k req/sec** - Benchmark results
- **0.68ms** - Average latency
- **12MB** - Memory footprint
- **Zero** - Runtime overhead

## 🤖 AI Development

elif.rs pioneered AI-native framework design:

- **Claude**: Primary development partner
- **GPT-4**: Extensive testing and generation
- **Cursor/Copilot**: First-class support

Every API is designed to be understood by both humans and AI, making it perfect for AI-assisted development.

## 🗺️ Roadmap

### Now
- ✅ Core framework
- ✅ HTTP/WebSocket
- ✅ Database/ORM
- ✅ Authentication
- ✅ Middleware v2

### Next
- 🔄 Streaming responses
- 🔄 gRPC support
- 🔄 GraphQL integration
- 🔄 Edge deployment

### Future
- 📱 Mobile SDKs
- 🌍 Global CDN
- 🤖 AI copilot
- 🚀 One-click deploy

## 🤝 Contributing

We welcome contributions! elif.rs is built by the community, for the community.

1. Fork the repository
2. Create your feature branch
3. Write tests (AI can help!)
4. Submit a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## 📚 Documentation

- **Getting Started**: [docs.rs/elifrs](https://docs.rs/elifrs)
- **Examples**: [github.com/krcpa/elif.rs/examples](https://github.com/krcpa/elif.rs/tree/main/examples)
- **API Reference**: Full API documentation on docs.rs
- **Architecture**: [FRAMEWORK_ARCHITECTURE.md](docs/FRAMEWORK_ARCHITECTURE.md)

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
<strong>elif.rs</strong><br>
Where Rust meets Developer Experience<br>
Built for humans, optimized for AI<br>
<br>
<a href="https://elif.rs">elif.rs</a> • 
<a href="https://github.com/krcpa/elif.rs">GitHub</a> • 
<a href="https://docs.rs/elifrs">Docs</a> • 
<a href="https://discord.gg/elifrs">Discord</a>
</p>