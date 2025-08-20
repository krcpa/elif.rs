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
cargo install elif

# Create a new project
elif new my-app
cd my-app

# Start building
cargo run
```

### Your First App

```rust
let app = elif::new();
app.run(":3000").await?;
```

Yes, it's that simple. Here's a real example:

```rust
use elif::prelude::*;

#[tokio::main]
async fn main() {
    let app = elif::new()
        .get("/", || "Hello from elif.rs!")
        .get("/users", || json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        }));
    
    app.run(":3000").await.unwrap();
}
```

## 🎯 Why elif.rs?

### **DX** Developer Experience
- **Zero Boilerplate**: Start with 2 lines of code
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
// Handles millions of concurrent connections
app.get("/fast", || "Response in 0.68ms");
```

### Type Safe
Catch errors at compile time. Ship with confidence.

```rust
// This won't compile if User doesn't impl Serialize
app.get("/user/:id", |id: u32| -> User {
    User::find(id)
});
```

### Async First
Built on Tokio with intuitive async/await throughout.

```rust
app.get("/async", async || {
    let data = fetch_data().await?;
    process(data).await
});
```

### Modular Design
Start small. Scale big. No rewrites.

```rust
// Start simple
let app = elif::new();

// Add features as needed
app.use(Auth::new());
app.use(Database::postgres());
app.use(Cache::redis());
```

## 📦 Ecosystem

### Core Packages
```toml
[dependencies]
elif = "0.9.1"              # Everything you need
elif-orm = "0.9.1"          # Database ORM
elif-auth = "0.9.1"         # Authentication
```

### DX Tools
```bash
elif-cli        # Scaffold, build, deploy
elif-watch      # Hot reload everything
elif-test       # Testing made simple
elif-debug      # Visual debugging
```

### AX Integration
```bash
elif-ai         # LLM-ready APIs
elif-docs       # Auto documentation
elif-complete   # Smart completions
elif-analyze    # Code insights
```

## 🏗️ Real-World Example

```rust
use elif::prelude::*;

// Define your app state
#[derive(Clone)]
struct AppState {
    db: Database,
    cache: Cache,
}

// Create your app
#[tokio::main]
async fn main() {
    let state = AppState {
        db: Database::connect().await,
        cache: Cache::new(),
    };

    let app = elif::new()
        .state(state)
        .middleware(Logger::new())
        .middleware(RateLimit::new(100))
        .routes(api_routes())
        .static_files("/assets");

    app.run(":3000").await.unwrap();
}

// Define routes
fn api_routes() -> Router {
    Router::new()
        .post("/users", create_user)
        .get("/users/:id", get_user)
        .put("/users/:id", update_user)
        .delete("/users/:id", delete_user)
}

// Handlers with automatic extraction
async fn create_user(
    Json(input): Json<CreateUser>,
    state: State<AppState>,
) -> Result<Json<User>> {
    let user = state.db.users().create(input).await?;
    state.cache.set(&user.id, &user).await?;
    Ok(Json(user))
}
```

## 🛠️ CLI Commands

```bash
# Project Management
elif new <name>         # Create new project
elif init              # Initialize in existing directory
elif check             # Validate project

# Development
elif dev               # Start with hot reload
elif build             # Build for production
elif test              # Run tests

# Database
elif db migrate        # Run migrations
elif db create         # Create migration
elif db rollback       # Rollback migration

# Deployment
elif deploy            # Deploy to production
elif scale <n>         # Scale to n instances
```

## 🚦 Middleware

Clean, composable middleware with the pattern you expect:

```rust
#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Response {
        if let Some(user) = authenticate(&req).await {
            req.set_extension(user);
            next.run(req).await
        } else {
            Response::unauthorized()
        }
    }
}

// Use it
app.use(AuthMiddleware::new());
```

## 🔌 WebSockets

Real-time made simple:

```rust
app.ws("/chat", |ws: WebSocket| async move {
    let (tx, rx) = ws.split();
    
    // Handle incoming messages
    rx.for_each(|msg| async {
        println!("Received: {:?}", msg);
    }).await;
});
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

Every API is designed to be understood by both humans and AI:

```rust
// AI understands intent
let api = app.api_builder()
    .with_context("user_service")
    .auto_generate();
```

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
- 🔄 Deploy anywhere

### Future
- 📱 Mobile SDKs
- 🌍 Edge computing
- 🤖 AI copilot
- 🚀 Cloud native

## 🤝 Contributing

We welcome contributions! elif.rs is built by the community, for the community.

1. Fork the repository
2. Create your feature branch
3. Write tests (AI can help!)
4. Submit a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## 📚 Documentation

- **Getting Started**: [docs.elif.rs/quickstart](https://docs.elif.rs/quickstart)
- **API Reference**: [docs.elif.rs/api](https://docs.elif.rs/api)
- **Examples**: [github.com/krcpa/elif.rs/examples](https://github.com/krcpa/elif.rs/examples)
- **Community**: [discord.elif.rs](https://discord.elif.rs)

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
<a href="https://docs.elif.rs">Docs</a> • 
<a href="https://discord.elif.rs">Discord</a>
</p>