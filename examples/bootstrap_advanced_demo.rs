//! Advanced Bootstrap Macro Demonstration
//! 
//! This example shows advanced usage of the bootstrap macro
//! with custom configuration and middleware.

use elif::prelude::*;

// Example app module
#[module(is_app)]  
struct AppModule;

// Custom configuration helper
fn custom_config() -> HttpConfig {
    HttpConfig::default()
}

// Mock middleware functions (in real app these would be proper middleware)
fn cors_middleware() -> impl Middleware {
    // Return a dummy middleware for demo
    struct DummyMiddleware;
    
    impl Middleware for DummyMiddleware {
        fn handle(&self, _req: ElifRequest, _next: Box<dyn Fn(ElifRequest) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>>) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> {
            unimplemented!("Demo middleware")
        }
    }
    
    DummyMiddleware
}

fn auth_middleware() -> impl Middleware {
    struct DummyAuthMiddleware;
    
    impl Middleware for DummyAuthMiddleware {
        fn handle(&self, _req: ElifRequest, _next: Box<dyn Fn(ElifRequest) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>>) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> {
            unimplemented!("Demo auth middleware")
        }
    }
    
    DummyAuthMiddleware
}

// Advanced bootstrap with all options
#[elif::bootstrap(
    AppModule,
    addr = "0.0.0.0:8080",
    config = custom_config(),
    middleware = [cors_middleware(), auth_middleware()]
)]
async fn main() -> Result<(), HttpError> {
    println!("🚀 Advanced bootstrap demo starting on port 8080!");
    
    // Custom setup can still happen here
    println!("📦 With custom configuration and middleware");
    
    Ok(())
}