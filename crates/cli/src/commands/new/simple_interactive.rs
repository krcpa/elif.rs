use elif_core::ElifError;
use inquire::{Select, Text, Confirm};
use console::style;

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub project_type: String,
    pub modules_enabled: bool,
    pub database_enabled: bool,
    pub database_type: String,
    pub database_name: Option<String>,
    pub include_seeders: bool,
    pub auth_enabled: bool,
    pub jwt_enabled: bool,
    pub features: Vec<String>,
    pub directory: Option<String>,
}

pub async fn run_simple_wizard() -> Result<(), ElifError> {
    println!();
    println!("{}", style("🦀 Welcome to elif.rs - The Laravel of Rust!").cyan().bold());
    println!("{}", style("Let's create your new web application...").dim());
    println!();

    // Get project name
    let name = Text::new("What's your project name?")
        .with_placeholder("my-awesome-app")
        .prompt()
        .map_err(|e| ElifError::validation(&format!("Failed to get project name: {}", e)))?;

    // Get project type
    let project_types = vec![
        "API Server",
        "Full-Stack Web App", 
        "Minimal Setup",
    ];
    
    let project_type = Select::new("What type of application?", project_types)
        .prompt()
        .map_err(|e| ElifError::validation(&format!("Failed to get project type: {}", e)))?;

    // Get database preference
    let use_database = Confirm::new("Include database support?")
        .with_default(true)
        .prompt()
        .map_err(|e| ElifError::validation(&format!("Failed to get database preference: {}", e)))?;

    // Get modules preference
    let use_modules = Confirm::new("Enable module system?")
        .with_default(true)
        .prompt()
        .map_err(|e| ElifError::validation(&format!("Failed to get module preference: {}", e)))?;

    // Show summary
    println!();
    println!("{}", style("📋 Configuration Summary:").bold());
    println!("  📦 Project: {}", style(&name).cyan());
    println!("  🏗️  Type: {}", style(project_type).cyan());
    println!("  🗄️  Database: {}", style(if use_database { "Yes" } else { "No" }).cyan());
    println!("  🧩 Modules: {}", style(if use_modules { "Yes" } else { "No" }).cyan());
    println!();

    let confirmed = Confirm::new("Create project?")
        .with_default(true)
        .prompt()
        .map_err(|e| ElifError::validation(&format!("Failed to get confirmation: {}", e)))?;

    if !confirmed {
        return Err(ElifError::validation("Project creation cancelled"));
    }

    // Convert to template format and call existing create function
    let _template = match project_type {
        "API Server" => "api",
        "Full-Stack Web App" => "web", 
        "Minimal Setup" => "minimal",
        _ => "api",
    };

    println!();
    println!("{} Creating your elif.rs application...", style("🚀").green());
    
    // Create config for template generation
    let config = ProjectConfig {
        name: name.clone(),
        project_type: project_type.to_string(),
        modules_enabled: use_modules,
        database_enabled: use_database,
        database_type: if use_database { "postgresql".to_string() } else { "none".to_string() },
        database_name: if use_database { Some(format!("{}_development", name)) } else { None },
        include_seeders: false,
        auth_enabled: false,
        jwt_enabled: false,
        features: vec![],
        directory: None,
    };

    // Call the template generator
    super::template_generator::generate_project_from_template(&config).await?;

    println!();
    println!("{} {}", 
        style("🎉 Success!").green().bold(),
        style("Your elif.rs application is ready!")
    );
    println!();
    println!("{}", style("Next steps:").bold());
    println!("  cd {}", name);
    println!("  elifrs dev");
    println!();

    Ok(())
}