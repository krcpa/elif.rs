mod routes;
mod introspection;

use elif_core::{Container, container::test_implementations::*};
use elif_http::{Server, HttpConfig, ElifRouter};
use elif_security::CorsMiddleware;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create container with DI services
    let config = Arc::new(create_test_config());
    let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    
    let container = Container::builder()
        .config(config)
        .database(database)
        .build()?
        .into();

    // Create HTTP configuration
    let mut http_config = HttpConfig::default();
    http_config.port = 8080;

    // Create application router with framework abstractions
    let router = create_app_router();
    
    // Create and configure server using framework
    let mut server = Server::with_container(container, http_config)?;
    server.use_router(router);
    
    // Add CORS middleware using framework middleware
    server.use_middleware(CorsMiddleware::permissive());

    println!("🚀 Server running on http://0.0.0.0:8080");
    println!("📖 OpenAPI docs at http://0.0.0.0:8080/_ui");
    println!("🗺️  Project map at http://0.0.0.0:8080/_map.json");
    println!("🔧 Framework: Pure Elif.rs abstractions");

    // Start server using framework
    server.listen("0.0.0.0:8080").await?;
    
    Ok(())
}

fn create_app_router() -> ElifRouter {
    ElifRouter::new()
        .merge(introspection::framework_router())
        .merge(routes::framework_router())
}