# elif.rs

> An LLM-friendly Rust web framework under active development, designed for AI agent-driven development

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)
[![Downloads](https://img.shields.io/crates/d/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is an experimental Rust web framework under active development that enables both human developers and AI agents to build web applications through structured, safe code generation. This is an early-stage project with solid foundations being built iteratively.

## 🚧 **Current Status: Active Development**

elif.rs is in **active development** with several foundational layers now complete and ready for experimentation:

### ✅ **What's Working Now**

#### Phase 1 Complete: Architecture Foundation
- **🔧 Dependency Injection**: Robust DI container with service resolution  
- **⚙️ Configuration Management**: Environment-based config with validation
- **🔄 Application Lifecycle**: Graceful startup/shutdown with signal handling
- **📦 Module System**: Organize features with dependency resolution
- **✅ Status**: 33/33 tests passing, stable foundation

#### Phase 2 Complete: Web Foundation  
- **🌐 HTTP Server**: Axum-based server with DI integration
- **🛣️ Routing System**: Dynamic params, route groups, middleware support
- **📡 Request/Response**: JSON handling, error responses  
- **⚙️ Middleware Pipeline**: Logging, timing, extensible middleware
- **🎯 Controller System**: Service-oriented controllers with database integration
- **✅ Status**: 61/61 tests passing, functional web stack

#### Phase 2.1 Complete: Advanced ORM
- **📊 Model System**: CRUD operations with timestamps, soft deletes
- **🔍 Query Builder**: Type-safe fluent API with advanced features
- **⚡ Complex Queries**: Subqueries, aggregations, joins, pagination
- **🧪 Well Tested**: 36 unit tests + performance benchmarks
- **✅ Status**: Functional ORM layer, ready for experimentation

#### Phase 3.1 Complete: CORS Security
- **🛡️ CORS Middleware**: Complete Cross-Origin Resource Sharing implementation
- **🏗️ Tower Integration**: Works seamlessly with Axum middleware pipeline
- **⚙️ Flexible Config**: Builder pattern API with security defaults
- **✅ Status**: 5/5 tests passing, first security middleware complete

### 🚧 **Currently Working On**

#### Phase 3.2: CSRF Protection (In Progress)
- **🔐 CSRF Middleware**: Cross-Site Request Forgery protection (partially implemented)
- **⏱️ Rate Limiting**: Request limiting (planned)
- **🔒 Security Headers**: Additional security middleware (planned)

### 📊 **Test Coverage: 135+ Tests**
- **Core Architecture**: 33 tests
- **HTTP Server**: 61 tests  
- **ORM Layer**: 36 tests
- **Security**: 5 tests (CORS)
- **Total**: 135+ tests passing across framework

## 🚀 **Try It Now (Experimental)**

### Quick Installation

```bash
# Install the experimental CLI
cargo install elifrs

# Create a new project
elifrs new my-experiment
cd my-experiment

# Build and run (basic functionality works)
cargo run
```

**What you get:**
- ✅ **HTTP Server** that starts and handles requests
- ✅ **Dependency Injection** for service management  
- ✅ **Database Integration** with working ORM
- ✅ **CORS Security** for cross-origin requests
- ✅ **Configuration System** with environment support
- ⚠️ **Basic functionality** - many features still in development

## 🎯 **Project Goals**

### **For Human Developers**
- **🏗️ Clean Architecture**: Dependency injection and modular design
- **📝 Type Safety**: Rust's type system for reliable web applications
- **⚡ Performance**: Built on Axum/Tokio for high performance
- **🧪 Well Tested**: Comprehensive test coverage for reliability

### **For AI Agents**
- **🤖 LLM-Optimized**: Framework designed with AI code generation in mind
- **📋 Spec-Driven**: Clear specifications and safe editing zones
- **🔍 Introspective**: APIs for understanding project structure
- **🛡️ Safe Zones**: MARKER blocks for AI-safe code modification

## 📦 **Available Packages (Experimental)**

```bash
cargo install elifrs  # v0.2.0 - CLI tool for project scaffolding
```

```toml
[dependencies]
elif-core = "0.1.0"        # Architecture foundation
elif-orm = "0.2.0"         # Database layer with query builder
elif-http = "0.2.0"        # HTTP server with Axum integration  
elif-security = "0.1.0"    # Security middleware (CORS implemented)
```

## 💡 **What You Can Experiment With**

### 🌐 **Basic Web Applications**
```rust
use elif_http::*;
use axum::{routing::get, Router};

async fn hello() -> &'static str {
    "Hello from elif.rs!"
}

// Basic HTTP server (works)
let app = Router::new()
    .route("/", get(hello));
    
// Run on localhost:3000
```

### 📊 **Database Operations**
```rust
use elif_orm::*;

// Define a model (works)
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<Uuid>,
    email: String,  
    name: String,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

// Use the query builder (works)
let users = User::query()
    .where_eq("active", true)
    .order_by("created_at")  
    .limit(10)
    .get(&pool)  // Note: You need to set up the pool
    .await?;
```

### 🛡️ **CORS Security**  
```rust
use elif_security::CorsMiddleware;

// CORS protection (works)
let cors = CorsMiddleware::new(CorsConfig::default())
    .allow_origin("https://localhost:3000")
    .allow_methods(vec![Method::GET, Method::POST]);

let app = Router::new()
    .route("/api/data", get(get_data))
    .layer(CorsLayer::new(cors.config));
```

### 🏗️ **Dependency Injection**
```rust
use elif_core::*;

// Service registration (works)
let app = Application::builder()
    .provider(DatabaseProvider)  
    .module(ApiModule)
    .build()?;

// Note: You'll need to implement your providers
```

## 🏗️ **Architecture Overview**

```
elif.rs/ (Under Development)
├── crates/
│   ├── elif-core/         # ✅ Architecture foundation
│   │   ├── container/     # Dependency injection
│   │   ├── provider/      # Service providers
│   │   ├── module/        # Module system  
│   │   └── config/        # Configuration
│   │
│   ├── elif-http/         # ✅ HTTP server basics
│   │   ├── server/        # Axum integration
│   │   ├── routing/       # Route handling
│   │   ├── middleware/    # Basic middleware
│   │   └── controller/    # Controller system
│   │
│   ├── elif-orm/          # ✅ Database layer
│   │   ├── model/         # Model definitions
│   │   ├── query/         # Query builder
│   │   └── primary_key/   # Key handling
│   │
│   ├── elif-security/     # 🚧 Security (CORS done)
│   │   ├── cors/          # ✅ CORS middleware
│   │   ├── csrf/          # 🚧 CSRF (in progress)
│   │   └── headers/       # ❌ Security headers (planned)
│   │
│   └── elif-cli/          # ✅ Basic CLI tools
│
└── plan/                  # Development roadmap
    ├── phase1/            # ✅ COMPLETE
    ├── phase2/            # ✅ COMPLETE  
    ├── phase3/            # 🚧 IN PROGRESS (3.1 done, 3.2 active)
    └── phase4-9/          # ❌ PLANNED
```

**Legend**: ✅ Working | 🚧 In Progress | ❌ Planned

## 🧪 **Testing Status**

```bash
# Run all tests (they pass!)
cargo test --workspace                     # ✅ 135+ tests passing

# Test individual components  
cargo test -p elif-core                    # 33/33 tests
cargo test -p elif-http                    # 61/61 tests
cargo test -p elif-orm                     # 36/36 tests  
cargo test -p elif-security                # 5/5 tests

# Build the project
cargo build --release                      # Clean compilation
```

## 📋 **Development Roadmap**

### ✅ **Completed Phases**
- **Phase 1**: Architecture Foundation (33 tests)
- **Phase 2**: Web Foundation (61 tests) 
- **Phase 2.1**: Advanced ORM (36 tests)
- **Phase 3.1**: CORS Security (5 tests)

### 🚧 **Current Work (Phase 3.2)**
- [ ] CSRF protection middleware
- [ ] Rate limiting implementation
- [ ] Security headers middleware
- [ ] Input validation system

### 📅 **Upcoming Phases**
- **Phase 4**: Authentication & Authorization  
- **Phase 5**: Advanced ORM features
- **Phase 6**: Developer experience tools
- **Phase 7**: Production features
- **Phase 8**: Advanced features

**Track Progress**: [GitHub Project Board](https://github.com/users/krcpa/projects/1/views/1)

## ⚠️ **Important Notes**

### **This is Experimental Software**
- **Not ready for production use**
- **APIs may change** as development continues
- **Missing features** that production apps need
- **Documentation is incomplete**
- **Use at your own risk** for experiments only

### **What's Missing**
- Authentication & authorization
- Comprehensive validation  
- Production security features
- Advanced ORM relationships
- Caching layer
- Job queues
- File handling
- Email integration
- Many other production necessities

### **Best For**
- 🧪 **Experimentation** with Rust web frameworks
- 🤖 **AI development** research and testing
- 📚 **Learning** modern Rust web architecture
- 🔬 **Contributing** to framework development

## 🤖 **AI Agent Development**

elif.rs is specifically designed to work well with AI agents:

```bash
# 1. Plan: AI can understand the project structure
elifrs new my-experiment

# 2. Implement: AI works within safe MARKER zones  
# Safe code generation in predefined areas

# 3. Test: Comprehensive testing provides feedback
cargo test  # 135+ tests guide AI development

# 4. Iterate: Fast compilation enables rapid iteration
```

### **LLM-Friendly Features**
- **🛡️ MARKER Zones**: Safe areas for AI code modification
- **📋 Clear Specs**: Detailed specifications reduce AI confusion  
- **🔍 Introspection**: Built-in project understanding
- **⚡ Fast Feedback**: Quick compilation and testing

## 🤝 **Contributing**

elif.rs needs contributors! This is an active development project.

### **Current Priorities**
- **Phase 3.2**: CSRF Protection (Issue #30)
- **Phase 3.3**: Rate Limiting (Issue #31)
- **Phase 3.4**: Input Validation (Issue #32)

### **Development Setup**
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs
cargo build
cargo test --workspace  # Should see 135+ tests passing
```

### **How to Help**
1. **Try it out** and report issues
2. **Implement missing features** from the roadmap
3. **Improve documentation** and examples
4. **Add more tests** for better coverage
5. **Share feedback** on the API design

## 📊 **Current Stats**

- **Framework Status**: ⚠️ Experimental, not production-ready
- **Test Coverage**: ✅ 135+ tests passing
- **Core Components**: ✅ 4/9 major phases complete  
- **Security**: ✅ Basic CORS, 🚧 CSRF in progress
- **Database**: ✅ Functional ORM with advanced queries
- **HTTP Stack**: ✅ Basic server with middleware support
- **Build Status**: ✅ Clean compilation
- **Package Status**: ✅ Published to crates.io for experimentation

## 📄 **License**

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 **Links**

- **Repository**: [https://github.com/krcpa/elif.rs](https://github.com/krcpa/elif.rs)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Project Board**: [Development Progress](https://github.com/users/krcpa/projects/1/views/1)
- **Discussions**: [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)

---

**🚧 Experimental LLM-Friendly Web Framework 🤖**

> *Active Development - Try the experiment with `cargo install elifrs`*  
> *Phase 1 ✅ Architecture Complete (33 tests)*  
> *Phase 2 ✅ Web Foundation Complete (61 tests)*  
> *Phase 2.1 ✅ Advanced ORM Complete (36 tests)*  
> *Phase 3.1 ✅ CORS Security Complete (5 tests)*  
> *Phase 3.2 🚧 CSRF Protection In Progress*  
> *Total: 135+ Tests - Solid Foundation, Many Features Still Needed*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>