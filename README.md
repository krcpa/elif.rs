# elif.rs

> Early preview of an LLM-friendly Rust web framework designed for AI agent-driven development

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/krcpa/elif.rs)
[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)
[![Downloads](https://img.shields.io/crates/d/elifrs.svg)](https://crates.io/crates/elifrs)

**elif.rs** is an LLM-friendly Rust web framework under active development that enables both human developers and AI agents to build web applications through structured, safe code generation. Try the early preview now!

## 🚀 **Get Started Now**

### Quick Installation

```bash
# Install the CLI globally
cargo install elifrs

# Create a new project
elifrs new my-web-app
cd my-web-app

# Build and run
cargo run
```

**That's it!** You now have a working web application with:
- ✅ **Dependency injection** container ready for services
- ✅ **Configuration management** with environment support  
- ✅ **Application lifecycle** with graceful startup/shutdown
- ✅ **Early preview ORM** with advanced query builder
- ✅ **Module system** for organizing features

## 🎯 **Why elif.rs?**

### **For Human Developers**
- **🏗️ Architecture-First**: Robust dependency injection and clean separation
- **⚡ High Performance**: 3μs query building, minimal memory overhead
- **🧪 Well-Tested**: 69+ tests ensuring reliability in early preview
- **📝 Clear APIs**: Intuitive, well-documented interfaces

### **For AI Agents**  
- **🤖 LLM-Optimized**: Early preview designed specifically for AI code generation
- **📋 Spec-Driven**: Configuration over convention approach
- **🔍 Introspective**: Built-in project understanding capabilities (in development)
- **🛡️ Safe Editing**: MARKER zones prevent AI from breaking core logic

## 📦 **Available Packages**

### CLI Tool (Early Preview)
```bash
cargo install elifrs  # Global CLI for project management
```

### Framework Crates (Early Preview)
```toml
[dependencies]
elif-core = "0.1.0"     # Architecture foundation  
elif-orm = "0.2.0"      # Database layer with advanced ORM (early preview)
```

## 🏃‍♂️ **Current Status: Active Development with Solid Foundation**

elif.rs is under **active development** with **two complete foundational layers** available for early preview:

### ✅ **Phase 1 Complete**: Architecture Foundation
- **🔧 Dependency Injection**: Robust DI container with service resolution
- **⚙️ Configuration Management**: Environment-based config with validation  
- **🔄 Application Lifecycle**: Graceful startup/shutdown with signal handling
- **📦 Module System**: Organize features with dependency resolution
- **✅ Status**: 33/33 tests passing, stable foundation

### ✅ **Phase 2.1 Complete**: Database & ORM Foundation  
- **📊 Advanced ORM**: Complete Model trait with CRUD operations
- **🔍 Query Builder**: Type-safe fluent API with 940+ lines of functionality
- **⚡ High Performance**: 3μs query building (excellent early performance)
- **🧪 Comprehensive Testing**: 36 unit tests + 6 performance benchmarks
- **🎯 Advanced Features**: Subqueries, aggregations, pagination, soft deletes
- **✅ Status**: All tests passing, available as early preview v0.2.0

### 🚧 **In Active Development**: 
- **Phase 2.2**: Connection pooling and transaction management
- **Phase 2.3**: Model events and observers  
- **Phase 2.4**: Database seeding and factory system

### 📅 **Coming Soon**:
- **Phase 3**: Authentication & Authorization (JWT, sessions, RBAC)  
- **Phase 4**: Developer experience tools (hot reload, introspection)
- **Phase 5**: Production features (monitoring, clustering)
- **Phase 6**: Advanced features (real-time, job queues, caching)

## 💡 **What Can You Try in the Early Preview?**

With the current foundation, you can experiment with:

### 🌐 **Web Applications**
```bash
elifrs new my-api
cd my-api
cargo run  # HTTP server on localhost:3000
```

### 📊 **Database-Driven Apps**
```rust
use elif_orm::*;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: Option<Uuid>,
    email: String,
    name: String,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

// Use the production-ready ORM
let users = User::query()
    .where_eq("active", true)
    .where_like("name", "%john%")
    .order_by("created_at")
    .limit(10)
    .get(&pool)
    .await?;
```

### 🏗️ **Service-Oriented Architecture**
```rust
use elif_core::*;

// Dependency injection with automatic service resolution
let app = Application::builder()
    .provider(DatabaseProvider)
    .provider(AuthProvider)
    .module(ApiModule)
    .module(WebModule)
    .build()?;

app.start().await?;  // Early preview lifecycle management
```

## 📚 **Quick Start Guide**

### Option 1: Try the Early Preview CLI (Recommended)
```bash
# Install the early preview globally
cargo install elifrs

# Create new project  
elifrs new my-project
cd my-project

# Everything is ready to experiment with
cargo run
```

### Option 2: Add to Existing Project
```toml
[dependencies]
elif-core = "0.1.0"    # Architecture foundation
elif-orm = "0.2.0"     # Early preview ORM
```

### Option 3: Explore the Source
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs
cargo test --workspace  # Run all 69+ tests
cargo run -p elif-api   # Example application
```

## 🏗️ Architecture Overview

elif.rs follows a modular, dependency-injection-based architecture:

```
elif.rs/
├── crates/
│   ├── core/           # 🟢 Architecture foundation (Phase 1)
│   │   ├── container/  # Dependency injection container
│   │   ├── provider/   # Service provider system  
│   │   ├── module/     # Module system & app lifecycle
│   │   └── config/     # Configuration management
│   │
│   ├── orm/            # 🟢 ORM foundation (Phase 2.1)
│   │   ├── model/      # Model trait with CRUD operations
│   │   ├── query/      # Advanced query builder
│   │   └── error/      # Error handling system
│   │
│   ├── auth/           # 🔴 Authentication (Phase 3)
│   ├── security/       # 🔴 Security middleware (Phase 3)
│   ├── cli/            # 🟡 Command line interface
│   └── codegen/        # 🔴 Code generation (Phase 4+)
│
├── apps/
│   └── api/            # Example API application
│
└── plan/               # Development roadmap & specifications
    ├── phase1/         # 🟢 Architecture (COMPLETE)
    ├── phase2/         # 🟡 Database layer (IN PROGRESS - 2.1 COMPLETE)
    └── phase3-6/       # 🔴 Future phases
```

**Legend**: 🟢 Complete | 🟡 In Progress | 🔴 Planned

## 🤖 **AI Agent Development**

elif.rs follows the **"Plan → Implement → Test → Deploy"** workflow optimized for AI agents:

```bash
# 1. Plan: Create project structure
elifrs new my-app

# 2. Implement: AI-safe development with MARKER zones
# Code generation works within predefined safe zones

# 3. Test: Comprehensive testing built-in
cargo test  # ✅ 69+ tests covering all functionality

# 4. Deploy: Production-ready from day one  
cargo run   # Graceful startup, shutdown, lifecycle management
```

### **LLM-Optimized Features**
- **🛡️ MARKER Zones**: Safe areas for AI code modification
- **📋 Spec-Driven**: Configuration over convention reduces AI confusion
- **🔍 Introspection**: Built-in APIs help AI understand project structure  
- **⚡ Fast Feedback**: 3μs query building means rapid iteration

## 🧪 **Testing & Performance**

```bash
# Run all tests - everything just works
cargo test --workspace                     # ✅ 69+ tests passing

# Performance benchmarks
cargo test -p elif-orm performance_tests   # 3μs query building
cargo test -p elif-core                    # 33/33 architecture tests

# Production build
cargo build --release                      # Clean compilation
```

### **Performance Results**
- **Query Building**: 3μs (333x better than 10ms target)
- **Memory Usage**: 208 bytes QueryBuilder overhead  
- **Model Instances**: 104 bytes each
- **Test Coverage**: 36 ORM tests + 33 architecture tests

## 🛠️ **Core APIs**

### **Dependency Injection** - Service Resolution Made Easy
```rust
use elif_core::*;

let app = Application::builder()
    .provider(DatabaseProvider)    // Auto-resolves dependencies
    .provider(AuthProvider)
    .module(ApiModule)             // Organizes features
    .build()?;

app.start().await?;                // Production-ready lifecycle
```

### **Advanced ORM** - Type-Safe Database Operations  
```rust
use elif_orm::*;

// Fluent query building
let active_users = User::query()
    .where_eq("active", true)
    .where_gt("created_at", last_month)
    .order_by_desc("last_login")
    .paginate(20)                  // Built-in pagination
    .get(&pool).await?;

// Advanced features work out of the box
let stats = User::query()
    .select_count("*", Some("total_users"))
    .select_avg("age", Some("avg_age")) 
    .group_by("country")
    .having_eq("COUNT(*)", 100)
    .get_raw(&pool).await?;        // Raw results for complex queries
```

### **Configuration** - Environment-Aware Settings
```rust
use elif_core::AppConfig;

// Automatic environment detection and validation
let config = AppConfig::from_env()?;
println!("Server running on port {}", config.server.port);
```

## 📋 Development Roadmap

### Phase 1: Architecture Foundation ✅ (Complete)
- [x] Dependency injection system  
- [x] Service provider lifecycle management
- [x] Module system with dependency resolution
- [x] Configuration management with validation
- [x] Application lifecycle and bootstrapping
- **Status**: All 33 core tests passing, solid foundation

### Phase 2.1: ORM Foundation ✅ (Complete)
- [x] Model trait with CRUD operations, timestamps, soft deletes
- [x] Type-safe query builder with fluent API (940+ lines)
- [x] Advanced query features: subqueries, aggregations, cursor pagination
- [x] Comprehensive testing: 36 unit tests, 6 performance benchmarks
- [x] Excellent performance: 3μs query building, 208 bytes memory overhead
- **Status**: Early preview ORM foundation with solid test coverage

### Phase 2.2-2.4: Database Layer 🚧 (Next)
- [ ] Connection pooling and transaction management  
- [ ] Model events and observers
- [ ] Database seeding and factory system

### Phase 3: Security Core 🔴 (Planned)
- [ ] Authentication system (JWT, session)
- [ ] Authorization with roles and permissions
- [ ] Input validation and sanitization
- [ ] Security middleware (CORS, CSRF, rate limiting)

### Phase 4-6: Developer Experience & Production Features 🔴 (Future)
- [ ] Hot reload and development tools
- [ ] Introspection APIs and project understanding
- [ ] Production monitoring and clustering
- [ ] Advanced features (real-time, jobs, caching)

**Track Progress**: [GitHub Project Board](https://github.com/users/krcpa/projects/1/views/1)

## 🤝 Contributing

elif.rs is built for the AI development community. Contributions welcome!

### Development Setup
```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs
cargo build --release
cargo test --workspace  # Ensure all tests pass
```

### Contribution Guidelines
1. **Phase-based development**: Focus on current phase (Phase 2: Database Layer)
2. **Test-driven**: All features must have comprehensive tests
3. **AI-friendly**: Code should be easily understood by LLMs
4. **Documentation**: Clear examples and inline documentation

### Current Priorities (Phase 2.2-2.4)
- Database connection pooling and transaction management (Issue #7)
- Model events and observers (Issue #11)
- Database seeding and factory system (Issue #12)

## 📊 Project Stats

- **Architecture**: ✅ Solid foundation (Phase 1)
- **ORM Foundation**: ✅ Early preview with advanced features (Phase 2.1)
- **Tests**: ✅ 69+ tests, all passing (36 ORM + 33 core)
- **Performance**: ✅ Excellent early results (3μs query building)
- **Build**: ✅ Clean compilation, minimal warnings
- **Documentation**: ✅ Comprehensive inline docs
- **AI Compatibility**: ✅ LLM-optimized code structure (early preview)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Repository**: [https://github.com/krcpa/elif.rs](https://github.com/krcpa/elif.rs)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Project Board**: [Development Roadmap](https://github.com/users/krcpa/projects/1/views/1)
- **Discussions**: [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)

---

**Built for the future of AI-driven development** 🤖

> *Early Preview Available - Try it now with `cargo install elifrs`*  
> *Phase 1 ✅ Complete: Architecture Foundation*  
> *Phase 2.1 ✅ Complete: ORM Foundation with 36 tests*  
> *Under Active Development: Phase 2.2 Connection Pooling & Transaction Management*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>