# elifrs

> LLM-friendly Rust web framework CLI - AI agent-optimized development tools

[![Crates.io](https://img.shields.io/crates/v/elifrs.svg)](https://crates.io/crates/elifrs)
[![Documentation](https://docs.rs/elifrs/badge.svg)](https://docs.rs/elifrs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**elifrs** is the command-line interface for the elif.rs framework - a spec-first, AI-agent-optimized Rust web framework designed for LLM-driven development.

## 🚀 Quick Start

### Installation

```bash
cargo install elifrs
```

### Create Your First App

```bash
# Create a new elif.rs application
elif new my-todo-app
cd my-todo-app

# Add some routes (coming in Phase 2)
elif route add GET /health health_check

# Explore the generated structure
ls -la
```

## 📋 Available Commands

| Command | Description | Status |
|---------|-------------|--------|
| `elif new <name>` | Create new application | ✅ Ready |
| `elif route add METHOD /path handler` | Add HTTP route | 🚧 Phase 2 |
| `elif model add Name fields` | Add database model | 🚧 Phase 2 |
| `elif generate` | Generate from specs | 🚧 Phase 2 |
| `elif check` | Lint and validate | 🚧 Phase 2 |
| `elif migrate create/run/status` | Database migrations | 🚧 Phase 2 |
| `elif map --json` | Project structure map | 🚧 Phase 2 |
| `elif openapi export` | Export OpenAPI spec | 🚧 Phase 2 |

## 🏗️ Current Status

**Phase 1 (Architecture Foundation): ✅ Complete**
- Dependency injection system
- Service provider lifecycle management  
- Module system with dependency resolution
- Configuration management
- Application lifecycle and bootstrapping

**Phase 2 (Database Layer): 🚧 In Development**
- Full ORM implementation
- Database migrations
- Connection pooling
- Model generation

The `elif new` command creates a fully structured application with the Phase 1 architecture foundation ready for development.

## 🤖 AI-Friendly Development

elif.rs is specifically designed for AI agents and LLM-driven development:

```rust
// Generated application structure uses clean patterns
use elif_core::{Application, Module, ServiceProvider};

let app = Application::builder()
    .provider(DatabaseProvider)
    .module(ApiModule)
    .build()?;

app.start().await?;
```

## 🛠️ Framework Features

- **Modular Architecture**: Clean separation with dependency injection
- **Configuration Management**: Environment-based with validation
- **Lifecycle Management**: Graceful startup/shutdown with hooks
- **AI-Safe Patterns**: Predictable code structure for LLM development
- **Production Ready**: Comprehensive error handling and testing

## 🔗 Links

- **Framework Repository**: [github.com/krcpa/elif.rs](https://github.com/krcpa/elif.rs)
- **Documentation**: [docs.rs/elifrs](https://docs.rs/elifrs)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Project Board**: [Development Roadmap](https://github.com/users/krcpa/projects/1/views/1)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/krcpa/elif.rs/blob/main/LICENSE) file for details.

---

**Built for the future of AI-driven development** 🤖

> Phase 1 Complete: Solid architectural foundation ready for your next web application