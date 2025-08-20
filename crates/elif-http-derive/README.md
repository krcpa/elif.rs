# elif-http-derive

Derive macros for the elif-http declarative routing and controller system.

## Overview

This crate provides procedural macros that enable declarative controller development in elif.rs, significantly reducing boilerplate and improving developer experience.

## Features

- `#[controller]`: Define controller base path and metadata
- `#[get]`, `#[post]`, `#[put]`, `#[delete]`, etc.: HTTP method routing macros
- `#[middleware]`: Apply middleware to controllers and methods
- `#[param]`: Route parameter specifications
- `#[body]`: Request body type specifications

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
elif-http = { version = "0.7.0", features = ["derive"] }
```

## Example

```rust
use elif_http::{controller, get, post, middleware, ElifRequest, ElifResponse, HttpResult};

#[controller("/users")]
#[middleware("logging", "cors")]
pub struct UserController;

impl UserController {
    #[get("")]
    #[middleware("cache")]
    pub async fn list(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        // List all users
        Ok(ElifResponse::ok().json(&vec!["user1", "user2"])?)
    }
    
    #[get("/{id}")]
    #[middleware("auth")]
    #[param(id: int)]
    pub async fn show(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = request.path_param_int("id")?;
        // Get user by ID
        Ok(ElifResponse::ok().json(&format!("User {}", id))?)
    }
    
    #[post("")]
    #[middleware("auth", "validation")]
    pub async fn create(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        // Create new user
        Ok(ElifResponse::created().json(&"User created")?)
    }
}
```

## Comparison with Manual Registration

### Before (Manual Registration)
```rust
impl ElifController for UserController {
    fn name(&self) -> &str { "UserController" }
    fn base_path(&self) -> &str { "/users" }
    
    fn routes(&self) -> Vec<ControllerRoute> {
        vec![
            ControllerRoute::new(HttpMethod::GET, "", "list"),
            ControllerRoute::new(HttpMethod::GET, "/{id}", "show")
                .add_param(RouteParam::new("id", ParamType::Integer)),
            ControllerRoute::new(HttpMethod::POST, "", "create"),
        ]
    }
    
    fn handle_request(&self, method_name: String, request: ElifRequest) 
        -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> 
    {
        match method_name.as_str() {
            "list" => Box::pin(Self::list(self, request)),
            "show" => Box::pin(Self::show(self, request)),
            "create" => Box::pin(Self::create(self, request)),
            _ => Box::pin(async move {
                Ok(ElifResponse::not_found().text("Handler not found"))
            })
        }
    }
}
```

### After (Declarative Macros)
```rust
#[controller("/users")]
pub struct UserController;

impl UserController {
    #[get("")]
    pub async fn list(&self, _req: ElifRequest) -> HttpResult<ElifResponse> { /* ... */ }
    
    #[get("/{id}")]
    #[param(id: int)]
    pub async fn show(&self, request: ElifRequest) -> HttpResult<ElifResponse> { /* ... */ }
    
    #[post("")]
    pub async fn create(&self, request: ElifRequest) -> HttpResult<ElifResponse> { /* ... */ }
}
```

**Result: ~70% reduction in boilerplate code**

## Status

This is the initial implementation for issue #241 in the elif.rs epic #236. The current implementation provides:

- ✅ Basic macro structure and compilation
- ✅ Integration with elif-http crate
- ✅ Compile-time validation of macro usage
- ✅ Comprehensive test suite with trybuild
- ✅ Meaningful error messages for invalid usage
- ✅ All HTTP method macros (GET, POST, PUT, DELETE, etc.)
- 🚧 Runtime route registration (needs integration with controller system)
- 🚧 Automatic ElifController trait implementation
- 🚧 Advanced parameter validation and extraction

## Testing

The crate includes comprehensive testing:

- **Unit tests**: Basic functionality and parsing
- **Integration tests**: Real macro usage verification
- **UI tests with trybuild**: Compile-time behavior validation
  - Pass tests for valid usage scenarios
  - Fail tests with expected error messages
  - Edge case handling verification

## Development Status

This implementation represents the foundation for declarative routing macros. Future enhancements will include:

1. **Route Registration**: Automatic integration with the routing system
2. **Parameter Extraction**: Advanced parameter parsing and validation
3. **Middleware Composition**: Intelligent middleware ordering and application
4. **Compile-time Validation**: Route conflict detection and optimization
5. **IDE Support**: Enhanced autocomplete and error reporting

## License

MIT