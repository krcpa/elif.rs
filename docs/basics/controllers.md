# Controllers

Controllers encapsulate request handling and business logic. The CLI generates a controller with constructor injection for the `ServiceContainer`, conventional CRUD methods, and helpful stubs.

Generate via CLI
- `elifrs make resource Post --fields title:string,content:text --api`  
  Creates `PostController` with `index/show/store/update/destroy`.

Generated controller skeleton
```rust
use elif_http::{ElifRequest, ElifResponse, HttpError, HttpResult};
use elif_http::response::response; // Laravel-style builder
use elif_core::ServiceContainer;
use std::sync::Arc;

#[controller]
pub struct PostController {
    container: Arc<ServiceContainer>,
}

impl PostController {
    pub fn new(container: Arc<ServiceContainer>) -> Self { Self { container } }

    pub async fn index(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        let posts = vec![];
        response().json(posts).send()
    }

    pub async fn show(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = req.path_param_u32("id")?;
        response().json(serde_json::json!({"id": id})).send()
    }

    pub async fn store(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let input: serde_json::Value = req.json()?;
        response().json(input).created().send()
    }

    pub async fn update(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = req.path_param_u32("id")?;
        let input: serde_json::Value = req.json()?;
        response().json(serde_json::json!({"id": id, "data": input})).send()
    }

    pub async fn destroy(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let _id: u32 = req.path_param_u32("id")?;
        response().no_content().send()
    }
}
```

Tips
- Inject services via the container in `new`.
- Prefer `ElifRequest` helpers: `path_param_u32`, `json`, typed query access.
- Use `HttpError` helpers or the `response()` builder’s status helpers.
