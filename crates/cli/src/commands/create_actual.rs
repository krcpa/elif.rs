use elif_core::ElifError;
use std::fs;
use std::path::Path;
use crate::generators::TemplateEngine;
use tera::Context;

/// Create a new elif application with module system templates
pub async fn app(name: &str, path: Option<&str>, template: &str, modules: bool) -> Result<(), ElifError> {
    let target_path = path.unwrap_or(".");
    let app_path = Path::new(target_path).join(name);
    
    // Check if directory already exists
    if app_path.exists() {
        return Err(ElifError::new(&format!("Directory '{}' already exists", app_path.display())));
    }
    
    println!("🚀 Creating elif.rs application '{}'", name);
    println!("📁 Path: {}", app_path.display());
    println!("📋 Template: {}", template);
    if modules {
        println!("🧩 Including module system setup");
    }
    
    // Create directory structure
    create_directory_structure(&app_path, template, modules)?;
    
    // Generate files using templates
    generate_files_from_templates(&app_path, name, template, modules)?;
    
    // Generate module system files if requested
    if modules {
        generate_module_system(&app_path)?;
    }
    
    // Generate configuration files
    generate_config_files(&app_path, template)?;
    
    // Initialize git repository and create initial commit
    initialize_git_repo(&app_path)?;
    
    println!("\n✅ Successfully created elif.rs application '{}'", name);
    println!("\n📖 Next steps:");
    println!("   cd {}", name);
    if modules {
        println!("   elifrs add module AppModule --controllers=AppController");
    }
    println!("   elifrs dev");
    println!("\n🎯 Happy coding with elif.rs - The Laravel of Rust! 🦀");
    
    Ok(())
}

fn generate_files_from_templates(path: &Path, name: &str, _template: &str, modules: bool) -> Result<(), ElifError> {
    let template_engine = TemplateEngine::new()?;
    
    // Create context for templates
    let mut context = Context::new();
    context.insert("project_name", name);
    context.insert("http_enabled", &true);
    context.insert("modules_enabled", &modules);
    context.insert("database_enabled", &true);
    context.insert("database_type", "postgresql");
    context.insert("auth_enabled", &false);
    
    // Generate Cargo.toml
    let cargo_toml = template_engine.render_with_context("cargo_toml.stub", &context)?;
    fs::write(path.join("Cargo.toml"), cargo_toml)?;
    
    // Generate main.rs with bootstrap template (Laravel-style one-liner)
    let main_rs = template_engine.render_with_context("main_bootstrap.stub", &context)?;
    fs::write(path.join("src/main.rs"), main_rs)?;
    
    // Generate controllers and services
    let controllers_mod = template_engine.render_with_context("controllers_mod.stub", &context)?;
    fs::write(path.join("src/controllers/mod.rs"), controllers_mod)?;
    
    let user_controller = template_engine.render_with_context("user_controller.stub", &context)?;
    fs::write(path.join("src/controllers/user_controller.rs"), user_controller)?;
    
    let services_mod = template_engine.render_with_context("services_mod.stub", &context)?;
    fs::write(path.join("src/services/mod.rs"), services_mod)?;
    
    let user_service = template_engine.render_with_context("user_service.stub", &context)?;
    fs::write(path.join("src/services/user_service.rs"), user_service)?;
    
    if modules && !path.join("src/modules/mod.rs").exists() {
        let modules_mod = "pub mod app_module;";
        fs::write(path.join("src/modules/mod.rs"), modules_mod)?;
        
        // Add template variables for bootstrap module
        context.insert("controller_name", "UserController");
        context.insert("service_name", "UserService");
        let app_module = template_engine.render_with_context("app_module_bootstrap.stub", &context)?;
        fs::write(path.join("src/modules/app_module.rs"), app_module)?;
    }
    
    Ok(())
}

fn initialize_git_repo(path: &Path) -> Result<(), ElifError> {
    use std::process::Command;
    
    // Initialize git repository
    let output = Command::new("git")
        .args(&["init"])
        .current_dir(path)
        .output();
        
    match output {
        Ok(output) if output.status.success() => {
            println!("🔧 Initialized git repository");
        }
        _ => {
            // Git might not be available, continue silently
        }
    }
    
    // Add all files
    let _ = Command::new("git")
        .args(&["add", "."])
        .current_dir(path)
        .output();
    
    // Create initial commit
    let _ = Command::new("git")
        .args(&["commit", "-m", "Initial commit - Created with elif.rs CLI"])
        .current_dir(path)
        .output();
    
    if let Ok(output) = Command::new("git").args(&["status", "--porcelain"]).current_dir(path).output() {
        if output.stdout.is_empty() {
            println!("📝 Created initial commit");
        }
    }
    
    Ok(())
}

fn create_directory_structure(path: &Path, template: &str, modules: bool) -> Result<(), ElifError> {
    // Create base directories
    fs::create_dir_all(path)?;
    fs::create_dir_all(path.join("src"))?;
    
    let dirs = [
        "src/controllers",
        "src/services",
        "src/middleware", 
        "src/models",
        "src/routes",
        "resources",
        "migrations",
        "tests",
    ];
    
    for dir in &dirs {
        fs::create_dir_all(path.join(dir))?;
    }
    
    if modules {
        fs::create_dir_all(path.join("src/modules"))?;
    }
    
    match template {
        "api" => {
            // API-specific directories already created above
        }
        "web" => {
            fs::create_dir_all(path.join("src/controllers"))?;
            fs::create_dir_all(path.join("src/models"))?;
            fs::create_dir_all(path.join("src/middleware"))?;
            fs::create_dir_all(path.join("src/views"))?;
            fs::create_dir_all(path.join("public"))?;
        }
        "minimal" => {
            // Just src/ directory
        }
        _ => {
            return Err(ElifError::new(&format!("Unknown template: {}", template)));
        }
    }
    
    // Create common directories
    fs::create_dir_all(path.join("migrations"))?;
    fs::create_dir_all(path.join("tests"))?;
    
    // Create modules directory if module system enabled
    if modules {
        fs::create_dir_all(path.join("src/modules"))?;
    }
    
    Ok(())
}

fn generate_cargo_toml(path: &Path, name: &str, template: &str) -> Result<(), ElifError> {
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
elif-http = {{ version = "0.8.0", features = ["derive"] }}
elif-core = "0.6.0"
elif-orm = "0.7.0"
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"
{}
"#, name, match template {
        "api" => r#"
# API-specific dependencies
elif-openapi = "0.2.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
"#,
        "web" => r#"
# Web-specific dependencies  
elif-cache = "0.3.0"
elif-storage = "0.2.0"
askama = "0.12"
"#,
        _ => ""
    });
    
    fs::write(path.join("Cargo.toml"), cargo_toml)?;
    Ok(())
}

fn generate_main_file(path: &Path, template: &str, modules: bool) -> Result<(), ElifError> {
    let main_content = match template {
        "api" if modules => r#"use elif_http::{Server, Router, ElifRequest, ElifResponse, HttpResult};
use elif_core::container::Container;

mod modules;

use modules::app_module::AppModule;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize IoC container with modules
    let container = Container::builder()
        .register_module(AppModule)
        .build()?;
    
    // Build router with module-aware routing
    let router = Router::new()
        .with_container(container);
    
    // Start server
    println!("🚀 Starting elif.rs API server on http://127.0.0.1:3000");
    Server::new()
        .router(router)
        .listen("127.0.0.1:3000")
        .await?;
    
    Ok(())
}
"#,
        "api" => r#"use elif_http::{Server, Router, get, ElifRequest, ElifResponse, HttpResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .route("/", get(hello))
        .route("/health", get(health_check));
    
    println!("🚀 Starting elif.rs API server on http://127.0.0.1:3000");
    Server::new()
        .router(router)
        .listen("127.0.0.1:3000")
        .await?;
    
    Ok(())
}

async fn hello(_req: ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::json(&serde_json::json!({
        "message": "Hello from elif.rs - The Laravel of Rust! 🦀",
        "framework": "elif.rs",
        "version": "0.8.0"
    }))?)
}

async fn health_check(_req: ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::json(&serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    }))?)
}
"#,
        "web" if modules => r#"use elif_http::{Server, Router, get, ElifRequest, ElifResponse, HttpResult};
use elif_core::container::Container;

mod modules;

use modules::app_module::AppModule;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize IoC container with modules
    let container = Container::builder()
        .register_module(AppModule)
        .build()?;
    
    // Build router with module-aware routing
    let router = Router::new()
        .with_container(container)
        .route("/", get(home));
    
    // Start server
    println!("🚀 Starting elif.rs web server on http://127.0.0.1:3000");
    Server::new()
        .router(router)
        .listen("127.0.0.1:3000")
        .await?;
    
    Ok(())
}

async fn home(_req: ElifRequest) -> HttpResult<ElifResponse> {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Welcome to elif.rs</title>
    <style>
        body { font-family: system-ui; margin: 40px auto; max-width: 800px; }
        .header { text-align: center; margin-bottom: 40px; }
        .emoji { font-size: 4rem; }
    </style>
</head>
<body>
    <div class="header">
        <div class="emoji">🦀</div>
        <h1>Welcome to elif.rs</h1>
        <p>The Laravel of Rust - Simple, Elegant, Productive</p>
    </div>
    
    <h2>🚀 Getting Started</h2>
    <p>Your elif.rs application is up and running! Here's what you can do next:</p>
    
    <ul>
        <li><strong>Add a module:</strong> <code>elifrs add module UserModule</code></li>
        <li><strong>Create a controller:</strong> <code>elifrs add controller UserController --to=UserModule</code></li>
        <li><strong>Add middleware:</strong> <code>elifrs add middleware AuthMiddleware</code></li>
        <li><strong>Inspect your app:</strong> <code>elifrs inspect modules --graph</code></li>
    </ul>
    
    <p>Happy coding! 🎯</p>
</body>
</html>"#;
    
    Ok(ElifResponse::ok().html(html)?)
}
"#,
        "minimal" => r#"use elif_http::{Server, Router, get, ElifRequest, ElifResponse, HttpResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .route("/", get(hello));
    
    Server::new()
        .router(router)
        .listen("127.0.0.1:3000")
        .await?;
    
    Ok(())
}

async fn hello(_req: ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().text("Hello from elif.rs!")?)
}
"#,
        _ => r#"use elif_http::{Server, Router, get, ElifRequest, ElifResponse, HttpResult};

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()
        .route("/", get(hello));
    
    Server::new()
        .router(router)
        .listen("127.0.0.1:3000")
        .await?;
    
    Ok(())
}

async fn hello(_req: ElifRequest) -> HttpResult<ElifResponse> {
    Ok(ElifResponse::ok().text("Hello from elif.rs!")?)
}
"#
    };
    
    fs::write(path.join("src").join("main.rs"), main_content)?;
    Ok(())
}

fn generate_module_system(path: &Path) -> Result<(), ElifError> {
    // Create modules/mod.rs
    let modules_mod = r#"pub mod app_module;
"#;
    fs::write(path.join("src").join("modules").join("mod.rs"), modules_mod)?;
    
    // Create app_module.rs
    let app_module = r#"use elif_core::container::module;
use elif::prelude::*;

#[module(
    controllers = [],
    providers = [],
    imports = [],
    exports = []
)]
pub struct AppModule;
"#;
    fs::write(path.join("src").join("modules").join("app_module.rs"), app_module)?;
    
    Ok(())
}

fn generate_config_files(path: &Path, _template: &str) -> Result<(), ElifError> {
    // Generate .env file
    let env_content = r#"# Application Environment
APP_NAME=ElifApp
APP_ENV=development
APP_KEY=generate_with_elifrs_auth_generate_key

# Database
DATABASE_URL=postgresql://user:password@localhost/elifapp_development

# Server
HOST=127.0.0.1
PORT=3000
"#;
    fs::write(path.join(".env"), env_content)?;
    
    // Generate elif.toml configuration
    let elif_toml = r#"[app]
name = "ElifApp"
version = "0.1.0"

[server]
host = "127.0.0.1"
port = 3000

[database]
url = "${DATABASE_URL}"
max_connections = 10

[cache]
driver = "memory"

[logging]
level = "info"
format = "json"
"#;
    fs::write(path.join("elif.toml"), elif_toml)?;
    
    // Generate .gitignore
    let gitignore = r#"# Rust
/target/
Cargo.lock

# IDE
.vscode/
.idea/

# Environment
.env
.env.local
.env.production

# Logs
*.log

# OS
.DS_Store
Thumbs.db
"#;
    fs::write(path.join(".gitignore"), gitignore)?;
    
    Ok(())
}