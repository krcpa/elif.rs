use elif_core::ElifError;
use super::simple_interactive::ProjectConfig;
use crate::generators::TemplateEngine;
use git2::Repository;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub async fn generate_project_from_template(config: &ProjectConfig) -> Result<(), ElifError> {
    let app_path = match &config.directory {
        Some(dir) => format!("{}/{}", dir, config.name),
        None => format!("./{}", config.name),
    };
    
    let app_dir = Path::new(&app_path);
    
    if app_dir.exists() {
        return Err(ElifError::validation(&format!("Directory '{}' already exists", app_path)));
    }
    
    println!("🚀 Creating elif.rs application '{}'", config.name);
    println!("📁 Path: {}", app_dir.display());
    println!("📋 Template: {}", config.project_type);
    if config.modules_enabled {
        println!("🧩 Including module system setup");
    }
    
    // Create directory structure
    create_directory_structure(app_dir, config).await?;
    
    // Initialize template engine
    let template_engine = TemplateEngine::new()?;
    
    // Generate files using templates
    generate_cargo_toml_from_template(app_dir, config, &template_engine).await?;
    generate_main_from_template(app_dir, config, &template_engine).await?;
    generate_config_files_from_template(app_dir, config).await?;
    
    // Generate controllers and services for non-minimal projects
    if config.project_type != "Minimal Setup" {
        generate_controllers_and_services_from_template(app_dir, config, &template_engine).await?;
    }
    
    if config.modules_enabled {
        generate_module_system_from_template(app_dir, config, &template_engine).await?;
    }
    
    // Initialize git repository
    initialize_git_repository(app_dir)?;
    
    println!("\n✅ Successfully created elif.rs application '{}'", config.name);
    println!("\n📖 Next steps:");
    println!("   cd {}", config.name);
    if config.modules_enabled {
        println!("   elifrs add module AppModule --controllers=AppController");
    }
    println!("   elifrs dev");
    println!("\n🎯 Happy coding with elif.rs - The Laravel of Rust! 🦀");
    
    Ok(())
}

async fn create_directory_structure(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    let mut dirs = vec![
        "src",
        "tests",
    ];
    
    // Add directories based on project type
    match config.project_type.as_str() {
        "API Server" | "api" => {
            dirs.extend(vec![
                "src/controllers",
                "src/middleware",
                "src/models",
                "src/services",
                "docs",
            ]);
        }
        "Full-Stack Web App" | "web" => {
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
        "Minimal Setup" | "minimal" => {
            // Just src/ and tests/
        }
        _ => {}
    }
    
    // Add modules directory if enabled
    if config.modules_enabled {
        dirs.push("src/modules");
    }
    
    // Add database directories if database is enabled
    if config.database_enabled {
        dirs.push("migrations");
        if config.include_seeders {
            dirs.push("database/seeders");
        }
    }
    
    // Create all directories
    for dir in dirs {
        fs::create_dir_all(app_dir.join(dir)).await?;
    }
    
    Ok(())
}

async fn generate_cargo_toml_from_template(
    app_dir: &Path,
    config: &ProjectConfig,
    template_engine: &TemplateEngine,
) -> Result<(), ElifError> {
    let mut template_data = HashMap::new();
    template_data.insert("project_name".to_string(), serde_json::Value::String(config.name.clone()));
    template_data.insert("project_type".to_string(), serde_json::Value::String(format_project_type(&config.project_type)));
    template_data.insert("http_enabled".to_string(), serde_json::Value::Bool(config.project_type != "minimal"));
    template_data.insert("modules_enabled".to_string(), serde_json::Value::Bool(config.modules_enabled));
    template_data.insert("database_enabled".to_string(), serde_json::Value::Bool(config.database_enabled));
    template_data.insert("auth_enabled".to_string(), serde_json::Value::Bool(config.auth_enabled));
    template_data.insert("jwt_enabled".to_string(), serde_json::Value::Bool(config.jwt_enabled));
    template_data.insert("testing_enabled".to_string(), serde_json::Value::Bool(config.features.contains(&"Testing Framework".to_string())));
    
    if config.database_enabled {
        template_data.insert("database_type".to_string(), serde_json::Value::String(config.database_type.clone()));
    }
    
    if !config.features.is_empty() {
        template_data.insert("features".to_string(), serde_json::Value::Array(
            config.features.iter().map(|f| serde_json::Value::String(f.clone())).collect()
        ));
    }
    
    let cargo_toml_content = template_engine.render("cargo_toml.stub", &template_data)?;
    fs::write(app_dir.join("Cargo.toml"), cargo_toml_content).await?;
    
    Ok(())
}

async fn generate_main_from_template(
    app_dir: &Path,
    config: &ProjectConfig,
    template_engine: &TemplateEngine,
) -> Result<(), ElifError> {
    let mut template_data = HashMap::new();
    template_data.insert("project_name".to_string(), serde_json::Value::String(config.name.clone()));
    template_data.insert("modules_enabled".to_string(), serde_json::Value::Bool(config.modules_enabled));
    template_data.insert("database_enabled".to_string(), serde_json::Value::Bool(config.database_enabled));
    template_data.insert("auth_enabled".to_string(), serde_json::Value::Bool(config.auth_enabled));
    
    let template_name = match config.project_type.as_str() {
        "API Server" | "api" => "main_api.stub",
        "Full-Stack Web App" | "web" => "main_web.stub", // We can create this later
        "Minimal Setup" | "minimal" => "main_minimal.stub", // We can create this later
        _ => "main_api.stub",
    };
    
    let main_content = template_engine.render(template_name, &template_data)?;
    fs::write(app_dir.join("src").join("main.rs"), main_content).await?;
    
    Ok(())
}

async fn generate_config_files_from_template(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
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
    if config.database_enabled {
        let db_url = match config.database_type.as_str() {
            "postgresql" => {
                let default_name = format!("{}_development", config.name);
                let db_name = config.database_name.as_deref().unwrap_or(&default_name);
                format!("postgresql://user:password@localhost/{}", db_name)
            },
            "mysql" => {
                let default_name = format!("{}_development", config.name);
                let db_name = config.database_name.as_deref().unwrap_or(&default_name);
                format!("mysql://user:password@localhost/{}", db_name)
            },
            "sqlite" => format!("./{}.db", config.name),
            _ => "".to_string(),
        };
        env_content.push_str(&format!("\n# Database\nDATABASE_URL={}\n", db_url));
    }

    fs::write(app_dir.join(".env"), env_content).await?;

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

async fn generate_module_system_from_template(
    app_dir: &Path,
    _config: &ProjectConfig,
    _template_engine: &TemplateEngine,
) -> Result<(), ElifError> {
    // Create modules/mod.rs
    let modules_mod = r#"pub mod app_module;
"#;
    fs::write(app_dir.join("src").join("modules").join("mod.rs"), modules_mod).await?;

    // Create app_module.rs
    let app_module = r#"use elif_core::container::module;
use elif_http_derive::module;

#[module(
    controllers = [],
    providers = [],
    imports = [],
    exports = []
)]
pub struct AppModule;
"#;
    fs::write(app_dir.join("src").join("modules").join("app_module.rs"), app_module).await?;

    Ok(())
}

fn format_project_type(project_type: &str) -> String {
    match project_type {
        "api" => "RESTful API server",
        "web" => "full-stack web application", 
        "microservice" => "cloud-native microservice",
        "cli" => "command-line application",
        "minimal" => "Rust application",
        _ => "Rust application",
    }.to_string()
}

fn initialize_git_repository(app_dir: &Path) -> Result<(), ElifError> {
    match Repository::init(app_dir) {
        Ok(repo) => {
            println!("🔧 Initialized git repository");
            
            // Create an initial commit with all generated files
            let signature = git2::Signature::now("elifrs", "elifrs@localhost")
                .map_err(|e| ElifError::system_error(format!("Failed to create git signature: {}", e)))?;
            
            let mut index = repo.index()
                .map_err(|e| ElifError::system_error(format!("Failed to get git index: {}", e)))?;
            
            // Add all files to the index
            index.add_all(&["*"], git2::IndexAddOption::DEFAULT, None)
                .map_err(|e| ElifError::system_error(format!("Failed to add files to git index: {}", e)))?;
            
            index.write()
                .map_err(|e| ElifError::system_error(format!("Failed to write git index: {}", e)))?;
            
            // Create tree from index
            let tree_id = index.write_tree()
                .map_err(|e| ElifError::system_error(format!("Failed to create git tree: {}", e)))?;
            
            let tree = repo.find_tree(tree_id)
                .map_err(|e| ElifError::system_error(format!("Failed to find git tree: {}", e)))?;
            
            // Create initial commit
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit - generated by elifrs",
                &tree,
                &[],
            ).map_err(|e| ElifError::system_error(format!("Failed to create initial commit: {}", e)))?;
            
            println!("📝 Created initial commit");
            Ok(())
        }
        Err(e) => {
            // Don't fail the entire project creation if git init fails
            eprintln!("⚠️  Warning: Failed to initialize git repository: {}", e);
            eprintln!("   You can initialize git manually with: git init");
            Ok(())
        }
    }
}

async fn generate_controllers_and_services_from_template(
    app_dir: &Path,
    _config: &ProjectConfig,
    template_engine: &TemplateEngine,
) -> Result<(), ElifError> {
    let mut template_data = HashMap::new();
    template_data.insert("project_name".to_string(), serde_json::Value::String(_config.name.clone()));
    
    // Generate controllers/mod.rs
    let controllers_mod = template_engine.render("controllers_mod.stub", &template_data)?;
    fs::write(app_dir.join("src/controllers/mod.rs"), controllers_mod).await?;
    
    // Generate controllers/user_controller.rs
    let user_controller = template_engine.render("user_controller.stub", &template_data)?;
    fs::write(app_dir.join("src/controllers/user_controller.rs"), user_controller).await?;
    
    // Generate services/mod.rs
    let services_mod = template_engine.render("services_mod.stub", &template_data)?;
    fs::write(app_dir.join("src/services/mod.rs"), services_mod).await?;
    
    // Generate services/user_service.rs
    let user_service = template_engine.render("user_service.stub", &template_data)?;
    fs::write(app_dir.join("src/services/user_service.rs"), user_service).await?;
    
    Ok(())
}
