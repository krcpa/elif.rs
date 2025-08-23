use elif_core::ElifError;
use std::process::Command;

pub async fn run(unit: bool, integration: bool, watch: bool, coverage: bool, module: Option<&str>) -> Result<(), ElifError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    
    if unit {
        println!("🧪 Running unit tests...");
        cmd.arg("--lib");
    } else if integration {
        println!("🔗 Running integration tests...");
        cmd.arg("--test");
    } else {
        println!("🧪 Running all tests...");
    }
    
    if let Some(mod_name) = module {
        println!("   Focusing on module: {}", mod_name);
        cmd.arg("--").arg(&format!("test_{}_", mod_name.to_lowercase()));
    }
    
    if watch {
        println!("👀 Watch mode enabled (note: requires cargo-watch)");
        // This would require cargo-watch to be installed
    }
    
    if coverage {
        println!("📊 Coverage enabled (note: requires cargo-tarpaulin)");
        // This would require cargo-tarpaulin to be installed
    }
    
    let output = cmd.output()
        .map_err(|e| ElifError::Codegen { message: format!("Failed to run tests: {}", e) })?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::Validation { message: format!("Tests failed:\n{}", stderr) });
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);
    println!("✓ Tests passed");
    
    Ok(())
}