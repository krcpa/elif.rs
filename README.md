# elif.rs

> An LLM-friendly Rust web framework under active development, designed for AI agent-driven development

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-failing-red.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is an experimental Rust web framework under active development that enables both human developers and AI agents to build web applications through structured, safe code generation. This is an early-stage project with solid foundations being built iteratively.

## 🚧 **Current Status: Phase 4 - Database Foundation**

elif.rs is in **active development** with core web and security layers complete, now building database operations foundation:

### ✅ **What's Working Now**

#### Phase 1 Complete: Architecture Foundation
- **🔧 Dependency Injection**: Robust DI container with service resolution  
- **⚙️ Configuration Management**: Environment-based config with validation
- **🔄 Application Lifecycle**: Graceful startup/shutdown with signal handling
- **📦 Module System**: Organize features with dependency resolution
- **✅ Status**: 33/33 tests passing, stable foundation

#### Phase 2 Complete: Web Foundation  
- **🌐 HTTP Server**: Pure framework server with DI integration
- **🛣️ Routing System**: Dynamic params, route groups, middleware support
- **📡 Request/Response**: JSON handling, error responses  
- **⚙️ Middleware Pipeline**: Logging, timing, extensible middleware
- **🎯 Controller System**: Service-oriented controllers with database integration
- **✅ Status**: 112/112 tests passing, production-ready web stack

#### Phase 2.1 Complete: Advanced ORM Foundation
- **📊 Model System**: CRUD operations with timestamps, soft deletes
- **🔍 Query Builder**: Type-safe fluent API with advanced features
- **⚡ Complex Queries**: Subqueries, aggregations, joins, pagination
- **🧪 Well Tested**: 39 unit tests + performance benchmarks
- **✅ Status**: Functional ORM layer, ready for database integration

#### Phase 3 Complete: Security & Framework Consistency
- **🛡️ CORS Middleware**: Complete Cross-Origin Resource Sharing implementation
- **🔐 CSRF Protection**: Full Cross-Site Request Forgery protection with token management  
- **🚫 Rate Limiting**: Request rate limiting with pure framework types
- **🔒 Input Validation**: Request sanitization and validation system
- **📊 Enhanced Logging**: Request tracing and security monitoring
- **🔧 Security Headers**: Complete security headers middleware
- **🏗️ Pure Framework Architecture**: All framework types, no external dependencies exposed
- **✅ Status**: 151/151 security tests passing, architecturally consistent

### 🚧 **Currently Working On**

#### Phase 4: Database Operations Foundation (In Progress)
- **✅ Database Service Integration** - DI container integration (Issue #60 - Complete)
- **✅ Basic Connection Pool Management** - Connection lifecycle (Issue #61 - Complete) 
- **🔄 Database Architecture Refactor** - Move DB from http to orm crate (Issue #66 - Active)
- **📋 Basic Migration System** - Schema migrations (Issue #63 - Pending)
- **🔗 Model-Database Integration** - Connect ORM to database (Issue #64 - Pending)
- **💾 Basic CRUD Operations** - Working database operations (Issue #65 - Pending)

**Goal**: Complete foundational database layer with proper architecture, transactions, migrations, and working CRUD operations.

### ⚠️ **Build Status: Fixing Migration System**
- **Current Issue**: Migration system implementation causing build failures
- **Impact**: Database foundation work temporarily blocked
- **Priority**: High - resolving ORM/database integration issues
- **Expected Resolution**: Architecture refactor in progress

### 📊 **Test Coverage: 300+ Tests (When Building)**
- **Core Architecture**: 33 tests ✅
- **HTTP Web Stack**: 112 tests ✅  
- **ORM Foundation**: 39 tests ✅
- **Security & Validation**: 151 tests ✅
- **Database Operations**: In development 🚧
- **Total**: 335+ tests across completed phases

## 🚀 **Try It Now (Experimental)**

### Quick Installation

```bash
# Install the experimental CLI
cargo install elifrs

# Create a new project
elifrs new my-experiment
cd my-experiment

# Build and run (web server works)
cargo run
```

**What you get:**
- ✅ **HTTP Server** with pure framework abstractions
- ✅ **Dependency Injection** for service management  
- ✅ **Advanced ORM** with query builder and model system
- ✅ **Complete Security Stack** - CORS, CSRF, rate limiting, validation, headers
- ✅ **Pure Framework Architecture** - consistent types, no external deps exposed
- ✅ **Configuration System** with environment support
- 🚧 **Database Operations** - foundation layer in development
- ⚠️ **Experimental** - solid web/security foundation, database integration in progress

## 🎯 **Project Goals**

### **For Human Developers**
- **🏗️ Clean Architecture**: Dependency injection and modular design
- **📝 Type Safety**: Rust's type system for reliable web applications
- **⚡ Performance**: Built on Tokio for high performance
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
elif-core = "0.2.0"         # Architecture foundation
elif-orm = "0.3.0"          # Database layer with query builder (in development)
elif-http = "0.2.0"         # HTTP server with pure framework abstractions
elif-security = "0.2.1"     # Complete security middleware stack
elif-validation = "0.1.0"   # Input validation and sanitization
```

## 💡 **What You Can Experiment With**

### 🌐 **Pure Framework Web Applications**
```rust
use elif_core::{Container, container::test_implementations::*};
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};
use std::sync::Arc;

async fn hello() -> ElifResponse {
    ElifResponse::text("Hello from elif.rs - Pure Framework!")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create container with DI services
    let config = Arc::new(create_test_config());
    let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    
    let container = Container::builder()
        .config(config)
        .database(database)
        .build()?
        .into();

    // Create router using pure framework abstractions
    let router = ElifRouter::new()
        .get("/", hello);
    
    // Create and configure server
    let mut server = Server::with_container(container, HttpConfig::default())?;
    server.use_router(router);
    
    // Start server - no external types exposed
    server.listen("0.0.0.0:3000").await?;
    Ok(())
}
```

### 🛡️ **Complete Security Stack**  
```rust
use elif_core::{Container, container::test_implementations::*};
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};
use elif_security::{SecurityMiddlewareBuilder, RateLimitConfig};
use std::sync::Arc;

async fn secure_api() -> ElifResponse {
    ElifResponse::json(serde_json::json!({"secure": true, "message": "All security middleware active"}))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let container = Container::builder()
        .config(Arc::new(create_test_config()))
        .database(Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>)
        .build()?
        .into();

    let router = ElifRouter::new()
        .get("/api/secure", secure_api);
    
    let mut server = Server::with_container(container, HttpConfig::default())?;
    server.use_router(router);
    
    // Complete security stack - all pure framework types
    server.use_middleware(
        SecurityMiddlewareBuilder::new()
            .cors_permissive()                    // CORS protection
            .csrf_with_token_header()             // CSRF protection  
            .rate_limit(RateLimitConfig::default()) // Rate limiting
            .request_sanitization()               // Input sanitization
            .security_headers()                   // Security headers
            .enhanced_logging()                   // Security logging
            .build()
    );
    
    server.listen("0.0.0.0:3000").await?;
    Ok(())
}
```

### 📊 **Advanced ORM (Database Integration In Development)**
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

// Use the advanced query builder (ORM complete, DB integration in progress)
let users = User::query()
    .where_eq("active", true)
    .join("profiles", "users.id", "profiles.user_id")
    .select(&["users.*", "profiles.avatar"])
    .order_by("created_at")  
    .limit(10)
    .get(&pool)  // Database integration being refined
    .await?;
```

## 🏗️ **Architecture Overview**

```
elif.rs/ (Phase 4 Database Foundation In Progress)
├── crates/
│   ├── elif-core/         # ✅ Architecture foundation
│   │   ├── container/     # Dependency injection
│   │   ├── provider/      # Service providers
│   │   ├── module/        # Module system  
│   │   └── config/        # Configuration
│   │
│   ├── elif-http/         # ✅ Pure framework HTTP stack
│   │   ├── server/        # Framework-native server
│   │   ├── routing/       # Route handling
│   │   ├── middleware/    # Framework middleware
│   │   └── controller/    # Controller system
│   │
│   ├── elif-orm/          # ✅ ORM + 🚧 DB integration
│   │   ├── model/         # Model definitions
│   │   ├── query/         # Advanced query builder
│   │   ├── migration/     # Migration system (in development)
│   │   └── primary_key/   # Key handling
│   │
│   ├── elif-security/     # ✅ Complete security stack
│   │   ├── cors/          # ✅ CORS middleware
│   │   ├── csrf/          # ✅ CSRF protection
│   │   ├── rate_limit/    # ✅ Rate limiting
│   │   ├── validation/    # ✅ Input validation
│   │   ├── headers/       # ✅ Security headers
│   │   └── logging/       # ✅ Security logging
│   │
│   ├── elif-validation/   # ✅ Input validation
│   │
│   └── elif-cli/          # ✅ CLI tools
│
└── plan/                  # Development roadmap
    ├── phase1/            # ✅ COMPLETE
    ├── phase2/            # ✅ COMPLETE  
    ├── phase3/            # ✅ COMPLETE
    ├── phase4/            # 🚧 IN PROGRESS (database foundation)
    └── phase5-11/         # ❌ PLANNED (auth, advanced features)
```

**Legend**: ✅ Production-Ready | 🚧 In Development | ❌ Planned

## 🧪 **Testing Status**

```bash
# Note: Build currently failing due to database migration work
# Working to resolve architecture issues

# When building, you get:
cargo test --workspace                     # 335+ tests (when resolved)

# Test individual components  
cargo test -p elif-core                    # 33/33 tests ✅
cargo test -p elif-http                    # 112/112 tests ✅  
cargo test -p elif-orm                     # 39/39 tests ✅
cargo test -p elif-security                # 151/151 tests ✅
cargo test -p elif-validation              # Tests ✅

# Build status
cargo build --release                      # Currently failing - migration system fixes needed
```

## 📋 **Development Roadmap**

### ✅ **Completed Phases**
- **Phase 1**: Architecture Foundation (33 tests) ✅
- **Phase 2**: Web Foundation (112 tests) ✅
- **Phase 2.1**: Advanced ORM Foundation (39 tests) ✅
- **Phase 3**: Security & Framework Consistency (151 tests) ✅

### 🚧 **Current Work (Phase 4)**
- **Phase 4.1**: ✅ Database Service Integration (Complete)
- **Phase 4.2**: ✅ Basic Connection Pool Management (Complete)
- **Phase 4.7**: 🔄 Database Architecture Refactor (Active) 
- **Phase 4.4**: 📋 Basic Migration System (Pending)
- **Phase 4.5**: 📋 Model-Database Integration (Pending)
- **Phase 4.6**: 📋 Basic CRUD Operations (Pending)

### 📅 **Upcoming Phases**
- **Phase 5**: Authentication & Authorization (Issues #38-41)
- **Phase 6**: Advanced ORM relationships & caching
- **Phase 7**: Developer experience & CLI enhancements
- **Phase 8**: Production features (monitoring, deployment)
- **Phase 9**: Advanced features (WebSocket, files, email)
- **Phase 10-11**: Laravel/NestJS feature parity

**Track Progress**: [GitHub Project Board](https://github.com/users/krcpa/projects/1/views/1)

## ⚠️ **Important Notes**

### **This is Experimental Software**
- **Not ready for production use** - database integration in development
- **Build currently failing** - migration system refactoring in progress
- **APIs may change** as development continues
- **Use at your own risk** for experiments only

### **What's Solid**
- ✅ **Web Foundation**: HTTP server, routing, middleware all production-ready
- ✅ **Security Stack**: Complete CORS, CSRF, rate limiting, validation, headers
- ✅ **Architecture**: Pure framework design, no external types exposed
- ✅ **ORM Foundation**: Advanced query builder with 940+ lines of features
- ✅ **Test Coverage**: 335+ tests across completed components

### **What's In Progress**
- 🚧 **Database Integration**: Connecting ORM to actual databases
- 🚧 **Migration System**: Schema migration implementation
- 🚧 **Build Issues**: Resolving compilation errors in database layer

### **Best For**
- 🧪 **Experimentation** with mature Rust web architecture
- 🤖 **AI development** research and testing
- 📚 **Learning** modern security-first web frameworks
- 🔬 **Contributing** to framework development

## 🤖 **AI Agent Development**

elif.rs is specifically designed to work well with AI agents:

```bash
# 1. Plan: AI can understand the project structure
elifrs new my-experiment

# 2. Implement: AI works within safe MARKER zones  
# Safe code generation in predefined areas

# 3. Test: Comprehensive testing provides feedback
cargo test  # 335+ tests guide AI development (when building)

# 4. Iterate: Fast compilation enables rapid iteration
```

### **LLM-Friendly Features**
- **🛡️ MARKER Zones**: Safe areas for AI code modification
- **📋 Clear Specs**: Detailed specifications reduce AI confusion  
- **🔍 Introspection**: Built-in project understanding
- **⚡ Fast Feedback**: Quick compilation and testing
- **🏗️ Pure Architecture**: Consistent framework-native types

## 🤝 **Contributing**

elif.rs needs contributors! This is an active development project with solid foundations.

### **Current Priorities**
- **Phase 4.7**: Database Architecture Refactor (Issue #66) - **CRITICAL**
- **Phase 4.4**: Basic Migration System (Issue #63)
- **Phase 4.5**: Model-Database Integration (Issue #64)
- **Phase 4.6**: Basic CRUD Operations (Issue #65)

### **Development Setup**
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs

# Note: Build currently failing due to migration system work
# Check latest issues for current status
cargo build --workspace
```

### **How to Help**
1. **Fix build issues** in database migration system (high priority)
2. **Implement database integration** features from Phase 4 roadmap
3. **Improve documentation** and examples
4. **Add more tests** for database operations
5. **Share feedback** on the pure framework architecture

## 📊 **Current Stats**

- **Framework Status**: ⚠️ Experimental, build issues in database layer
- **Web Foundation**: ✅ Production-ready (112 tests)
- **Security Stack**: ✅ Complete (151 tests) 
- **ORM Foundation**: ✅ Advanced query builder (39 tests)
- **Database Integration**: 🚧 In development (architecture refactor)
- **Core Components**: ✅ 3.5/11 major phases complete  
- **Architecture**: ✅ Pure framework design implemented
- **Test Coverage**: ✅ 335+ tests (when building)
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

> *Phase 4 Database Foundation In Progress*  
> *Phase 1-3 ✅ Complete: Architecture + Web + Security (335+ tests)*  
> *Database Integration: Architecture Refactor Active*  
> *Build Status: ⚠️ Resolving Migration System Issues*  
> *Try: `cargo install elifrs` - Web/Security Stack Production-Ready*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>