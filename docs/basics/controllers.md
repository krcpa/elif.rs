# Controllers

Controllers encapsulate request handling and business logic. The CLI generates a controller with constructor injection for the `ServiceContainer`, conventional CRUD methods, and helpful stubs.

Generate via CLI
- `elif make resource Post --fields title:string,content:text --api`  
  Creates `PostController` with `index/show/store/update/destroy`.

Generated controller skeleton
```rust
use elif_http::prelude::*;
use elif_core::ServiceContainer;
use std::sync::Arc;

#[controller]
pub struct PostController {
    container: Arc<ServiceContainer>,
}

impl PostController {
    pub fn new(container: Arc<ServiceContainer>) -> Self { Self { container } }

    pub async fn index(&self, request: Request) -> Result<Response, HttpError> {
        // Query + paginate, then return JSON
        Ok(Response::json(vec![]))
    }

    pub async fn show(&self, request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")?; // strong typing helpers available
        Ok(Response::json(serde_json::json!({"id": id})))
    }

    pub async fn store(&self, mut request: Request) -> Result<Response, HttpError> {
        let input = request.json::<serde_json::Value>().await?;
        Ok(Response::json(input).status(201))
    }

    pub async fn update(&self, mut request: Request) -> Result<Response, HttpError> {
        let id = request.path_param("id")?;
        let input = request.json::<serde_json::Value>().await?;
        Ok(Response::json(serde_json::json!({"id": id, "data": input})))
    }

    pub async fn destroy(&self, request: Request) -> Result<Response, HttpError> {
        let _id = request.path_param("id")?;
        Ok(Response::no_content())
    }
}
```

Tips
- Inject services via the container in `new`.
- Prefer `Request` helpers: `path_param`, `json`, borrowing APIs for perf.
- Use `HttpError` helpers: `bad_request`, `not_found`, `unauthorized`, etc.
