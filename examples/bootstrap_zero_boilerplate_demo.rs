//! Zero-Boilerplate Bootstrap Demonstration
//! 
//! This example shows the new zero-boilerplate #[elif::bootstrap] macro
//! that works without requiring an AppModule.

use elif::prelude::*;

// Example using the NEW zero-boilerplate bootstrap macro!
#[elif::bootstrap]
async fn main() -> Result<(), HttpError> {
    // The macro automatically generates:
    // 1. Module discovery from compile-time registry
    // 2. Controller auto-registration from static registry
    // 3. DI container configuration  
    // 4. Router setup with all controllers
    // 5. Server startup on 127.0.0.1:3000
    
    // Any additional setup code can go here
    println!("🚀 Zero-boilerplate application started!");
    
    // The server will start automatically - no manual configuration needed!
    Ok(())
}