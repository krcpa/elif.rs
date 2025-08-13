use elif_core::ElifError;
use std::fs;
use std::path::Path;

pub async fn create_app(name: &str, target_path: Option<&str>) -> Result<(), ElifError> {
    let app_path = match target_path {
        Some(path) => format!("{}/{}", path, name),
        None => format!("../{}", name),
    };
    
    let app_dir = Path::new(&app_path);
    
    if app_dir.exists() {
        return Err(ElifError::Validation(
            format!("Directory {} already exists", app_path)
        ));
    }
    
    println!("📦 Creating new elif application: {}", name);
    
    // Create directory structure
    create_app_structure(&app_dir, name)?;
    
    // Create configuration files
    create_config_files(&app_dir, name)?;
    
    // Create source files
    create_source_files(&app_dir, name)?;
    
    println!("✅ Application '{}' created successfully!", name);
    println!("📂 Location: {}", app_dir.display());
    println!("\n🚀 To get started:");
    println!("   cd {}", app_path);
    println!("   elif route add GET /hello hello_controller");
    println!("   cargo run");
    
    Ok(())
}

fn create_app_structure(app_dir: &Path, _name: &str) -> Result<(), ElifError> {
    let dirs = [
        "src/controllers",
        "src/middleware", 
        "src/models",
        "src/routes",
        "resources",
        "migrations",
        "tests",
        ".elif",
    ];
    
    for dir in &dirs {
        fs::create_dir_all(app_dir.join(dir))?;
    }
    
    Ok(())
}

fn create_config_files(app_dir: &Path, name: &str) -> Result<(), ElifError> {
    // Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
elif-core = {{ path = "../../Code/elif/crates/core" }}
elif-orm = {{ path = "../../Code/elif/crates/orm" }}
axum = "0.7"
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
uuid = {{ version = "1.0", features = ["v4", "serde"] }}
tracing = "0.1"
tracing-subscriber = "0.3"
sqlx = {{ version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }}
tower = "0.4"
tower-http = {{ version = "0.5", features = ["cors"] }}
"#, name);
    
    fs::write(app_dir.join("Cargo.toml"), cargo_toml)?;
    
    // .elif/manifest.yaml
    let manifest = format!(r#"name: {}
version: "0.1.0" 
database:
  url_env: DATABASE_URL
  migrations_dir: migrations
server:
  host: "0.0.0.0"
  port: 3000
routes:
  prefix: "/api/v1"
"#, name);
    
    fs::write(app_dir.join(".elif/manifest.yaml"), manifest)?;
    
    // .env
    let env_content = r#"DATABASE_URL=postgresql://localhost/elif_dev
RUST_LOG=info
"#;
    
    fs::write(app_dir.join(".env"), env_content)?;
    
    Ok(())
}

fn create_source_files(app_dir: &Path, name: &str) -> Result<(), ElifError> {
    // src/main.rs
    let main_rs = r#"mod controllers;
mod middleware;
mod models;
mod routes;

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
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    println!("🚀 Server running on http://0.0.0.0:3000");
    println!("📖 Add routes with: elif route add GET /path controller_name");
    
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE])
        .allow_origin(Any);
    
    Router::new()
        .merge(routes::router())
        .layer(cors)
}
"#;
    
    fs::write(app_dir.join("src/main.rs"), main_rs)?;
    
    // src/routes/mod.rs
    let routes_mod = r#"use axum::Router;

pub fn router() -> Router {
    Router::new()
        // Routes will be added here by `elif route add` command
        // Example: .route("/hello", get(crate::controllers::hello_controller))
}
"#;
    
    fs::write(app_dir.join("src/routes/mod.rs"), routes_mod)?;
    
    // src/controllers/mod.rs
    let controllers_mod = r#"// Controllers will be added here by `elif route add` command
// use axum::{Json, response::Json as ResponseJson};
// use serde_json::Value;

// Example controller:
// pub async fn hello_controller() -> ResponseJson<Value> {
//     ResponseJson(serde_json::json!({"message": "Hello from elif!"}))
// }
"#;
    
    fs::write(app_dir.join("src/controllers/mod.rs"), controllers_mod)?;
    
    // src/models/mod.rs
    fs::write(app_dir.join("src/models/mod.rs"), "// Models will be added here\n")?;
    
    // src/middleware/mod.rs
    fs::write(app_dir.join("src/middleware/mod.rs"), "// Middleware will be added here\n")?;
    
    // README.md
    let readme = format!(r#"# {}

Created with elif.rs - LLM-friendly Rust web framework.

## Quick Start

```bash
# Add a route
elif route add GET /hello hello_controller

# Add a model  
elif model add User name:string email:string

# Run the server
cargo run
```

## Available Commands

- `elif route add METHOD /path controller_name` - Add HTTP route
- `elif model add Name field:type` - Add database model
- `elif migrate` - Run database migrations
- `elif routes` - List all routes

## Structure

- `src/controllers/` - HTTP controllers
- `src/models/` - Database models  
- `src/routes/` - Route definitions
- `src/middleware/` - HTTP middleware
- `migrations/` - Database migrations
- `resources/` - Resource specifications
"#, name);
    
    fs::write(app_dir.join("README.md"), readme)?;
    
    Ok(())
}