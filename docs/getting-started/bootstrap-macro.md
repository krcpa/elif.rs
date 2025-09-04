# Zero-Boilerplate Bootstrap with #[elif::bootstrap]

The `#[elif::bootstrap]` macro embodies elif.rs's core philosophy of **"convention over configuration"** by providing true Laravel-style zero-boilerplate application startup.

## The Laravel Moment

Remember how Laravel revolutionized PHP by making complex things simple? The `#[elif::bootstrap]` macro brings that same breakthrough to Rust:

### Before: Manual Setup (The Old Way)
```rust
#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Manual server setup - lots of boilerplate
    let container = IocContainer::new();
    let router = ElifRouter::new().controller(UserController);
    let server = Server::new(container, config)?;
    server.use_router(router);
    server.listen("127.0.0.1:3000").await?;
    Ok(())
}
```

### After: Zero Boilerplate (The elif.rs Way) ✨
```rust
use elif::prelude::*;

#[module(
    controllers: [UserController],
    providers: [UserService],
    is_app
)]
struct AppModule;

#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    // Everything happens automatically! 🚀
}
```

**Result**: **70% less code**, automatic configuration, and Laravel-level developer experience.

## How It Works

The bootstrap macro generates all the setup code you would normally write by hand:

1. **🔍 Module Discovery** - Finds all modules in your application automatically
2. **⚡ DI Container Setup** - Configures dependency injection with all services
3. **🛣️ Route Registration** - Registers all controllers and their endpoints
4. **🚀 Server Startup** - Starts the HTTP server with proper middleware pipeline

## Basic Usage

### Simple Application
```rust
use elif::prelude::*;

// Define your app module
#[module(is_app)]
struct AppModule;

// Zero-boilerplate startup
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Server starting...");
    // Server automatically starts on 127.0.0.1:3000
}
```

### With Controllers and Services
```rust
use elif::prelude::*;

// Your controllers (using declarative macros)
#[controller("/api/users")]
impl UserController {
    #[get("")]
    async fn list(&self) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().json(&vec!["user1", "user2"])?)
    }
}

// Your services  
#[derive(Default)]
struct UserService;

// App module bringing everything together
#[module(
    controllers: [UserController],
    providers: [UserService],
    is_app
)]
struct AppModule;

// Zero-boilerplate server setup
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    // All wiring happens automatically!
    // ✅ UserController registered at /api/users/*
    // ✅ UserService available for dependency injection
    // ✅ Server listening on 127.0.0.1:3000
}
```

## Configuration Options

The bootstrap macro supports flexible configuration for different environments:

### Custom Address/Port
```rust
#[elif::bootstrap(AppModule, addr = "0.0.0.0:8080")]
async fn main() -> Result<(), HttpError> {
    // Server starts on all interfaces, port 8080
}
```

### Production Configuration
```rust
#[elif::bootstrap(
    AppModule,
    addr = "0.0.0.0:8080",
    config = HttpConfig::production()
)]
async fn main() -> Result<(), HttpError> {
    // Uses production-optimized settings
}
```

### Global Middleware
```rust
#[elif::bootstrap(
    AppModule,
    middleware = [cors(), logging(), rate_limiting()]
)]
async fn main() -> Result<(), HttpError> {
    // Applies middleware to all routes
}
```

### Full Configuration
```rust
#[elif::bootstrap(
    AppModule,
    addr = "0.0.0.0:8080",
    config = HttpConfig::production(),
    middleware = [cors(), auth(), logging()]
)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Production server starting on port 8080");
    // Fully configured production setup
}
```

## Real-World Example

Here's a complete blog API using the bootstrap macro:

```rust
use elif::prelude::*;

// Controllers
#[controller("/api/posts")]
#[middleware("cors")]
impl PostController {
    #[get("")]
    #[middleware("cache")]
    async fn list(&self) -> HttpResult<ElifResponse> {
        let posts = vec!["Post 1", "Post 2"];
        Ok(ElifResponse::ok().json(&posts)?)
    }
    
    #[post("")]
    #[middleware("auth")]
    async fn create(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let post: CreatePost = req.json().await?;
        Ok(ElifResponse::created().json(&post)?)
    }
}

#[controller("/api/users")]  
impl UserController {
    #[post("/register")]
    async fn register(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let user: RegisterUser = req.json().await?;
        Ok(ElifResponse::created().json(&user)?)
    }
}

// Services
#[derive(Default)]
struct PostService;

#[derive(Default)] 
struct UserService;

#[derive(Default)]
struct EmailService;

// App module - defines the entire application
#[module(
    controllers: [PostController, UserController],
    providers: [PostService, UserService, EmailService],
    is_app
)]
struct BlogApp;

// Production configuration
fn production_config() -> HttpConfig {
    HttpConfig::default()
        .with_timeout(Duration::from_secs(30))
        .with_max_payload_size(1024 * 1024) // 1MB
}

// Zero-boilerplate production server
#[elif::bootstrap(
    BlogApp,
    addr = "0.0.0.0:8080", 
    config = production_config(),
    middleware = [cors(), logging(), rate_limiting()]
)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Blog API started!");
    println!("📝 Endpoints available:");
    println!("   GET  /api/posts     - List posts");
    println!("   POST /api/posts     - Create post (auth required)"); 
    println!("   POST /api/users/register - Register user");
    
    // Server automatically handles:
    // ✅ Module discovery and dependency injection
    // ✅ Route registration with middleware
    // ✅ CORS, logging, and rate limiting 
    // ✅ Production-optimized HTTP config
    // ✅ Graceful error handling
}
```

**What this generates**: A complete production-ready API server with dependency injection, middleware pipeline, and proper error handling - all from ~10 lines of actual setup code!

## Generated Code Deep Dive

Want to see what the macro generates? Here's the equivalent manual code:

<details>
<summary>Click to see generated code</summary>

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = env_logger::try_init();
    
    // Original function logic
    async fn inner_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Module discovery and bootstrap
        let bootstrapper = BlogApp::bootstrap()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            .with_config(production_config())
            .with_middleware(vec![
                Box::new(cors()),
                Box::new(logging()), 
                Box::new(rate_limiting())
            ]);
        
        // Start the server
        bootstrapper
            .listen("0.0.0.0:8080".parse().expect("Invalid socket address"))
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        Ok(())
    }
    
    // Run with error handling
    match inner_main().await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Application bootstrap failed: {}", e);
            Err(e)
        }
    }
}
```

</details>

## Error Handling

The bootstrap macro provides clear, actionable error messages:

### Invalid Function Signature
```rust
#[elif::bootstrap(AppModule)]
fn main() {  // ❌ Missing async and Result
}
```

**Compile Error**:
```
error: Bootstrap macro can only be applied to async functions

💡 Change your function to:
async fn main() -> Result<(), HttpError> {}
```

### Unknown Parameter
```rust
#[elif::bootstrap(AppModule, invalid_param = "value")]  // ❌
async fn main() -> Result<(), HttpError> {}
```

**Compile Error**:
```
error: Unknown bootstrap parameter 'invalid_param'. Valid parameters are: addr, config, middleware

💡 Usage examples:
• #[elif::bootstrap(AppModule)]
• #[elif::bootstrap(AppModule, addr = "127.0.0.1:3000")]
• #[elif::bootstrap(AppModule, config = my_config())]
• #[elif::bootstrap(AppModule, middleware = [cors(), auth()])]
```

## Development vs Production

### Development Setup
```rust
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    // Uses development defaults:
    // ✅ 127.0.0.1:3000 (localhost only)
    // ✅ Debug logging enabled
    // ✅ Hot reload support (when available)
    // ✅ Detailed error messages
}
```

### Production Setup  
```rust
#[elif::bootstrap(
    AppModule,
    addr = "0.0.0.0:8080",
    config = HttpConfig::production(),
    middleware = [cors(), auth(), rate_limiting(), logging()]
)]
async fn main() -> Result<(), HttpError> {
    // Production optimized:
    // ✅ Binds to all interfaces
    // ✅ Production HTTP settings
    // ✅ Full middleware pipeline
    // ✅ Structured logging
    // ✅ Performance monitoring
}
```

## Migration Guide

### From Manual Setup

**Old code**:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let container = IocContainer::new();
    container.register::<UserService>();
    
    let router = ElifRouter::new()
        .controller(UserController)
        .controller(PostController);
        
    let server = Server::new(container, HttpConfig::default())?
        .use_router(router)
        .use_middleware(cors())
        .use_middleware(auth());
        
    server.listen("127.0.0.1:3000").await?;
    Ok(())
}
```

**New code**:
```rust
#[module(
    controllers: [UserController, PostController],
    providers: [UserService],
    is_app
)]
struct AppModule;

#[elif::bootstrap(AppModule, middleware = [cors(), auth()])]
async fn main() -> Result<(), HttpError> {
    // All the setup happens automatically!
}
```

**Benefits**:
- ✅ 70% less code
- ✅ Compile-time validation
- ✅ No manual container configuration
- ✅ No manual route registration
- ✅ Automatic dependency resolution

## Best Practices

### 1. **App Module Organization**
```rust
// ✅ Good - organized by feature
#[module(
    controllers: [UserController, PostController],
    providers: [DatabaseService, CacheService, EmailService],
    imports: [AuthModule, ApiModule],
    is_app
)]
struct BlogApp;
```

### 2. **Environment-Specific Configuration**
```rust
// ✅ Good - different configs for different environments
#[cfg(debug_assertions)]
#[elif::bootstrap(AppModule, addr = "127.0.0.1:3000")]
async fn main() -> Result<(), HttpError> {}

#[cfg(not(debug_assertions))]
#[elif::bootstrap(
    AppModule,
    addr = "0.0.0.0:8080",
    config = HttpConfig::production(),
    middleware = [cors(), auth(), rate_limiting()]
)]
async fn main() -> Result<(), HttpError> {}
```

### 3. **Custom Setup Code**
```rust
// ✅ Good - bootstrap handles server, you handle business logic
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    // Database migrations
    run_migrations().await?;
    
    // Cache warm-up
    warm_caches().await?;
    
    // Custom initialization
    initialize_external_services().await?;
    
    println!("🚀 Application ready!");
    // Server starts automatically after this function
}
```

## FAQ

### Q: Can I still do custom server configuration?
**A**: Yes! The bootstrap macro generates the standard server setup, but you can override any part using the `config` parameter or by doing setup before the server starts.

### Q: How do I add custom middleware?
**A**: Use the `middleware` parameter: `middleware = [cors(), auth(), your_middleware()]`

### Q: Can I use this with multiple app modules?
**A**: The bootstrap macro expects one root app module, but your app module can import other modules using the `imports` parameter.

### Q: What about testing?
**A**: Create separate test modules without `is_app` flag for testing specific components. For integration tests, you can create a test-specific app module.

### Q: How do I debug what the macro generates?
**A**: Use `cargo expand` to see the generated code, or check the compiler errors which show the generated function signatures.

## What's Next?

Now that you understand zero-boilerplate bootstrap, explore:

- **[Project Structure](project-structure.md)** - How to organize your elif.rs applications
- **[Controllers](../basics/controllers.md)** - Build declarative HTTP endpoints
- **[Dependency Injection](../basics/dependency-injection.md)** - Organize your services
- **[Middleware](../basics/middleware.md)** - Add cross-cutting concerns
- **[Configuration](configuration.md)** - Environment-specific settings

**The Laravel Experience in Rust** - Complex infrastructure made simple through intelligent conventions. 🚀
