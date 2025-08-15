# elif.rs

> An LLM-friendly Rust web framework under active development, designed for AI agent-driven development

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is an experimental Rust web framework under active development that enables both human developers and AI agents to build web applications through structured, safe code generation. This is an early-stage project with solid foundations being built iteratively.

## 🚧 **Current Status: Phase 6.2.5 - Type-Safe Relationship Loading**

elif.rs is in **active development** with Phase 5 Authentication & Authorization complete, now building advanced ORM relationships:

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

#### Phase 4 Complete: Database Operations Foundation
- **💾 Database Service Integration**: Complete DI container integration
- **🔗 Connection Pooling**: Transaction support and connection lifecycle
- **🔄 Migration System**: CLI commands and schema migration system
- **📊 Model-Database Integration**: CRUD operations with database integration
- **🏗️ Database Architecture**: Clean separation in elif-orm crate
- **✅ Status**: 79/79 database tests passing, production-ready database layer

#### Phase 5 Complete: Authentication & Authorization
- **🔑 Authentication Core**: Comprehensive error handling and infrastructure
- **🎫 JWT Token Management**: Complete JWT system with middleware
- **🔐 Session Authentication**: Session management with storage backends
- **👤 User Authentication**: Middleware and authentication guards
- **🛡️ Role-Based Access Control**: Complete RBAC system with permissions
- **📱 Multi-Factor Authentication**: TOTP and backup codes support
- **⚙️ CLI Integration**: Authentication scaffolding commands
- **✅ Status**: 86/86 authentication tests passing, production-ready auth system

#### Phase 6.1 Complete: Relationship System Core
- **🔗 Relationship Trait System** - Base types and relationship foundation ✅
- **👥 HasOne and HasMany** - One-to-one and one-to-many relationships ✅
- **🔄 BelongsTo and BelongsToMany** - Inverse relationships and many-to-many ✅
- **🌐 HasManyThrough** - Relationships through intermediate models ✅
- **🔀 Polymorphic Relationships** - MorphTo, MorphMany foundation ✅
- **🔁 Self-Referencing** - Models that reference themselves ✅
- **⚙️ Relationship Constraints** - Cascading and constraint handling ✅
- **✅ Status**: Core relationship system complete with all patterns

#### Phase 6.2.5 Complete: Type-Safe Relationship Loading
- **🎯 Type-Safe Containers** - Generic TypeSafeRelationship<T> with compile-time safety ✅
- **📦 Specialized Types** - HasOne<T>, HasMany<T>, ManyToMany<T>, BelongsTo<T> ✅
- **🔄 Loading States** - NotLoaded, Loading, Loaded, Failed with proper tracking ✅
- **🧬 Polymorphic Support** - MorphOne<T>, MorphMany<T>, MorphTo implementations ✅
- **💧 Hydration System** - Type-safe model hydration with RelationshipHydrator trait ✅
- **🔍 Inference Engine** - Automatic foreign key and table name inference ✅
- **⚡ Eager Loading** - TypeSafeEagerLoader for efficient relationship loading ✅
- **✅ Status**: All tests passing, zero runtime type casting achieved

### 🚧 **Currently Working On**

#### Phase 6.2.6: Unified Caching System (Next)
- **🗄️ Query Result Caching** - Cache loaded relationships to avoid N+1 queries
- **♻️ Identity Map Pattern** - Ensure single instance per entity
- **🔄 Cache Invalidation** - Smart invalidation on updates
- **📊 Memory Management** - Efficient memory usage for large datasets

**Goal**: Implement comprehensive caching to optimize relationship loading performance

### 📊 **Test Coverage: 500+ Tests (When Building)**
- **Core Architecture**: 33 tests ✅
- **HTTP Web Stack**: 112 tests ✅  
- **ORM Foundation**: 39 tests ✅
- **Security & Validation**: 151 tests ✅
- **Database Operations**: 79 tests ✅
- **Authentication & Authorization**: 86 tests ✅
- **Relationship System**: In development 🚧
- **Total**: 500+ tests across completed phases

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
- ✅ **Database Operations** - connection pooling, migrations, CRUD operations
- ✅ **Authentication & Authorization** - JWT, sessions, RBAC, MFA support
- ✅ **Pure Framework Architecture** - consistent types, no external deps exposed
- ✅ **Configuration System** with environment support
- 🚧 **ORM Relationships** - relationship system in development  
- ⚠️ **Experimental** - solid foundation, relationship system has build issues

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
cargo install elifrs  # v0.5.2 - CLI tool with migration system and scaffolding
```

```toml
[dependencies]
elif-core = "0.2.1"         # Architecture foundation with database error support
elif-orm = "0.5.1"          # Database layer with advanced query builder and CRUD
elif-http = "0.2.0"         # HTTP server with pure framework abstractions
elif-security = "0.2.1"     # Complete security middleware stack
elif-validation = "0.1.0"   # Input validation and sanitization
elif-auth = "0.3.0"         # Complete authentication with JWT, sessions, RBAC, MFA
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
# Note: Build currently failing due to relationship system work
# Working to resolve type integration issues

# When building, you get:
cargo test --workspace                     # 500+ tests (when resolved)

# Test individual components  
cargo test -p elif-core                    # 33/33 tests ✅
cargo test -p elif-http                    # 112/112 tests ✅  
cargo test -p elif-orm                     # 39/39 tests ✅ (base ORM)
cargo test -p elif-security                # 151/151 tests ✅
cargo test -p elif-validation              # Tests ✅
cargo test -p elif-auth                    # 86/86 tests ✅

# Build status
cargo build --release                      # Currently failing - relationship system fixes needed
```

## 📋 **Development Roadmap**

### ✅ **Completed Phases**
- **Phase 1**: Architecture Foundation (33 tests) ✅
- **Phase 2**: Web Foundation (112 tests) ✅
- **Phase 2.1**: Advanced ORM Foundation (39 tests) ✅
- **Phase 3**: Security & Framework Consistency (151 tests) ✅
- **Phase 4**: Database Operations Foundation (79 tests) ✅
- **Phase 5**: Authentication & Authorization (86 tests) ✅

### 🚧 **Current Work (Phase 6.1)**
- **Phase 6.1**: 🔄 Relationship System Core (Active - Issue #83)
  - **Relationship Trait System** - Base types and foundation
  - **HasOne/HasMany** - One-to-one and one-to-many relationships
  - **BelongsTo/BelongsToMany** - Inverse and many-to-many relationships
  - **HasManyThrough** - Relationships through intermediate models
  - **Polymorphic Foundation** - MorphTo, MorphMany basic support

### 📅 **Upcoming Phases**
- **Phase 6.2-6.6**: Advanced ORM relationships - eager loading, lazy loading, events, queries, polymorphic
- **Phase 7**: Developer experience & CLI enhancements (Issues #75-81)
- **Phase 8**: Production features (monitoring, deployment)
- **Phase 9**: Advanced features (WebSocket, files, email)
- **Phase 10-11**: Laravel/NestJS feature parity

**Track Progress**: [GitHub Project Board](https://github.com/users/krcpa/projects/1/views/1)

## ⚠️ **Important Notes**

### **This is Experimental Software**
- **Not ready for production use** - relationship system in development
- **Build currently failing** - relationship system type integration issues
- **APIs may change** as development continues
- **Use at your own risk** for experiments only

### **What's Solid**
- ✅ **Web Foundation**: HTTP server, routing, middleware all production-ready
- ✅ **Security Stack**: Complete CORS, CSRF, rate limiting, validation, headers
- ✅ **Database Layer**: Connection pooling, migrations, CRUD operations working
- ✅ **Authentication**: Complete JWT, sessions, RBAC, MFA system
- ✅ **Architecture**: Pure framework design, no external types exposed
- ✅ **ORM Foundation**: Advanced query builder with 940+ lines of features
- ✅ **Test Coverage**: 500+ tests across completed components

### **What's In Progress**
- 🚧 **ORM Relationships**: HasOne, HasMany, BelongsTo, polymorphic relationships
- 🚧 **Type System**: Resolving relationship trait integration
- 🚧 **Build Issues**: Resolving compilation errors in relationship system

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
- **Phase 6.1**: Relationship System Core (Issue #83) - **CRITICAL**
- **Type System Integration**: Fix relationship trait compilation errors
- **Relationship API**: Complete HasOne, HasMany, BelongsTo implementations  
- **Testing Framework**: Add comprehensive relationship tests

### **Development Setup**
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs

# Note: Build currently failing due to relationship system work
# Check latest issues for current status
cargo build --workspace
```

### **How to Help**
1. **Fix build issues** in relationship system (high priority)
2. **Implement relationship types** from Phase 6.1 roadmap
3. **Add relationship tests** and integration examples
4. **Improve documentation** and relationship examples
5. **Share feedback** on the ORM relationship API design

## 📊 **Current Stats**

- **Framework Status**: ⚠️ Experimental, build issues in relationship system
- **Web Foundation**: ✅ Production-ready (112 tests)
- **Security Stack**: ✅ Complete (151 tests) 
- **Database Layer**: ✅ Production-ready (79 tests)
- **Authentication**: ✅ Complete with RBAC & MFA (86 tests)
- **ORM Foundation**: ✅ Advanced query builder (39 tests)
- **Relationship System**: 🚧 In development (type integration issues)
- **Core Components**: ✅ 5.1/11 major phases complete  
- **Architecture**: ✅ Pure framework design implemented
- **Test Coverage**: ✅ 500+ tests (when building)
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

> *Phase 6.1 Relationship System Core In Progress*  
> *Phase 1-5 ✅ Complete: Architecture + Web + Security + Database + Auth (500+ tests)*  
> *ORM Relationships: Type System Integration Active*  
> *Build Status: ⚠️ Resolving Relationship Compilation Issues*  
> *Try: `cargo install elifrs` - Complete Web/Auth/Database Stack Available*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>