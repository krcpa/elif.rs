use elif_core::ElifError;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml::Value as TomlValue;

pub async fn run(comprehensive: bool, module: Option<&str>) -> Result<(), ElifError> {
    let mut results = CheckResults::new();

    if comprehensive {
        println!("\n🔍 Running comprehensive project health check...");
    } else {
        println!("\n🔍 Running basic project health check...");
    }

    if let Some(mod_name) = module {
        println!("   Focusing on module: {}", mod_name);
    }

    // 1. Project structure validation
    println!("\n📁 Checking project structure...");
    check_project_structure(&mut results)?;

    // 2. Cargo manifest validation
    println!("📦 Checking Cargo configuration...");
    check_cargo_manifest(&mut results)?;

    // 3. Source code structure
    println!("🦀 Checking source code structure...");
    check_source_structure(&mut results)?;

    if comprehensive {
        // 4. Module system validation
        if module.is_none() {
            println!("🔗 Checking module system...");
            check_module_system(&mut results)?;
        } else {
            println!("🔗 Checking module: {}...", module.unwrap());
            check_specific_module(&mut results, module.unwrap())?;
        }

        // 5. Configuration validation
        println!("⚙️  Checking configuration...");
        check_configuration(&mut results)?;

        // 6. Dependency compatibility
        println!("📚 Checking dependencies...");
        check_dependencies(&mut results)?;

        // 7. Framework health checks
        println!("🏥 Checking framework health...");
        check_framework_health(&mut results)?;

        // 8. Security configuration
        println!("🔒 Checking security configuration...");
        check_security_config(&mut results)?;
    }

    // 7. Code quality checks (existing)
    println!("✨ Checking code quality...");
    check_code_quality(&mut results)?;

    // 8. Resource specifications (existing)
    println!("📋 Checking resource specifications...");
    check_resource_specs_enhanced(&mut results)?;

    // Print final results
    results.print_summary();

    if results.has_errors() {
        return Err(ElifError::Validation {
            message: format!("Health check failed with {} errors", results.error_count()),
        });
    }

    Ok(())
}

#[derive(Default)]
struct CheckResults {
    successes: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
    recommendations: Vec<String>,
}

impl CheckResults {
    fn new() -> Self {
        Default::default()
    }

    fn success(&mut self, message: &str) {
        self.successes.push(message.to_string());
        println!("  ✅ {}", message);
    }

    fn warning(&mut self, message: &str) {
        self.warnings.push(message.to_string());
        println!("  ⚠️  {}", message);
    }

    fn error(&mut self, message: &str) {
        self.errors.push(message.to_string());
        println!("  ❌ {}", message);
    }

    fn recommend(&mut self, message: &str) {
        self.recommendations.push(message.to_string());
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn error_count(&self) -> usize {
        self.errors.len()
    }

    fn print_summary(&self) {
        println!("\n📊 Health Check Summary:");
        println!("   ✅ Successes: {}", self.successes.len());
        println!("   ⚠️  Warnings:  {}", self.warnings.len());
        println!("   ❌ Errors:    {}", self.errors.len());

        if !self.recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for rec in &self.recommendations {
                println!("   • {}", rec);
            }
        }

        if self.errors.is_empty() && self.warnings.is_empty() {
            println!("\n🎉 All checks passed! Your elif.rs project is healthy.");
        } else if self.errors.is_empty() {
            println!("\n✨ Project is healthy with minor warnings.");
        }
    }
}

fn check_project_structure(results: &mut CheckResults) -> Result<(), ElifError> {
    // Check essential files
    let essential_files = ["Cargo.toml", "src/main.rs"];
    for file in &essential_files {
        if Path::new(file).exists() {
            results.success(&format!("Found {}", file));
        } else {
            results.error(&format!("Missing essential file: {}", file));
            results.recommend(&format!(
                "Create {} to establish proper project structure",
                file
            ));
        }
    }

    // Check recommended directories
    let recommended_dirs = ["src", "tests"];
    for dir in &recommended_dirs {
        if Path::new(dir).is_dir() {
            results.success(&format!("Found {} directory", dir));
        } else {
            results.warning(&format!("Missing recommended directory: {}", dir));
            results.recommend(&format!("Create {} directory for better organization", dir));
        }
    }

    // Check optional but useful files
    let optional_files = ["README.md", ".gitignore", ".env.example"];
    for file in &optional_files {
        if Path::new(file).exists() {
            results.success(&format!("Found {}", file));
        } else {
            results.warning(&format!("Consider adding {}", file));
        }
    }

    Ok(())
}

fn check_cargo_manifest(results: &mut CheckResults) -> Result<(), ElifError> {
    let cargo_path = Path::new("Cargo.toml");
    if !cargo_path.exists() {
        results.error("Cargo.toml not found");
        return Ok(());
    }

    let content = fs::read_to_string(cargo_path).map_err(|e| ElifError::Validation {
        message: format!("Failed to read Cargo.toml: {}", e),
    })?;

    let manifest: TomlValue = toml::from_str(&content).map_err(|e| ElifError::Validation {
        message: format!("Invalid Cargo.toml: {}", e),
    })?;

    // Check package section
    if let Some(package) = manifest.get("package") {
        if package.get("name").is_some() {
            results.success("Package name defined");
        } else {
            results.error("Package name missing");
        }

        if package.get("version").is_some() {
            results.success("Package version defined");
        } else {
            results.warning("Package version missing");
        }

        if package.get("edition").is_some() {
            results.success("Rust edition specified");
        } else {
            results.warning("Consider specifying Rust edition");
        }
    } else {
        results.error("Package section missing in Cargo.toml");
    }

    // Check for elif dependencies
    if let Some(deps) = manifest.get("dependencies") {
        let elif_deps = ["elif-core", "elif-http", "elif-orm", "elif-auth"];
        let mut found_elif = false;

        for dep in &elif_deps {
            if deps.get(dep).is_some() {
                results.success(&format!("Found elif dependency: {}", dep));
                found_elif = true;
            }
        }

        if !found_elif {
            results.warning("No elif.rs dependencies found");
            results.recommend("Add elif.rs dependencies like 'elif-http' for web functionality");
        }
    }

    Ok(())
}

fn check_source_structure(results: &mut CheckResults) -> Result<(), ElifError> {
    let src_dir = Path::new("src");
    if !src_dir.is_dir() {
        results.error("src directory not found");
        return Ok(());
    }

    // Check main.rs or lib.rs
    let has_main = src_dir.join("main.rs").exists();
    let has_lib = src_dir.join("lib.rs").exists();

    if has_main {
        results.success("Found src/main.rs (binary crate)");
    } else if has_lib {
        results.success("Found src/lib.rs (library crate)");
    } else {
        results.error("Missing src/main.rs or src/lib.rs");
        results.recommend("Create src/main.rs for a binary crate or src/lib.rs for a library");
    }

    // Check for common elif.rs patterns
    let common_files = ["controllers", "services", "models", "middleware", "modules"];
    for file in &common_files {
        let as_file = src_dir.join(format!("{}.rs", file));
        let as_dir = src_dir.join(file);

        if as_file.exists() || as_dir.is_dir() {
            results.success(&format!("Found {} organization", file));
        } else {
            results.warning(&format!(
                "Consider adding a '{}' directory or file for better organization",
                file
            ));
        }
    }

    Ok(())
}

fn check_module_system(results: &mut CheckResults) -> Result<(), ElifError> {
    // Look for module definitions in src/
    let src_dir = Path::new("src");
    if !src_dir.is_dir() {
        return Ok(());
    }

    let mut modules = HashSet::new();

    // Simple module detection (can be enhanced)
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem != "main" && stem != "lib" {
                    modules.insert(stem.to_string());
                }
            }
        } else if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                modules.insert(name.to_string());
            }
        }
    }

    if modules.is_empty() {
        results.warning("No modules detected");
        results.recommend("Consider organizing code into modules for better structure");
    } else {
        results.success(&format!("Found {} modules", modules.len()));
    }

    // TODO: Add circular dependency detection

    Ok(())
}

fn check_specific_module(results: &mut CheckResults, module_name: &str) -> Result<(), ElifError> {
    let module_file = Path::new("src").join(format!("{}.rs", module_name));
    let module_dir = Path::new("src").join(module_name);

    if module_file.exists() {
        results.success(&format!("Module {} found as file", module_name));
    } else if module_dir.is_dir() {
        results.success(&format!("Module {} found as directory", module_name));
    } else {
        results.error(&format!("Module {} not found", module_name));
        results.recommend(&format!(
            "Create src/{}.rs or src/{}/mod.rs",
            module_name, module_name
        ));
    }

    Ok(())
}

fn check_configuration(results: &mut CheckResults) -> Result<(), ElifError> {
    // Check .env files
    if Path::new(".env").exists() {
        results.success("Found .env file");
    } else {
        results.warning("No .env file found");
        results.recommend("Create .env file for environment configuration");
    }

    if Path::new(".env.example").exists() {
        results.success("Found .env.example file");
    } else {
        results.warning("No .env.example file found");
        results.recommend("Create .env.example to document required environment variables");
    }

    // Check configuration files
    let config_files = ["config.toml", "elif.toml", "settings.toml"];
    let mut found_config = false;

    for config in &config_files {
        if Path::new(config).exists() {
            results.success(&format!("Found configuration file: {}", config));
            found_config = true;
        }
    }

    if !found_config {
        results.warning("No configuration files found");
        results.recommend("Consider adding configuration files for better project setup");
    }

    Ok(())
}

fn check_dependencies(results: &mut CheckResults) -> Result<(), ElifError> {
    // Check if Cargo.lock exists
    if Path::new("Cargo.lock").exists() {
        results.success("Found Cargo.lock (dependencies locked)");
    } else {
        results.warning("No Cargo.lock found");
        results.recommend("Run 'cargo build' to generate Cargo.lock");
    }

    // Try to run cargo check to validate dependencies
    let output = Command::new("cargo").args(["check", "--quiet"]).output();

    match output {
        Ok(output) if output.status.success() => {
            results.success("All dependencies resolve correctly");
        }
        Ok(_) => {
            results.error("Dependency resolution issues found");
            results.recommend("Run 'cargo check' for detailed dependency errors");
        }
        Err(_) => {
            results.warning("Could not verify dependencies (cargo not available)");
        }
    }

    Ok(())
}

fn check_code_quality(results: &mut CheckResults) -> Result<(), ElifError> {
    // Run cargo fmt check
    let fmt_output = Command::new("cargo").args(["fmt", "--check"]).output();

    match fmt_output {
        Ok(output) if output.status.success() => {
            results.success("Code formatting is correct");
        }
        Ok(_) => {
            results.warning("Code formatting issues found");
            results.recommend("Run 'cargo fmt' to fix formatting");
        }
        Err(_) => {
            results.warning("Could not check formatting (rustfmt not available)");
        }
    }

    // Run cargo clippy
    let clippy_output = Command::new("cargo")
        .args(["clippy", "--quiet", "--", "-D", "warnings"])
        .output();

    match clippy_output {
        Ok(output) if output.status.success() => {
            results.success("No clippy warnings found");
        }
        Ok(_) => {
            results.warning("Clippy warnings found");
            results.recommend("Run 'cargo clippy' to see and fix warnings");
        }
        Err(_) => {
            results.warning("Could not run clippy (clippy not available)");
        }
    }

    Ok(())
}

fn check_resource_specs_enhanced(results: &mut CheckResults) -> Result<(), ElifError> {
    let resources_dir = Path::new("resources");
    if !resources_dir.exists() {
        results.warning("No resources directory found");
        return Ok(());
    }

    let mut spec_count = 0;

    for entry in fs::read_dir(resources_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "yaml")
            && path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.ends_with(".resource"))
        {
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            match fs::read_to_string(&path) {
                Ok(content) => match elif_core::ResourceSpec::from_yaml(&content) {
                    Ok(_) => {
                        results.success(&format!("Validated {}", file_name));
                        spec_count += 1;
                    }
                    Err(e) => {
                        results.error(&format!("Invalid resource spec {}: {}", file_name, e));
                    }
                },
                Err(e) => {
                    results.error(&format!("Could not read {}: {}", file_name, e));
                }
            }
        }
    }

    if spec_count == 0 {
        results.warning("No resource specifications found");
    } else {
        results.success(&format!("Found {} resource specifications", spec_count));
    }

    Ok(())
}

fn check_framework_health(results: &mut CheckResults) -> Result<(), ElifError> {
    // Check database connectivity if configured
    if std::env::var("DATABASE_URL").is_ok() {
        // Test database connection using port check
        if is_port_in_use(5432) {
            results.success("Database service is running");
        } else {
            results.warning("Database service appears to be down");
            results.recommend("Start your database service (PostgreSQL)");
        }
    }

    // Check Redis connectivity if configured
    if std::env::var("REDIS_URL").is_ok() || is_port_in_use(6379) {
        if is_port_in_use(6379) {
            results.success("Redis service is running");
        } else {
            results.warning("Redis service appears to be down");
            results.recommend("Start Redis service if using caching");
        }
    }

    // Check if application is running
    let common_ports = [3000, 8000, 8080];
    let mut app_running = false;
    for port in &common_ports {
        if is_port_in_use(*port) {
            results.success(&format!("Application running on port {}", port));
            app_running = true;
            break;
        }
    }

    if !app_running {
        results.warning("Application is not currently running");
        results.recommend("Start the application with: elifrs serve");
    }

    // Check target directory and build artifacts
    if Path::new("target/debug").exists() {
        results.success("Debug build artifacts found");
    } else {
        results.warning("No debug build artifacts found");
        results.recommend("Run 'cargo build' to create build artifacts");
    }

    Ok(())
}

fn check_security_config(results: &mut CheckResults) -> Result<(), ElifError> {
    // Check for sensitive files that shouldn't be committed
    let sensitive_files = [".env", "private.key", "secrets.yaml", "database.url"];
    for file in &sensitive_files {
        if Path::new(file).exists() {
            results.warning(&format!("Sensitive file found: {} - ensure it's in .gitignore", file));
        }
    }

    // Check .gitignore exists and has common patterns
    if Path::new(".gitignore").exists() {
        let gitignore_content = fs::read_to_string(".gitignore").unwrap_or_default();
        if gitignore_content.contains("target/") {
            results.success("Build artifacts are ignored in git");
        } else {
            results.warning("Build artifacts (target/) not ignored in git");
            results.recommend("Add 'target/' to .gitignore");
        }

        if gitignore_content.contains(".env") {
            results.success("Environment files are ignored in git");
        } else {
            results.warning("Environment files (.env) not ignored in git");
            results.recommend("Add '.env' to .gitignore");
        }
    } else {
        results.error(".gitignore file missing");
        results.recommend("Create .gitignore to prevent committing sensitive files");
    }

    // Check environment variable security
    if let Ok(secret_key) = std::env::var("SECRET_KEY") {
        if secret_key.len() < 32 {
            results.warning("SECRET_KEY is too short (should be 32+ characters)");
            results.recommend("Use a longer SECRET_KEY for better security");
        } else {
            results.success("SECRET_KEY length is adequate");
        }

        if secret_key == "changeme" || secret_key == "your-secret-key" {
            results.error("SECRET_KEY appears to be a default value");
            results.recommend("Change SECRET_KEY to a secure random value");
        }
    } else {
        results.warning("SECRET_KEY environment variable not set");
        results.recommend("Set SECRET_KEY environment variable for security");
    }

    // Check for debug settings in production-like environments
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        if rust_log.contains("debug") || rust_log.contains("trace") {
            results.warning("Debug logging enabled - consider for production");
            results.recommend("Use 'info' or 'warn' log levels in production");
        }
    }

    Ok(())
}

fn is_port_in_use(port: u16) -> bool {
    use std::net::{TcpListener, SocketAddr};
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).is_err()
}
