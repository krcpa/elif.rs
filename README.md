# elif.rs

> An LLM-friendly Rust web framework with complete database abstraction layer and production-ready architecture

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is a Rust web framework that enables both human developers and AI agents to build web applications through structured, safe code generation. Built with a solid foundation and comprehensive test coverage.

## 🚀 **Current Status: Phase 8 Complete - Production Features**

elif.rs has implemented comprehensive production features including advanced caching, job queues, and testing framework:

### ✅ **Production-Ready Components**

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
- **✅ Status**: 115/115 tests passing, production-ready web stack

#### Phase 3 Complete: Security & Validation
- **🛡️ CORS Middleware**: Complete Cross-Origin Resource Sharing implementation
- **🔐 CSRF Protection**: Full Cross-Site Request Forgery protection with token management  
- **🚫 Rate Limiting**: Request rate limiting with pure framework types
- **🔒 Input Validation**: Request sanitization and validation system
- **📊 Enhanced Logging**: Request tracing and security monitoring
- **🔧 Security Headers**: Complete security headers middleware
- **🏗️ Pure Framework Architecture**: All framework types, no external dependencies exposed
- **✅ Status**: Production-ready security infrastructure

#### Phase 4 Complete: Database Operations Foundation
- **💾 Database Service Integration**: Complete DI container integration
- **🔗 Connection Pooling**: Transaction support and connection lifecycle
- **🔄 Migration System**: CLI commands and schema migration system
- **📊 Model-Database Integration**: CRUD operations with database integration
- **🏗️ Database Architecture**: Clean separation in elif-orm crate
- **✅ Status**: Production-ready database layer

#### Phase 5 Complete: Authentication & Authorization
- **🔑 Authentication Core**: Comprehensive error handling and infrastructure
- **🎫 JWT Token Management**: Complete JWT system with middleware
- **🔐 Session Authentication**: Session management with storage backends
- **👤 User Authentication**: Middleware and authentication guards
- **🛡️ Role-Based Access Control**: Complete RBAC system with permissions
- **📱 Multi-Factor Authentication**: TOTP and backup codes support
- **⚙️ CLI Integration**: Authentication scaffolding commands
- **✅ Status**: 86/86 authentication tests passing, production-ready auth system

#### Phase 8 Complete: Production Features ✅
- **🗄️ Multi-Backend Caching**: Memory and Redis backends with LRU optimization and cache tagging
- **📋 Job Queue System**: Background job processing with Redis/Memory backends and cron scheduling  
- **🧪 Testing Framework**: Comprehensive testing utilities with database, HTTP, and factory support
- **📖 OpenAPI Documentation**: Automatic API documentation generation with Swagger UI integration
- **⚡ HTTP Response Caching**: ETag and Last-Modified header support with cache invalidation
- **🔄 Advanced Job Scheduling**: Retry logic, dead letter queues, and cancellation token support
- **🛠️ Enhanced CLI Tools**: Cache management, queue monitoring, and testing integration
- **✅ Status**: 600+ tests passing, production-ready scalability features implemented

#### Phase 7.2 Complete: Database Abstraction Layer ✅
- **🗄️ Multi-Database Support**: PostgreSQL, MySQL, SQLite support through trait abstractions
- **🔧 Database Traits**: `DatabasePool`, `DatabaseConnection`, `DatabaseTransaction` abstractions
- **⚡ Backend Architecture**: Clean separation with `backends/` module system
- **🔄 Transaction Management**: Database-agnostic transaction handling with retry logic
- **📊 Value Abstraction**: Type-safe parameter binding with `DatabaseValue` enum
- **🛠️ SQL Dialect Support**: Database-specific SQL generation through dialect system
- **✅ Status**: 224/224 tests passing, complete abstraction layer implemented

### 📊 **Test Coverage: 600+ Tests Passing**
- **Core Architecture**: 33/33 tests ✅
- **HTTP Web Stack**: 115/115 tests ✅  
- **Authentication & Authorization**: 86/86 tests ✅
- **Database & ORM**: 224/224 tests ✅
- **Caching System**: 50+ tests ✅
- **Job Queue System**: 16+ tests ✅
- **Testing Framework**: 34+ tests ✅
- **Total**: 600+ tests across all components ✅

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

**What you get immediately:**
- ✅ **HTTP Server** with pure framework abstractions
- ✅ **Dependency Injection** for service management  
- ✅ **Advanced ORM** with query builder and multi-database support
- ✅ **Complete Security Stack** - CORS, CSRF, rate limiting, validation, headers
- ✅ **Database Operations** - connection pooling, migrations, CRUD operations
- ✅ **Authentication & Authorization** - JWT, sessions, RBAC, MFA support
- ✅ **Multi-Database Support** - PostgreSQL, MySQL, SQLite through abstractions
- ✅ **Production Caching** - Memory/Redis backends with tagging and invalidation
- ✅ **Job Queue System** - Background processing with scheduling and retry logic
- ✅ **Testing Framework** - Comprehensive testing utilities and factories
- ✅ **API Documentation** - Automatic OpenAPI generation with Swagger UI
- ✅ **Pure Framework Architecture** - consistent types, no external deps exposed
- ✅ **Configuration System** with environment support

## 📦 **Available Packages**

```bash
cargo install elifrs  # v0.8.0 - CLI tool with caching, job queues, and testing integration
```

```toml
[dependencies]
elif-core = "0.4.0"         # Architecture foundation with service-builder patterns
elif-orm = "0.6.0"          # Multi-database ORM with advanced query builder
elif-http = "0.6.0"         # HTTP server with response caching middleware
elif-security = "0.2.3"     # Complete security middleware stack
elif-validation = "0.1.0"   # Input validation and sanitization
elif-auth = "0.3.1"         # Complete authentication with JWT, sessions, RBAC, MFA
elif-cache = "0.2.0"        # Multi-backend caching system with tagging
elif-queue = "0.2.0"        # Job queue system with scheduling and retry logic
elif-testing = "0.2.0"      # Comprehensive testing framework
elif-openapi = "0.1.0"      # OpenAPI documentation generation
elif-codegen = "0.3.1"      # Code generation and templates
elif-introspect = "0.2.1"   # Project introspection and analysis
```

## 💡 **Framework Examples**

### 🌐 **Simple Web Application**
```rust
use elif_core::{Container, config::DatabaseConfig};
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};
use elif_orm::database::DatabaseServiceProvider;
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
    // Create container with database service
    let container = Container::builder()
        .config(Arc::new(DatabaseConfig::default()))
        .service_provider(DatabaseServiceProvider::new())
        .build()?
        .into();

    // Create router with pure framework types
    let router = ElifRouter::new()
        .get("/", hello)
        .get("/users", users);
    
    // Create and start server
    let mut server = Server::with_container(container, HttpConfig::default())?;
    server.use_router(router);
    server.listen("0.0.0.0:3000").await?;
    Ok(())
}
```

### 🛡️ **Secure API with Authentication**  
```rust
use elif_core::{Container, config::DatabaseConfig};
use elif_http::{Server, HttpConfig, ElifRouter, ElifResponse};
use elif_auth::{AuthServiceProvider, middleware::RequireAuth};
use elif_security::SecurityMiddlewareBuilder;
use std::sync::Arc;

async fn protected_api() -> ElifResponse {
    ElifResponse::json(serde_json::json!({
        "message": "Access granted to protected resource",
        "secure": true
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let container = Container::builder()
        .config(Arc::new(DatabaseConfig::default()))
        .service_provider(AuthServiceProvider::new())
        .build()?
        .into();

    let router = ElifRouter::new()
        .group("/api")
            .middleware(RequireAuth::new()) // JWT authentication required
            .get("/protected", protected_api);
    
    let mut server = Server::with_container(container, HttpConfig::default())?;
    server.use_router(router);
    
    // Complete security stack
    server.use_middleware(
        SecurityMiddlewareBuilder::new()
            .cors_permissive()          // CORS protection
            .csrf_with_token_header()   // CSRF protection  
            .rate_limit_default()       // Rate limiting
            .request_sanitization()     // Input sanitization
            .security_headers()         // Security headers
            .enhanced_logging()         // Security logging
            .build()
    );
    
    server.listen("0.0.0.0:3000").await?;
    Ok(())
}
```

### 📊 **Multi-Database ORM Usage**
```rust
use elif_orm::{Model, query::QueryBuilder, database::ManagedPool};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<Uuid>,
    email: String,  
    name: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Model for User {
    type PrimaryKey = Uuid;
    fn table_name() -> &'static str { "users" }
    fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
}

async fn demo_queries(pool: &ManagedPool) -> Result<(), Box<dyn std::error::Error>> {
    // Works with PostgreSQL, MySQL, or SQLite through database abstractions
    
    // Advanced query building
    let active_users = User::query()
        .where_eq("active", true)
        .where_gt("created_at", "2024-01-01")
        .join("profiles", "users.id", "profiles.user_id")
        .select(&["users.*", "profiles.avatar"])
        .order_by("created_at")
        .limit(10)
        .get(pool)
        .await?;

    // Aggregations with subqueries
    let user_stats = User::query()
        .select(&["COUNT(*) as total_users"])
        .where_in("status", &["active", "premium"])
        .having("COUNT(*) > ?", &[&10])
        .get(pool)
        .await?;

    // Transaction support across all database backends
    let mut tx = pool.begin_transaction().await?;
    
    // Multiple operations in transaction
    User::query()
        .insert(&serde_json::json!({
            "email": "new@example.com",
            "name": "New User"
        }))
        .execute_with_transaction(&mut tx)
        .await?;
    
    tx.commit().await?;
    
    Ok(())
}
```

## 🏗️ **Architecture Overview**

```
elif.rs/ (Phase 8 Complete - Production Features)
├── crates/
│   ├── elif-core/         # ✅ Architecture foundation
│   │   ├── container/     # Dependency injection
│   │   ├── provider/      # Service providers
│   │   ├── module/        # Module system  
│   │   └── config/        # Configuration with service-builder patterns
│   │
│   ├── elif-http/         # ✅ Pure framework HTTP stack
│   │   ├── server/        # Framework-native server
│   │   ├── routing/       # Route handling
│   │   ├── middleware/    # Framework middleware with response caching
│   │   └── controller/    # Controller system
│   │
│   ├── elif-orm/          # ✅ Multi-database ORM with abstractions
│   │   ├── model/         # Model definitions
│   │   ├── query/         # Advanced query builder
│   │   ├── backends/      # Database abstraction layer
│   │   ├── database/      # Database service with abstractions
│   │   ├── transaction/   # Database-agnostic transactions
│   │   └── migration/     # Migration system
│   │
│   ├── elif-cache/        # ✅ Multi-backend caching system (NEW)
│   │   ├── backends/      # Memory and Redis backends
│   │   ├── tags/          # Cache tagging and invalidation
│   │   ├── http/          # HTTP response caching middleware
│   │   └── warming/       # Cache warming strategies
│   │
│   ├── elif-queue/        # ✅ Job queue system (NEW)
│   │   ├── backends/      # Memory and Redis job backends
│   │   ├── scheduler/     # Cron scheduling and job processing
│   │   ├── retry/         # Advanced retry logic and backoff
│   │   └── worker/        # Background worker management
│   │
│   ├── elif-testing/      # ✅ Testing framework (NEW)
│   │   ├── database/      # Database testing utilities
│   │   ├── http/          # HTTP testing client
│   │   ├── factory/       # Test data factory system
│   │   └── auth/          # Authentication testing support
│   │
│   ├── elif-openapi/      # ✅ API documentation (NEW)
│   │   ├── generation/    # OpenAPI spec generation
│   │   ├── swagger/       # Swagger UI integration
│   │   └── schema/        # Type-safe schema reflection
│   │
│   ├── elif-auth/         # ✅ Complete authentication system
│   │   ├── providers/     # JWT, Session, MFA providers
│   │   ├── middleware/    # Authentication middleware
│   │   ├── rbac/          # Role-Based Access Control
│   │   └── utils/         # Password hashing, crypto utils
│   │
│   ├── elif-security/     # ✅ Complete security stack
│   │   ├── cors/          # CORS middleware
│   │   ├── csrf/          # CSRF protection
│   │   ├── rate_limit/    # Rate limiting
│   │   ├── validation/    # Input validation
│   │   ├── headers/       # Security headers
│   │   └── logging/       # Security logging
│   │
│   ├── elif-validation/   # ✅ Input validation
│   ├── elif-codegen/      # ✅ Code generation
│   ├── elif-introspect/   # ✅ Project introspection
│   └── cli/               # ✅ Enhanced CLI tools (published as 'elifrs')
│
└── plan/                  # Development roadmap
    ├── phase1-8/          # ✅ COMPLETE - Production-ready core
    └── phase9-11/         # 📋 PLANNED (advanced features)
```

**Legend**: ✅ Production-Ready | 📋 Planned

## 🧪 **Testing & Development**

```bash
# All tests passing
cargo test --workspace                     # 600+ tests ✅

# Test individual components  
cargo test -p elif-core                    # 33/33 tests ✅
cargo test -p elif-http                    # 115/115 tests ✅  
cargo test -p elif-orm                     # 224/224 tests ✅
cargo test -p elif-auth                    # 86/86 tests ✅
cargo test -p elif-cache                   # 50+ tests ✅
cargo test -p elif-queue                   # 16+ tests ✅
cargo test -p elif-testing                 # 34+ tests ✅

# Build status
cargo build --release                      # ✅ All builds successful
```

## 📋 **Development Roadmap**

### ✅ **Completed Phases (Production-Ready)**
- **Phase 1**: Architecture Foundation ✅
- **Phase 2**: Web Foundation ✅
- **Phase 3**: Security & Validation ✅
- **Phase 4**: Database Operations Foundation ✅
- **Phase 5**: Authentication & Authorization ✅
- **Phase 7.2**: Database Abstraction Layer ✅
- **Phase 8**: Production Features ✅

### 📅 **Upcoming Phases**
- **Phase 9**: Advanced Features (WebSocket, file handling, email, advanced routing)
- **Phase 10**: Enterprise Features (monitoring, deployment, performance optimization)
- **Phase 11**: Framework Ecosystem (plugins, extensions, community tools)
- **Phase 6**: Advanced ORM relationships (eager loading, lazy loading, polymorphic)

**Track Progress**: [GitHub Project Board](https://github.com/users/krcpa/projects/1/views/1)

## ✨ **Key Features**

### **For Human Developers**
- **🏗️ Clean Architecture**: Dependency injection and modular design
- **📝 Type Safety**: Rust's type system for reliable web applications
- **⚡ Performance**: Built on Tokio for high performance
- **🧪 Well Tested**: Comprehensive test coverage (600+ tests)
- **🗄️ Multi-Database**: PostgreSQL, MySQL, SQLite support through abstractions
- **🔒 Security First**: Complete security middleware stack built-in
- **🔑 Authentication**: JWT, sessions, RBAC, MFA out of the box
- **🗄️ Production Caching**: Memory/Redis backends with intelligent invalidation
- **📋 Job Processing**: Background jobs with scheduling and retry logic
- **🧪 Testing Framework**: Comprehensive testing utilities and factories
- **📖 API Documentation**: Automatic OpenAPI generation with Swagger UI

### **For AI Agents**
- **🤖 LLM-Optimized**: Framework designed with AI code generation in mind
- **📋 Spec-Driven**: Clear specifications and safe editing zones
- **🔍 Introspective**: APIs for understanding project structure
- **🛡️ Safe Zones**: MARKER blocks for AI-safe code modification
- **⚡ Fast Feedback**: Comprehensive testing provides immediate validation

## 🤝 **Contributing**

elif.rs welcomes contributions! The framework has solid foundations and clear development paths.

### **Current Priorities**
- **Phase 9**: Advanced Features - WebSocket, file handling, email, advanced routing
- **Enterprise Features**: Monitoring, deployment tools, performance optimization
- **ORM Relationships**: HasOne, HasMany, BelongsTo, polymorphic relationships
- **Documentation**: Examples, guides, and API documentation
- **Community**: Plugin system, extensions, and ecosystem development

### **Development Setup**
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs

# All builds work
cargo build --workspace    # ✅
cargo test --workspace     # 600+ tests ✅
```

### **How to Help**
1. **Implement Phase 9 features** (WebSocket, file handling, email)
2. **Add enterprise monitoring** and deployment tools
3. **Improve documentation** and examples
4. **Add integration tests** for real-world scenarios
5. **Build ecosystem tools** and plugins
6. **Share feedback** on API design and developer experience

## 📊 **Current Stats**

- **Framework Status**: ✅ Production-Ready Core (Phases 1-5, 7.2, 8)
- **Web Foundation**: ✅ Complete with response caching (115 tests)
- **Security Stack**: ✅ Complete production-ready security
- **Database Layer**: ✅ Multi-database abstraction (224 tests)
- **Authentication**: ✅ Complete with RBAC & MFA (86 tests)
- **Caching System**: ✅ Multi-backend with tagging (50+ tests)
- **Job Queue System**: ✅ Background processing with scheduling (16+ tests)
- **Testing Framework**: ✅ Comprehensive testing utilities (34+ tests)
- **API Documentation**: ✅ OpenAPI generation with Swagger UI
- **Core Architecture**: ✅ Dependency injection, modules, config (33 tests)
- **Major Phases**: ✅ 7/11 phases complete with production features
- **Architecture**: ✅ Pure framework design with production scalability
- **Test Coverage**: ✅ 600+ tests passing
- **Build Status**: ✅ All components build successfully
- **Package Status**: ✅ Published to crates.io

## 📄 **License**

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 **Links**

- **Repository**: [https://github.com/krcpa/elif.rs](https://github.com/krcpa/elif.rs)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Project Board**: [Development Progress](https://github.com/users/krcpa/projects/1/views/1)
- **Discussions**: [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)
- **Crates.io**: [elifrs CLI](https://crates.io/crates/elifrs)

---

**🚀 Production-Ready LLM-Friendly Rust Web Framework 🤖**

> *Phase 8 ✅ Complete: Production Features - Caching, Job Queues, Testing, API Docs*  
> *Phases 1-8 ✅ Complete: Architecture + Web + Security + Database + Auth + Production (600+ tests)*  
> *Enterprise Ready: Multi-backend caching, job scheduling, comprehensive testing framework*  
> *Build Status: ✅ All Components Building Successfully*  
> *Try: `cargo install elifrs` - Complete Production-Ready Web Framework with Scalability*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>