use elif_core::ElifError;
use inquire::{Select, Text, Confirm, MultiSelect};
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;

static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
static SPARKLES: Emoji<'_, '_> = Emoji("✨ ", "");
static GEAR: Emoji<'_, '_> = Emoji("⚙️ ", "");
static DATABASE: Emoji<'_, '_> = Emoji("🗄️ ", "");
static SHIELD: Emoji<'_, '_> = Emoji("🛡️ ", "");
static PACKAGE: Emoji<'_, '_> = Emoji("📦 ", "");

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub project_type: String,
    pub template: String,
    pub database_config: DatabaseConfig,
    pub auth_config: AuthConfig,
    pub features: Vec<String>,
    pub modules_enabled: bool,
    pub directory: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub provider: String,
    pub database_name: Option<String>,
    pub include_migrations: bool,
    pub include_seeders: bool,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub provider: String,
    pub include_jwt: bool,
    pub include_rbac: bool,
    pub include_middleware: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            provider: "postgresql".to_string(),
            database_name: None,
            include_migrations: true,
            include_seeders: false,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            provider: "jwt".to_string(),
            include_jwt: true,
            include_rbac: false,
            include_middleware: true,
        }
    }
}

pub async fn run_interactive_wizard() -> Result<ProjectConfig, ElifError> {
    println!("{}", style("").bold());
    println!("{}", style("╔═══════════════════════════════════════════════════════════════╗").dim());
    println!("{}", style("║                                                               ║").dim());
    println!("{}", style("║  🦀 Welcome to elif.rs - The Laravel of Rust! 🦀             ║").cyan().bold());
    println!("{}", style("║                                                               ║").dim());
    println!("{}", style("║  Let's create your new web application with zero boilerplate ║").dim());
    println!("{}", style("║  and maximum productivity. Just answer a few questions...    ║").dim());
    println!("{}", style("║                                                               ║").dim());
    println!("{}", style("╚═══════════════════════════════════════════════════════════════╝").dim());
    println!();

    // Step 1: Project basics
    let name = get_project_name().await?;
    let directory = get_project_directory().await?;
    
    // Step 2: Project type selection
    let project_type = get_project_type().await?;
    let template = determine_template(&project_type);
    
    // Step 3: Database configuration
    let database_config = get_database_config(&project_type).await?;
    
    // Step 4: Authentication setup
    let auth_config = get_auth_config(&project_type).await?;
    
    // Step 5: Additional features
    let features = get_additional_features(&project_type).await?;
    
    // Step 6: Module system
    let modules_enabled = get_module_system_preference(&project_type).await?;
    
    // Step 7: Configuration summary
    let config = ProjectConfig {
        name: name.clone(),
        project_type: project_type.clone(),
        template,
        database_config,
        auth_config,
        features,
        modules_enabled,
        directory,
    };
    
    show_configuration_summary(&config).await?;
    
    let confirmed = Confirm::new("Ready to create your elif.rs application?")
        .with_default(true)
        .with_help_message("This will create the project structure and generate all necessary files")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get confirmation: {}", e)))?;
    
    if !confirmed {
        println!("\n{} Project creation cancelled.", style("❌").red());
        return Err(ElifError::new("Project creation cancelled by user"));
    }
    
    Ok(config)
}

async fn get_project_name() -> Result<String, ElifError> {
    println!("{}{}", ROCKET, style("Project Setup").bold().cyan());
    println!();
    
    let name = Text::new("What's your project name?")
        .with_help_message("This will be used for the directory name and package name (e.g., my-api)")
        .with_placeholder("my-awesome-app")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get project name: {}", e)))?;
    
    // Validate project name
    if name.trim().is_empty() {
        return Err(ElifError::new("Project name cannot be empty"));
    }
    
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ElifError::new("Project name can only contain letters, numbers, hyphens, and underscores"));
    }
    
    Ok(name.trim().to_string())
}

async fn get_project_directory() -> Result<Option<String>, ElifError> {
    let use_custom_dir = Confirm::new("Use a custom directory?")
        .with_default(false)
        .with_help_message("By default, the project will be created in the current directory")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get directory preference: {}", e)))?;
    
    if use_custom_dir {
        let dir = Text::new("Enter the target directory:")
            .with_placeholder("./projects")
            .prompt()
            .map_err(|e| ElifError::new(&format!("Failed to get directory: {}", e)))?;
        Ok(Some(dir))
    } else {
        Ok(None)
    }
}

async fn get_project_type() -> Result<String, ElifError> {
    println!();
    println!("{}{}", SPARKLES, style("Project Type").bold().cyan());
    println!();
    
    let project_types = vec![
        ("API Server", "A RESTful API server with OpenAPI documentation and database integration"),
        ("Full-Stack Web App", "Complete web application with frontend templates, static assets, and database"),
        ("Microservice", "Lightweight service optimized for cloud deployment and inter-service communication"),
        ("CLI Application", "Command-line application with rich terminal UI and configuration management"),
        ("Minimal Setup", "Bare-bones setup with just the essentials - perfect for learning or prototyping"),
    ];
    
    let selection = Select::new("What type of application are you building?", project_types.clone())
        .with_help_message("This determines the initial template, dependencies, and suggested architecture")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get project type: {}", e)))?;
    
    match selection.0 {
        "API Server" => Ok("api".to_string()),
        "Full-Stack Web App" => Ok("web".to_string()),
        "Microservice" => Ok("microservice".to_string()),
        "CLI Application" => Ok("cli".to_string()),
        "Minimal Setup" => Ok("minimal".to_string()),
        _ => Ok("api".to_string()),
    }
}

fn determine_template(project_type: &str) -> String {
    match project_type {
        "api" => "api".to_string(),
        "web" => "web".to_string(),
        "microservice" => "microservice".to_string(),
        "cli" => "cli".to_string(),
        "minimal" => "minimal".to_string(),
        _ => "api".to_string(),
    }
}

async fn get_database_config(project_type: &str) -> Result<DatabaseConfig, ElifError> {
    if project_type == "minimal" || project_type == "cli" {
        return Ok(DatabaseConfig {
            provider: "none".to_string(),
            database_name: None,
            include_migrations: false,
            include_seeders: false,
        });
    }
    
    println!();
    println!("{}{}", DATABASE, style("Database Configuration").bold().cyan());
    println!();
    
    let database_providers = vec![
        ("PostgreSQL", "Production-ready with excellent JSON support and full-text search"),
        ("MySQL/MariaDB", "Widely supported with good performance characteristics"),
        ("SQLite", "Embedded database perfect for development and small applications"),
        ("None", "Skip database setup - you can add it later"),
    ];
    
    let db_selection = Select::new("Which database would you like to use?", database_providers)
        .with_help_message("PostgreSQL is recommended for most applications")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get database selection: {}", e)))?;
    
    let provider = match db_selection.0 {
        "PostgreSQL" => "postgresql",
        "MySQL/MariaDB" => "mysql",
        "SQLite" => "sqlite",
        "None" => "none",
        _ => "postgresql",
    }.to_string();
    
    if provider == "none" {
        return Ok(DatabaseConfig {
            provider,
            database_name: None,
            include_migrations: false,
            include_seeders: false,
        });
    }
    
    let database_name = if provider != "sqlite" {
        Some(Text::new("Database name:")
            .with_help_message("This will be used in your connection string")
            .with_placeholder("myapp_development")
            .prompt()
            .map_err(|e| ElifError::new(&format!("Failed to get database name: {}", e)))?)
    } else {
        None
    };
    
    let include_migrations = Confirm::new("Include database migrations?")
        .with_default(true)
        .with_help_message("Migrations help you version and evolve your database schema")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get migrations preference: {}", e)))?;
    
    let include_seeders = if include_migrations {
        Confirm::new("Include database seeders?")
            .with_default(false)
            .with_help_message("Seeders populate your database with test or initial data")
            .prompt()
            .map_err(|e| ElifError::new(&format!("Failed to get seeders preference: {}", e)))?
    } else {
        false
    };
    
    Ok(DatabaseConfig {
        provider,
        database_name,
        include_migrations,
        include_seeders,
    })
}

async fn get_auth_config(project_type: &str) -> Result<AuthConfig, ElifError> {
    if project_type == "minimal" || project_type == "cli" {
        return Ok(AuthConfig {
            provider: "none".to_string(),
            include_jwt: false,
            include_rbac: false,
            include_middleware: false,
        });
    }
    
    println!();
    println!("{}{}", SHIELD, style("Authentication Setup").bold().cyan());
    println!();
    
    let auth_providers = vec![
        ("JWT (JSON Web Tokens)", "Stateless authentication perfect for APIs and SPAs"),
        ("Session-based", "Traditional server-side sessions with cookies"),
        ("OAuth2 Provider", "Ready for integration with Google, GitHub, etc."),
        ("None", "Skip authentication setup - add it later when needed"),
    ];
    
    let auth_selection = Select::new("How do you want to handle authentication?", auth_providers)
        .with_help_message("JWT is recommended for modern applications")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get auth selection: {}", e)))?;
    
    let provider = match auth_selection.0 {
        "JWT (JSON Web Tokens)" => "jwt",
        "Session-based" => "session",
        "OAuth2 Provider" => "oauth2",
        "None" => "none",
        _ => "jwt",
    }.to_string();
    
    if provider == "none" {
        return Ok(AuthConfig {
            provider,
            include_jwt: false,
            include_rbac: false,
            include_middleware: false,
        });
    }
    
    let include_jwt = provider == "jwt";
    
    let include_rbac = Confirm::new("Include Role-Based Access Control (RBAC)?")
        .with_default(false)
        .with_help_message("RBAC helps manage user permissions and roles")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get RBAC preference: {}", e)))?;
    
    let include_middleware = Confirm::new("Include authentication middleware?")
        .with_default(true)
        .with_help_message("Middleware automatically protects your routes")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get middleware preference: {}", e)))?;
    
    Ok(AuthConfig {
        provider,
        include_jwt,
        include_rbac,
        include_middleware,
    })
}

async fn get_additional_features(project_type: &str) -> Result<Vec<String>, ElifError> {
    if project_type == "minimal" {
        return Ok(vec![]);
    }
    
    println!();
    println!("{}{}", PACKAGE, style("Additional Features").bold().cyan());
    println!();
    
    let mut available_features = vec![];
    
    if project_type != "cli" {
        available_features.extend(vec![
            ("Caching", "Redis and in-memory caching with automatic invalidation"),
            ("File Storage", "Local and cloud storage with image processing"),
            ("WebSockets", "Real-time communication with channel management"),
            ("Email Service", "Template-based emails with multiple providers"),
            ("Queue System", "Background job processing with Redis/database"),
        ]);
    }
    
    available_features.extend(vec![
        ("OpenAPI Documentation", "Auto-generated API docs with Swagger UI"),
        ("Testing Framework", "Comprehensive test utilities and factories"),
        ("Docker Setup", "Production-ready Dockerfile and docker-compose"),
        ("GitHub Actions", "CI/CD pipeline with testing and deployment"),
    ]);
    
    if available_features.is_empty() {
        return Ok(vec![]);
    }
    
    let features = MultiSelect::new("Select additional features:", available_features)
        .with_help_message("You can always add these later with 'elifrs add' commands")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get feature selection: {}", e)))?;
    
    Ok(features.into_iter().map(|(name, _)| name.to_string()).collect())
}

async fn get_module_system_preference(project_type: &str) -> Result<bool, ElifError> {
    if project_type == "minimal" {
        return Ok(false);
    }
    
    println!();
    println!("{}{}", GEAR, style("Module System").bold().cyan());
    println!();
    
    println!("{}",
        style("The elif.rs module system provides Laravel-style dependency injection")
            .dim()
    );
    println!("{}",
        style("and automatic service registration with zero boilerplate.")
            .dim()
    );
    println!();
    
    let enable_modules = Confirm::new("Enable the module system?")
        .with_default(true)
        .with_help_message("Recommended for all but the simplest applications")
        .prompt()
        .map_err(|e| ElifError::new(&format!("Failed to get module preference: {}", e)))?;
    
    Ok(enable_modules)
}

async fn show_configuration_summary(config: &ProjectConfig) -> Result<(), ElifError> {
    println!();
    println!("{}{}", SPARKLES, style("Configuration Summary").bold().cyan());
    println!();
    println!("{}", style("╔═══════════════════════════════════════════════════════════════╗").dim());
    println!("{}", style("║                        Project Overview                       ║").dim());
    println!("{}", style("╠═══════════════════════════════════════════════════════════════╣").dim());
    
    println!("{}{}",
        style("║  📦 Project:        ").dim(),
        style(&format!("{:<38} ║", config.name)).cyan()
    );
    println!("{}{}",
        style("║  🏗️  Type:           ").dim(),
        style(&format!("{:<38} ║", format_project_type(&config.project_type))).cyan()
    );
    
    if let Some(ref dir) = config.directory {
        println!("{}{}",
            style("║  📁 Directory:      ").dim(),
            style(&format!("{:<38} ║", dir)).cyan()
        );
    }
    
    println!("{}{}",
        style("║  🗄️  Database:       ").dim(),
        style(&format!("{:<38} ║", format_database(&config.database_config))).cyan()
    );
    println!("{}{}",
        style("║  🛡️  Authentication: ").dim(),
        style(&format!("{:<38} ║", format_auth(&config.auth_config))).cyan()
    );
    println!("{}{}",
        style("║  🧩 Module System:  ").dim(),
        style(&format!("{:<38} ║", if config.modules_enabled { "Enabled" } else { "Disabled" })).cyan()
    );
    
    if !config.features.is_empty() {
        println!("{}", style("║                                                               ║").dim());
        println!("{}", style("║  📋 Additional Features:                                      ║").dim());
        for feature in &config.features {
            println!("{}{}",
                style("║    • ").dim(),
                style(&format!("{:<50} ║", feature)).green()
            );
        }
    }
    
    println!("{}", style("╚═══════════════════════════════════════════════════════════════╝").dim());
    println!();
    
    Ok(())
}

fn format_project_type(project_type: &str) -> String {
    match project_type {
        "api" => "API Server".to_string(),
        "web" => "Full-Stack Web App".to_string(),
        "microservice" => "Microservice".to_string(),
        "cli" => "CLI Application".to_string(),
        "minimal" => "Minimal Setup".to_string(),
        _ => project_type.to_string(),
    }
}

fn format_database(config: &DatabaseConfig) -> String {
    match config.provider.as_str() {
        "none" => "None".to_string(),
        "postgresql" => "PostgreSQL".to_string(),
        "mysql" => "MySQL/MariaDB".to_string(),
        "sqlite" => "SQLite".to_string(),
        _ => config.provider.clone(),
    }
}

fn format_auth(config: &AuthConfig) -> String {
    match config.provider.as_str() {
        "none" => "None".to_string(),
        "jwt" => "JWT".to_string(),
        "session" => "Session-based".to_string(),
        "oauth2" => "OAuth2 Provider".to_string(),
        _ => config.provider.clone(),
    }
}

pub fn show_progress(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}