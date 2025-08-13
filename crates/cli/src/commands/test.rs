use elif_core::ElifError;
use std::process::Command;

pub async fn run(focus: Option<String>) -> Result<(), ElifError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    
    if let Some(resource) = focus {
        cmd.arg("--").arg(&format!("test_{}_", resource.to_lowercase()));
        println!("Running tests for resource: {}", resource);
    } else {
        println!("Running all tests...");
    }
    
    let output = cmd.output()
        .map_err(|e| ElifError::Codegen(format!("Failed to run tests: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::Validation(format!("Tests failed:\n{}", stderr)));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);
    println!("✓ Tests passed");
    
    Ok(())
}