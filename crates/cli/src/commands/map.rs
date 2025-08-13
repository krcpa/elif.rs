use elif_core::ElifError;
use elif_introspect::MapGenerator;

pub async fn run(json: bool) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;
    
    let generator = MapGenerator::new(project_root);
    let map = generator.generate()?;
    
    if json {
        let json_output = serde_json::to_string_pretty(&map)?;
        
        // Write to target/_map.json
        std::fs::create_dir_all("target")?;
        std::fs::write("target/_map.json", &json_output)?;
        
        // Also output to stdout
        println!("{}", json_output);
    } else {
        println!("Project Map:");
        println!("Routes: {}", map.routes.len());
        println!("Models: {}", map.models.len());
        println!("Specs: {}", map.specs.len());
    }
    
    Ok(())
}