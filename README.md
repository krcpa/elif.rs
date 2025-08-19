# elif.rs

> An LLM-friendly Rust web framework designed for both human developers and AI agents

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/krcpa/elif.rs)

**elif.rs** is a modern Rust web framework that enables both human developers and AI agents to build web applications through structured, safe code generation. Currently in active development with solid foundations.

## 🚧 **Current Status: Active Development**

elif.rs is under active development with core foundations implemented and working. **Not yet production-ready** - use in development and experimentation.

### ✅ **Implemented & Working**

#### Core Architecture
- **🔧 Dependency Injection**: Robust DI container with service resolution  
- **⚙️ Configuration Management**: Environment-based config with validation
- **🔄 Application Lifecycle**: Graceful startup/shutdown with signal handling
- **📦 Module System**: Organize features with dependency resolution

#### Web Foundation  
- **🌐 HTTP Server**: Framework server with DI integration
- **🛣️ Routing System**: Dynamic params, route groups, middleware support
- **📡 Request/Response**: JSON handling, error responses  
- **⚙️ Middleware Pipeline**: Logging, timing, extensible middleware
- **🎯 Controller System**: Service-oriented controllers
- **🔌 WebSocket Support**: Foundation implemented with connection management

#### Security & Validation
- **🛡️ CORS Middleware**: Cross-Origin Resource Sharing implementation
- **🔐 CSRF Protection**: Cross-Site Request Forgery protection with tokens  
- **🚫 Rate Limiting**: Request rate limiting
- **🔒 Input Validation**: Request sanitization and validation system
- **📊 Enhanced Logging**: Request tracing and monitoring
- **🔧 Security Headers**: Security headers middleware

#### Database Layer
- **💾 Database Service Integration**: Complete DI container integration
- **🔗 Connection Pooling**: Transaction support and connection lifecycle
- **🔄 Migration System**: CLI commands and schema migration system
- **📊 Model-Database Integration**: CRUD operations with database integration
- **🗄️ Multi-Database Support**: PostgreSQL, MySQL, SQLite through abstractions

#### Authentication & Authorization
- **🔑 Authentication Core**: Comprehensive error handling and infrastructure
- **🎫 JWT Token Management**: Complete JWT system with middleware
- **🔐 Session Authentication**: Session management with storage backends
- **👤 User Authentication**: Middleware and authentication guards
- **🛡️ Role-Based Access Control**: RBAC system with permissions
- **📱 Multi-Factor Authentication**: TOTP and backup codes support

#### Production Features
- **🗄️ Multi-Backend Caching**: Memory and Redis backends with LRU and tagging
- **📋 Job Queue System**: Background job processing with scheduling
- **🧪 Testing Framework**: Comprehensive testing utilities and factories
- **📖 OpenAPI Documentation**: Automatic API documentation with Swagger UI
- **⚡ HTTP Response Caching**: ETag and Last-Modified header support

### 📊 **Test Coverage: 600+ Tests Passing**
- **Core Architecture**: 33+ tests ✅
- **HTTP Web Stack**: 115+ tests ✅  
- **Authentication & Authorization**: 86+ tests ✅
- **Database & ORM**: 224+ tests ✅
- **Caching System**: 50+ tests ✅
- **Job Queue System**: 16+ tests ✅
- **Testing Framework**: 34+ tests ✅

## 🚀 **Quick Start**

### Installation

```bash
# Install the CLI
cargo install elifrs

# Create a new project
elifrs new my-app
cd my-app

# Build and run
cargo run
# Server starts at http://localhost:3000
```

### Simple Web Application

```rust
use elif_core::{Container, config::DatabaseConfig};
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};
use std::sync::Arc;

async fn hello() -> ElifResponse {
    ElifResponse::text("Hello from elif.rs!")
}

async fn users() -> ElifResponse {
    ElifResponse::json(serde_json::json!({
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

## 📦 **Available Packages**

```toml
[dependencies]
elif-core = "0.5.0"         # Architecture foundation
elif-orm = "0.7.0"          # Multi-database ORM
elif-http = "0.7.0"         # HTTP server with WebSocket support
elif-security = "0.3.0"     # Security middleware stack
elif-validation = "0.2.0"   # Input validation
elif-auth = "0.4.0"         # Authentication with JWT, sessions, RBAC, MFA
elif-cache = "0.3.0"        # Multi-backend caching system
elif-queue = "0.3.0"        # Job queue system
elif-testing = "0.3.0"      # Testing framework
elif-openapi = "0.2.0"      # OpenAPI documentation generation
elif-codegen = "0.4.0"      # Code generation and templates
elif-introspect = "0.3.0"   # Project introspection
```

CLI tool:
```bash
cargo install elifrs  # v0.9.0 - CLI with project scaffolding and management
```

## 🏗️ **Architecture Overview**

```
elif.rs/
├── crates/
│   ├── elif-core/         # ✅ Architecture foundation
│   │   ├── container/     # Dependency injection
│   │   ├── provider/      # Service providers
│   │   ├── module/        # Module system  
│   │   └── config/        # Configuration management
│   │
│   ├── elif-http/         # ✅ HTTP stack with WebSocket
│   │   ├── server/        # HTTP server
│   │   ├── routing/       # Route handling
│   │   ├── middleware/    # Middleware pipeline
│   │   ├── websocket/     # WebSocket foundation
│   │   └── controller/    # Controller system
│   │
│   ├── elif-orm/          # ✅ Multi-database ORM
│   │   ├── model/         # Model definitions
│   │   ├── query/         # Query builder
│   │   ├── backends/      # Database abstraction layer
│   │   ├── database/      # Database service
│   │   └── migration/     # Migration system
│   │
│   ├── elif-cache/        # ✅ Multi-backend caching
│   ├── elif-queue/        # ✅ Job queue system
│   ├── elif-testing/      # ✅ Testing framework
│   ├── elif-openapi/      # ✅ API documentation
│   ├── elif-auth/         # ✅ Authentication system
│   ├── elif-security/     # ✅ Security stack
│   └── cli/               # ✅ CLI tools (published as 'elifrs')
│
└── docs/                  # Documentation and guides
```

## 🔮 **Roadmap & Development**

### Current Development Focus

We're actively working on completing the core framework features:

- **🔌 WebSocket Enhancement** - [Complete WebSocket message handling](https://github.com/krcpa/elif.rs/labels/websocket)
- **📁 File Handling** - [File upload/download system](https://github.com/krcpa/elif.rs/labels/file-handling)  
- **📧 Email System** - [Email service with templates](https://github.com/krcpa/elif.rs/labels/email)
- **🔗 Advanced Routing** - [Route parameters and advanced matching](https://github.com/krcpa/elif.rs/labels/routing)
- **🔄 ORM Relationships** - [Model relationships and eager loading](https://github.com/krcpa/elif.rs/labels/orm)

### Upcoming Features

- **📊 Monitoring & Observability** - Metrics, tracing, health checks
- **🚀 Deployment Tools** - Docker, cloud deployment utilities
- **⚡ Performance Optimization** - Caching strategies, connection pooling
- **🔌 Plugin System** - Framework extensibility
- **📖 Documentation** - Comprehensive guides and examples

**Track Progress**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues) | [Project Board](https://github.com/users/krcpa/projects/1)

## ✨ **Key Features**

### **For Human Developers**
- **🏗️ Clean Architecture**: Dependency injection and modular design
- **📝 Type Safety**: Rust's type system for reliable applications
- **⚡ Performance**: Built on Tokio for high performance
- **🧪 Well Tested**: Comprehensive test coverage (600+ tests)
- **🗄️ Multi-Database**: PostgreSQL, MySQL, SQLite support
- **🔒 Security First**: Complete security middleware stack
- **🔑 Authentication**: JWT, sessions, RBAC, MFA support
- **🔌 Real-time**: WebSocket support for live applications

### **For AI Agents**
- **🤖 LLM-Optimized**: Framework designed for AI code generation
- **📋 Spec-Driven**: Clear specifications and safe editing zones
- **🔍 Introspective**: APIs for understanding project structure
- **🛡️ Safe Zones**: MARKER blocks for AI-safe code modification
- **⚡ Fast Feedback**: Comprehensive testing for validation

## 🧪 **Testing & Development**

```bash
# Run all tests
cargo test --workspace                     # 600+ tests ✅

# Test individual components  
cargo test -p elif-core                    # Core architecture
cargo test -p elif-http                    # HTTP & WebSocket
cargo test -p elif-orm                     # Database & ORM
cargo test -p elif-auth                    # Authentication
cargo test -p elif-cache                   # Caching system
cargo test -p elif-queue                   # Job queues

# Build everything
cargo build --release                      # ✅ Clean builds
```

## 🤝 **Contributing**

elif.rs welcomes contributions! The framework has solid foundations and clear development paths.

### **How to Help**

1. **🔌 Implement WebSocket features** - [WebSocket Issues](https://github.com/krcpa/elif.rs/labels/websocket)
2. **📁 Add file handling** - [File Handling Issues](https://github.com/krcpa/elif.rs/labels/file-handling)
3. **📧 Build email system** - [Email Issues](https://github.com/krcpa/elif.rs/labels/email)
4. **🔗 Enhance routing** - [Routing Issues](https://github.com/krcpa/elif.rs/labels/routing)
5. **📖 Improve documentation** - Examples, guides, tutorials
6. **🧪 Add integration tests** - Real-world scenario testing
7. **🐛 Fix bugs** - [Bug Reports](https://github.com/krcpa/elif.rs/labels/bug)

### **Development Setup**
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs

cargo build --workspace    # ✅ Clean build
cargo test --workspace     # 600+ tests ✅
```

### **Good First Issues**
Looking for [good first issues](https://github.com/krcpa/elif.rs/labels/good%20first%20issue) to get started!

## 📊 **Current Stats**

- **Framework Status**: 🚧 Active Development (not production-ready)
- **Core Foundation**: ✅ Complete and stable
- **Web Stack**: ✅ HTTP server, routing, middleware (115+ tests)
- **Security**: ✅ CORS, CSRF, rate limiting, validation
- **Database**: ✅ Multi-database ORM with abstractions (224+ tests)
- **Authentication**: ✅ JWT, sessions, RBAC, MFA (86+ tests)
- **Caching**: ✅ Multi-backend with tagging (50+ tests)  
- **Job Queues**: ✅ Background processing with scheduling (16+ tests)
- **Testing**: ✅ Comprehensive framework (34+ tests)
- **WebSocket**: ✅ Foundation implemented, enhancement in progress
- **Test Coverage**: ✅ 600+ tests passing across all components
- **Build Status**: ✅ All components build successfully

## 🎯 **When to Use elif.rs**

### **✅ Great For:**
- **Learning Rust web development**
- **Prototyping and experimentation**  
- **Contributing to open source**
- **AI-assisted development**
- **Building internal tools**

### **❌ Not Ready For:**
- **Production applications** (yet)
- **Mission-critical systems**
- **High-traffic websites**

We're working hard to make elif.rs production-ready. [Follow our progress](https://github.com/krcpa/elif.rs/issues) and consider contributing!

## 📄 **License**

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 **Links**

- **Repository**: [https://github.com/krcpa/elif.rs](https://github.com/krcpa/elif.rs)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)
- **Project Board**: [Development Progress](https://github.com/users/krcpa/projects/1)
- **Crates.io**: [elifrs CLI](https://crates.io/crates/elifrs)

---

**🚀 Modern LLM-Friendly Rust Web Framework 🤖**

> *Currently in active development with solid foundations*  
> *600+ tests passing • Multi-database support • WebSocket foundation*  
> *Try: `cargo install elifrs` to get started*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>