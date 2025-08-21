# Routing

Define routes using expressive macros from `elif-http-derive`. Group routes under a prefix (e.g., API versions), declare REST resources, and use typed parameters for safe extraction.

Example: mixed routes and a grouped API v1

```rust
use elif_http_derive::{routes, group, resource, get, post, put, delete, patch};

struct AppRoutes;

#[routes]
impl AppRoutes {
    #[get("/health")]            // GET /health
    pub fn health() -> String { "OK".into() }

    #[post("/posts")]            // POST /posts
    pub fn create_post() -> String { "created".into() }

    #[get("/posts/{id}")]        // Path param: id: u32
    pub fn show_post(id: u32) -> String { format!("{}", id) }

    #[resource("/users")]        // RESTful users resource
    pub fn users() -> String { "Users CRUD".into() }
}

struct ApiV1;

#[group("/api/v1")]
impl ApiV1 {
    #[get("/profile")]           // GET /api/v1/profile
    pub fn profile() -> String { "profile".into() }
}
```

Key points
- Method macros: `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`.
- Params: declare typed function params (e.g., `id: u32`) to extract from the path.
- Groups: `#[group("/api/v1")]` prefixes all routes in the impl block.
- Resources: `#[resource("/posts")]` expands to a conventional CRUD set.

Notes
- Ensure the `derive` feature is enabled for `elif-http` in your `Cargo.toml` or depend on `elif-http-derive` directly to use these macros.

See also: the advanced patterns in `crates/elif-http-derive/examples/advanced_routing_demo.rs`.
