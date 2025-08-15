# Phase 2: Web Foundation 🌐

**Duration**: 3-4 weeks  
**Goal**: Working HTTP server with database integration  
**Status**: 80% Complete - Controllers In Progress

## Overview

Phase 2 creates the essential web server foundation that transforms elif.rs from a DI container into a working web framework. This includes HTTP routing, middleware pipeline, request/response handling, and integration with our existing ORM foundation.

## Dependencies

- **Phase 1**: ✅ Complete (DI container, module system, config management)
- **Phase 2.1**: ✅ Complete (ORM foundation - Model trait, Query builder)

## Key Components

### 1. HTTP Server & Routing
**Files**: `crates/elif-http/src/server.rs`, `crates/elif-http/src/router.rs`

Modern HTTP server with async support and flexible routing.

**Requirements**:
- HTTP/1.1 and HTTP/2 support
- Route registration with parameters (/{id}, /user/{user}/posts/{post})  
- HTTP method handling (GET, POST, PUT, DELETE, PATCH, OPTIONS)
- Route groups and prefixes
- Route parameter binding and validation
- Static file serving

**API Design**:
```rust
#[derive(Clone)]
pub struct HttpServer {
    router: Arc<Router>,
    middleware: MiddlewareStack,
    config: ServerConfig,
}

// Route definition
Route::get("/users/{id}", UserController::show)
    .middleware(AuthMiddleware::new())
    .name("users.show");

Route::group("/api/v1", |group| {
    group.resource("users", UserController::new());
    group.resource("posts", PostController::new());
});
```

### 2. Middleware Pipeline Architecture
**File**: `crates/elif-http/src/middleware.rs`

Flexible middleware system for request/response processing.

**Requirements**:
- Middleware trait for before/after request processing
- Pipeline composition and ordering
- Context passing between middleware
- Short-circuiting (early returns)
- Error handling middleware
- Built-in middleware (logging, timing, CORS basics)

**API Design**:
```rust
pub trait Middleware: Send + Sync {
    async fn handle(&self, request: Request, next: Next) -> Response;
}

// Usage
app.middleware(LoggingMiddleware::new())
   .middleware(TimingMiddleware::new())
   .middleware(ErrorHandlerMiddleware::new());
```

### 3. Request/Response System
**Files**: `crates/elif-http/src/request.rs`, `crates/elif-http/src/response.rs`

Rich request/response abstractions with JSON, form data, and file handling.

**Requirements**:
- Request parsing (headers, query params, form data, JSON)
- File upload handling
- Response builders with status codes and headers
- JSON serialization/deserialization
- Content negotiation basics
- Request validation integration

**API Design**:
```rust
// Request handling
pub struct Request {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
    extensions: Extensions,
}

impl Request {
    pub fn json<T: DeserializeOwned>(&mut self) -> Result<T>;
    pub fn form<T: DeserializeOwned>(&mut self) -> Result<T>;
    pub fn param(&self, key: &str) -> Option<&str>;
    pub fn query(&self, key: &str) -> Option<&str>;
}

// Response building
Response::json(user)
    .status(201)
    .header("Location", format!("/users/{}", user.id));

Response::ok()
    .json(json!({"message": "Success", "data": posts}));
```

### 4. Controller System
**File**: `crates/elif-http/src/controller.rs`

Controller pattern for organizing request handlers.

**Requirements**:
- Controller trait with standard REST methods
- Resource controllers (index, show, create, store, edit, update, destroy)
- Controller dependency injection
- Action method routing
- Controller middleware

**API Design**:
```rust
pub trait Controller: Send + Sync {
    fn index(&self, request: Request) -> impl Future<Output = Response>;
    fn show(&self, request: Request) -> impl Future<Output = Response>;
    fn store(&self, request: Request) -> impl Future<Output = Response>;
    fn update(&self, request: Request) -> impl Future<Output = Response>;
    fn destroy(&self, request: Request) -> impl Future<Output = Response>;
}

// Example controller
pub struct UserController {
    user_service: Arc<UserService>,
}

impl Controller for UserController {
    async fn index(&self, request: Request) -> Response {
        let users = User::all(self.user_service.pool()).await?;
        Response::json(users)
    }
    
    async fn show(&self, request: Request) -> Response {
        let id: u64 = request.param("id")?.parse()?;
        let user = User::find(self.user_service.pool(), id).await?;
        Response::json(user)
    }
}
```

### 5. Integration with Existing ORM
**File**: `crates/elif-http/src/database.rs`

Seamless integration between HTTP layer and our existing ORM foundation.

**Requirements**:
- Request-scoped database connections
- Transaction middleware for atomic operations
- Database connection injection into controllers
- Error handling for database operations
- Connection pool management integration

**API Design**:
```rust
// Database middleware
pub struct DatabaseMiddleware {
    pool: Arc<Pool<Postgres>>,
}

// In controllers
impl UserController {
    async fn store(&self, mut request: Request) -> Response {
        let pool = request.database_pool()?;
        let user_data: CreateUserRequest = request.json()?;
        
        let user = User::create(pool, user_data.into()).await?;
        Response::json(user).status(201)
    }
}
```

### 6. Error Handling & Response Formatting
**File**: `crates/elif-http/src/errors.rs`

Comprehensive error handling with consistent JSON error responses.

**Requirements**:
- HTTP error types (400, 404, 422, 500, etc.)
- Error response formatting
- Error middleware for catching panics
- Validation error handling
- Database error conversion

**API Design**:
```rust
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Not found")]
    NotFound,
    
    #[error("Validation failed: {0}")]
    ValidationError(ValidationErrors),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] ModelError),
}

// Error response format
{
  "error": {
    "code": "NOT_FOUND",
    "message": "User not found",
    "details": {
      "resource": "User",
      "id": 123
    }
  }
}
```

## Implementation Plan

### Week 1: HTTP Server Core ✅ **COMPLETED**
- [x] HTTP server implementation with async runtime
- [x] Basic routing system with parameter extraction
- [x] Request/Response abstractions
- [x] Integration with DI container

### Week 2: Middleware System ✅ **COMPLETED**
- [x] Middleware trait and pipeline
- [x] Built-in middleware (logging, timing, error handling)
- [x] Middleware composition and ordering
- [x] Context passing and extensions

### Week 3: Controllers & Database Integration 🚧 **IN PROGRESS**
- [ ] Controller trait and resource controllers (Issue #27 - **CURRENT**)
- [ ] Database middleware and connection injection
- [ ] ORM integration with request lifecycle
- [ ] Transaction handling middleware

### Week 4: Polish & Testing 📋 **NEXT**
- [ ] Comprehensive error handling (Issue #28)
- [ ] File upload support
- [ ] Static file serving
- [ ] Integration testing with example API

## Testing Strategy

### Unit Tests
- Route matching and parameter extraction
- Middleware pipeline execution
- Request/response parsing
- Controller method dispatching

### Integration Tests
- Full HTTP request lifecycle
- Database operations through HTTP
- Error handling scenarios
- File upload functionality

### Example Application
Build a simple blog API to demonstrate:
- CRUD operations for users and posts
- Authentication middleware
- Validation and error handling
- Database relationships through HTTP

## Success Criteria

### Functional Requirements
- [ ] Can serve HTTP requests with routing
- [ ] Middleware pipeline processes requests/responses
- [ ] Controllers handle CRUD operations with database
- [ ] JSON APIs work end-to-end
- [ ] Error handling provides consistent responses

### Performance Requirements
- [ ] Handle 1000+ concurrent requests
- [ ] Response time <50ms for simple operations
- [ ] Database connection pooling efficiency

### API Completeness
- [ ] RESTful resource controllers
- [ ] Parameter binding and validation
- [ ] File upload handling
- [ ] Static file serving

## Deliverables

1. **HTTP Server System**:
   - Complete HTTP/1.1 server with routing
   - Middleware pipeline architecture
   - Request/response abstractions

2. **Controller Framework**:
   - Controller trait and implementations
   - Resource routing and method dispatch
   - Database integration layer

3. **Example Application**:
   - Blog API with users and posts
   - Demonstrates full HTTP + Database functionality
   - Testing and documentation

4. **Documentation**:
   - HTTP server configuration guide
   - Controller and middleware development guide
   - API design best practices

## Files Structure
```
crates/elif-http/
├── src/
│   ├── lib.rs              # Public exports
│   ├── server.rs           # HTTP server implementation
│   ├── router.rs           # Route matching and dispatch
│   ├── middleware.rs       # Middleware system
│   ├── request.rs          # Request abstraction
│   ├── response.rs         # Response building
│   ├── controller.rs       # Controller pattern
│   ├── database.rs         # Database integration
│   └── errors.rs           # HTTP error types
├── tests/
│   ├── server_tests.rs
│   ├── routing_tests.rs
│   ├── middleware_tests.rs
│   └── integration_tests.rs
└── Cargo.toml

examples/blog-api/
├── src/
│   ├── main.rs             # Application entry point
│   ├── controllers/
│   │   ├── users.rs        # User controller
│   │   └── posts.rs        # Post controller
│   └── models/
│       ├── user.rs         # User model
│       └── post.rs         # Post model
└── Cargo.toml
```

This phase transforms elif.rs into a working web framework that can serve HTTP requests, handle database operations, and provide a foundation for more advanced features.