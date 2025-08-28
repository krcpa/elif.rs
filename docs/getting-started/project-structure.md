# Project Structure & Conventions

Understanding elif.rs project organization, file naming conventions, and where to put different types of code. This guide covers the complete anatomy of an elif.rs application.

## Standard Project Layout

When you run `elifrs new my-app`, you get this structure:

```
my-app/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library exports (optional)
│   ├── config/
│   │   ├── mod.rs             # Configuration management
│   │   ├── database.rs        # Database configuration
│   │   ├── server.rs          # Server settings
│   │   └── middleware.rs      # Middleware configuration
│   ├── controllers/
│   │   ├── mod.rs             # Controller exports
│   │   ├── health_controller.rs  # Health check endpoint
│   │   └── api/               # API versioned controllers
│   │       ├── mod.rs
│   │       ├── v1/
│   │       │   ├── mod.rs
│   │       │   ├── user_controller.rs
│   │       │   └── post_controller.rs
│   │       └── v2/            # Future API versions
│   ├── models/
│   │   ├── mod.rs             # Model exports  
│   │   ├── user.rs            # User model
│   │   ├── post.rs            # Post model
│   │   └── traits/            # Shared model traits
│   │       └── timestamped.rs
│   ├── services/
│   │   ├── mod.rs             # Service exports
│   │   ├── user_service.rs    # User business logic
│   │   ├── email_service.rs   # Email functionality
│   │   └── storage_service.rs # File storage
│   ├── middleware/
│   │   ├── mod.rs             # Middleware exports
│   │   ├── auth.rs            # Authentication middleware
│   │   ├── cors.rs            # CORS configuration
│   │   └── logging.rs         # Request logging
│   ├── requests/              # Request validation
│   │   ├── mod.rs
│   │   ├── user_requests.rs   # User create/update requests
│   │   └── post_requests.rs   # Post create/update requests
│   ├── responses/             # Response DTOs
│   │   ├── mod.rs
│   │   ├── user_response.rs   # User serialization
│   │   └── post_response.rs   # Post serialization
│   ├── policies/              # Authorization policies
│   │   ├── mod.rs
│   │   ├── user_policy.rs     # User access control
│   │   └── post_policy.rs     # Post access control
│   └── utils/
│       ├── mod.rs             # Utility functions
│       ├── validators.rs      # Custom validators
│       └── helpers.rs         # Common helpers
├── migrations/                # Database migrations
│   ├── 001_create_users_table.sql
│   ├── 002_create_posts_table.sql
│   └── 003_add_post_user_foreign_key.sql
├── seeds/                     # Database seeders
│   ├── user_seeder.rs
│   └── post_seeder.rs
├── tests/                     # Integration tests
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── user_tests.rs
│   │   └── post_tests.rs
│   └── fixtures/              # Test data
│       ├── users.json
│       └── posts.json
├── docs/                      # Project documentation
│   ├── api.md                # API documentation
│   ├── deployment.md         # Deployment guide
│   └── openapi/             # Generated OpenAPI specs
│       └── api_v1.yml
├── .env                      # Environment variables (dev)
├── .env.example             # Environment template
├── .gitignore               # Git ignore rules
├── Cargo.toml              # Rust dependencies
├── elifrs.toml            # elif.rs configuration
├── Dockerfile             # Container configuration
├── docker-compose.yml     # Local development stack
└── README.md              # Project overview
```

## File Organization Principles

### 1. **Feature-Based Organization**

elif.rs encourages organizing code by feature rather than technical layer:

```
src/
├── users/                 # User feature module
│   ├── mod.rs
│   ├── controller.rs      # HTTP handlers
│   ├── model.rs          # Database model
│   ├── service.rs        # Business logic
│   ├── requests.rs       # Validation
│   ├── responses.rs      # Serialization
│   └── tests.rs         # Unit tests
├── posts/                # Post feature module
│   ├── mod.rs
│   ├── controller.rs
│   ├── model.rs
│   └── service.rs
└── shared/               # Cross-cutting concerns
    ├── middleware/
    ├── policies/
    └── utils/
```

### 2. **Layer-Based Organization** (Default)

The standard layout separates by technical responsibility:
- **Controllers**: Handle HTTP requests and responses
- **Services**: Contain business logic and orchestration  
- **Models**: Define data structures and database interactions
- **Middleware**: Process requests/responses (auth, logging, etc.)
- **Policies**: Authorization and access control logic

## Core Directories Explained

### `/src/main.rs` - Application Entry Point

```rust
use elif_http::Server;
use elif_core::container::ServiceContainer;

mod config;
mod controllers;
mod models;
mod services;
mod middleware;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize service container
    let container = ServiceContainer::new()
        .register_database_services()?
        .register_auth_services()?
        .register_business_services()?;

    // Configure routes
    let router = config::routes::setup_router(container.clone());
    
    // Start server
    Server::new()
        .router(router)
        .container(container)
        .listen("0.0.0.0:3000")
        .await?;

    Ok(())
}
```

### `/src/controllers/` - HTTP Request Handlers

Controllers handle HTTP requests using elif.rs's declarative syntax:

```rust
// src/controllers/api/v1/user_controller.rs
use elif_http::{controller, get, post, put, delete, param, body, middleware};

#[controller("/api/v1/users")]
#[middleware("auth")]
pub struct UserController {
    user_service: Arc<UserService>,
}

impl UserController {
    #[get("")]
    async fn index(&self) -> HttpResult<ElifResponse> {
        let users = self.user_service.list_users().await?;
        Ok(ElifResponse::ok().json(&users)?)
    }

    #[post("")]
    #[body(request: CreateUserRequest)]
    async fn store(&self, request: CreateUserRequest) -> HttpResult<ElifResponse> {
        let user = self.user_service.create_user(request).await?;
        Ok(ElifResponse::created().json(&user)?)
    }
}
```

### `/src/models/` - Database Models

Models define your data structures and database interactions:

```rust
// src/models/user.rs
use elif_orm::{Model, HasMany, BelongsTo};
use serde::{Serialize, Deserialize};

#[derive(Model, Serialize, Deserialize)]
#[table = "users"]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    // Define relationships
    pub fn posts(&self) -> HasMany<Post> {
        self.has_many::<Post>("user_id")
    }
    
    pub fn profile(&self) -> BelongsTo<UserProfile> {
        self.belongs_to::<UserProfile>("profile_id")
    }
}
```

### `/src/services/` - Business Logic

Services contain your application's core business logic:

```rust
// src/services/user_service.rs
use elif_core::Injectable;

#[derive(Injectable)]
pub struct UserService {
    db: Arc<DatabaseConnection>,
    email_service: Arc<EmailService>,
}

impl UserService {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, ServiceError> {
        // Validate business rules
        if self.email_exists(&request.email).await? {
            return Err(ServiceError::EmailAlreadyExists);
        }
        
        // Create user
        let user = User::create(CreateUserData {
            name: request.name,
            email: request.email,
            password: hash_password(&request.password)?,
        }).await?;
        
        // Send welcome email
        self.email_service.send_welcome_email(&user).await?;
        
        Ok(user)
    }
}
```

### `/src/requests/` - Input Validation

Request structs define validation rules for incoming data:

```rust
// src/requests/user_requests.rs
use elif_validation::{Validate, Rule};
use serde::Deserialize;

#[derive(Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(required, min_length = 2, max_length = 100)]
    pub name: String,
    
    #[validate(required, email)]
    pub email: String,
    
    #[validate(required, min_length = 8, confirmed)]
    pub password: String,
    
    #[validate(required)]
    pub password_confirmation: String,
}

#[derive(Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(min_length = 2, max_length = 100)]
    pub name: Option<String>,
    
    #[validate(email)]
    pub email: Option<String>,
    
    #[validate(min_length = 8, confirmed)]
    pub password: Option<String>,
    
    pub password_confirmation: Option<String>,
}
```

### `/src/responses/` - Output Serialization

Response structs control how data is serialized for API responses:

```rust
// src/responses/user_response.rs
use serde::Serialize;

#[derive(Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts_count: Option<i64>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            created_at: user.created_at.to_rfc3339(),
            posts_count: None,
        }
    }
}

#[derive(Serialize)]
pub struct UserCollection {
    pub data: Vec<UserResponse>,
    pub meta: PaginationMeta,
}
```

## Configuration Files

### `elifrs.toml` - Framework Configuration

```toml
[project]
name = "my-app"
version = "0.1.0"
description = "My elif.rs application"

[server]
host = "127.0.0.1"
port = 3000
workers = 4

[database]
url = "${DATABASE_URL}"
max_connections = 10
auto_migrate = true

[middleware]
cors = { enabled = true, origins = ["*"] }
rate_limiting = { enabled = true, requests_per_minute = 60 }

[openapi]
title = "My API"
version = "1.0.0"
enabled = true
```

### `.env` - Environment Variables

```bash
# Application
RUST_ENV=development
RUST_LOG=debug
SECRET_KEY=your-secret-key-here

# Database
DATABASE_URL=postgresql://user:password@localhost/myapp_dev

# Server  
HOST=127.0.0.1
PORT=3000

# External Services
SMTP_HOST=smtp.mailgun.org
SMTP_USERNAME=your-username
SMTP_PASSWORD=your-password

# Storage
S3_BUCKET=my-app-uploads
S3_REGION=us-east-1
S3_ACCESS_KEY=your-access-key
S3_SECRET_KEY=your-secret-key
```

## Naming Conventions

### Files & Directories
- **Snake case** for files: `user_controller.rs`, `email_service.rs`
- **Lowercase** for directories: `controllers/`, `models/`, `services/`
- **Plural** for collections: `users/`, `posts/`, `comments/`

### Rust Code
- **PascalCase** for structs/enums: `UserController`, `CreateUserRequest`
- **snake_case** for functions/variables: `create_user()`, `user_name`
- **SCREAMING_SNAKE_CASE** for constants: `MAX_UPLOAD_SIZE`, `DEFAULT_TIMEOUT`

### Database
- **Snake case** for tables: `users`, `blog_posts`, `user_profiles`
- **Snake case** for columns: `user_id`, `created_at`, `email_verified_at`
- **Singular** for foreign keys: `user_id`, `post_id`, `category_id`

## Code Organization Best Practices

### 1. **Keep Controllers Thin**

Controllers should only handle HTTP concerns:

```rust
// ✅ Good - thin controller
impl UserController {
    #[post("")]
    #[body(request: CreateUserRequest)]
    async fn store(&self, request: CreateUserRequest) -> HttpResult<ElifResponse> {
        let user = self.user_service.create_user(request).await?;
        Ok(ElifResponse::created().json(&user)?)
    }
}

// ❌ Bad - fat controller with business logic
impl UserController {
    #[post("")]
    #[body(request: CreateUserRequest)]
    async fn store(&self, request: CreateUserRequest) -> HttpResult<ElifResponse> {
        // Validation
        if request.email.is_empty() { /* ... */ }
        
        // Business logic
        if User::where("email", &request.email).exists() { /* ... */ }
        
        // Database operations
        let user = User::create(/* ... */);
        
        // Email sending
        EmailService::send_welcome_email(&user);
        
        Ok(ElifResponse::created().json(&user)?)
    }
}
```

### 2. **Use Services for Business Logic**

Move complex logic into dedicated services:

```rust
// ✅ Good - business logic in service
impl UserService {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, ServiceError> {
        self.validate_business_rules(&request).await?;
        
        let user = self.repository.create(request.into()).await?;
        
        self.trigger_user_created_events(&user).await?;
        
        Ok(user)
    }
    
    async fn validate_business_rules(&self, request: &CreateUserRequest) -> Result<(), ServiceError> {
        // Complex validation logic
    }
    
    async fn trigger_user_created_events(&self, user: &User) -> Result<(), ServiceError> {
        // Event publishing, email sending, etc.
    }
}
```

### 3. **Organize Related Code Together**

Group related functionality:

```rust
// src/users/mod.rs
mod controller;
mod service;
mod model;
mod requests;
mod responses;
mod policies;

pub use controller::UserController;
pub use service::UserService;
pub use model::User;
```

### 4. **Use Proper Error Handling**

Define domain-specific error types:

```rust
// src/services/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    #[error("Email already exists: {email}")]
    EmailAlreadyExists { email: String },
    
    #[error("User not found: {id}")]
    UserNotFound { id: i32 },
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

## Testing Structure

### Unit Tests
```rust
// src/services/user_service.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_user_success() {
        // Test logic here
    }
    
    #[tokio::test]
    async fn test_create_user_duplicate_email() {
        // Test logic here
    }
}
```

### Integration Tests
```rust
// tests/integration/user_tests.rs
use elif_testing::TestApp;

#[tokio::test]
async fn test_create_user_endpoint() {
    let app = TestApp::new().await;
    
    let response = app
        .post("/api/users")
        .json(&json!({
            "name": "John Doe",
            "email": "john@example.com",
            "password": "password123",
            "password_confirmation": "password123"
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), 201);
}
```

## Next Steps

Now that you understand elif.rs project structure:

- **[Controllers](../basics/controllers.md)** - Learn declarative request handling
- **[Models](../database/models.md)** - Master the ORM and relationships  
- **[Services](../advanced/service-modules.md)** - Organize business logic effectively
- **[Testing](../testing/introduction.md)** - Write comprehensive tests

**Next**: [Core Concepts →](../basics/routing.md)