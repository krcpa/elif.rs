# Responses

Build responses with either `ElifResponse` methods or the fluent `response()` builder.

Fluent response builder
```rust
use elif_http::{ElifResponse, HttpResult};
use elif_http::response::response;

async fn create_user() -> HttpResult<ElifResponse> {
    let user = serde_json::json!({"id": 1, "name": "Alice"});
    response()
        .json(user)
        .created()
        .location("/users/1")
        .header("x-request-id", "abc-123")
        .send()
}

async fn redirect() -> HttpResult<ElifResponse> {
    response().redirect("/login").permanent().send()
}
```

Direct `ElifResponse` API
```rust
use elif_http::{ElifResponse, ElifStatusCode};

let resp = ElifResponse::with_status(ElifStatusCode::OK)
    .json(&serde_json::json!({"ok": true}))?;
```

Headers, cookies, CORS, and security
```rust
let resp = response()
    .json(serde_json::json!({"ok": true}))
    .header("cache-control", "no-cache")
    .cookie("session=abc123; Path=/; HttpOnly; Secure")
    .cors_with_credentials("https://example.com")
    .with_security_headers()
    .send()?;
```

Errors
- Use `HttpError` to convert errors to responses with appropriate status.
- Or build JSON error shapes via `response().error(msg)` / `validation_error(details)`.
