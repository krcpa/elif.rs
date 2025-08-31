use elif_core::ElifError;
use std::path::Path;
use tokio::fs;

pub async fn create_app(name: &str, target_path: Option<&str>) -> Result<(), ElifError> {
    let app_path = match target_path {
        Some(path) => format!("{}/{}", path, name),
        None => format!("./{}", name),
    };
    
    let app_dir = Path::new(&app_path);
    
    if app_dir.exists() {
        return Err(ElifError::Validation { message: format!("Directory {} already exists", app_path) });
    }
    
    println!("📦 Creating new elif application: {}", name);
    
    // Create directory structure
    create_app_structure(&app_dir, name).await?;
    
    // Create configuration files
    super::new_templates::create_config_files(&app_dir, name).await?;
    
    // Create source files
    super::new_templates::create_source_files(&app_dir, name).await?;
    
    println!("✅ Application '{}' created successfully!", name);
    println!("📂 Location: {}", app_dir.display());
    println!("\n🚀 To get started:");
    println!("   cd {}", app_path);
    println!("   elif route add GET /hello hello_controller");
    println!("   cargo run");
    
    Ok(())
}

async fn create_app_structure(app_dir: &Path, _name: &str) -> Result<(), ElifError> {
    let dirs = [
        "src/controllers",
        "src/services",
        "src/modules",
        "src/middleware", 
        "src/models",
        "src/routes",
        "resources",
        "migrations",
        "tests",
        ".elif",
    ];
    
    for dir in &dirs {
        fs::create_dir_all(app_dir.join(dir)).await?;
    }
    
    Ok(())
}