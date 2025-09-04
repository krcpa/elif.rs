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
    println!("🧩 Including module system setup");
    
    // Create directory structure
    create_directory_structure(app_dir, config).await?;
    
    // Initialize template engine
    let template_engine = TemplateEngine::new()?;
    
    // Generate files using templates
    generate_cargo_toml_from_template(app_dir, config, &template_engine).await?;
    generate_main_from_template(app_dir, config, &template_engine).await?;
    generate_config_files_from_template(app_dir, config).await?;
    
    // Always generate NestJS-style services
    generate_controllers_and_services_from_template(app_dir, config, &template_engine).await?;
    
    // Initialize git repository
    initialize_git_repository(app_dir)?;
    
    println!("\n✅ Successfully created elif.rs application '{}'", config.name);
    println!("\n📖 Next steps:");
    println!("   cd {}", config.name);
    println!("   elifrs dev");
    println!("\n🎯 Happy coding with elif.rs - The Laravel of Rust! 🦀");
    
    Ok(())
}

async fn create_directory_structure(app_dir: &Path, config: &ProjectConfig) -> Result<(), ElifError> {
    let mut dirs = vec![
        "src",
        "tests",
    ];
    
    // NestJS-style modular directory structure
    dirs.extend(vec![
        "src/modules",
        "src/modules/app",
        "src/modules/users",
        "src/modules/users/dto",
    ]);

    // Add extra directories based on project type  
    match config.project_type.as_str() {
        "Full-Stack Web App" | "web" => {
            dirs.extend(vec![
                "src/views",
                "public",
                "public/css",
                "public/js",
                "public/images",
                "docs",
            ]);
        }
        _ => {
            // All projects get docs by default
            dirs.push("docs");
        }
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
    template_data.insert("modules_enabled".to_string(), serde_json::Value::Bool(true));
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
    template_data.insert("modules_enabled".to_string(), serde_json::Value::Bool(true));
    template_data.insert("database_enabled".to_string(), serde_json::Value::Bool(config.database_enabled));
    template_data.insert("auth_enabled".to_string(), serde_json::Value::Bool(config.auth_enabled));
    template_data.insert("http_enabled".to_string(), serde_json::Value::Bool(config.project_type != "minimal"));
    template_data.insert("openapi_enabled".to_string(), serde_json::Value::Bool(config.features.contains(&"OpenAPI Documentation".to_string())));
    
    let template_name = match config.project_type.as_str() {
        "Minimal Setup" | "minimal" => "main_minimal.stub",
        _ => "main_modular.stub" // Use new modular structure
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
    config: &ProjectConfig,
    template_engine: &TemplateEngine,
) -> Result<(), ElifError> {
    let mut template_data = HashMap::new();
    template_data.insert("project_name".to_string(), serde_json::Value::String(config.name.clone()));
    
    // Generate main modules/mod.rs
    let modules_mod_content = "pub mod app;\npub mod users;";
    fs::write(app_dir.join("src/modules/mod.rs"), modules_mod_content).await?;
    
    // Generate app module files
    generate_app_module_files(app_dir, template_engine, &template_data).await?;
    
    // Generate users module files if http is enabled
    if config.project_type != "minimal" {
        generate_users_module_files(app_dir, template_engine, &template_data).await?;
    }
    
    Ok(())
}

/// Generic module file generation configuration
struct ModuleConfig {
    /// Module name (e.g., "app", "users")
    name: String,
    /// mod.rs content
    mod_content: String,
    /// Template data for this specific module
    template_data: HashMap<String, serde_json::Value>,
    /// List of (template_path, output_filename) pairs
    template_files: Vec<(String, String)>,
}

/// Generic helper function to generate module files from templates
async fn generate_module_files(
    app_dir: &Path,
    template_engine: &TemplateEngine,
    config: ModuleConfig,
) -> Result<(), ElifError> {
    let module_path = app_dir.join("src/modules").join(&config.name);
    
    // Generate mod.rs
    fs::write(module_path.join("mod.rs"), config.mod_content).await?;
    
    // Generate all template files
    for (template_path, output_filename) in config.template_files {
        let rendered_content = template_engine.render(&template_path, &config.template_data)?;
        let output_path = module_path.join(&output_filename);
        
        // Create parent directories if they don't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(output_path, rendered_content).await?;
    }
    
    Ok(())
}

async fn generate_app_module_files(
    app_dir: &Path,
    template_engine: &TemplateEngine,
    template_data: &HashMap<String, serde_json::Value>,
) -> Result<(), ElifError> {
    let config = ModuleConfig {
        name: "app".to_string(),
        mod_content: "pub mod app_module;\npub mod app_controller;\npub mod app_service;\n\npub use app_module::AppModule;".to_string(),
        template_data: template_data.clone(),
        template_files: vec![
            ("modules/app_module.stub".to_string(), "app_module.rs".to_string()),
            ("modules/app_controller.stub".to_string(), "app_controller.rs".to_string()),
            ("modules/app_service.stub".to_string(), "app_service.rs".to_string()),
        ],
    };
    
    generate_module_files(app_dir, template_engine, config).await
}

async fn generate_users_module_files(
    app_dir: &Path,
    template_engine: &TemplateEngine,
    template_data: &HashMap<String, serde_json::Value>,
) -> Result<(), ElifError> {
    // Create template data for users module
    let mut users_template_data = template_data.clone();
    users_template_data.insert("feature_name".to_string(), serde_json::Value::String("users".to_string()));
    users_template_data.insert("feature_name_pascal".to_string(), serde_json::Value::String("Users".to_string()));
    users_template_data.insert("feature_name_plural".to_string(), serde_json::Value::String("users".to_string()));
    
    let config = ModuleConfig {
        name: "users".to_string(),
        mod_content: "pub mod users_module;\npub mod users_controller;\npub mod users_service;\npub mod dto;\n\npub use users_module::UsersModule;".to_string(),
        template_data: users_template_data,
        template_files: vec![
            ("modules/feature_module.stub".to_string(), "users_module.rs".to_string()),
            ("modules/module_controller.stub".to_string(), "users_controller.rs".to_string()),
            ("modules/module_service.stub".to_string(), "users_service.rs".to_string()),
            ("modules/dto/mod_dto.stub".to_string(), "dto/mod.rs".to_string()),
            ("modules/dto/create_dto.stub".to_string(), "dto/create_users.rs".to_string()),
            ("modules/dto/update_dto.stub".to_string(), "dto/update_users.rs".to_string()),
        ],
    };
    
    generate_module_files(app_dir, template_engine, config).await
}
