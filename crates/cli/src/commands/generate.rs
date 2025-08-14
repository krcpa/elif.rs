use elif_core::ElifError;
use elif_codegen::CodeGenerator;

pub async fn run() -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;
    
    let generator = CodeGenerator::new(project_root);
    generator.generate_all()?;
    
    println!("✓ Code generation completed");
    Ok(())
}