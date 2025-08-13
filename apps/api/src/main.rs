mod routes;
mod introspection;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = create_app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    
    println!("🚀 Server running on http://0.0.0.0:8080");
    println!("📖 OpenAPI docs at http://0.0.0.0:8080/_ui");
    println!("🗺️  Project map at http://0.0.0.0:8080/_map.json");

    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([CONTENT_TYPE])
        .allow_origin(Any);

    Router::new()
        .merge(introspection::router())
        .merge(routes::router())
        .layer(cors)
}