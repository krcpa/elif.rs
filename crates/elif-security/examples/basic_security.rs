//! Basic Security Example
//!
//! This example demonstrates how to set up a web server with basic security middleware
//! including CORS, CSRF protection, and rate limiting.

use elif_core::Container;
use elif_http::{Server, routing::Router, response::Response, request::Request, HttpResult, HttpConfig};
use elif_security::basic_security_pipeline;

/// A simple handler that returns a greeting
async fn hello_handler(_req: Request) -> HttpResult<Response> {
    Ok(Response::builder()
        .status(200)
        .body("Hello, World! This endpoint is protected by security middleware.".into())
        .unwrap())
}

/// A handler that demonstrates CSRF-protected state changes
async fn create_user_handler(_req: Request) -> HttpResult<Response> {
    // In a real application, this would:
    // 1. Validate CSRF token (automatically handled by middleware)
    // 2. Create user in database
    // 3. Return success response
    
    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(r#"{"message": "User created successfully", "id": 123}"#.into())
        .unwrap())
}

/// A public API endpoint (could be exempted from some security checks)
async fn public_api_handler(_req: Request) -> HttpResult<Response> {
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(r#"{"data": "This is public data", "version": "1.0"}"#.into())
        .unwrap())
}

/// Health check endpoint (typically exempted from rate limiting)
async fn health_check_handler(_req: Request) -> HttpResult<Response> {
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(r#"{"status": "healthy", "timestamp": "2024-01-01T12:00:00Z"}"#.into())
        .unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create dependency injection container
    let mut container = Container::default();

    // Create basic security pipeline
    // This includes:
    // - Permissive CORS (allows common origins)
    // - Default rate limiting (100 requests per minute by IP)
    // - Default CSRF protection (with secure defaults)
    let security_pipeline = basic_security_pipeline();

    // Create router with protected endpoints
    let router = Router::new()
        .get("/", hello_handler)
        .post("/users", create_user_handler)  // Protected by CSRF
        .get("/api/public", public_api_handler)
        .get("/health", health_check_handler);

    // Create and configure server
    let config = HttpConfig::default();
    let mut server = Server::with_container(std::sync::Arc::new(container), config)?
        .use_middleware(security_pipeline);  // Apply security middleware
    
    server.use_router(router);

    println!("🛡️  Starting server with basic security middleware...");
    println!("📍 Server running at http://127.0.0.1:3000");
    println!("🔒 Security features enabled:");
    println!("   ✅ CORS protection");
    println!("   ✅ CSRF protection (for POST/PUT/DELETE/PATCH requests)");
    println!("   ✅ Rate limiting (100 requests/minute per IP)");
    println!();
    println!("Try these requests:");
    println!("  GET  http://127.0.0.1:3000/           - Basic protected endpoint");
    println!("  POST http://127.0.0.1:3000/users      - CSRF protected endpoint");
    println!("  GET  http://127.0.0.1:3000/api/public - Public API endpoint");
    println!("  GET  http://127.0.0.1:3000/health     - Health check");
    println!();
    println!("Security headers to try:");
    println!("  Origin: https://example.com           - Test CORS");
    println!("  X-CSRF-Token: <token>                 - Include CSRF token for POST requests");

    // Start the server
    server.listen("127.0.0.1:3000").await?;

    Ok(())
}

// Example of how to test this server:
// 
// 1. Start the server: cargo run --example basic_security
//
// 2. Test CORS preflight:
//    curl -X OPTIONS http://127.0.0.1:3000/users \
//      -H "Origin: https://example.com" \
//      -H "Access-Control-Request-Method: POST" \
//      -H "Access-Control-Request-Headers: content-type,x-csrf-token"
//
// 3. Test rate limiting:
//    for i in {1..105}; do curl http://127.0.0.1:3000/ & done
//
// 4. Test CSRF protection:
//    curl -X POST http://127.0.0.1:3000/users \
//      -H "Content-Type: application/json" \
//      -d '{"name": "John Doe"}'
//    # Should fail without CSRF token
//
// 5. Test with CSRF token (first get token from cookie/header):
//    curl -X POST http://127.0.0.1:3000/users \
//      -H "Content-Type: application/json" \
//      -H "X-CSRF-Token: <token-from-previous-request>" \
//      -d '{"name": "John Doe"}'