# Middleware Migration Guide

This guide helps you migrate from the old middleware system to the new V2 middleware system in Elif.rs.

## Overview of Changes

### Key Improvements in V2

1. **Laravel-style API**: Simple `handle(request, next)` pattern
2. **Full async support**: Native async/await throughout
3. **Better composition**: Easy middleware chaining and grouping
4. **Pure framework types**: No Axum types exposed to users
5. **Built-in debugging**: Introspection and profiling tools
6. **Conditional execution**: Skip middleware based on paths/methods

### Breaking Changes

- **New trait signature**: `handle()` method instead of `call()`
- **Async by default**: All middleware must be async
- **Different imports**: New module structure
- **Type changes**: Framework types instead of Axum types

## Migration Steps

### Step 1: Update Imports

**Old:**
```rust
use elif_http::middleware::Middleware;
use axum::{Request, Response};
```

**New:**
```rust
use elif_http::middleware::v2::{Middleware, Next, NextFuture};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;
```

### Step 2: Update Trait Implementation

**Old System:**
```rust
#[derive(Debug)]
pub struct AuthMiddleware {
    secret: String,
}

impl Middleware for AuthMiddleware {
    fn call(&self, req: Request) -> Response {
        // Synchronous processing
        let token = extract_token(&req);
        if !validate_token(&token, &self.secret) {
            return Response::unauthorized();
        }
        
        // Continue to next middleware (old way)
        self.next.call(req)
    }
}
```

**New System:**
```rust
#[derive(Debug)]
pub struct AuthMiddleware {
    secret: String,
}

impl Middleware for AuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let secret = self.secret.clone();
        Box::pin(async move {
            // Async processing
            let token = extract_token(&request);
            if !validate_token_async(&token, &secret).await {
                return ElifResponse::unauthorized()
                    .json_value(json!({
                        "error": {
                            "code": "unauthorized",
                            "message": "Invalid token"
                        }
                    }));
            }
            
            // Continue to next middleware (new way)
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "AuthMiddleware"
    }
}
```

### Step 3: Update Server Registration

**Old:**
```rust
server.use_middleware(Box::new(AuthMiddleware::new("secret".to_string())));
```

**New:**
```rust
server.use_middleware(AuthMiddleware::new("secret".to_string()));
```

## Common Migration Patterns

### 1. Request Validation Middleware

**Old:**
```rust
impl Middleware for ValidationMiddleware {
    fn call(&self, req: Request) -> Response {
        if !self.validate_request(&req) {
            return Response::bad_request();
        }
        self.next.call(req)
    }
}
```

**New:**
```rust
impl Middleware for ValidationMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            if !self.validate_request(&request).await {
                return ElifResponse::bad_request()
                    .json_value(json!({
                        "error": {
                            "code": "validation_failed",
                            "message": "Request validation failed"
                        }
                    }));
            }
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "ValidationMiddleware"
    }
}
```

### 2. Logging Middleware

**Old:**
```rust
impl Middleware for LoggingMiddleware {
    fn call(&self, req: Request) -> Response {
        let start = Instant::now();
        println!("Request: {} {}", req.method(), req.uri());
        
        let response = self.next.call(req);
        
        let duration = start.elapsed();
        println!("Response: {} - {:?}", response.status(), duration);
        response
    }
}
```

**New:**
```rust
impl Middleware for LoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let start = Instant::now();
            let method = request.method.clone();
            let path = request.path().to_string();
            
            println!("Request: {} {}", method, path);
            
            let response = next.run(request).await;
            
            let duration = start.elapsed();
            println!("Response: {} - {:?}", response.status_code(), duration);
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}
```

### 3. Error Handling Middleware

**Old:**
```rust
impl Middleware for ErrorHandlerMiddleware {
    fn call(&self, req: Request) -> Response {
        match panic::catch_unwind(|| self.next.call(req)) {
            Ok(response) => response,
            Err(_) => Response::internal_server_error()
        }
    }
}
```

**New:**
```rust
impl Middleware for ErrorHandlerMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Use tokio's catch_unwind for async
            match tokio::task::catch_unwind(next.run(request)).await {
                Ok(response) => response,
                Err(_) => ElifResponse::internal_server_error()
                    .json_value(json!({
                        "error": {
                            "code": "internal_error",
                            "message": "An internal server error occurred"
                        }
                    }))
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "ErrorHandlerMiddleware"
    }
}
```

## Advanced Migration Scenarios

### Conditional Middleware

**Old (manual path checking):**
```rust
impl Middleware for AuthMiddleware {
    fn call(&self, req: Request) -> Response {
        // Manual path checking
        if req.uri().path().starts_with("/public") {
            return self.next.call(req);
        }
        
        // Auth logic
        // ...
    }
}
```

**New (built-in conditional):**
```rust
// Use ConditionalMiddleware wrapper
let auth = ConditionalMiddleware::new(AuthMiddleware::new("secret".to_string()))
    .skip_paths(vec!["/public/*", "/health"]);

server.use_middleware(auth);
```

### Middleware Composition

**Old (manual chaining):**
```rust
server.use_middleware(Box::new(CorsMiddleware::new()));
server.use_middleware(Box::new(AuthMiddleware::new("secret".to_string())));
server.use_middleware(Box::new(LoggingMiddleware::new()));
```

**New (composition utilities):**
```rust
use elif_http::middleware::v2::composition;

let middleware_stack = composition::compose3(
    CorsMiddleware::new(),
    AuthMiddleware::new("secret".to_string()),
    LoggingMiddleware::new(),
);

server.use_middleware(middleware_stack);
```

### State Sharing Between Middleware

**Old (thread-local storage or request extensions):**
```rust
impl Middleware for UserMiddleware {
    fn call(&self, mut req: Request) -> Response {
        let user = self.get_user(&req);
        req.extensions_mut().insert(user);
        self.next.call(req)
    }
}
```

**New (same approach, but with ElifRequest):**
```rust
impl Middleware for UserMiddleware {
    fn handle(&self, mut request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let user = self.get_user(&request).await;
            // Store user in request extensions or similar
            // request.extensions_mut().insert(user);
            
            next.run(request).await
        })
    }
    
    fn name(&self) -> &'static str {
        "UserMiddleware"
    }
}
```

## Testing Migration

### Old Testing Approach

**Old:**
```rust
#[test]
fn test_auth_middleware() {
    let middleware = AuthMiddleware::new("secret".to_string());
    let req = Request::builder()
        .header("authorization", "Bearer secret")
        .body(Body::empty())
        .unwrap();
    
    let response = middleware.call(req);
    assert_eq!(response.status(), StatusCode::OK);
}
```

### New Testing Approach

**New:**
```rust
#[tokio::test]
async fn test_auth_middleware() {
    let middleware = AuthMiddleware::new("secret".to_string());
    let pipeline = MiddlewarePipelineV2::new().add(middleware);
    
    let mut headers = ElifHeaderMap::new();
    headers.insert("authorization".parse().unwrap(), "Bearer secret".parse().unwrap());
    let request = ElifRequest::new(ElifMethod::GET, "/protected".parse().unwrap(), headers);
    
    let response = pipeline.execute(request, |_req| {
        Box::pin(async {
            ElifResponse::ok().text("Protected content")
        })
    }).await;
    
    assert_eq!(response.status_code(), ElifStatusCode::OK);
}
```

## Migration Checklist

### Before You Start
- [ ] Review your current middleware implementations
- [ ] Identify dependencies on Axum types
- [ ] Plan for async conversion of synchronous operations
- [ ] Backup your current middleware code

### During Migration
- [ ] Update imports to V2 middleware
- [ ] Convert `call()` method to `handle()` method
- [ ] Wrap logic in `Box::pin(async move { ... })`
- [ ] Replace Axum types with Elif types
- [ ] Add `name()` method implementation
- [ ] Update error responses to use JSON format
- [ ] Convert synchronous operations to async

### After Migration
- [ ] Update tests to use new testing patterns
- [ ] Test middleware functionality
- [ ] Update middleware registration in server setup
- [ ] Consider using new features (conditional, composition)
- [ ] Enable debugging tools for development

## Common Pitfalls

### 1. Forgetting async/await

**Wrong:**
```rust
fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
    Box::pin(async move {
        let response = next.run(request); // Missing .await
        response
    })
}
```

**Correct:**
```rust
fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
    Box::pin(async move {
        let response = next.run(request).await; // With .await
        response
    })
}
```

### 2. Moving values into async block

**Wrong:**
```rust
fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
    Box::pin(async move {
        let secret = self.secret; // Can't move from self
        // ...
    })
}
```

**Correct:**
```rust
fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
    let secret = self.secret.clone(); // Clone outside
    Box::pin(async move {
        // Use cloned value
        // ...
    })
}
```

### 3. Incorrect error response format

**Old format:**
```rust
return Response::builder()
    .status(401)
    .body("Unauthorized")
    .unwrap();
```

**New format:**
```rust
return ElifResponse::unauthorized()
    .json_value(json!({
        "error": {
            "code": "unauthorized",
            "message": "Authentication required",
            "hint": "Include Authorization header"
        }
    }));
```

## New Features to Adopt

### 1. Conditional Middleware
```rust
let auth = ConditionalMiddleware::new(AuthMiddleware::new("secret".to_string()))
    .skip_paths(vec!["/public/*"])
    .only_methods(vec![ElifMethod::POST, ElifMethod::PUT]);
```

### 2. Middleware Composition
```rust
let security_stack = composition::compose3(
    CorsMiddleware::new(),
    RateLimitMiddleware::new(),
    AuthMiddleware::new("secret".to_string()),
);
```

### 3. Built-in Debugging
```rust
// Inspect middleware pipeline
server.inspect_middleware();

// Enable debug mode
server.debug_middleware(true);

// Add profiler
server.use_profiler();
```

### 4. Factory Functions
```rust
server.use_middleware(factories::cors());
server.use_middleware(factories::rate_limit(100));
server.use_middleware(factories::timeout(Duration::from_secs(30)));
```

## Performance Considerations

The new middleware system offers several performance improvements:

1. **Zero-cost abstractions**: Framework types have no runtime overhead
2. **Better async support**: Native async eliminates blocking operations
3. **Conditional execution**: Skip expensive middleware when not needed
4. **Composition optimization**: Pipeline execution is optimized

## Getting Help

If you encounter issues during migration:

1. Check the [Examples](./examples/) directory for patterns
2. Review the [API Documentation](./api.md)
3. Ask in [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)
4. Report bugs in [GitHub Issues](https://github.com/krcpa/elif.rs/issues)

## Migration Timeline

We recommend this migration approach:

1. **Week 1**: Migrate critical middleware (auth, CORS)
2. **Week 2**: Migrate utility middleware (logging, validation)
3. **Week 3**: Add new features (conditional, composition)
4. **Week 4**: Testing and performance optimization

The old middleware system will be deprecated in version 1.0 and removed in version 2.0.

## Example: Complete Migration

Here's a complete before/after example:

### Before (Old System)
```rust
use elif_http::middleware::Middleware;
use axum::{Request, Response, StatusCode};

#[derive(Debug)]
pub struct ApiMiddleware {
    api_key: String,
}

impl ApiMiddleware {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl Middleware for ApiMiddleware {
    fn call(&self, req: Request) -> Response {
        // Check API key
        let api_key = req.headers()
            .get("x-api-key")
            .and_then(|h| h.to_str().ok());
        
        match api_key {
            Some(key) if key == self.api_key => {
                // Continue to next middleware
                self.next.call(req)
            }
            _ => Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Invalid API key".into())
                .unwrap()
        }
    }
}

// Usage
server.use_middleware(Box::new(ApiMiddleware::new("secret".to_string())));
```

### After (New System)
```rust
use elif_http::middleware::v2::{Middleware, Next, NextFuture};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;
use serde_json::json;

#[derive(Debug)]
pub struct ApiMiddleware {
    api_key: String,
}

impl ApiMiddleware {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl Middleware for ApiMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let api_key = self.api_key.clone();
        Box::pin(async move {
            // Check API key
            let provided_key = request.header("x-api-key")
                .and_then(|h| h.to_str().ok());
            
            match provided_key {
                Some(key) if key == api_key => {
                    // Continue to next middleware
                    next.run(request).await
                }
                _ => ElifResponse::unauthorized()
                    .json_value(json!({
                        "error": {
                            "code": "invalid_api_key",
                            "message": "Invalid or missing API key",
                            "hint": "Include 'X-API-Key' header with valid key"
                        }
                    }))
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "ApiMiddleware"
    }
}

// Usage with new features
server.use_middleware(
    ConditionalMiddleware::new(ApiMiddleware::new("secret".to_string()))
        .skip_paths(vec!["/health", "/metrics"])
);
```

The new system provides the same functionality with better error handling, async support, and additional features like conditional execution.