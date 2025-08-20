# Middleware Developer Guide

## Overview

This guide covers everything you need to know about developing middleware in Elif.rs. Middleware provides a powerful way to process HTTP requests and responses in your application.

## Quick Start

### Creating Your First Middleware

Generate a new middleware using the CLI:

```bash
elifrs generate middleware auth --debug --conditional --tests
```

This creates a complete middleware template:

```rust
use elif::prelude::*;
use elif_http::middleware::v2::{Middleware, Next, NextFuture};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;

#[derive(Debug)]
pub struct Auth {
    // Add your configuration fields here
}

impl Auth {
    pub fn new() -> Self {
        Self {}
    }
}

impl Middleware for Auth {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Before request processing
            // Add your pre-processing logic here
            
            // Process the request through the rest of the middleware chain
            let response = next.run(request).await;
            
            // After response processing
            // Add your post-processing logic here
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "Auth"
    }
}
```

### Using Middleware

Add middleware to your server:

```rust
use elif_http::{Server, HttpConfig};
use elif_core::Container;

let container = Container::new();
let mut server = Server::new(container, HttpConfig::default())?;

// Add middleware
server.use_middleware(Auth::new());
```

## Middleware Concepts

### Request/Response Flow

Middleware executes in a nested pattern:

1. **Before Processing**: Middleware executes in registration order
2. **Handler Execution**: The actual route handler runs
3. **After Processing**: Middleware executes in reverse order

```
Request → MW1 → MW2 → MW3 → Handler → MW3 → MW2 → MW1 → Response
```

### The Middleware Trait

All middleware must implement the `Middleware` trait:

```rust
pub trait Middleware: Send + Sync + std::fmt::Debug {
    /// Handle the request and call the next middleware in the chain
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static>;
    
    /// Optional middleware name for debugging
    fn name(&self) -> &'static str {
        "Middleware"
    }
}
```

## Common Middleware Patterns

### 1. Request Validation

```rust
impl Middleware for ValidationMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Validate request
            if !is_valid_request(&request) {
                return ElifResponse::bad_request()
                    .json_value(json!({
                        "error": {
                            "code": "invalid_request",
                            "message": "Request validation failed"
                        }
                    }));
            }
            
            next.run(request).await
        })
    }
}
```

### 2. Authentication

```rust
impl Middleware for AuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let required_token = self.required_token.clone();
        Box::pin(async move {
            // Extract and validate token
            let token = extract_token(&request)?;
            if !validate_token(&token, &required_token) {
                return ElifResponse::unauthorized()
                    .json_value(json!({
                        "error": {
                            "code": "unauthorized",
                            "message": "Invalid or missing authentication token"
                        }
                    }));
            }
            
            // Add user info to request context if needed
            let response = next.run(request).await;
            response
        })
    }
}
```

### 3. Logging and Monitoring

```rust
impl Middleware for LoggingMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let start = std::time::Instant::now();
            let method = request.method.clone();
            let path = request.path().to_string();
            
            // Log incoming request
            println!("→ {} {}", method, path);
            
            let response = next.run(request).await;
            
            // Log response
            let duration = start.elapsed();
            println!("← {} {} - {} - {:?}", 
                method, path, response.status_code(), duration);
            
            response
        })
    }
}
```

### 4. Response Transformation

```rust
impl Middleware for CompressionMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            let mut response = next.run(request).await;
            
            // Add compression headers
            let _ = response.add_header("Content-Encoding", "gzip");
            
            // In a real implementation, you'd compress the body here
            
            response
        })
    }
}
```

## Advanced Features

### Conditional Middleware

Skip middleware execution based on paths or HTTP methods:

```rust
// Skip authentication for public assets
let auth = ConditionalMiddleware::new(AuthMiddleware::new())
    .skip_paths(vec!["/public/*", "/health", "/metrics"])
    .only_methods(vec![ElifMethod::POST, ElifMethod::PUT, ElifMethod::DELETE]);

server.use_middleware(auth);
```

### Instrumented Middleware (Debug Mode)

Track middleware performance:

```rust
let auth = InstrumentedMiddleware::new(
    AuthMiddleware::new(),
    "auth".to_string()
);

server.use_middleware(auth);
```

### Middleware Composition

Combine multiple middleware:

```rust
use elif_http::middleware::v2::composition;

// Compose two middleware into a pipeline
let security_pipeline = composition::compose(
    CorsMiddleware::new(),
    RateLimitMiddleware::new()
);

server.use_middleware(security_pipeline);
```

## Built-in Middleware

### Rate Limiting

```rust
use elif_http::middleware::v2::factories;

server.use_middleware(factories::rate_limit(100)); // 100 requests per minute
```

### CORS

```rust
server.use_middleware(factories::cors_with_origins(vec![
    "https://example.com".to_string(),
    "https://app.example.com".to_string(),
]));
```

### Request Timeout

```rust
use std::time::Duration;

server.use_middleware(factories::timeout(Duration::from_secs(30)));
```

### Body Size Limits

```rust
server.use_middleware(factories::body_limit(1024 * 1024)); // 1MB limit
```

## Testing Middleware

### Unit Testing

```rust
#[tokio::test]
async fn test_auth_middleware() {
    let middleware = AuthMiddleware::new("secret123".to_string());
    let pipeline = MiddlewarePipelineV2::new().add(middleware);

    // Test with valid token
    let mut headers = ElifHeaderMap::new();
    headers.insert("authorization".parse().unwrap(), "Bearer secret123".parse().unwrap());
    let request = ElifRequest::new(ElifMethod::GET, "/protected".parse().unwrap(), headers);

    let response = pipeline.execute(request, |_req| {
        Box::pin(async {
            ElifResponse::ok().text("Protected content")
        })
    }).await;

    assert_eq!(response.status_code(), ElifStatusCode::OK);
}
```

### Integration Testing

```rust
use elif_http::testing::TestServer;

#[tokio::test]
async fn test_middleware_integration() {
    let mut server = TestServer::new();
    
    server.use_middleware(AuthMiddleware::new("secret".to_string()));
    server.get("/protected", |_req| async {
        Ok(ElifResponse::ok().text("Protected"))
    });

    // Test unauthorized access
    let response = server.get("/protected").send().await;
    assert_eq!(response.status(), 401);

    // Test authorized access
    let response = server
        .get("/protected")
        .header("Authorization", "Bearer secret")
        .send()
        .await;
    assert_eq!(response.status(), 200);
}
```

## Debugging and Introspection

### Pipeline Inspection

```rust
// Inspect your middleware pipeline
server.inspect_middleware();
```

Output:
```
🔍 Middleware Pipeline Inspection
   Total middleware: 4
   Execution order:
     1. LoggingMiddleware
     2. CorsMiddleware  
     3. RateLimitMiddleware
     4. ProfilerMiddleware

💡 Tip: Use debug_middleware(true) for runtime execution logs
```

### Debug Mode

```rust
// Enable detailed execution logs
server.debug_middleware(true);
```

### Performance Profiling

```rust
// Add profiler to track request timings
server.use_profiler();
```

Output:
```
⏱️  [PROFILER] Starting request GET /api/users
⏱️  [PROFILER] Completed GET /api/users in 45.2ms - Status: 200 OK
```

## Performance Best Practices

### 1. Minimize Allocations

```rust
// Good: Reuse strings
let path = request.path();

// Avoid: Unnecessary string allocations
let path = request.path().to_string();
```

### 2. Early Returns

```rust
impl Middleware for AuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Return early for public endpoints
            if request.path().starts_with("/public/") {
                return next.run(request).await;
            }
            
            // Only do expensive auth for protected endpoints
            // ... auth logic
        })
    }
}
```

### 3. Conditional Middleware

```rust
// Use conditional middleware instead of path checks in every request
let auth = ConditionalMiddleware::new(AuthMiddleware::new())
    .skip_paths(vec!["/public/*"]);
```

### 4. Async-First Design

```rust
// Good: Async operations
async fn validate_token_async(token: &str) -> bool {
    // Async validation logic
    true
}

// In middleware:
if !validate_token_async(&token).await {
    return ElifResponse::unauthorized();
}
```

## Security Considerations

### 1. Input Validation

Always validate input in middleware:

```rust
impl Middleware for ValidationMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Validate headers
            if let Some(header) = request.header("X-Custom-Header") {
                if !is_safe_header_value(header) {
                    return ElifResponse::bad_request()
                        .json_value(json!({
                            "error": "Invalid header value"
                        }));
                }
            }
            
            next.run(request).await
        })
    }
}
```

### 2. Rate Limiting

Always implement rate limiting for public APIs:

```rust
server.use_middleware(factories::rate_limit_with_window(
    1000, // requests
    Duration::from_secs(3600) // per hour
));
```

### 3. CORS Configuration

Be specific with CORS origins:

```rust
// Good: Specific origins
server.use_middleware(factories::cors_with_origins(vec![
    "https://yourdomain.com".to_string(),
]));

// Avoid: Wildcard in production
server.use_middleware(factories::cors()); // Allows all origins
```

## Migration Guide

### From Old Middleware System

If you have existing middleware using the old system, here's how to migrate:

**Old System:**
```rust
// Old trait (deprecated)
impl Middleware for OldAuth {
    fn call(&self, req: Request) -> Response {
        // Old implementation
    }
}
```

**New System:**
```rust
// New trait
impl Middleware for NewAuth {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // New implementation with async support
            next.run(request).await
        })
    }
}
```

### Key Differences

1. **Async Support**: New system is fully async
2. **Better Composition**: Easier to combine middleware
3. **Debugging Tools**: Built-in inspection and profiling
4. **Type Safety**: Uses framework types, not Axum types
5. **Conditional Execution**: Built-in path and method filtering

## Examples Repository

Find more examples in the [middleware examples directory](./examples/):

- [Authentication Middleware](./examples/auth_middleware.rs)
- [Rate Limiting](./examples/rate_limit_middleware.rs)
- [Request Logging](./examples/logging_middleware.rs)
- [CORS Handler](./examples/cors_middleware.rs)
- [Request Validation](./examples/validation_middleware.rs)
- [Response Caching](./examples/cache_middleware.rs)
- [Security Headers](./examples/security_middleware.rs)
- [API Versioning](./examples/versioning_middleware.rs)
- [Error Handling](./examples/error_middleware.rs)
- [Middleware Composition](./examples/composition_examples.rs)

## Next Steps

- Try the [Middleware Tutorial](./tutorial.md)
- Read the [API Reference](./api.md)
- Check out [Common Patterns](./patterns.md)
- Join our [Discord Community](https://discord.gg/elif) for help

## Troubleshooting

### Common Issues

#### "Middleware not executing"
Check middleware registration order and ensure you're calling `next.run(request).await`.

#### "Type errors with Axum types"
Use Elif framework types (`ElifRequest`, `ElifResponse`) instead of Axum types.

#### "Performance issues"
Use conditional middleware and early returns. Enable profiling to identify bottlenecks.

#### "Testing difficulties"
Use `TestServer` for integration tests and `MiddlewarePipelineV2` for unit tests.

Need more help? Check our [FAQ](./faq.md) or ask in [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions).