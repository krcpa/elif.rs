use elif_http::{Server, HttpConfig};
use elif_http::middleware::v2::{LoggingMiddleware, ProfilerMiddleware};
use elif_http::middleware::v2::factories;
use elif_core::Container;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create container and server
    let container = Container::new();
    let mut server = Server::new(container, HttpConfig::default())?;

    // Add some middleware
    server
        .use_middleware(LoggingMiddleware)
        .use_middleware(factories::cors())
        .use_middleware(factories::rate_limit(60))
        .use_profiler();

    // Demonstrate debugging tools
    println!("=== Middleware Debugging Tools Demo ===\n");

    // Inspect middleware pipeline
    server.inspect_middleware();
    
    println!();
    
    // Enable debug mode
    server.debug_middleware(true);

    println!("\n✅ Debugging tools demonstration complete!");
    println!("   In production, you would now call server.listen(\"0.0.0.0:3000\").await");
    
    Ok(())
}