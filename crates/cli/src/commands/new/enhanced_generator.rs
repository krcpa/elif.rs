use elif_core::ElifError;
use super::interactive_wizard::{ProjectConfig, show_progress};
use indicatif::ProgressBar;
use std::path::Path;
use tokio::fs;

pub async fn generate_project_from_config(config: &ProjectConfig) -> Result<(), ElifError> {
    let pb = show_progress("Creating project structure...");
    
    let app_path = match &config.directory {
        Some(dir) => format!("{}/{}", dir, config.name),
        None => format!("./{}", config.name),
    };
    
    let app_dir = Path::new(&app_path);
    
    if app_dir.exists() {
        pb.finish_with_message("❌ Directory already exists");
        return Err(ElifError::new(&format!("Directory '{}' already exists", app_path)));
    }
    
    // Create directory structure
    pb.set_message("Creating directories...");
    create_enhanced_directory_structure(app_dir, config).await?;
    
    // Generate Cargo.toml with selected dependencies
    pb.set_message("Generating Cargo.toml...");
    generate_enhanced_cargo_toml(app_dir, config).await?;
    
    // Generate main.rs based on configuration
    pb.set_message("Creating main application file...");
    generate_enhanced_main_file(app_dir, config).await?;
    
    // Generate configuration files
    pb.set_message("Setting up configuration...");
    generate_enhanced_config_files(app_dir, config).await?;
    
    // Generate database files if enabled
    if config.database_config.provider != "none" {
        pb.set_message("Setting up database...");
        generate_database_files(app_dir, config).await?;
    }
    
    // Generate auth files if enabled
    if config.auth_config.provider != "none" {
        pb.set_message("Setting up authentication...");
        generate_auth_files(app_dir, config).await?;
    }
    
    // Generate module system if enabled
    if config.modules_enabled {
        pb.set_message("Setting up module system...");
        generate_module_system_files(app_dir, config).await?;
    }
    
    // Generate additional features
    for feature in &config.features {
        pb.set_message(&format!("Adding {}...", feature));
        generate_feature_files(app_dir, config, feature).await?;
    }
    
    // Generate Docker setup if selected
    if config.features.contains(&"Docker Setup".to_string()) {
        pb.set_message("Creating Docker configuration...");
        generate_docker_files(app_dir, config).await?;
    }
    
    // Generate GitHub Actions if selected
    if config.features.contains(&"GitHub Actions".to_string()) {
        pb.set_message("Setting up CI/CD pipeline...");
        generate_github_actions(app_dir, config).await?;
    }
    
    pb.finish_with_message("✅ Project created successfully!");
    
    // Show completion message
    show_completion_message(config, &app_path);
    
    Ok(())
}

async fn create_enhanced_directory_structure(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    let mut dirs = vec![
        "src",
        "tests",
        "migrations",
    ];
    
    // Add directories based on project type
    match config.project_type.as_str() {
        "api" | "microservice" => {
            dirs.extend(vec![
                "src/controllers",
                "src/middleware",
                "src/models",
                "src/services",
                "docs",
            ]);
        }
        "web" => {
            dirs.extend(vec![
                "src/controllers",
                "src/middleware", 
                "src/models",
                "src/services",
                "src/views",
                "public",
                "public/css",
                "public/js",
                "public/images",
                "docs",
            ]);
        }
        "cli" => {
            dirs.extend(vec![
                "src/commands",
                "src/utils",
                "config",
            ]);
        }
        "minimal" => {
            // Just src/ and tests/
        }
        _ => {}
    }
    
    // Add modules directory if enabled
    if config.modules_enabled {
        dirs.push("src/modules");
    }
    
    // Add database directories if database is enabled
    if config.database_config.provider != "none" {
        if config.database_config.include_seeders {
            dirs.push("database/seeders");
        }
        dirs.push("database/migrations");
    }
    
    // Create all directories
    for dir in dirs {
        fs::create_dir_all(app_dir.join(dir)).await?;
    }
    
    Ok(())
}

async fn generate_enhanced_cargo_toml(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    let mut dependencies = vec![
        r#"tokio = { version = "1.0", features = ["full"] }"#.to_string(),
        r#"serde = { version = "1.0", features = ["derive"] }"#.to_string(),
        r#"serde_json = "1.0""#.to_string(),
        r#"anyhow = "1.0""#.to_string(),
    ];
    
    // Add elif dependencies based on project type
    if config.project_type != "cli" {
        dependencies.push(r#"elif-http = { version = "0.8.0", features = ["derive"] }"#.to_string());
    }
    
    if config.modules_enabled {
        dependencies.push(r#"elif-core = "0.6.0""#.to_string());
    }
    
    // Add database dependencies
    match config.database_config.provider.as_str() {
        "postgresql" => {
            dependencies.push(r#"elif-orm = "0.7.0""#.to_string());
            dependencies.push(r#"sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }"#.to_string());
        }
        "mysql" => {
            dependencies.push(r#"elif-orm = "0.7.0""#.to_string());
            dependencies.push(r#"sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "mysql", "uuid", "chrono"] }"#.to_string());
        }
        "sqlite" => {
            dependencies.push(r#"elif-orm = "0.7.0""#.to_string());
            dependencies.push(r#"sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono"] }"#.to_string());
        }
        _ => {}
    }
    
    // Add auth dependencies
    if config.auth_config.provider != "none" {
        dependencies.push(r#"elif-auth = "0.4.0""#.to_string());
        if config.auth_config.include_jwt {
            dependencies.push(r#"jsonwebtoken = "9.0""#.to_string());
        }
    }
    
    // Add feature dependencies
    for feature in &config.features {
        match feature.as_str() {
            "Caching" => {
                dependencies.push(r#"elif-cache = "0.3.0""#.to_string());
            }
            "File Storage" => {
                dependencies.push(r#"elif-storage = "0.2.0""#.to_string());
            }
            "WebSockets" => {
                dependencies.push(r#"tokio-tungstenite = "0.20""#.to_string());
            }
            "Email Service" => {
                dependencies.push(r#"elif-email = "0.1.0""#.to_string());
            }
            "Queue System" => {
                dependencies.push(r#"elif-queue = "0.1.0""#.to_string());
            }
            "OpenAPI Documentation" => {
                dependencies.push(r#"elif-openapi = "0.2.0""#.to_string());
            }
            "Testing Framework" => {
                dependencies.push(r#"elif-testing = "0.1.0""#.to_string());
            }
            _ => {}
        }
    }
    
    // Add common dependencies
    dependencies.push(r#"uuid = { version = "1.0", features = ["v4", "serde"] }"#.to_string());
    dependencies.push(r#"chrono = { version = "0.4", features = ["serde"] }"#.to_string());
    
    // Create Cargo.toml content
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "A {} built with elif.rs - The Laravel of Rust"
authors = ["Your Name <your.email@example.com>"]

[dependencies]
{}

[dev-dependencies]
tempfile = "3.8"
{}
"#,
        config.name,
        match config.project_type.as_str() {
            "api" => "RESTful API server",
            "web" => "full-stack web application", 
            "microservice" => "cloud-native microservice",
            "cli" => "command-line application",
            "minimal" => "Rust application",
            _ => "Rust application",
        },
        dependencies.join("\n"),
        if config.features.contains(&"Testing Framework".to_string()) {
            r#"tokio-test = "0.4""#
        } else {
            ""
        }
    );
    
    fs::write(app_dir.join("Cargo.toml"), cargo_toml).await?;
    Ok(())
}

async fn generate_enhanced_main_file(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    let main_content = match config.project_type.as_str() {
        "api" | "microservice" => generate_api_main(config),
        "web" => generate_web_main(config),
        "cli" => generate_cli_main(config),
        "minimal" => generate_minimal_main(),
        _ => generate_api_main(config),
    };
    
    fs::write(app_dir.join("src").join("main.rs"), main_content).await?;
    Ok(())
}

fn generate_api_main(config: &ProjectConfig) -> String {
    let mut imports = vec![
        "use elif_web::prelude::*;".to_string(),
    ];
    
    let mut setup_code = vec![];
    let mut routes = vec![
        r#"        .route("/", get(hello))"#.to_string(),
        r#"        .route("/health", get(health_check))"#.to_string(),
    ];
    
    // Add module system imports if enabled
    if config.modules_enabled {
        imports.push("use elif_core::container::Container;".to_string());
        imports.push("mod modules;".to_string());
        imports.push("use modules::app_module::AppModule;".to_string());
        
        setup_code.push(r#"    // Initialize IoC container with modules
    let container = Container::builder()
        .register_module(AppModule)
        .build()?;"#.to_string());
        
        routes.insert(0, r#"        .with_container(container)"#.to_string());
    }
    
    // Add database setup if enabled
    if config.database_config.provider != "none" {
        setup_code.push(r#"    // Initialize database connection
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&database_url).await?;"#.to_string());
    }
    
    // Add auth routes if enabled
    if config.auth_config.provider != "none" {
        routes.push(r#"        .route("/auth/login", post(auth_login))"#.to_string());
        routes.push(r#"        .route("/auth/register", post(auth_register))"#.to_string());
        imports.push("use elif_http::post;".to_string());
    }
    
    let formatted_imports = imports.join("\n");
    let formatted_setup = if setup_code.is_empty() {
        "".to_string()
    } else {
        format!("\n{}\n", setup_code.join("\n"))
    };
    let formatted_routes = routes.join("\n");
    
    format!(r#"{}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{{formatted_setup}
    let router = Router::new()
{};

    println!("🚀 Starting {} server on http://127.0.0.1:3000");
    println!("📊 Health check: http://127.0.0.1:3000/health");
    println!("📚 API docs: http://127.0.0.1:3000/docs");
    
    Server::new()
        .router(router)
        .listen("127.0.0.1:3000")
        .await?;

    Ok(())
}}

async fn hello(_req: Request) -> HttpResult<Response> {{
    Ok(Response::json(&serde_json::json!({{
        "message": "Hello from {} - The Laravel of Rust! 🦀",
        "framework": "elif.rs",
        "version": "0.8.0",
        "project": "{}"
    }}))?)
}}

async fn health_check(_req: Request) -> HttpResult<Response> {{
    Ok(Response::json(&serde_json::json!({{
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "service": "{}"
    }}))?)
}}{}"#,
        formatted_imports,
        formatted_routes,
        config.name,
        config.project_type,
        config.name,
        config.name,
        if config.auth_config.provider != "none" {
            r#"

async fn auth_login(_req: Request) -> HttpResult<Response> {
    // TODO: Implement authentication login
    Ok(Response::json(&serde_json::json!({
        "message": "Login endpoint - implement authentication logic here"
    }))?)
}

async fn auth_register(_req: Request) -> HttpResult<Response> {
    // TODO: Implement user registration
    Ok(Response::json(&serde_json::json!({
        "message": "Register endpoint - implement registration logic here"
    }))?)
}"#
        } else {
            ""
        }
    )
}

fn generate_web_main(config: &ProjectConfig) -> String {
    let imports = if config.modules_enabled {
        "use elif_core::container::Container;\nmod modules;\nuse modules::app_module::AppModule;"
    } else {
        ""
    };

    let main_template = format!(
        "use elif_web::prelude::*;
{}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let router = Router::new()
        .route(\"/\", get(home))
        .route(\"/about\", get(about));

    println!(\"🚀 Starting {} web server on http://127.0.0.1:3000\");
    println!(\"🏠 Home page: http://127.0.0.1:3000\");
    
    Server::new()
        .router(router)
        .listen(\"127.0.0.1:3000\")
        .await?;

    Ok(())
}}

async fn home(_req: Request) -> HttpResult<Response> {{
    let html = r#\"<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>{} - Built with elif.rs</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 40px; background: #f8fafc; }}
        .container {{ max-width: 800px; margin: 0 auto; background: white; padding: 40px; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); }}
        .header {{ text-align: center; margin-bottom: 40px; }}
        .emoji {{ font-size: 4rem; margin-bottom: 20px; }}
        h1 {{ color: #1a202c; margin: 0; }}
        p {{ color: #4a5568; line-height: 1.6; }}
        .features {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin: 30px 0; }}
        .feature {{ padding: 20px; background: #f7fafc; border-radius: 8px; border-left: 4px solid #4299e1; }}
        .feature h3 {{ margin: 0 0 10px 0; color: #2d3748; }}
        .cta {{ text-align: center; margin-top: 40px; }}
        .cta a {{ display: inline-block; background: #4299e1; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; font-weight: 500; }}
    </style>
</head>
<body>
    <div class=\"container\">
        <div class=\"header\">
            <div class=\"emoji\">🦀</div>
            <h1>Welcome to {}</h1>
            <p>Built with elif.rs - The Laravel of Rust</p>
        </div>
        
        <div class=\"features\">
            <div class=\"feature\">
                <h3>🚀 Fast Development</h3>
                <p>Laravel-inspired patterns with zero boilerplate and maximum productivity.</p>
            </div>
            <div class=\"feature\">
                <h3>🛡️ Type Safety</h3>
                <p>Rust's compile-time guarantees ensure your application is robust and reliable.</p>
            </div>
            <div class=\"feature\">
                <h3>📦 Batteries Included</h3>
                <p>Everything you need: routing, middleware, database ORM, authentication, and more.</p>
            </div>
            <div class=\"feature\">
                <h3>🧩 Modular Architecture</h3>
                <p>Clean, organized code with dependency injection and service modules.</p>
            </div>
        </div>
        
        <div class=\"cta\">
            <a href=\"/about\">Learn More</a>
        </div>
    </div>
</body>
</html>\"#;
    
    Ok(Response::ok().html(html)?)
}}

async fn about(_req: Request) -> HttpResult<Response> {{
    let html = r#\"<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>About - {}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 40px; background: #f8fafc; }}
        .container {{ max-width: 800px; margin: 0 auto; background: white; padding: 40px; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); }}
        h1 {{ color: #1a202c; }}
        p {{ color: #4a5568; line-height: 1.6; }}
        .back {{ color: #4299e1; text-decoration: none; }}
    </style>
</head>
<body>
    <div class=\"container\">
        <h1>About {}</h1>
        <p>This application was created using elif.rs, a modern Rust web framework inspired by Laravel's elegance and productivity.</p>
        <p>elif.rs brings Laravel's convention-over-configuration philosophy to Rust, making it easy to build robust, type-safe web applications with minimal boilerplate.</p>
        <p><a href=\"/\" class=\"back\">← Back to Home</a></p>
    </div>
</body>
</html>\"#;
    
    Ok(Response::ok().html(html)?)
}}",
        imports,
        config.name,
        config.name,
        config.name,
        config.name
    );

    main_template
}

fn generate_cli_main(config: &ProjectConfig) -> String {
    format!(r#"use clap::{{Parser, Subcommand}};

#[derive(Parser)]
#[command(name = "{}")]
#[command(about = "A CLI application built with elif.rs")]
struct Cli {{
    #[command(subcommand)]
    command: Commands,
}}

#[derive(Subcommand)]
enum Commands {{
    /// Say hello to someone
    Hello {{
        /// Name of the person to greet
        name: Option<String>,
    }},
    /// Show version information
    Version,
}}

fn main() {{
    let cli = Cli::parse();

    match cli.command {{
        Commands::Hello {{ name }} => {{
            match name {{
                Some(n) => println!("Hello, {{}}! 👋", n),
                None => println!("Hello, World! 🌍"),
            }}
        }}
        Commands::Version => {{
            println!("{} v0.1.0", "{}");
            println!("Built with elif.rs - The Laravel of Rust! 🦀");
        }}
    }}
}}"#,
        config.name,
        config.name
    )
}

fn generate_minimal_main() -> String {
    r#"fn main() {
    println!("Hello from elif.rs - The Laravel of Rust! 🦀");
}"#.to_string()
}

async fn generate_enhanced_config_files(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    // Generate .env file
    let mut env_content = format!(r#"# Application Environment
APP_NAME={}
APP_ENV=development
APP_KEY=generate_with_elifrs_auth_generate_key

# Server
HOST=127.0.0.1
PORT=3000
"#, config.name);

    // Add database config if enabled
    if config.database_config.provider != "none" {
        let db_url = match config.database_config.provider.as_str() {
            "postgresql" => {
                let db_name = config.database_config.database_name.as_deref().unwrap_or(&format!("{}_development", config.name));
                format!("postgresql://user:password@localhost/{}", db_name)
            },
            "mysql" => {
                let db_name = config.database_config.database_name.as_deref().unwrap_or(&format!("{}_development", config.name));
                format!("mysql://user:password@localhost/{}", db_name)
            },
            "sqlite" => format!("./{}.db", config.name),
            _ => "".to_string(),
        };
        env_content.push_str(&format!("\n# Database\nDATABASE_URL={}\n", db_url));
    }

    // Add Redis config if caching is enabled
    if config.features.contains(&"Caching".to_string()) {
        env_content.push_str("\n# Cache\nREDIS_URL=redis://localhost:6379\n");
    }

    fs::write(app_dir.join(".env"), env_content).await?;

    // Generate elif.toml configuration
    let elif_toml = format!(r#"[app]
name = "{}"
version = "0.1.0"

[server]
host = "127.0.0.1"
port = 3000

[database]
url = "${{DATABASE_URL}}"
max_connections = 10

[cache]
driver = "memory"

[logging]
level = "info"
format = "json"
"#, config.name);

    fs::write(app_dir.join("elif.toml"), elif_toml).await?;

    // Generate .gitignore
    let gitignore = r#"# Rust
/target/
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo

# Environment
.env
.env.local
.env.production

# Logs
*.log

# Database
*.db
*.sqlite

# OS
.DS_Store
Thumbs.db

# Development
/tmp/
/temp/
"#;

    fs::write(app_dir.join(".gitignore"), gitignore).await?;

    Ok(())
}

async fn generate_database_files(_app_dir: &Path, _config: &ProjectConfig) -> Result<(), ElifError> {
    // TODO: Generate database migration files and models
    Ok(())
}

async fn generate_auth_files(_app_dir: &Path, _config: &ProjectConfig) -> Result<(), ElifError> {
    // TODO: Generate authentication middleware and handlers
    Ok(())
}

async fn generate_module_system_files(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    // Create modules/mod.rs
    let modules_mod = r#"pub mod app_module;
"#;
    fs::write(app_dir.join("src").join("modules").join("mod.rs"), modules_mod).await?;

    // Create app_module.rs
    let mut providers = vec![];
    let mut controllers = vec![];
    
    if config.auth_config.provider != "none" {
        providers.push("AuthService");
    }
    
    if config.database_config.provider != "none" {
        providers.push("DatabaseService");
    }
    
    let providers_str = if providers.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", providers.join(", "))
    };
    
    let controllers_str = if controllers.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", controllers.join(", "))
    };

    let app_module = format!(r#"use elif_core::container::module;
use elif_web::prelude::*;

#[module(
    controllers = {},
    providers = {},
    imports = [],
    exports = []
)]
pub struct AppModule;
"#, controllers_str, providers_str);

    fs::write(app_dir.join("src").join("modules").join("app_module.rs"), app_module).await?;

    Ok(())
}

async fn generate_feature_files(_app_dir: &Path, _config: &ProjectConfig, _feature: &str) -> Result<(), ElifError> {
    // TODO: Generate files for specific features
    Ok(())
}

async fn generate_docker_files(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    let dockerfile = r#"# Multi-stage build for optimized production image
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/app /usr/local/bin/app

EXPOSE 3000

CMD ["app"]
"#;

    let docker_compose = format!(r#"version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/{}
      - RUST_LOG=info
    depends_on:
      - db
    volumes:
      - .:/app
    working_dir: /app

  db:
    image: postgres:15
    environment:
      POSTGRES_DB: {}
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
"#, config.name, config.name);

    fs::write(app_dir.join("Dockerfile"), dockerfile).await?;
    fs::write(app_dir.join("docker-compose.yml"), docker_compose).await?;

    Ok(())
}

async fn generate_github_actions(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    fs::create_dir_all(app_dir.join(".github").join("workflows")).await?;

    let ci_workflow = format!(r#"name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: {}_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/cache
          ~/.cargo/registry
          target/
        key: ${{{{ runner.os }}}}-cargo-${{{{ hashFiles('**/Cargo.lock') }}}}
        
    - name: Run tests
      run: cargo test --verbose
      env:
        DATABASE_URL: postgresql://postgres:postgres@localhost:5432/{}_test
        
    - name: Check formatting
      run: cargo fmt -- --check
      
    - name: Run clippy
      run: cargo clippy -- -D warnings

  build:
    runs-on: ubuntu-latest
    needs: test
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      
    - name: Build
      run: cargo build --release --verbose
"#, config.name, config.name);

    fs::write(
        app_dir.join(".github").join("workflows").join("ci.yml"),
        ci_workflow
    ).await?;

    Ok(())
}

fn show_completion_message(config: &ProjectConfig, app_path: &str) {
    println!();
    println!("🎉 {} Your elif.rs {} is ready!", 
        console::style("Success!").green().bold(),
        match config.project_type.as_str() {
            "api" => "API server",
            "web" => "web application", 
            "microservice" => "microservice",
            "cli" => "CLI application",
            "minimal" => "application",
            _ => "application",
        }
    );
    println!();
    println!("📂 {}", console::style(&format!("Project created at: {}", app_path)).cyan());
    println!();
    println!("{}", console::style("🚀 Next steps:").bold());
    println!("   cd {}", config.name);
    
    if config.database_config.provider != "none" {
        println!("   {} Copy .env.example to .env and configure your database", console::style("1.").yellow());
        if config.database_config.include_migrations {
            println!("   {} Run database migrations: elifrs migrate up", console::style("2.").yellow());
        }
    }
    
    match config.project_type.as_str() {
        "cli" => {
            println!("   {} Build your CLI: cargo build --release", console::style("3.").yellow());
            println!("   {} Run your CLI: cargo run -- --help", console::style("4.").yellow());
        }
        _ => {
            println!("   {} Start development server: elifrs dev", console::style("3.").yellow());
            println!("   {} Visit http://127.0.0.1:3000", console::style("4.").yellow());
        }
    }
    
    println!();
    println!("{}", console::style("📚 Useful commands:").bold());
    if config.project_type != "cli" {
        println!("   elifrs add controller UserController  # Add a new controller");
        if config.modules_enabled {
            println!("   elifrs add module UserModule         # Add a new module");
        }
        println!("   elifrs make api User                 # Generate full CRUD API");
    }
    println!("   elifrs doctor                        # Check project health");
    println!("   elifrs --help                        # Show all commands");
    println!();
    println!("🦀 {} {}", 
        console::style("Happy coding with elif.rs -").dim(),
        console::style("The Laravel of Rust!").cyan().bold()
    );
}