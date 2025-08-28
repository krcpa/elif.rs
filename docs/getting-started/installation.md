# Installation & Setup

Get elif.rs running on your system in under 5 minutes. This guide covers installing the CLI, setting up your development environment, and creating your first project.

## Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/) if you haven't already
- **PostgreSQL** - For database functionality (MySQL and SQLite support coming soon)
- **Git** - For project scaffolding and version control

### Quick Rust Installation
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### PostgreSQL Setup
```bash
# macOS (Homebrew)
brew install postgresql
brew services start postgresql

# Ubuntu/Debian
sudo apt-get install postgresql postgresql-contrib
sudo systemctl start postgresql

# Windows (Chocolatey)
choco install postgresql
```

## Install elif.rs CLI

### Option 1: Install from crates.io (Recommended)
```bash
cargo install elifrs
```

### Option 2: Install from Source
```bash
git clone https://github.com/krcpa/elif.rs.git
cd elif.rs
cargo build --release -p elifrs
# Add target/release to your PATH
```

### Verify Installation
```bash
elifrs --version
# Should output: elifrs 0.9.0 (or latest version)
```

## Create Your First Project

### 1. Generate a New Project
```bash
elifrs new blog-api
cd blog-api
```

This creates a complete project structure:
```
blog-api/
├── src/
│   ├── main.rs              # Application entry point
│   ├── controllers/         # Request handlers
│   ├── models/             # Database models  
│   ├── services/           # Business logic
│   ├── middleware/         # Custom middleware
│   └── config/            # Configuration
├── migrations/            # Database migrations
├── tests/                # Integration tests
├── .env                  # Environment variables
├── Cargo.toml           # Dependencies
└── elifrs.toml         # elif.rs configuration
```

### 2. Configure Your Database
Edit `.env` in your project root:
```bash
# Database Configuration
DATABASE_URL=postgresql://username:password@localhost/blog_api_dev

# Server Configuration  
HOST=127.0.0.1
PORT=3000

# Environment
RUST_ENV=development
RUST_LOG=debug
```

### 3. Set Up the Database
```bash
# Create the database
createdb blog_api_dev

# Run initial migrations
elifrs migrate run
```

### 4. Start the Development Server
```bash
# Start with hot reload
elifrs serve --hot-reload --port 3000

# Or use cargo directly
cargo run
```

Visit `http://127.0.0.1:3000` - you should see the elif.rs welcome page! 🎉

## Development Workflow

### Essential Commands
```bash
# Development
elifrs serve --hot-reload    # Start dev server with auto-reload
elifrs check                # Validate project structure
cargo test                  # Run all tests
cargo clippy                # Lint your code

# Code Generation  
elifrs make controller User  # Generate new controller
elifrs make model Post      # Generate model + migration
elifrs make resource Blog   # Generate full CRUD resource

# Database
elifrs migrate create add_users_table  # Create migration
elifrs migrate run                      # Apply migrations
elifrs migrate rollback                 # Rollback last migration
elifrs db seed                         # Run database seeders

# API Documentation
elifrs openapi generate     # Generate OpenAPI spec
elifrs openapi serve        # Start Swagger UI server
```

## IDE Setup

### VS Code (Recommended)
Install these extensions for the best experience:
- **rust-analyzer** - Language server
- **CodeLLDB** - Debugging support
- **Crates** - Manage dependencies
- **Even Better TOML** - TOML syntax highlighting

#### VS Code Settings
Create `.vscode/settings.json`:
```json
{
  "rust-analyzer.cargo.features": ["derive"],
  "rust-analyzer.checkOnSave.command": "clippy",
  "files.watcherExclude": {
    "**/target/**": true
  }
}
```

### Other IDEs
- **IntelliJ IDEA/CLion** - Install Rust plugin
- **Vim/Neovim** - Use `rust-analyzer` with your LSP client
- **Emacs** - Configure `rustic` with `lsp-mode`

## Project Configuration

### elifrs.toml
The `elifrs.toml` file controls framework behavior:
```toml
[project]
name = "blog-api"
version = "0.1.0"
description = "A blog API built with elif.rs"

[server]
host = "127.0.0.1"
port = 3000
threads = 4

[database] 
url = "${DATABASE_URL}"
max_connections = 10
auto_migrate = true

[middleware]
cors = { enabled = true, origins = ["*"] }
rate_limiting = { enabled = true, requests_per_minute = 60 }
logging = { enabled = true, format = "json" }

[openapi]
title = "Blog API"
version = "1.0.0"
description = "RESTful blog API with full CRUD operations"
```

### Environment Variables
Key environment variables elif.rs recognizes:
```bash
# Required
DATABASE_URL=postgresql://...

# Server
HOST=127.0.0.1              # Server bind address
PORT=3000                   # Server port  
WORKERS=4                   # Number of worker threads

# Environment  
RUST_ENV=development        # development, testing, production
RUST_LOG=debug             # Logging level

# Security (production)
SECRET_KEY=your-secret-key  # For JWT tokens, session encryption
CORS_ORIGINS=https://yourdomain.com
```

## Verification Checklist

Ensure everything is working:

- [ ] `elifrs --version` shows the correct version
- [ ] `cargo run` starts the server without errors
- [ ] `http://127.0.0.1:3000` shows the welcome page
- [ ] `elifrs migrate run` executes successfully  
- [ ] `cargo test` passes all tests
- [ ] `elifrs openapi generate` creates OpenAPI spec

## Troubleshooting

### Common Issues

**"command not found: elifrs"**
- Ensure `~/.cargo/bin` is in your PATH
- Restart your terminal after installation

**Database connection errors**
- Verify PostgreSQL is running: `brew services list | grep postgresql`
- Check DATABASE_URL format: `postgresql://user:pass@host:port/database`
- Ensure database exists: `createdb your_database_name`

**Compilation errors**  
- Update Rust: `rustup update`
- Clear cache: `cargo clean`
- Check Rust version: `rustc --version` (need 1.70+)

**Port already in use**
- Change port: `elifrs serve --port 3001`
- Kill existing process: `lsof -ti:3000 | xargs kill -9`

### Getting Help

- **Documentation**: You're reading it! Check other sections for specific topics
- **GitHub Issues**: [github.com/krcpa/elif.rs/issues](https://github.com/krcpa/elif.rs/issues)
- **Discord Community**: [discord.gg/elifrs](https://discord.gg/elifrs)
- **Stack Overflow**: Use the `elif.rs` tag

## Next Steps

With elif.rs installed and running, you're ready to build your first application:

- **[Quickstart Guide](quickstart-no-rust.md)** - Build a complete CRUD API in 10 minutes
- **[Project Structure](project-structure.md)** - Understand elif.rs conventions
- **[Controllers](../basics/controllers.md)** - Learn declarative request handling

**Next**: [Quickstart (No Rust) →](quickstart-no-rust.md)