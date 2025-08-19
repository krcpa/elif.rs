# Middleware System

## Overview

The elif.rs framework provides a comprehensive middleware system supporting both legacy and modern patterns. All middleware uses the new V2 system with Laravel-style `handle(request, next)` pattern.

## New Middleware (V2 System)

### CompressionMiddleware

Provides response compression using tower-http's battle-tested CompressionLayer.

```rust
use elif_http::middleware::CompressionMiddleware;
use tower_http::compression::CompressionLevel;

let middleware = CompressionMiddleware::new()
    .best()                 // Maximum compression level
    .gzip_only()           // Only use gzip (disable brotli)
    .level(CompressionLevel::Precise(6)); // Custom compression level

// Usage in pipeline
let pipeline = MiddlewarePipelineV2::new()
    .add(CompressionMiddleware::new())
    .add(other_middleware);
```

Features:
- Uses tower-http's proven compression implementation
- Gzip, Brotli, and Deflate support
- Configurable compression levels (fast/best/precise)
- Automatic Accept-Encoding negotiation
- Built-in content-type and size filtering

### ETagMiddleware

Generates HTTP ETags and handles conditional requests (If-None-Match, If-Match).

```rust
use elif_http::middleware::{ETagMiddleware, ETagStrategy};

let middleware = ETagMiddleware::new()
    .strategy(ETagStrategy::WeakBodyHash)  // Use weak ETags
    .min_size(512)                        // Minimum response size
    .content_type("text/html");           // Additional content types

// Usage
let pipeline = MiddlewarePipelineV2::new()
    .add(ETagMiddleware::new())
    .add(other_middleware);
```

Features:
- Strong and weak ETag generation
- RFC 7232 compliant conditional request handling
- 304 Not Modified for GET/HEAD, 412 Precondition Failed for state-changing methods
- Configurable generation strategies
- Content-type filtering

### ContentNegotiationMiddleware

Handles HTTP content negotiation based on Accept headers.

```rust
use elif_http::middleware::{ContentNegotiationMiddleware, ContentType};

let middleware = ContentNegotiationMiddleware::new()
    .default_type(ContentType::Html)    // Default when negotiation fails
    .support(ContentType::Csv)          // Add CSV support
    .converter(ContentType::Xml, |json_value| {
        // Custom XML converter
        Ok(b"<xml>converted</xml>".to_vec())
    });
```

Supported formats:
- JSON (default)
- HTML with pretty formatting
- Plain text
- XML (with custom converter)
- CSV (with custom converter)

### RequestIdMiddleware

Generates and tracks unique request IDs for distributed tracing.

```rust
use elif_http::middleware::{RequestIdMiddleware, RequestIdStrategy};

let middleware = RequestIdMiddleware::new()
    .header_name("x-trace-id")          // Custom header name
    .prefixed("api")                    // Prefix: "api-{uuid}"
    .override_existing()                // Replace existing request IDs
    .no_logging();                      // Disable automatic logging

// Access request ID in handlers
use elif_http::middleware::RequestIdExt;

async fn handler(request: ElifRequest) -> ElifResponse {
    if let Some(request_id) = request.request_id() {
        println!("Processing request: {}", request_id);
    }
    ElifResponse::ok().text("Hello")
}
```

ID generation strategies:
- UUID v4 (random, default)
- UUID v1 (timestamp-based)
- Counter (not recommended for distributed systems)
- Prefixed UUID
- Custom generator function

### MaintenanceModeMiddleware

Enables temporary service maintenance mode.

```rust
use elif_http::middleware::{MaintenanceModeMiddleware, MaintenanceResponse, PathMatch};

let middleware = MaintenanceModeMiddleware::new()
    .allow_path("/health")              // Always allow health checks
    .allow_prefix("/admin")             // Allow admin panel
    .allow_ip("192.168.1.100")          // Bypass for specific IP
    .bypass_header("x-admin-key", "secret") // Bypass with header
    .response(MaintenanceResponse::Html(
        r#"<h1>Under Maintenance</h1><p>Back soon!</p>"#.to_string()
    ))
    .retry_after(3600);                 // 1 hour retry

// Dynamic control
let builder = MaintenanceModeBuilder::new();
let middleware = builder.build();

// Enable/disable at runtime
builder.enable();   // Activate maintenance mode
builder.disable();  // Deactivate maintenance mode
```

Features:
- Dynamic enable/disable
- Path whitelisting (exact, prefix, regex)
- IP whitelisting
- Bypass headers
- Custom maintenance responses (HTML, JSON, text, file)
- Retry-After header support

## Usage Patterns

### Basic Pipeline

```rust
use elif_http::middleware::v2::MiddlewarePipelineV2;
use elif_http::middleware::{
    CompressionMiddleware, ETagMiddleware, ContentNegotiationMiddleware,
    RequestIdMiddleware, MaintenanceModeMiddleware
};

let pipeline = MiddlewarePipelineV2::new()
    .add(MaintenanceModeMiddleware::new())      // Check maintenance first
    .add(RequestIdMiddleware::new())            // Add request ID
    .add(ETagMiddleware::new())                 // Handle conditional requests
    .add(ContentNegotiationMiddleware::new())   // Content negotiation
    .add(CompressionMiddleware::new());         // Compress responses last
```

### Advanced Configuration

```rust
// Comprehensive setup
let pipeline = MiddlewarePipelineV2::new()
    .add(MaintenanceModeMiddleware::new()
        .allow_path("/health")
        .bypass_header("x-admin", "secret"))
    .add(RequestIdMiddleware::new()
        .prefixed("api")
        .header_name("x-request-id"))
    .add(ETagMiddleware::new()
        .weak()                    // Use weak ETags
        .min_size(1024))
    .add(ContentNegotiationMiddleware::new()
        .default_type(ContentType::Json))
    .add(CompressionMiddleware::new()
        .level(9)                  // Maximum compression
        .min_size(2048));
```

### Router Integration

```rust
use elif_http::Router;

let router = Router::new()
    .middleware_pipeline(pipeline)
    .get("/api/data", handler)
    .post("/api/submit", submit_handler);
```

## Testing

All middleware includes comprehensive unit tests. To run tests:

```bash
cargo test middleware::utils
```

Individual middleware tests:
```bash
cargo test compression
cargo test etag  
cargo test content_negotiation
cargo test request_id
cargo test maintenance_mode
```

## Performance Considerations

1. **Middleware Order**: Place cheaper middleware first (RequestId) and expensive ones last (Compression)
2. **Compression**: Only enable for appropriate content types and sizes
3. **ETag**: Use weak ETags for better performance when semantic equivalence is sufficient
4. **Content Negotiation**: Consider caching converted responses for high-traffic endpoints
5. **Maintenance Mode**: Minimal overhead when disabled

## Migration from V1

The old middleware system is deprecated. To migrate:

```rust
// Old V1 middleware
impl Middleware for MyMiddleware {
    fn process_request(&self, request: Request) -> BoxFuture<Result<Request, Response>> {
        // ... 
    }
    fn process_response(&self, response: Response) -> BoxFuture<Response> {
        // ...
    }
}

// New V2 middleware  
#[derive(Debug)]
struct MyMiddleware;

impl crate::middleware::v2::Middleware for MyMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        Box::pin(async move {
            // Before request processing
            
            let response = next.run(request).await;
            
            // After response processing
            
            response
        })
    }
}
```

For backward compatibility, use `MiddlewareAdapter`:

```rust
use elif_http::middleware::v2::MiddlewareAdapter;

let pipeline = MiddlewarePipelineV2::new()
    .add(MiddlewareAdapter::new(OldMiddleware::new()))  // Wrap old middleware
    .add(NewV2Middleware::new());                       // Use new directly
```

## Error Handling

All middleware follows the framework error format:

```json
{
    "error": {
        "code": "middleware_error",
        "message": "Human readable message",
        "hint": "Optional hint for resolution"
    }
}
```

Common error scenarios:
- **406 Not Acceptable**: Content negotiation failed
- **412 Precondition Failed**: ETag validation failed  
- **503 Service Unavailable**: Maintenance mode active

## Best Practices

1. **Always implement Debug** for middleware structs
2. **Use builder patterns** for configuration
3. **Handle errors gracefully** - don't panic
4. **Log important events** but avoid sensitive data
5. **Test edge cases** including malformed headers
6. **Document configuration options** clearly
7. **Follow the Laravel-style pattern** consistently