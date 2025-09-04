# Zero-Boilerplate Quick Start

Experience the power of Laravel-style "convention over configuration" in Rust. Build a complete API in under 5 minutes with the `#[elif::bootstrap]` macro.

## What You'll Build

A complete user management API with:
- ✅ Zero manual server configuration
- ✅ Automatic dependency injection
- ✅ Declarative routing with middleware
- ✅ Full CRUD operations
- ✅ Production-ready error handling

**Time**: ~5 minutes  
**Lines of setup code**: **3 lines**

## Step 1: Create Your Project

```bash
# Create new project
cargo new my-api --bin
cd my-api

# Add elif.rs with bootstrap support
cargo add elif --features="bootstrap,derive"
cargo add tokio --features="full" 
cargo add serde --features="derive"
```

## Step 2: Define Your API (Zero Boilerplate!)

Replace `src/main.rs` with this complete application:

```rust
use elif::prelude::*;
use serde::{Deserialize, Serialize};

// Your data models
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

// Your service layer
#[derive(Default)]
struct UserService {
    users: std::sync::Mutex<Vec<User>>,
}

impl UserService {
    fn create_user(&self, data: CreateUser) -> User {
        let mut users = self.users.lock().unwrap();
        let id = users.len() as u32 + 1;
        let user = User {
            id,
            name: data.name,
            email: data.email,
        };
        users.push(user.clone());
        user
    }
    
    fn get_all_users(&self) -> Vec<User> {
        self.users.lock().unwrap().clone()
    }
}

// Your controller - declarative and clean
#[controller("/api/users")]
#[middleware("cors")]
impl UserController {
    #[inject(user_service: UserService)]
    fn new(user_service: UserService) -> Self {
        Self { user_service }
    }
    
    // GET /api/users
    #[get("")]
    async fn list(&self) -> HttpResult<ElifResponse> {
        let users = self.user_service.get_all_users();
        Ok(ElifResponse::ok().json(&users)?)
    }
    
    // POST /api/users  
    #[post("")]
    async fn create(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let data: CreateUser = req.json().await?;
        let user = self.user_service.create_user(data);
        Ok(ElifResponse::created().json(&user)?)
    }
    
    // GET /api/users/{id}
    #[get("/{id}")]
    #[param(id: int)]
    async fn show(&self, id: u32) -> HttpResult<ElifResponse> {
        let users = self.user_service.get_all_users();
        if let Some(user) = users.into_iter().find(|u| u.id == id) {
            Ok(ElifResponse::ok().json(&user)?)
        } else {
            Ok(ElifResponse::not_found().json(&json!({
                "error": "User not found"
            }))?)
        }
    }
}

// Your app module - brings everything together
#[module(
    controllers: [UserController],
    providers: [UserService],
    is_app
)]
struct MyApp;

// Zero-boilerplate server setup! ✨
#[elif::bootstrap(MyApp)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 API server starting!");
    println!("📝 Available endpoints:");
    println!("   GET  /api/users     - List all users");
    println!("   POST /api/users     - Create user");
    println!("   GET  /api/users/{{id}} - Get user by ID");
    
    // Server automatically starts on 127.0.0.1:3000
    // All routes registered, DI configured, middleware applied!
}
```

## Step 3: Run Your API

```bash
# Start the server
cargo run
```

**Output**:
```
🚀 API server starting!
📝 Available endpoints:
   GET  /api/users     - List all users
   POST /api/users     - Create user
   GET  /api/users/{id} - Get user by ID

INFO  Server listening on http://127.0.0.1:3000
```

**That's it!** Your complete API is running! 🎉

## Step 4: Test Your API

### Create a user
```bash
curl -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Johnson", "email": "alice@example.com"}'
```

**Response**:
```json
{
  "id": 1,
  "name": "Alice Johnson", 
  "email": "alice@example.com"
}
```

### List all users
```bash
curl http://127.0.0.1:3000/api/users
```

### Get specific user
```bash
curl http://127.0.0.1:3000/api/users/1
```

## What Just Happened?

The `#[elif::bootstrap(MyApp)]` macro generated all this setup code automatically:

<details>
<summary>Generated server setup (click to expand)</summary>

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging initialization
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = env_logger::try_init();
    
    // Module discovery and dependency injection
    let bootstrapper = MyApp::bootstrap()?
        .with_config(HttpConfig::default());
    
    // Server startup with all routes and middleware
    bootstrapper.listen("127.0.0.1:3000").await?;
    
    Ok(())
}
```

Plus dependency injection wiring, route registration, middleware application, and error handling - all automatically generated!

</details>

## Step 5: Add Production Configuration

For production, customize the bootstrap:

```rust
#[elif::bootstrap(
    MyApp,
    addr = "0.0.0.0:8080",                    // Production address
    config = HttpConfig::production(),         // Production optimized
    middleware = [cors(), logging(), auth()]   // Global middleware
)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Production API server starting on port 8080!");
}
```

## Compare to Traditional Setup

### Traditional Rust Web Framework (~50+ lines)
```rust
use actix_web::{web, App, HttpServer, HttpResponse, Result};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::default())
            .service(
                web::scope("/api")
                    .service(
                        web::resource("/users")
                            .route(web::get().to(list_users))
                            .route(web::post().to(create_user))
                    )
                    .service(
                        web::resource("/users/{id}")
                            .route(web::get().to(get_user))
                    )
            )
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}

// Plus separate handler functions, manual DI, error handling...
```

### elif.rs Way (~10 lines)
```rust
#[module(controllers: [UserController], providers: [UserService], is_app)]
struct MyApp;

#[elif::bootstrap(MyApp)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Server starting!");
}
```

**Result**: **80% less boilerplate**, automatic everything, Laravel-level developer experience!

## Next Steps

### Add More Features

**Database Integration**:
```rust
#[module(
    controllers: [UserController], 
    providers: [UserService, DatabaseService],
    is_app
)]
struct MyApp;
```

**Authentication**:
```rust
#[controller("/api/users")]
#[middleware("auth")]  // Protect all endpoints
impl UserController {
    #[post("")]
    #[middleware("validate")] // Additional validation
    async fn create(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        // Implementation
    }
}
```

**Custom Address**:
```rust
#[elif::bootstrap(MyApp, addr = "0.0.0.0:8080")]
async fn main() -> Result<(), HttpError> {}
```

### Learn More

- **[Bootstrap Macro Guide](bootstrap-macro.md)** - Deep dive into all bootstrap options
- **[Controllers](../basics/controllers.md)** - Master declarative routing
- **[Dependency Injection](../basics/dependency-injection.md)** - Organize your services
- **[Project Structure](project-structure.md)** - Scale your applications

## The Laravel Moment in Rust

This is what web development in Rust should feel like:

- ✅ **Zero boilerplate** - Focus on business logic, not setup
- ✅ **Convention over configuration** - Sensible defaults that just work
- ✅ **Laravel-style DX** - Complex things made simple through intelligent conventions
- ✅ **Full Rust performance** - Zero runtime overhead from abstractions
- ✅ **AI-friendly** - Clear, predictable patterns that both humans and LLMs love

**Welcome to the future of Rust web development!** 🚀