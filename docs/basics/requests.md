# Requests

`ElifRequest` gives you typed access to path params, query params, headers, and body (JSON/form/bytes), plus extension storage for middleware.

Path and query parameters
```rust
use elif_http::ElifRequest;

fn show(req: ElifRequest) -> anyhow::Result<()> {
    // Strongly-typed path params
    let id: u32 = req.path_param_u32("id")?;
    let slug: String = req.path_param_string("slug")?;

    // Optional/required query params
    let page: Option<u32> = req.query_param_typed_new("page")?;
    let per_page: u32 = req.query_param_required_typed("per_page")?;
    Ok(())
}
```

Body parsing
```rust
use serde::Deserialize;
use elif_http::ElifRequest;

#[derive(Deserialize)]
struct CreatePost { title: String, content: String }

fn create(req: ElifRequest) -> anyhow::Result<CreatePost> {
    // JSON bodies
    let input: CreatePost = req.json()?;

    // Forms
    // let form: CreatePost = req.form()?;

    // Raw bytes
    // let bytes: &axum::body::Bytes = req.body_bytes().unwrap();
    Ok(input)
}
```

Headers and content type
```rust
let user_agent = req.header_string("user-agent")?; // Option<String>
let is_json = req.is_json();
```

Middleware extensions
```rust
// In middleware: insert typed data
req_mut.insert_extension::<MyCtx>(MyCtx { request_id });

// In handlers: retrieve typed data
if let Some(ctx) = req.get_extension::<MyCtx>() { /* ... */ }
```

Borrowing vs cloning
- Prefer typed extractors like `path_param_u32` and `query_param_typed_new` to avoid unnecessary allocations.
- Use `body_bytes()` when you need to parse once and reuse downstream.
