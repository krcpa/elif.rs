use elif_core::ElifError;
use std::process::Command;

pub async fn run() -> Result<(), ElifError> {
    println!("Running project checks...");
    
    // Run cargo fmt check
    let fmt_output = Command::new("cargo")
        .args(&["fmt", "--check"])
        .output()
        .map_err(|e| ElifError::Codegen { message: format!("Failed to run cargo fmt: {}", e) })?;
    
    if !fmt_output.status.success() {
        return Err(ElifError::Validation { message: "Code formatting issues found. Run `cargo fmt` to fix.".to_string() });
    }
    
    // Run cargo clippy
    let clippy_output = Command::new("cargo")
        .args(&["clippy", "--", "-D", "warnings"])
        .output()
        .map_err(|e| ElifError::Codegen { message: format!("Failed to run cargo clippy: {}", e) })?;
    
    if !clippy_output.status.success() {
        return Err(ElifError::Validation { message: "Clippy issues found.".to_string() });
    }
    
    // Check resource specifications
    check_resource_specs()?;
    
    println!("✓ All checks passed");
    Ok(())
}

fn check_resource_specs() -> Result<(), ElifError> {
    let resources_dir = std::path::Path::new("resources");
    if !resources_dir.exists() {
        return Ok(());
    }
    
    for entry in std::fs::read_dir(resources_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "yaml") &&
           path.file_stem().and_then(|s| s.to_str())
               .map_or(false, |s| s.ends_with(".resource")) {
            
            let content = std::fs::read_to_string(&path)?;
            let _spec = elif_core::ResourceSpec::from_yaml(&content)?;
            println!("✓ Validated {}", path.display());
        }
    }
    
    Ok(())
}