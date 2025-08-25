use elif_core::ElifError;
use std::path::Path;
use std::process::Command;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct ProjectInfo {
    name: String,
    version: String,
    edition: String,
    elif_version: String,
    dependencies: HashMap<String, String>,
    modules: Vec<ModuleInfo>,
    features: Vec<String>,
    project_stats: ProjectStats,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModuleInfo {
    name: String,
    path: String,
    dependencies: Vec<String>,
    providers: Vec<String>,
    controllers: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProjectStats {
    lines_of_code: u32,
    files_count: u32,
    test_coverage: Option<f32>,
    build_time: Option<String>,
}

pub async fn run(detailed: bool, modules: bool) -> Result<(), ElifError> {
    println!("ℹ️ elif.rs Framework Information");
    
    // Display basic framework info
    display_framework_info().await?;
    
    if detailed || modules {
        // Check if we're in a project directory
        if Path::new("Cargo.toml").exists() {
            let project_info = analyze_project().await?;
            
            if detailed {
                display_detailed_info(&project_info).await?;
            }
            
            if modules {
                display_module_info(&project_info).await?;
            }
        } else {
            println!("⚠️  Not in an elif.rs project directory - showing framework info only");
        }
    }
    
    Ok(())
}

async fn display_framework_info() -> Result<(), ElifError> {
    println!("   🦀 Version: 0.9.0");
    println!("   📖 Philosophy: The Laravel of Rust - LLM-friendly web framework");
    println!("   🏠 Homepage: https://github.com/krcpa/elif.rs");
    println!("   📚 Documentation: https://docs.rs/elifrs");
    
    // Get system info
    let rust_version = get_rust_version().await?;
    println!("   🔧 Rust Version: {}", rust_version);
    
    // Show core features
    println!("   ✨ Core Features:");
    println!("      • Module System with Dependency Injection");
    println!("      • Complete ORM with Migrations & Seeding");
    println!("      • Built-in Authentication & Authorization");
    println!("      • OpenAPI/Swagger Integration");
    println!("      • Hot Reload Development Mode");
    println!("      • Production-Ready CLI Tools");
    println!("      • Docker & Kubernetes Support");
    
    Ok(())
}

async fn get_rust_version() -> Result<String, ElifError> {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| ElifError::Io(e))?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("Unknown".to_string())
    }
}

async fn analyze_project() -> Result<ProjectInfo, ElifError> {
    println!("🔍 Analyzing project...");
    
    let mut project_info = ProjectInfo {
        name: "unknown".to_string(),
        version: "0.1.0".to_string(),
        edition: "2021".to_string(),
        elif_version: "0.9.0".to_string(),
        dependencies: HashMap::new(),
        modules: Vec::new(),
        features: Vec::new(),
        project_stats: ProjectStats {
            lines_of_code: 0,
            files_count: 0,
            test_coverage: None,
            build_time: None,
        },
    };

    // Parse Cargo.toml
    if let Ok(cargo_content) = tokio::fs::read_to_string("Cargo.toml").await {
        if let Ok(cargo_toml) = cargo_content.parse::<toml::Value>() {
            // Extract basic project info
            if let Some(package) = cargo_toml.get("package") {
                if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
                    project_info.name = name.to_string();
                }
                if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
                    project_info.version = version.to_string();
                }
                if let Some(edition) = package.get("edition").and_then(|e| e.as_str()) {
                    project_info.edition = edition.to_string();
                }
            }
            
            // Extract dependencies
            if let Some(deps) = cargo_toml.get("dependencies").and_then(|d| d.as_table()) {
                for (name, value) in deps {
                    let version = match value {
                        toml::Value::String(v) => v.clone(),
                        toml::Value::Table(t) => {
                            if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                                v.to_string()
                            } else if let Some(path) = t.get("path").and_then(|p| p.as_str()) {
                                format!("path: {}", path)
                            } else {
                                "unknown".to_string()
                            }
                        },
                        _ => "unknown".to_string(),
                    };
                    project_info.dependencies.insert(name.clone(), version);
                }
            }
        }
    }

    // Analyze modules
    project_info.modules = discover_modules().await?;
    
    // Analyze features
    project_info.features = discover_features().await?;
    
    // Get project statistics
    project_info.project_stats = calculate_project_stats().await?;
    
    Ok(project_info)
}

async fn discover_modules() -> Result<Vec<ModuleInfo>, ElifError> {
    let mut modules = Vec::new();
    
    if !Path::new("src").exists() {
        return Ok(modules);
    }
    
    // Look for module files in src/
    let mut entries = tokio::fs::read_dir("src").await.map_err(|e| ElifError::Io(e))?;
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| ElifError::Io(e))? {
        let path = entry.path();
        
        if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("rs")) {
            if let Some(file_name) = path.file_stem().and_then(|n| n.to_str()) {
                if file_name != "main" && file_name != "lib" {
                    // Try to analyze this as a potential module
                    let module_info = analyze_module_file(&path).await?;
                    if !module_info.providers.is_empty() || !module_info.controllers.is_empty() {
                        modules.push(module_info);
                    }
                }
            }
        }
    }
    
    // Also check for modules/ directory
    if Path::new("src/modules").exists() {
        let modules_in_dir = discover_modules_in_directory("src/modules").await?;
        modules.extend(modules_in_dir);
    }
    
    Ok(modules)
}

async fn discover_modules_in_directory(dir_path: &str) -> Result<Vec<ModuleInfo>, ElifError> {
    let mut modules = Vec::new();
    
    let mut entries = tokio::fs::read_dir(dir_path).await.map_err(|e| ElifError::Io(e))?;
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| ElifError::Io(e))? {
        let path = entry.path();
        
        if path.is_dir() {
            // Check for mod.rs or a main module file
            let mod_file = path.join("mod.rs");
            let main_file = path.join(format!("{}.rs", path.file_name().unwrap().to_string_lossy()));
            
            let module_file = if mod_file.exists() {
                mod_file
            } else if main_file.exists() {
                main_file
            } else {
                continue;
            };
            
            let module_info = analyze_module_file(&module_file).await?;
            if !module_info.providers.is_empty() || !module_info.controllers.is_empty() {
                modules.push(module_info);
            }
        } else if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("rs")) {
            let module_info = analyze_module_file(&path).await?;
            if !module_info.providers.is_empty() || !module_info.controllers.is_empty() {
                modules.push(module_info);
            }
        }
    }
    
    Ok(modules)
}

async fn analyze_module_file(file_path: &Path) -> Result<ModuleInfo, ElifError> {
    let content = tokio::fs::read_to_string(file_path).await.map_err(|e| ElifError::Io(e))?;
    
    let name = file_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let mut module_info = ModuleInfo {
        name,
        path: file_path.to_string_lossy().to_string(),
        dependencies: Vec::new(),
        providers: Vec::new(),
        controllers: Vec::new(),
    };
    
    // Simple pattern matching for elif.rs patterns
    for line in content.lines() {
        let line = line.trim();
        
        // Look for #[module(...)] attributes
        if line.starts_with("#[module(") || line.contains("#[module(") {
            // This is likely a module definition
            // Extract dependencies from the attribute if possible
            if let Some(start) = line.find("imports = [") {
                if let Some(end) = line[start..].find(']') {
                    let imports_str = &line[start + 11..start + end];
                    for import in imports_str.split(',') {
                        let import = import.trim().trim_matches('"').trim();
                        if !import.is_empty() {
                            module_info.dependencies.push(import.to_string());
                        }
                    }
                }
            }
        }
        
        // Look for providers
        if line.contains("Provider") && (line.contains("pub struct") || line.contains("struct")) {
            if let Some(struct_name) = extract_struct_name(line) {
                if struct_name.ends_with("Provider") {
                    module_info.providers.push(struct_name);
                }
            }
        }
        
        // Look for controllers
        if line.contains("Controller") && (line.contains("pub struct") || line.contains("struct")) {
            if let Some(struct_name) = extract_struct_name(line) {
                if struct_name.ends_with("Controller") {
                    module_info.controllers.push(struct_name);
                }
            }
        }
    }
    
    Ok(module_info)
}

fn extract_struct_name(line: &str) -> Option<String> {
    // Simple pattern to extract struct name from "pub struct Name" or "struct Name"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if let Some(pos) = parts.iter().position(|&x| x == "struct") {
        if let Some(name) = parts.get(pos + 1) {
            // Remove any generic parameters or braces
            let name = name.split('<').next().unwrap_or(name);
            let name = name.split('{').next().unwrap_or(name);
            return Some(name.to_string());
        }
    }
    None
}

async fn discover_features() -> Result<Vec<String>, ElifError> {
    let mut features = Vec::new();
    
    // Check for common elif.rs features by looking at dependencies and file structure
    if Path::new("migrations").exists() {
        features.push("Database Migrations".to_string());
    }
    
    if Path::new("seeds").exists() {
        features.push("Database Seeding".to_string());
    }
    
    if Path::new("tests").exists() {
        features.push("Testing Infrastructure".to_string());
    }
    
    // Check Cargo.toml for elif.rs related dependencies
    if let Ok(cargo_content) = tokio::fs::read_to_string("Cargo.toml").await {
        if cargo_content.contains("elif-auth") {
            features.push("Authentication".to_string());
        }
        if cargo_content.contains("elif-validation") {
            features.push("Validation".to_string());
        }
        if cargo_content.contains("elif-cache") {
            features.push("Caching".to_string());
        }
        if cargo_content.contains("elif-queue") {
            features.push("Job Queues".to_string());
        }
        if cargo_content.contains("elif-storage") {
            features.push("File Storage".to_string());
        }
        if cargo_content.contains("elif-email") {
            features.push("Email Services".to_string());
        }
        if cargo_content.contains("elif-openapi") {
            features.push("OpenAPI/Swagger".to_string());
        }
    }
    
    Ok(features)
}

async fn calculate_project_stats() -> Result<ProjectStats, ElifError> {
    let mut stats = ProjectStats {
        lines_of_code: 0,
        files_count: 0,
        test_coverage: None,
        build_time: None,
    };
    
    // Count lines of code in src/
    if Path::new("src").exists() {
        let (loc, files) = count_lines_recursive("src").await?;
        stats.lines_of_code = loc;
        stats.files_count = files;
    }
    
    // Try to get build time
    stats.build_time = get_build_time().await?;
    
    Ok(stats)
}

async fn count_lines_recursive(dir: &str) -> Result<(u32, u32), ElifError> {
    let mut total_lines = 0;
    let mut total_files = 0;
    
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| ElifError::Io(e))?;
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| ElifError::Io(e))? {
        let path = entry.path();
        
        if path.is_dir() {
            let (sub_lines, sub_files) = count_lines_recursive(&path.to_string_lossy()).await?;
            total_lines += sub_lines;
            total_files += sub_files;
        } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                total_lines += content.lines().count() as u32;
                total_files += 1;
            }
        }
    }
    
    Ok((total_lines, total_files))
}

async fn get_build_time() -> Result<Option<String>, ElifError> {
    // Try to run cargo build --dry-run to estimate build time
    let start = std::time::Instant::now();
    
    let output = Command::new("cargo")
        .args(&["check", "--quiet"])
        .output()
        .map_err(|e| ElifError::Io(e))?;
    
    if output.status.success() {
        let duration = start.elapsed();
        Ok(Some(format!("{:.2}s", duration.as_secs_f64())))
    } else {
        Ok(None)
    }
}

async fn display_detailed_info(project_info: &ProjectInfo) -> Result<(), ElifError> {
    println!("\n📋 Project Details:");
    println!("   📦 Name: {}", project_info.name);
    println!("   🏷️  Version: {}", project_info.version);
    println!("   📅 Edition: {}", project_info.edition);
    println!("   🔧 elif.rs Version: {}", project_info.elif_version);
    
    println!("\n📊 Project Statistics:");
    println!("   📝 Lines of Code: {}", project_info.project_stats.lines_of_code);
    println!("   📁 Files: {}", project_info.project_stats.files_count);
    
    if let Some(build_time) = &project_info.project_stats.build_time {
        println!("   ⏱️  Build Time: {}", build_time);
    }
    
    if let Some(coverage) = project_info.project_stats.test_coverage {
        println!("   🎯 Test Coverage: {:.1}%", coverage);
    }
    
    if !project_info.features.is_empty() {
        println!("\n✨ Enabled Features:");
        for feature in &project_info.features {
            println!("   • {}", feature);
        }
    }
    
    if !project_info.dependencies.is_empty() {
        println!("\n📦 Key Dependencies:");
        let mut elif_deps: Vec<_> = project_info.dependencies.iter()
            .filter(|(name, _)| name.starts_with("elif-"))
            .collect();
        elif_deps.sort_by_key(|(name, _)| *name);
        
        for (name, version) in elif_deps.iter().take(10) {
            println!("   • {} = {}", name, version);
        }
        
        if elif_deps.len() > 10 {
            println!("   • ... and {} more", elif_deps.len() - 10);
        }
    }
    
    Ok(())
}

async fn display_module_info(project_info: &ProjectInfo) -> Result<(), ElifError> {
    if project_info.modules.is_empty() {
        println!("\n📦 Modules: No modules detected");
        println!("   💡 Use 'elifrs add module <name>' to create your first module");
        return Ok(());
    }
    
    println!("\n📦 Module System:");
    println!("   📊 Total Modules: {}", project_info.modules.len());
    
    for module in &project_info.modules {
        println!("\n   🔧 Module: {}", module.name);
        println!("      📍 Path: {}", module.path);
        
        if !module.providers.is_empty() {
            println!("      🏭 Providers: {}", module.providers.join(", "));
        }
        
        if !module.controllers.is_empty() {
            println!("      🎮 Controllers: {}", module.controllers.join(", "));
        }
        
        if !module.dependencies.is_empty() {
            println!("      🔗 Dependencies: {}", module.dependencies.join(", "));
        }
    }
    
    println!("\n   💡 Use 'elifrs module graph' to visualize module dependencies");
    
    Ok(())
}
