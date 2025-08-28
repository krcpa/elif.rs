# Configuration & Environment Setup

Learn how to configure elif.rs applications for different environments, manage secrets, and optimize settings for development, testing, and production.

## Configuration Files Overview

elif.rs uses a layered configuration approach:

1. **`elifrs.toml`** - Framework and application settings
2. **`.env`** - Environment variables (never commit to version control)
3. **`config/`** - Programmatic configuration modules
4. **Environment variables** - Runtime configuration and secrets

## elifrs.toml - Main Configuration

The `elifrs.toml` file is the primary configuration file for your elif.rs application:

```toml
[project]
name = "my-app"
version = "0.1.0"
description = "My elif.rs application"
authors = ["Your Name <your.email@example.com>"]

[server]
host = "127.0.0.1"
port = 3000
workers = 4
keep_alive = 30
max_connections = 1000
request_timeout = 30

[database]
url = "${DATABASE_URL}"
max_connections = 10
min_connections = 2
connection_timeout = 5
auto_migrate = true
slow_query_threshold = 1.0

[middleware]
# CORS configuration
cors = { enabled = true, origins = ["*"], methods = ["GET", "POST", "PUT", "DELETE"], headers = ["*"] }

# Rate limiting
rate_limiting = { enabled = true, requests_per_minute = 60, burst = 10 }

# Request logging
logging = { enabled = true, format = "json", level = "info" }

# Security headers
security = { enabled = true, hsts = true, xss_protection = true, content_type_options = true }

[auth]
# JWT configuration
jwt_secret = "${JWT_SECRET}"
jwt_expiry = "24h"
refresh_token_expiry = "7d"

# Session configuration  
session_secret = "${SESSION_SECRET}"
session_timeout = "2h"

[cache]
default = "memory"
ttl = 300

[cache.redis]
url = "${REDIS_URL}"
max_connections = 10

[openapi]
title = "My API"
version = "1.0.0"
description = "My application API documentation"
enabled = true
path = "/docs"

[email]
default = "smtp"

[email.smtp]
host = "${SMTP_HOST}"
port = 587
username = "${SMTP_USERNAME}"
password = "${SMTP_PASSWORD}"
encryption = "tls"

[storage]
default = "local"

[storage.local]
path = "./storage/uploads"
base_url = "/uploads"

[storage.s3]
bucket = "${S3_BUCKET}"
region = "${S3_REGION}"
access_key = "${S3_ACCESS_KEY}"
secret_key = "${S3_SECRET_KEY}"

[queue]
default = "database"
retry_attempts = 3

[queue.redis]
url = "${REDIS_URL}"
```

## Environment Variables (.env)

Environment variables store sensitive configuration and environment-specific settings:

### Development (.env)
```bash
# Application Environment
RUST_ENV=development
RUST_LOG=debug,sqlx=info
SECRET_KEY=your-development-secret-key-here

# Database
DATABASE_URL=postgresql://postgres:password@localhost/myapp_dev

# Server
HOST=127.0.0.1
PORT=3000

# Authentication
JWT_SECRET=your-jwt-secret-development
SESSION_SECRET=your-session-secret-development

# Email (development - use Mailhog or similar)
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_ENCRYPTION=none

# Cache/Queue (optional for development)
REDIS_URL=redis://localhost:6379

# Storage (local development)
STORAGE_PATH=./storage/uploads

# External APIs (development keys)
STRIPE_SECRET_KEY=sk_test_...
GITHUB_CLIENT_ID=your_github_client_id
GITHUB_CLIENT_SECRET=your_github_client_secret
```

### Testing (.env.test)
```bash
# Test Environment
RUST_ENV=testing
RUST_LOG=warn
SECRET_KEY=test-secret-key

# Test Database (isolated)
DATABASE_URL=postgresql://postgres:password@localhost/myapp_test

# Disable external services in tests
SMTP_HOST=localhost
SMTP_PORT=1025
REDIS_URL=redis://localhost:6379

# Fast test configuration
JWT_SECRET=test-jwt-secret
SESSION_SECRET=test-session-secret
```

### Production (.env.production)
```bash
# Production Environment
RUST_ENV=production
RUST_LOG=info,myapp=debug
SECRET_KEY=${SECRET_KEY_FROM_SECRETS_MANAGER}

# Production Database
DATABASE_URL=${DATABASE_URL_FROM_SECRETS}

# Server
HOST=0.0.0.0
PORT=8080
WORKERS=8

# Authentication (secure secrets)
JWT_SECRET=${JWT_SECRET_FROM_SECRETS}
SESSION_SECRET=${SESSION_SECRET_FROM_SECRETS}

# Production Email
SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=${SENDGRID_API_KEY}
SMTP_ENCRYPTION=tls

# Redis (production cluster)
REDIS_URL=${REDIS_CLUSTER_URL}

# S3 Storage
S3_BUCKET=myapp-production-uploads
S3_REGION=us-east-1
S3_ACCESS_KEY=${S3_ACCESS_KEY}
S3_SECRET_KEY=${S3_SECRET_KEY}

# External APIs (production keys)
STRIPE_SECRET_KEY=${STRIPE_PRODUCTION_SECRET}
```

## Programmatic Configuration

For complex configuration logic, use the `src/config/` directory:

### Database Configuration
```rust
// src/config/database.rs
use elif_core::config::Config;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        Self {
            url: Config::get("DATABASE_URL").expect("DATABASE_URL must be set"),
            max_connections: Config::get("DB_MAX_CONNECTIONS").unwrap_or(10),
            min_connections: Config::get("DB_MIN_CONNECTIONS").unwrap_or(2),
            connection_timeout: Config::get("DB_CONNECTION_TIMEOUT").unwrap_or(5),
        }
    }
    
    pub async fn create_pool(&self) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.connection_timeout))
            .connect(&self.url)
            .await
    }
}
```

### Server Configuration
```rust
// src/config/server.rs
use elif_core::config::Config;

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub keep_alive: u64,
    pub max_connections: usize,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            host: Config::get("HOST").unwrap_or_else(|| "127.0.0.1".to_string()),
            port: Config::get("PORT").unwrap_or(3000),
            workers: Config::get("WORKERS").unwrap_or_else(|| num_cpus::get()),
            keep_alive: Config::get("KEEP_ALIVE").unwrap_or(30),
            max_connections: Config::get("MAX_CONNECTIONS").unwrap_or(1000),
        }
    }
    
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
```

## Configuration Best Practices

### 1. **Use Environment Variables for Secrets**
```rust
// ✅ Good
let db_password = std::env::var("DB_PASSWORD")?;

// ❌ Bad - secrets in code
let db_password = "hardcoded_password";
```

### 2. **Provide Sensible Defaults**
```rust
// ✅ Good
let port = std::env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse::<u16>()?;

// ❌ Bad - no defaults
let port = std::env::var("PORT")?.parse::<u16>()?;
```

### 3. **Validate Configuration Early**
```rust
// ✅ Good - validate at startup
fn main() -> Result<(), Box<dyn std::error::Error>> {
    validate_configuration()?;
    start_application()
}

// ❌ Bad - discover invalid config at runtime
fn handle_request() {
    let secret = std::env::var("SECRET").expect("SECRET not set"); // Runtime panic!
}
```

## Next Steps

With configuration mastered, you're ready to:

- **[Controllers](../basics/controllers.md)** - Handle HTTP requests declaratively
- **[Database](../database/introduction.md)** - Connect to your configured database
- **[Middleware](../basics/middleware.md)** - Process requests with your configured middleware
- **[Security](../security/authentication.md)** - Implement authentication with your secrets

**Next**: [Core Concepts →](../basics/routing.md)