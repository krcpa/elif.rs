# elif.rs

> A Rust web framework designed for both AI agents and developers - simple, intuitive, productive.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is a modern Rust web framework that makes web development accessible to both human developers and AI agents through intuitive APIs and clean architectural patterns. With 600+ tests passing and solid foundations, it's ready for experimentation and development use.

## 🚀 **Quick Start**

```bash
# Install the CLI
cargo install elifrs

# Create a new project
elifrs new my-app
cd my-app

# Run your app
cargo run
# Server starts at http://localhost:3000
```

### Your First App

```rust
use elif_core::Container;
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};

async fn hello() -> ElifResponse {
    ElifResponse::ok().text("Hello from elif.rs!")
}

async fn users() -> ElifResponse {
    ElifResponse::ok().json(&serde_json::json!({
        "users": [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let container = Container::new();
    
    let router = ElifRouter::new()
        .get("/", hello)
        .get("/users", users);
    
    let mut server = Server::with_container(container, HttpConfig::default())?;
    server.use_router(router);
    server.listen("0.0.0.0:3000").await?;
    Ok(())
}
```

## ✨ **Key Features**

### 🎯 **Developer Experience First**
- **Intuitive APIs**: Familiar patterns like `handle(request, next)` for middleware
- **Zero Axum/Hyper exposure**: Pure framework types throughout your code
- **AI-friendly design**: Clear patterns that LLMs can understand and generate
- **Comprehensive testing**: 600+ tests ensure reliability

### 🏗️ **Modern Architecture**
- **Dependency Injection**: Built-in DI container with automatic service resolution
- **Module System**: Organize features with automatic dependency management
- **V2 Middleware**: Clean `handle(request, next)` pattern for intuitive middleware
- **Configuration Management**: Environment-based config with validation

### 🌐 **Complete Web Stack**
- **HTTP Server**: High-performance server with full routing support
- **WebSocket Support**: Real-time communication with channel abstractions
- **Request/Response**: Intuitive APIs for handling HTTP data
- **Controller System**: Service-oriented controllers with DI integration

### 🔒 **Security & Validation**
- **Built-in Security**: CORS, CSRF, rate limiting, security headers
- **Input Validation**: Comprehensive request validation and sanitization
- **Authentication**: JWT, sessions, RBAC, and MFA support
- **Error Handling**: Panic recovery and graceful error responses

### 💾 **Database & ORM**
- **Multi-Database Support**: PostgreSQL, MySQL, SQLite
- **Query Builder**: Intuitive, type-safe query construction
- **Migrations**: Version control for your database schema
- **Connection Pooling**: Efficient database connection management

### ⚡ **Production Features**
- **Caching System**: Memory and Redis backends with tagging
- **Job Queue**: Background job processing with scheduling
- **OpenAPI Docs**: Automatic API documentation generation
- **Response Caching**: ETag and Last-Modified support

## 📦 **Available Packages**

```toml
[dependencies]
elif-core = "0.5.0"         # Core architecture
elif-http = "0.7.0"         # HTTP server & WebSocket
elif-orm = "0.7.0"          # Database ORM
elif-auth = "0.4.0"         # Authentication system
elif-security = "0.3.0"     # Security middleware
elif-cache = "0.3.0"        # Caching system
elif-queue = "0.3.0"        # Job queue
elif-validation = "0.2.0"   # Input validation
elif-testing = "0.3.0"      # Testing utilities
elif-openapi = "0.2.0"      # API documentation
```

## 🛠️ **CLI Commands**

```bash
# Project management
elifrs new <app>            # Create new project
elifrs generate             # Generate code from specs
elifrs check               # Validate project

# Database
elifrs migrate run         # Run migrations
elifrs migrate create      # Create new migration
elifrs migrate rollback    # Rollback migrations

# Development
cargo run                  # Start development server
cargo test                 # Run tests
cargo build --release      # Build for production
```

## 🏛️ **Architecture**

elif.rs follows a clean modular architecture:

```
my-app/
├── src/
│   ├── main.rs           # Application entry point
│   ├── modules/          # Feature modules
│   │   ├── users/        # User module
│   │   │   ├── mod.rs    # Module definition
│   │   │   ├── controller.rs
│   │   │   ├── service.rs
│   │   │   └── model.rs
│   │   └── auth/         # Auth module
│   ├── middleware/       # Custom middleware
│   ├── config/          # Configuration files
│   └── migrations/      # Database migrations
├── Cargo.toml
└── .env                # Environment variables
```

## 🔥 **Recent Updates**

### V2 Middleware System (Complete) ✅
All middleware has been migrated to the new intuitive pattern:

```rust
#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        // Check authentication
        if !is_authenticated(&request) {
            return ElifResponse::unauthorized().into_future();
        }
        
        // Continue to next middleware
        next.run(request).await
    }
}
```

### Features Added Recently:
- ✅ **Panic Recovery**: ErrorHandlerMiddleware now catches and handles panics
- ✅ **Pure Framework Types**: No Axum/Hyper types in public APIs
- ✅ **Enhanced Security**: Complete security middleware stack
- ✅ **WebSocket Channels**: Real-time communication abstractions
- ✅ **Response Borrowing API**: Efficient response manipulation

## 🗺️ **Roadmap**

### Currently In Progress
- 🔄 **Enhanced Email System**: Templates, queuing, multiple providers
- 🔄 **File Handling**: Upload/download with streaming support
- 🔄 **Advanced WebSocket**: Presence tracking, message queuing
- 🔄 **Body Buffering**: Enable response caching middleware

### Coming Soon
- 📊 **Monitoring**: Metrics, tracing, health checks
- 🔌 **Plugin System**: Extend framework functionality
- 🚀 **Deployment Tools**: Docker, cloud deployment
- 📱 **Client SDKs**: JavaScript/TypeScript clients

## 🤖 **AI Development**

elif.rs is heavily developed and tested with AI assistants:

- **Claude**: Primary development partner - understands the codebase deeply
- **GPT-4**: Excellent for generating boilerplate and tests
- **Gemini**: Great for code reviews and optimization suggestions

The framework's clean architecture and intuitive patterns make it ideal for AI-assisted development. Many features were implemented through AI collaboration, ensuring the APIs are AI-friendly by design.

## 🤝 **Contributing**

We welcome contributions! Check out our [open issues](https://github.com/krcpa/elif.rs/issues) or:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## 📚 **Documentation**

- **Getting Started**: See the [Quick Start](#-quick-start) section
- **Architecture**: Read [FRAMEWORK_ARCHITECTURE.md](docs/FRAMEWORK_ARCHITECTURE.md)
- **API Docs**: Visit [docs.rs/elifrs](https://docs.rs/elifrs)
- **Examples**: Check the [examples/](examples/) directory

## 📊 **Project Status**

- **Core Framework**: ✅ Stable
- **HTTP/WebSocket**: ✅ Stable  
- **Database/ORM**: ✅ Stable
- **Authentication**: ✅ Stable
- **Middleware V2**: ✅ Complete
- **Production Ready**: 🔄 In progress (use for development/experimentation)

## 📄 **License**

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
Built with ❤️ for developers and AI agents alike.<br>
Making Rust web development simple and intuitive.
</p>