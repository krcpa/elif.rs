# Elif Testing Guidelines

## ✅ CORRECT Testing Patterns

All tests in the elif framework must use **pure framework abstractions** and never expose internal implementation details.

### Integration Testing

Use `elif-testing::TestClient` for all HTTP integration tests:

```rust
use elif_testing::prelude::*;

#[tokio::test]
async fn test_user_creation() {
    let response = TestClient::new()
        .with_base_url("http://localhost:3001")
        .post("/users")
        .json(&json!({
            "name": "Alice",
            "email": "alice@example.com"
        }))
        .authenticated_with_token("admin-token")
        .send()
        .await
        .expect("Request should succeed")
        .assert_status(201)
        .assert_json_contains(json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com"
            }
        }))
        .expect("Should contain user data");
}
```

### Unit Testing Middleware

Use elif native types in middleware tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{request::ElifRequest, response::ElifResponse};
    
    #[tokio::test]
    async fn test_middleware_behavior() {
        let middleware = MyMiddleware::new();
        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::ElifHeaderMap::new(),
        );
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::ElifStatusCode::OK);
    }
}
```

### Authentication Testing

```rust
#[tokio::test]
async fn test_protected_endpoint() {
    TestClient::new()
        .authenticated_with_token("valid-jwt")
        .get("/protected")
        .send()
        .await?
        .assert_success()
        .assert_header_exists("authorization");
}
```

### Error Testing

```rust
#[tokio::test]
async fn test_validation_errors() {
    TestClient::new()
        .post("/users")
        .json(&json!({"name": ""})) // Invalid data
        .send()
        .await?
        .assert_status(422)
        .assert_validation_error("name", "required")?;
}
```

## ❌ FORBIDDEN Patterns

**Never import or use these types in tests:**

```rust
// ❌ NEVER DO THIS
use axum::extract::Request;
use axum::response::Response;
use axum::body::Body;
use axum::http::{Method, StatusCode, HeaderMap};
use hyper::Request;
use tower::Service;
```

**Never manually construct HTTP requests:**

```rust
// ❌ NEVER DO THIS
let request = Request::builder()
    .method(Method::GET)
    .uri("/test")
    .body(Body::empty())
    .unwrap();
```

**Never convert between Axum and Elif types in tests:**

```rust
// ❌ NEVER DO THIS
let (parts, body) = axum_request.into_parts();
let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;
let elif_request = ElifRequest::extract_elif_request(parts.method, ...);
```

## Rules

1. **Pure Framework Types**: Only import and use `elif_*` types
2. **TestClient First**: Always use `TestClient` for HTTP tests
3. **No Implementation Leakage**: Never expose Axum, Hyper, or Tower types
4. **Fluent API**: Use the chainable assertion methods
5. **Real Integration**: Tests should spin up actual servers, not mock responses

## Enforcement

Clippy rules prevent accidental use of forbidden types:

```bash
cargo clippy --tests
```

This will flag any use of `axum::*`, `hyper::*`, or `tower::*` types in test code.

## Migration from Old Patterns

If you find old test code using Axum types:

1. Remove the `axum::*` imports
2. Replace manual request construction with `TestClient`
3. Use elif native types for assertions
4. Leverage the fluent assertion API

## Examples Directory

See `/tests/proper_integration_tests.rs` for comprehensive examples of correct testing patterns.