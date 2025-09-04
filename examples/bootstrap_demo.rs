//! Bootstrap Macro Demonstration
//! 
//! This example shows how to use the new #[elif::bootstrap] macro
//! for zero-boilerplate application startup.

use elif::prelude::*;

// Example app module - in a real application this would be properly configured
#[module(is_app)]
struct AppModule;

// Example using the bootstrap macro - zero boilerplate!
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {
    // The macro automatically generates:
    // 1. Module discovery from compile-time registry
    // 2. DI container configuration  
    // 3. Router setup with all controllers
    // 4. Server startup on 127.0.0.1:3000
    
    // Any additional setup code can go here
    println!("🚀 Application started with bootstrap macro!");
    
    // The server will start automatically
    Ok(())
}