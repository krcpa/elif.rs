# Middleware

The default V2 middleware pipeline supports composable request/response mutation and short-circuiting. Apply globally or at group/scope depending on your router setup.

Concepts
- Middlewares implement `Middleware` and receive `ElifRequest` and `Next`.
- Return an `ElifResponse` or short-circuit with an error.
- Use `add_header`, `set_status`, and `set_json` to mutate responses.

Example
```rust
use elif_http::middleware::v2::{Middleware, Next};
use elif_http::{ElifRequest, ElifResponse, HttpResult};

#[derive(Clone)]
struct Logging;

#[async_trait::async_trait]
impl Middleware for Logging {
    async fn handle(&self, mut req: ElifRequest, next: Next) -> HttpResult<ElifResponse> {
        // add request context
        req.add_header("x-request-start", chrono::Utc::now().to_rfc3339())?;
        let mut resp = next.run(req).await?;
        resp.add_header("x-processed-by", "logging")?;
        Ok(resp)
    }
}
```

Attach
- Global: apply via your server/router initialization.
- Group/route-level: if using grouping APIs, attach middleware to those scopes.
