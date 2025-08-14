# elif.rs

> LLM-friendly Rust web framework designed for AI agent-driven development

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/krcpa/elif.rs)

**elif.rs** is a spec-first, AI-agent-optimized web framework designed to enable AI agents (like Claude) to build complex web applications through safe, structured code generation with MARKER-based editing zones.

## 🚧 Current Status: Phase 1 Complete (Architecture Foundation)

**elif.rs is in active development.** Phase 1 (Architecture Foundation) is now complete with all core systems implemented and tested.

### ✅ Completed (Phase 1: Architecture Foundation)
- **Dependency Injection System**: Complete DI container using `service-builder` crate
- **Service Provider System**: Lifecycle management, dependency resolution, boot ordering
- **Module System**: Feature organization with dependency resolution and topological sorting  
- **Configuration Management**: Environment-based config with validation, hot-reload support
- **Application Lifecycle**: Startup/shutdown management, signal handling, lifecycle hooks

### 🚧 In Development (Phase 2: Database Layer)
- Full ORM with relationships and query builder
- Connection pooling and transaction management  
- Model events and observers
- Database seeding and factory system

### 📋 Planned (Phase 3-6)
- Authentication & Authorization (JWT, sessions, RBAC)
- Security middleware (CORS, CSRF, rate limiting)  
- Developer experience tools (hot reload, introspection APIs)
- Production features (monitoring, clustering, deployment)
- Advanced features (real-time, job queues, caching)

## 🎯 Why elif.rs?

Traditional web frameworks are designed for human developers. **elif.rs** is specifically designed for AI agents:

- **🤖 AI-Safe Architecture**: Robust dependency injection and lifecycle management
- **📝 Spec-First Development**: Configuration-driven architecture
- **⚡ Modular Design**: Plugin system for extensible functionality
- **🔧 LLM-Optimized**: Clear separation of concerns and predictable patterns
- **🔍 Introspective**: Built-in project understanding capabilities (planned)

## 🚀 Quick Start

### 1. Prerequisites

- Rust 1.70+
- Git

### 2. Clone and Build

```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs
cargo build --release
```

### 3. Run Tests

```bash
cargo test --workspace
```

### 4. Explore the Architecture

```bash
# Check core functionality
cargo test -p elif-core

# View project structure
find crates -name "*.rs" | head -20
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
│   ├── orm/            # 🟡 Database layer (Phase 2)
│   ├── auth/           # 🔴 Authentication (Phase 3)
│   ├── security/       # 🔴 Security middleware (Phase 3)
│   ├── cli/            # 🟡 Command line interface
│   └── codegen/        # 🔴 Code generation (Phase 4+)
│
├── apps/
│   └── api/            # Example API application
│
└── plan/               # Development roadmap & specifications
    ├── phase1/         # ✅ Architecture (COMPLETE)
    ├── phase2/         # 🟡 Database layer (IN PROGRESS)
    └── phase3-6/       # 🔴 Future phases
```

**Legend**: 🟢 Complete | 🟡 In Progress | 🔴 Planned

## 🤖 AI Agent Development Model

elif.rs is designed for the **"Plan → Implement → Test → Deploy"** AI workflow:

### 1. **Plan**: Architecture-First Design
```rust
// Phase 1: Define application structure
use elif_core::{Application, Module, ServiceProvider};

let app = Application::builder()
    .provider(DatabaseProvider)
    .provider(AuthProvider) 
    .module(ApiModule)
    .module(WebModule)
    .build()?;
```

### 2. **Implement**: Module-Based Development
```rust
// Phase 2: Implement feature modules
pub struct BlogModule;

impl Module for BlogModule {
    fn name(&self) -> &'static str { "blog" }
    
    fn configure(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ModuleError> {
        // Configure services for this module
        Ok(builder.service(BlogService::new()))
    }
    
    fn routes(&self) -> Vec<RouteDefinition> {
        vec![
            RouteDefinition::new(HttpMethod::GET, "/posts", "list_posts"),
            RouteDefinition::new(HttpMethod::POST, "/posts", "create_post"),
        ]
    }
}
```

### 3. **Test**: Comprehensive Testing
```bash
# All tests pass with full coverage
cargo test --workspace  # ✅ 35+ tests passing
```

### 4. **Deploy**: Production-Ready
```rust
// Phase 1: Application lifecycle management
app.start().await?;  // Graceful startup
// ... handle requests ...
app.shutdown().await?;  // Graceful shutdown
```

## 🧪 Testing & Development

### Running Tests
```bash
# Core architecture tests
cargo test -p elif-core                    # ✅ 33/33 tests passing

# All workspace tests  
cargo test --workspace                     # ✅ All tests passing

# Specific test suites
cargo test -p elif-core -- module::tests  # Module system tests
cargo test -p elif-core -- provider::tests # Provider system tests
```

### Code Quality
```bash
# Check code formatting
cargo fmt --check

# Run clippy linting
cargo clippy -- -D warnings

# Build with optimizations
cargo build --release
```

## 🔧 Current Implementation Details

### Dependency Injection (Phase 1.1 ✅)
```rust
use elif_core::{Container, ContainerBuilder};

// Service registration with automatic dependency resolution
let container = Container::builder()
    .config(app_config)
    .database(database_connection)
    .build()?;

let config = container.config();
let db = container.database();
```

### Service Providers (Phase 1.2 ✅)
```rust
use elif_core::{ServiceProvider, ProviderRegistry};

pub struct DatabaseProvider;

impl ServiceProvider for DatabaseProvider {
    fn name(&self) -> &'static str { "database" }
    
    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError> {
        let db = create_database_connection()?;
        Ok(builder.database(Arc::new(db)))
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["config"]  // Depends on config provider
    }
}
```

### Module System (Phase 1.3 ✅)
```rust
use elif_core::{Module, ModuleRegistry, RouteDefinition, HttpMethod};

pub struct ApiModule;

impl Module for ApiModule {
    fn name(&self) -> &'static str { "api" }
    
    fn routes(&self) -> Vec<RouteDefinition> {
        vec![
            RouteDefinition::new(HttpMethod::GET, "/health", "health_check")
                .with_description("Health check endpoint"),
        ]
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["auth"]  // Requires auth module
    }
}
```

### Configuration Management (Phase 1.4 ✅)
```rust
use elif_core::{AppConfig, Environment, AppConfigTrait};

// Environment-based configuration with validation
let config = AppConfig::from_env()?;
assert_eq!(config.environment, Environment::Development);
assert_eq!(config.server.port, 3000);

// Configuration validation
config.validate()?;  // Ensures all required fields are present
```

### Application Lifecycle (Phase 1.5 ✅)
```rust
use elif_core::{Application, ApplicationState, LifecycleHook};

// Custom lifecycle hooks
pub struct DatabaseMigrationHook;

impl LifecycleHook for DatabaseMigrationHook {
    fn name(&self) -> &'static str { "database_migration" }
    
    fn before_start<'life0, 'async_trait>(
        &'life0 self,
        container: &'life0 Container,
    ) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Run database migrations before app starts
            run_migrations(container.database()).await?;
            Ok(())
        })
    }
}

// Application with lifecycle management
let mut app = Application::builder()
    .provider(DatabaseProvider)
    .module(ApiModule)
    .lifecycle_hook(DatabaseMigrationHook)
    .build()?;

// Graceful startup and shutdown
app.start().await?;
assert_eq!(app.state(), &ApplicationState::Running);

app.shutdown().await?;
assert_eq!(app.state(), &ApplicationState::Stopped);
```

## 📋 Development Roadmap

### Phase 1: Architecture Foundation ✅ (Complete)
- [x] Dependency injection system  
- [x] Service provider lifecycle management
- [x] Module system with dependency resolution
- [x] Configuration management with validation
- [x] Application lifecycle and bootstrapping
- **Status**: All 33 core tests passing, production-ready architecture

### Phase 2: Database Layer 🚧 (Next)
- [ ] Full ORM with relationships and query builder
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

### Current Priorities (Phase 2)
- ORM implementation with relationships
- Database connection pooling
- Transaction management
- Model event system

## 📊 Project Stats

- **Architecture**: ✅ Production-ready foundation
- **Tests**: ✅ 33+ tests, all passing  
- **Build**: ✅ Clean compilation, minimal warnings
- **Documentation**: ✅ Comprehensive inline docs
- **AI Compatibility**: ✅ LLM-optimized code structure

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Repository**: [https://github.com/krcpa/elif.rs](https://github.com/krcpa/elif.rs)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Project Board**: [Development Roadmap](https://github.com/users/krcpa/projects/1/views/1)
- **Discussions**: [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)

---

**Built for the future of AI-driven development** 🤖

> *Phase 1 Complete: Architecture Foundation Ready*  
> *Next: Phase 2 Database Layer Development*

---

<p align="center">
  <a href="#elif-rs">⬆ Back to Top</a>
</p>