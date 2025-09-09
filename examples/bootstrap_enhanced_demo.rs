//! Enhanced Bootstrap Demonstration
//! 
//! This example shows the enhanced #[elif::bootstrap] macro with parameters
//! but still using auto-discovery (no AppModule required).

use elif::prelude::*;

// Example using the enhanced bootstrap macro with parameters
#[elif::bootstrap(
    addr = "0.0.0.0:8080",
    config = HttpConfig::default(),
    middleware = []
)]
async fn main() -> Result<(), HttpError> {
    // The macro automatically generates:
    // 1. Module discovery from compile-time registry
    // 2. Controller auto-registration from static registry
    // 3. DI container configuration  
    // 4. Router setup with all controllers
    // 5. Server startup on 0.0.0.0:8080 (custom address)
    
    println!("🚀 Enhanced bootstrap application started on 0.0.0.0:8080!");
    
    Ok(())
}
