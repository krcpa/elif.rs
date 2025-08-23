use elif_core::ElifError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde_json::json;

/// List and visualize all modules
pub async fn modules(graph: bool, format: &str) -> Result<(), ElifError> {
    println!("🧩 Inspecting project modules...");
    
    let modules = discover_modules().await?;
    
    match format {
        "json" => {
            let json_output = json!({
                "modules": modules,
                "total": modules.len(),
                "framework": "elif.rs",
                "version": "0.8.0"
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        "dot" if graph => {
            generate_dot_graph(&modules)?;
        }
        _ => {
            print_modules_table(&modules, graph)?;
        }
    }
    
    Ok(())
}

/// Show dependencies for a specific module
pub async fn dependencies(module_name: &str, transitive: bool) -> Result<(), ElifError> {
    println!("🔍 Analyzing dependencies for module '{}'...", module_name);
    
    let modules = discover_modules().await?;
    
    if let Some(module) = modules.iter().find(|m| m.name == module_name) {
        print_module_dependencies(module, &modules, transitive)?;
    } else {
        return Err(ElifError::validation(&format!("Module '{}' not found", module_name)));
    }
    
    Ok(())
}

/// Show project structure and configuration
pub async fn config(detailed: bool, validate: bool) -> Result<(), ElifError> {
    println!("⚙️ Inspecting project configuration...");
    
    let config = load_project_config().await?;
    
    if validate {
        validate_configuration(&config).await?;
    }
    
    print_configuration(&config, detailed)?;
    
    Ok(())
}

// Data structures

#[derive(Debug, Clone, serde::Serialize)]
struct ModuleInfo {
    name: String,
    path: String,
    controllers: Vec<String>,
    providers: Vec<String>,
    imports: Vec<String>,
    exports: Vec<String>,
    file_count: usize,
    line_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ProjectConfig {
    name: String,
    version: String,
    dependencies: HashMap<String, String>,
    modules_enabled: bool,
    database_url: Option<String>,
    server_config: ServerConfig,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ServerConfig {
    host: String,
    port: u16,
    environment: String,
}

// Implementation

async fn discover_modules() -> Result<Vec<ModuleInfo>, ElifError> {
    let modules_dir = Path::new("src/modules");
    let mut modules = Vec::new();
    
    if !modules_dir.exists() {
        println!("⚠️ No modules directory found. Run 'elifrs add module MyModule' to get started.");
        return Ok(modules);
    }
    
    // Read all .rs files in modules directory
    for entry in fs::read_dir(modules_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("rs") && path.file_name().unwrap() != "mod.rs" {
            if let Some(module) = parse_module_file(&path).await? {
                modules.push(module);
            }
        }
    }
    
    Ok(modules)
}

async fn parse_module_file(path: &Path) -> Result<Option<ModuleInfo>, ElifError> {
    let content = fs::read_to_string(path)?;
    let file_name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    
    // Simple parsing - in a real implementation you'd use syn or similar
    let module_name = to_pascal_case(file_name);
    
    // Count lines and extract basic info
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len();
    
    // Extract module macro attributes (simplified)
    let controllers = extract_list_from_content(&content, "controllers");
    let providers = extract_list_from_content(&content, "providers");
    let imports = extract_list_from_content(&content, "imports");
    let exports = extract_list_from_content(&content, "exports");
    
    Ok(Some(ModuleInfo {
        name: module_name,
        path: path.to_string_lossy().to_string(),
        controllers,
        providers,
        imports,
        exports,
        file_count: 1, // Each module is one file for now
        line_count,
    }))
}

fn extract_list_from_content(content: &str, list_name: &str) -> Vec<String> {
    // Very simple extraction - in reality you'd parse the AST
    let mut items = Vec::new();
    
    for line in content.lines() {
        if line.trim().starts_with(&format!("{} =", list_name)) {
            // Extract items between [ and ]
            if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    let items_str = &line[start+1..end];
                    for item in items_str.split(',') {
                        let item = item.trim().trim_matches('"');
                        if !item.is_empty() {
                            items.push(item.to_string());
                        }
                    }
                }
            }
        }
    }
    
    items
}

fn print_modules_table(modules: &[ModuleInfo], show_graph: bool) -> Result<(), ElifError> {
    if modules.is_empty() {
        println!("📭 No modules found in this project.");
        println!("\n💡 Get started:");
        println!("   elifrs add module UserModule --controllers=UserController");
        return Ok(());
    }
    
    println!("\n📋 Module Overview:");
    println!("┌─────────────────────────┬─────────────┬─────────────┬────────────┬───────────┐");
    println!("│ Module                  │ Controllers │ Providers   │ Lines      │ Path      │");
    println!("├─────────────────────────┼─────────────┼─────────────┼────────────┼───────────┤");
    
    for module in modules {
        println!(
            "│ {:<23} │ {:<11} │ {:<11} │ {:<10} │ {}",
            truncate(&module.name, 23),
            module.controllers.len(),
            module.providers.len(),
            module.line_count,
            truncate(&module.path, 20)
        );
    }
    
    println!("└─────────────────────────┴─────────────┴─────────────┴────────────┴───────────┘");
    
    // Show detailed info for each module
    if show_graph {
        println!("\n🔗 Module Dependencies:");
        for module in modules {
            print_module_details(module)?;
        }
    }
    
    // Summary statistics
    let total_controllers: usize = modules.iter().map(|m| m.controllers.len()).sum();
    let total_providers: usize = modules.iter().map(|m| m.providers.len()).sum();
    let total_lines: usize = modules.iter().map(|m| m.line_count).sum();
    
    println!("\n📊 Summary:");
    println!("   Modules: {}", modules.len());
    println!("   Controllers: {}", total_controllers);
    println!("   Providers: {}", total_providers);
    println!("   Total lines: {}", total_lines);
    
    Ok(())
}

fn print_module_details(module: &ModuleInfo) -> Result<(), ElifError> {
    println!("\n🧩 {}", module.name);
    
    if !module.controllers.is_empty() {
        println!("   🎮 Controllers: {}", module.controllers.join(", "));
    }
    
    if !module.providers.is_empty() {
        println!("   ⚙️ Providers: {}", module.providers.join(", "));
    }
    
    if !module.imports.is_empty() {
        println!("   📥 Imports: {}", module.imports.join(", "));
    }
    
    if !module.exports.is_empty() {
        println!("   📤 Exports: {}", module.exports.join(", "));
    }
    
    Ok(())
}

fn print_module_dependencies(
    module: &ModuleInfo,
    all_modules: &[ModuleInfo],
    transitive: bool
) -> Result<(), ElifError> {
    println!("Module: {}", module.name);
    println!("Path: {}", module.path);
    
    println!("\n📥 Direct Dependencies:");
    if module.imports.is_empty() {
        println!("   (none)");
    } else {
        for import in &module.imports {
            println!("   • {}", import);
        }
    }
    
    println!("\n📤 Exports:");
    if module.exports.is_empty() {
        println!("   (none)");
    } else {
        for export in &module.exports {
            println!("   • {}", export);
        }
    }
    
    // Find dependents (modules that import this one)
    let dependents: Vec<&ModuleInfo> = all_modules
        .iter()
        .filter(|m| m.imports.contains(&module.name))
        .collect();
    
    println!("\n⬅️ Used by:");
    if dependents.is_empty() {
        println!("   (none)");
    } else {
        for dependent in dependents {
            println!("   • {}", dependent.name);
        }
    }
    
    if transitive {
        println!("\n🔄 Transitive Dependencies:");
        let transitive_deps = find_transitive_dependencies(module, all_modules);
        if transitive_deps.is_empty() {
            println!("   (none)");
        } else {
            for dep in transitive_deps {
                println!("   • {}", dep);
            }
        }
    }
    
    Ok(())
}

fn find_transitive_dependencies(module: &ModuleInfo, all_modules: &[ModuleInfo]) -> Vec<String> {
    let mut visited = std::collections::HashSet::new();
    let mut deps = Vec::new();
    
    fn collect_deps(
        module_name: &str,
        all_modules: &[ModuleInfo],
        visited: &mut std::collections::HashSet<String>,
        deps: &mut Vec<String>
    ) {
        if visited.contains(module_name) {
            return;
        }
        visited.insert(module_name.to_string());
        
        if let Some(module) = all_modules.iter().find(|m| m.name == module_name) {
            for import in &module.imports {
                deps.push(import.clone());
                collect_deps(import, all_modules, visited, deps);
            }
        }
    }
    
    collect_deps(&module.name, all_modules, &mut visited, &mut deps);
    deps.sort();
    deps.dedup();
    deps
}

fn generate_dot_graph(modules: &[ModuleInfo]) -> Result<(), ElifError> {
    println!("digraph ModuleDependencies {{");
    println!("  rankdir=TB;");
    println!("  node [shape=box, style=rounded];");
    
    // Define nodes
    for module in modules {
        let color = if module.controllers.is_empty() && module.providers.is_empty() {
            "lightgray"
        } else {
            "lightblue"
        };
        
        println!("  \"{}\" [fillcolor={}, style=filled];", module.name, color);
    }
    
    // Define edges
    for module in modules {
        for import in &module.imports {
            println!("  \"{}\" -> \"{}\";", import, module.name);
        }
    }
    
    println!("}}");
    println!("\n💡 Save to file: elifrs inspect modules --graph --format=dot > modules.dot");
    println!("   Generate image: dot -Tpng modules.dot -o modules.png");
    
    Ok(())
}

async fn load_project_config() -> Result<ProjectConfig, ElifError> {
    // Try to load from Cargo.toml
    let cargo_toml = fs::read_to_string("Cargo.toml")
        .map_err(|_| ElifError::validation("Cargo.toml not found - are you in a Rust project?"))?;
    
    // Parse basic info (simplified)
    let name = extract_cargo_field(&cargo_toml, "name").unwrap_or_else(|| "Unknown".to_string());
    let version = extract_cargo_field(&cargo_toml, "version").unwrap_or_else(|| "0.1.0".to_string());
    
    // Load elif.toml if exists
    let _elif_config = fs::read_to_string("elif.toml").ok();
    
    // Load .env if exists  
    let env_config = fs::read_to_string(".env").ok();
    
    let mut dependencies = HashMap::new();
    
    // Extract dependencies from Cargo.toml (simplified)
    if let Some(deps_section) = cargo_toml.split("[dependencies]").nth(1) {
        for line in deps_section.lines().take_while(|line| !line.starts_with('[')) {
            if let Some((key, _value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let version = "unknown".to_string(); // Simplified
                dependencies.insert(key, version);
            }
        }
    }
    
    let modules_enabled = Path::new("src/modules").exists();
    let database_url = env_config.as_ref()
        .and_then(|content| extract_env_var(content, "DATABASE_URL"));
    
    let server_config = ServerConfig {
        host: env_config.as_ref()
            .and_then(|content| extract_env_var(content, "HOST"))
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        port: env_config.as_ref()
            .and_then(|content| extract_env_var(content, "PORT"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000),
        environment: env_config.as_ref()
            .and_then(|content| extract_env_var(content, "APP_ENV"))
            .unwrap_or_else(|| "development".to_string()),
    };
    
    Ok(ProjectConfig {
        name,
        version,
        dependencies,
        modules_enabled,
        database_url,
        server_config,
    })
}

async fn validate_configuration(config: &ProjectConfig) -> Result<(), ElifError> {
    println!("✅ Validating project configuration...");
    
    let mut issues = Vec::new();
    
    // Check for elif dependencies
    if !config.dependencies.contains_key("elif-http") {
        issues.push("❌ Missing elif-http dependency");
    }
    
    if !config.dependencies.contains_key("elif-core") {
        issues.push("❌ Missing elif-core dependency");
    }
    
    // Check module system
    if !config.modules_enabled {
        issues.push("⚠️ Module system not enabled (no src/modules directory)");
    }
    
    // Check database configuration
    if config.database_url.is_none() {
        issues.push("⚠️ No DATABASE_URL configured");
    }
    
    // Check environment files
    if !Path::new(".env").exists() {
        issues.push("⚠️ No .env file found");
    }
    
    if issues.is_empty() {
        println!("✅ Configuration is valid!");
    } else {
        println!("⚠️ Configuration issues found:");
        for issue in issues {
            println!("   {}", issue);
        }
    }
    
    Ok(())
}

fn print_configuration(config: &ProjectConfig, detailed: bool) -> Result<(), ElifError> {
    println!("\n📋 Project Configuration:");
    println!("┌─────────────────────────┬─────────────────────────────────────────────────┐");
    println!("│ Setting                 │ Value                                           │");
    println!("├─────────────────────────┼─────────────────────────────────────────────────┤");
    println!("│ Name                    │ {:<47} │", truncate(&config.name, 47));
    println!("│ Version                 │ {:<47} │", config.version);
    println!("│ Module System           │ {:<47} │", if config.modules_enabled { "✅ Enabled" } else { "❌ Disabled" });
    println!("│ Server Host             │ {:<47} │", config.server_config.host);
    println!("│ Server Port             │ {:<47} │", config.server_config.port);
    println!("│ Environment             │ {:<47} │", config.server_config.environment);
    
    if let Some(db_url) = &config.database_url {
        let masked_url = mask_database_url(db_url);
        println!("│ Database URL            │ {:<47} │", truncate(&masked_url, 47));
    } else {
        println!("│ Database URL            │ {:<47} │", "Not configured");
    }
    
    println!("└─────────────────────────┴─────────────────────────────────────────────────┘");
    
    if detailed {
        println!("\n📦 Dependencies ({}):", config.dependencies.len());
        for (name, version) in &config.dependencies {
            println!("   • {} ({})", name, version);
        }
    }
    
    Ok(())
}

// Helper functions

fn to_pascal_case(snake_case: &str) -> String {
    snake_case
        .split('_')
        .map(|word| {
            let mut chars: Vec<char> = word.chars().collect();
            if !chars.is_empty() {
                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            }
            chars.into_iter().collect::<String>()
        })
        .collect::<String>()
        + "Module"
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn extract_cargo_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        if line.trim().starts_with(&format!("{} =", field)) {
            if let Some(value) = line.split('=').nth(1) {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn extract_env_var(content: &str, var_name: &str) -> Option<String> {
    for line in content.lines() {
        if line.trim().starts_with(&format!("{}=", var_name)) {
            if let Some(value) = line.split('=').nth(1) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn mask_database_url(url: &str) -> String {
    // Mask sensitive parts of database URL
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url.find("://") {
            let protocol = &url[..colon_pos + 3];
            let after_at = &url[at_pos..];
            format!("{}***:***{}", protocol, after_at)
        } else {
            "***masked***".to_string()
        }
    } else {
        url.to_string()
    }
}