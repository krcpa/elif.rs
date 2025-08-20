# Middleware Testing Guide

This guide covers comprehensive testing strategies for middleware in Elif.rs, including unit testing, integration testing, and performance testing.

## Overview

Testing middleware is crucial for ensuring your application behaves correctly. This guide covers:

- **Unit Testing**: Testing middleware in isolation
- **Integration Testing**: Testing middleware with the full server stack
- **Performance Testing**: Benchmarking middleware performance
- **Mock Testing**: Using mock middleware for testing
- **Pipeline Testing**: Testing middleware composition and execution order

## Quick Start

### Basic Unit Test

```rust
use elif_http::testing::{MiddlewareTestHarness, TestRequestBuilder};

#[tokio::test]
async fn test_auth_middleware() {
    let harness = MiddlewareTestHarness::new()
        .add_middleware(AuthMiddleware::new("secret".to_string()));
    
    let request = TestRequestBuilder::get("/protected")
        .auth_bearer("secret")
        .build();
    
    let result = harness.execute(request).await;
    result.assert_status(200);
}
```

## Testing Utilities

### MiddlewareTestHarness

The `MiddlewareTestHarness` provides an isolated environment for testing middleware:

```rust
use elif_http::testing::MiddlewareTestHarness;

// Create a test harness
let harness = MiddlewareTestHarness::new()
    .add_middleware(LoggingMiddleware::new())
    .add_middleware(AuthMiddleware::new("secret".to_string()));

// Execute a request
let request = TestRequestBuilder::get("/test").build();
let result = harness.execute(request).await;

// Make assertions
result.assert_status(200)
      .assert_execution_time(Duration::from_millis(100));
```

### TestRequestBuilder

Build test requests easily:

```rust
use elif_http::testing::TestRequestBuilder;
use serde_json::json;

// GET request
let request = TestRequestBuilder::get("/api/users").build();

// POST request with JSON body
let request = TestRequestBuilder::post("/api/users")
    .json_body(&json!({
        "name": "John Doe",
        "email": "john@example.com"
    }))
    .build();

// Request with authentication
let request = TestRequestBuilder::get("/protected")
    .auth_bearer("my-token")
    .build();

// Request with custom headers
let request = TestRequestBuilder::put("/api/users/1")
    .header("X-Custom-Header", "custom-value")
    .json()
    .build();
```

## Unit Testing Patterns

### 1. Basic Middleware Testing

```rust
use elif_http::testing::{MiddlewareTestHarness, TestRequestBuilder};

#[tokio::test]
async fn test_cors_middleware() {
    let middleware = CorsMiddleware::new()
        .allow_origins(vec!["https://example.com".to_string()]);
    
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    let request = TestRequestBuilder::get("/api/test").build();
    let result = harness.execute(request).await;
    
    result.assert_status(200)
          .assert_header("Access-Control-Allow-Origin", "https://example.com");
}
```

### 2. Authentication Middleware Testing

```rust
#[tokio::test]
async fn test_auth_middleware_with_valid_token() {
    let middleware = JwtAuthMiddleware::new("secret-key".to_string());
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    let request = TestRequestBuilder::get("/protected")
        .auth_bearer("valid-token")
        .build();
    
    let result = harness.execute(request).await;
    result.assert_status(200);
}

#[tokio::test] 
async fn test_auth_middleware_with_invalid_token() {
    let middleware = JwtAuthMiddleware::new("secret-key".to_string());
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    let request = TestRequestBuilder::get("/protected")
        .auth_bearer("invalid-token")
        .build();
    
    let result = harness.execute(request).await;
    result.assert_status(401);
}

#[tokio::test]
async fn test_auth_middleware_without_token() {
    let middleware = JwtAuthMiddleware::new("secret-key".to_string());
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    let request = TestRequestBuilder::get("/protected").build();
    
    let result = harness.execute(request).await;
    result.assert_status(401);
}
```

### 3. Rate Limiting Middleware Testing

```rust
#[tokio::test]
async fn test_rate_limit_middleware() {
    let middleware = RateLimitMiddleware::new().limit(2); // 2 requests max
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    // First request should succeed
    let request1 = TestRequestBuilder::get("/api/test").build();
    let result1 = harness.execute(request1).await;
    result1.assert_status(200);
    
    // Second request should succeed
    let request2 = TestRequestBuilder::get("/api/test").build();
    let result2 = harness.execute(request2).await;
    result2.assert_status(200);
    
    // Third request should be rate limited
    let request3 = TestRequestBuilder::get("/api/test").build();
    let result3 = harness.execute(request3).await;
    result3.assert_status(429); // Too Many Requests
}
```

### 4. Testing Middleware with Custom Handlers

```rust
#[tokio::test]
async fn test_error_handling_middleware() {
    let middleware = ErrorHandlingMiddleware::new();
    
    let harness = MiddlewareTestHarness::new()
        .add_middleware(middleware)
        .with_handler(|_req| {
            // Simulate a handler that panics
            panic!("Test error");
        });
    
    let request = TestRequestBuilder::get("/test").build();
    let result = harness.execute(request).await;
    
    // Should catch the panic and return 500
    result.assert_status(500);
}
```

## Mock Middleware

Use mock middleware for testing complex scenarios:

### Basic Mock Middleware

```rust
use elif_http::testing::{MockMiddleware, MockBehavior};

#[tokio::test]
async fn test_with_mock_middleware() {
    let mock = MockMiddleware::new("test-mock");
    let harness = MiddlewareTestHarness::new().add_middleware(mock.clone());
    
    let request = TestRequestBuilder::get("/test").build();
    harness.execute(request).await;
    
    // Verify mock was executed
    assert_eq!(mock.execution_count(), 1);
}
```

### Mock Middleware Behaviors

```rust
// Mock that returns a specific response
let mock = MockMiddleware::returns_response("auth-mock", 401, "Unauthorized");

// Mock that adds a header
let mock = MockMiddleware::adds_header("header-mock", "X-Test", "test-value");

// Mock that delays execution
let mock = MockMiddleware::delays("slow-mock", Duration::from_millis(100));

// Mock that simulates an error
let mock = MockMiddleware::new("error-mock").with_behavior(MockBehavior::Error("Test error".to_string()));
```

## Pipeline Testing

### Testing Middleware Execution Order

```rust
use elif_http::testing::MiddlewareAssertions;

#[tokio::test]
async fn test_middleware_execution_order() {
    let mock1 = MockMiddleware::new("first");
    let mock2 = MockMiddleware::new("second");
    let mock3 = MockMiddleware::new("third");
    
    let harness = MiddlewareTestHarness::new()
        .add_middleware(mock1.clone())
        .add_middleware(mock2.clone())
        .add_middleware(mock3.clone());
    
    // Verify pipeline structure
    let pipeline = harness.pipeline();
    MiddlewareAssertions::assert_execution_order(
        &pipeline, 
        &["first", "second", "third"]
    );
    
    // Execute request and verify all middleware ran
    let request = TestRequestBuilder::get("/test").build();
    harness.execute(request).await;
    
    MiddlewareAssertions::assert_mock_execution_count(&mock1, 1);
    MiddlewareAssertions::assert_mock_execution_count(&mock2, 1);
    MiddlewareAssertions::assert_mock_execution_count(&mock3, 1);
}
```

### Testing Conditional Middleware

```rust
#[tokio::test]
async fn test_conditional_middleware() {
    let auth = ConditionalMiddleware::new(AuthMiddleware::new("secret".to_string()))
        .skip_paths(vec!["/public/*"]);
    
    let harness = MiddlewareTestHarness::new().add_middleware(auth);
    
    // Public path should skip authentication
    let public_request = TestRequestBuilder::get("/public/assets/style.css").build();
    let result = harness.execute(public_request).await;
    result.assert_status(200);
    
    // Protected path should require authentication
    let protected_request = TestRequestBuilder::get("/api/users").build();
    let result = harness.execute(protected_request).await;
    result.assert_status(401); // Unauthorized because no token
}
```

## Integration Testing

### Testing with TestServerBuilder

```rust
use elif_http::testing::TestServerBuilder;

#[tokio::test]
async fn test_middleware_integration() {
    let server = TestServerBuilder::new()
        .with_middleware(LoggingMiddleware::new())
        .with_middleware(AuthMiddleware::new("secret".to_string()))
        .build();
    
    // In a real integration test, you would:
    // 1. Start the server
    // 2. Make HTTP requests using a client library
    // 3. Verify responses and middleware behavior
}
```

### Full Stack Integration Testing

```rust
use elif_http::{Server, HttpConfig, ElifRouter};
use elif_core::Container;

#[tokio::test]
async fn test_full_middleware_stack() {
    let container = Container::new();
    let mut server = Server::new(container, HttpConfig::default()).unwrap();
    
    // Add middleware stack
    server
        .use_middleware(CorsMiddleware::new())
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(AuthMiddleware::new("secret".to_string()));
    
    // Add routes
    let router = ElifRouter::new()
        .get("/api/users", |_req| async { 
            Ok(ElifResponse::ok().json_value(json!({"users": []})))
        });
    
    server.use_router(router);
    
    // Test middleware integration with routes
    // (This would typically involve starting the server and making HTTP requests)
}
```

## Performance Testing

### Benchmarking Middleware

```rust
use elif_http::testing::MiddlewareBenchmark;

#[tokio::test]
async fn benchmark_auth_middleware() {
    let middleware = AuthMiddleware::new("secret".to_string());
    
    let result = MiddlewareBenchmark::benchmark_middleware(middleware, 1000).await;
    
    result.print(); // Print benchmark results
    
    // Assert performance requirements
    assert!(result.average_duration < Duration::from_millis(10));
}
```

### Load Testing with Multiple Requests

```rust
#[tokio::test]
async fn test_middleware_under_load() {
    let middleware = RateLimitMiddleware::new().limit(100);
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    let mut tasks = Vec::new();
    
    // Spawn multiple concurrent requests
    for i in 0..50 {
        let harness = harness.clone();
        let task = tokio::spawn(async move {
            let request = TestRequestBuilder::get(&format!("/test/{}", i)).build();
            harness.execute(request).await
        });
        tasks.push(task);
    }
    
    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;
    
    // Verify results
    let successful = results.iter()
        .map(|r| r.as_ref().unwrap())
        .filter(|r| r.response.status_code().as_u16() == 200)
        .count();
    
    assert!(successful > 0, "At least some requests should succeed");
}
```

## Advanced Testing Scenarios

### Testing Middleware Composition

```rust
#[tokio::test]
async fn test_middleware_composition() {
    use elif_http::middleware::v2::composition;
    
    let composed = composition::compose3(
        CorsMiddleware::new(),
        LoggingMiddleware::new(),
        AuthMiddleware::new("secret".to_string()),
    );
    
    let harness = MiddlewareTestHarness::new().add_middleware(composed);
    
    let request = TestRequestBuilder::get("/api/test")
        .auth_bearer("secret")
        .build();
    
    let result = harness.execute(request).await;
    
    result.assert_status(200)
          .assert_header("Access-Control-Allow-Origin", "*");
}
```

### Testing Middleware Error Propagation

```rust
#[tokio::test]
async fn test_error_propagation() {
    let error_middleware = MockMiddleware::returns_response("error", 500, "Server Error");
    let logging_middleware = LoggingMiddleware::new();
    
    let harness = MiddlewareTestHarness::new()
        .add_middleware(logging_middleware)
        .add_middleware(error_middleware);
    
    let request = TestRequestBuilder::get("/test").build();
    let result = harness.execute(request).await;
    
    // Error should propagate through the pipeline
    result.assert_status(500);
}
```

### Testing Asynchronous Middleware

```rust
#[tokio::test]
async fn test_async_middleware() {
    let async_middleware = AsyncDatabaseMiddleware::new(db_connection);
    let harness = MiddlewareTestHarness::new().add_middleware(async_middleware);
    
    let request = TestRequestBuilder::get("/test").build();
    let result = harness.execute(request).await;
    
    result.assert_status(200)
          .assert_execution_time(Duration::from_millis(500)); // Account for DB time
}
```

## Test Organization

### Test Structure

Organize your middleware tests in a logical structure:

```
tests/
├── unit/
│   ├── middleware/
│   │   ├── auth_test.rs
│   │   ├── cors_test.rs
│   │   ├── logging_test.rs
│   │   └── rate_limit_test.rs
│   └── mod.rs
├── integration/
│   ├── middleware_integration_test.rs
│   └── full_stack_test.rs
└── performance/
    ├── middleware_benchmarks.rs
    └── load_tests.rs
```

### Test Utilities Module

Create a common test utilities module:

```rust
// tests/common/mod.rs
use elif_http::testing::*;

pub fn create_test_auth_middleware() -> AuthMiddleware {
    AuthMiddleware::new("test-secret".to_string())
}

pub fn create_test_user_request() -> ElifRequest {
    TestRequestBuilder::get("/api/user")
        .auth_bearer("valid-token")
        .build()
}

pub fn assert_successful_response(result: &MiddlewareTestResult) {
    result.assert_status(200)
          .assert_execution_time(Duration::from_millis(100));
}
```

## Best Practices

### 1. Test Edge Cases

```rust
#[tokio::test]
async fn test_auth_middleware_edge_cases() {
    let middleware = AuthMiddleware::new("secret".to_string());
    let harness = MiddlewareTestHarness::new().add_middleware(middleware);
    
    // Test empty authorization header
    let request = TestRequestBuilder::get("/protected")
        .header("Authorization", "")
        .build();
    let result = harness.execute(request).await;
    result.assert_status(401);
    
    // Test malformed authorization header
    let request = TestRequestBuilder::get("/protected")
        .header("Authorization", "InvalidFormat")
        .build();
    let result = harness.execute(request).await;
    result.assert_status(401);
    
    // Test authorization header without "Bearer"
    let request = TestRequestBuilder::get("/protected")
        .header("Authorization", "secret")
        .build();
    let result = harness.execute(request).await;
    result.assert_status(401);
}
```

### 2. Use Descriptive Test Names

```rust
#[tokio::test]
async fn auth_middleware_allows_valid_bearer_token() { /* ... */ }

#[tokio::test]
async fn auth_middleware_rejects_invalid_bearer_token() { /* ... */ }

#[tokio::test]
async fn auth_middleware_rejects_missing_authorization_header() { /* ... */ }
```

### 3. Test Both Success and Failure Cases

```rust
mod auth_middleware_tests {
    use super::*;
    
    mod success_cases {
        use super::*;
        
        #[tokio::test]
        async fn allows_valid_token() { /* ... */ }
        
        #[tokio::test]
        async fn allows_admin_token() { /* ... */ }
    }
    
    mod failure_cases {
        use super::*;
        
        #[tokio::test]
        async fn rejects_invalid_token() { /* ... */ }
        
        #[tokio::test]
        async fn rejects_expired_token() { /* ... */ }
        
        #[tokio::test]
        async fn rejects_missing_token() { /* ... */ }
    }
}
```

### 4. Use Test Fixtures

```rust
// Create test fixtures for common scenarios
fn create_authenticated_request() -> ElifRequest {
    TestRequestBuilder::get("/api/test")
        .auth_bearer("valid-token")
        .build()
}

fn create_unauthenticated_request() -> ElifRequest {
    TestRequestBuilder::get("/api/test").build()
}

fn create_admin_request() -> ElifRequest {
    TestRequestBuilder::get("/api/admin")
        .auth_bearer("admin-token")
        .build()
}
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Middleware Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Run middleware unit tests
      run: cargo test middleware --lib
      
    - name: Run middleware integration tests  
      run: cargo test middleware --test integration
      
    - name: Run middleware benchmarks
      run: cargo test middleware --test benchmarks --release
```

## Troubleshooting

### Common Issues

#### "Test hangs indefinitely"
- Ensure you're using `.await` on async operations
- Check for deadlocks in middleware logic
- Use timeouts in tests: `tokio::time::timeout(Duration::from_secs(5), test_future).await`

#### "Mock middleware not executing"
- Verify middleware is added to the harness correctly
- Check middleware execution order
- Ensure the request reaches the middleware

#### "Headers not found in assertions"
- Remember that headers are case-insensitive
- Check if middleware is actually adding the header
- Use the correct header name format

#### "Performance tests are flaky"
- Run benchmarks multiple times and average results
- Use appropriate test timeouts
- Consider system load when running performance tests

### Debugging Tips

1. **Use debug logging**: Enable debug logs in your middleware during tests
2. **Add print statements**: Temporarily add `println!` statements to trace execution
3. **Check execution order**: Use `inspect_middleware()` to verify pipeline structure
4. **Verify test setup**: Ensure test harness is configured correctly

## Next Steps

- Read the [Middleware Guide](./README.md) for implementation details
- Check out [Examples](./examples/) for real-world middleware patterns  
- Review [Performance Best Practices](./PERFORMANCE.md)
- Join our [Discord Community](https://discord.gg/elif) for help

Need help with testing? Check our [FAQ](./FAQ.md) or ask in [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions).