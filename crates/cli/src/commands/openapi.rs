use elif_core::ElifError;
use elif_openapi::{
    OpenApiGenerator, OpenApiConfig,
    discovery::ProjectDiscovery,
    generator::RouteMetadata,
    utils::{OpenApiUtils, OutputFormat},
};
use std::path::Path;

/// Generate OpenAPI specification from project
pub async fn generate(output_path: Option<String>, format: Option<String>) -> Result<(), ElifError> {
    println!("🔍 Discovering project structure...");
    
    let project_root = std::env::current_dir()?;
    
    let discovery = ProjectDiscovery::new(&project_root);
    let project_structure = discovery.discover()
        .map_err(|e| ElifError::Codegen(format!("Project discovery failed: {}", e)))?;
    
    println!("📊 Found {} controllers, {} models", 
        project_structure.controllers.len(), 
        project_structure.models.len());
    
    // Create OpenAPI configuration  
    let config = OpenApiConfig::new(&project_structure.metadata.name, &project_structure.metadata.version)
        .add_server("http://localhost:3000", Some("Development server"))
        .add_tag("API", Some("Generated API endpoints"));
    
    let mut generator = OpenApiGenerator::new(config);
    
    // Convert discovered routes to OpenAPI routes
    // For now, create a simple demo route since we don't have full route discovery yet
    let routes = vec![
        RouteMetadata {
            method: "GET".to_string(),
            path: "/health".to_string(),
            summary: Some("Health check endpoint".to_string()),
            description: Some("Returns the health status of the API".to_string()),
            operation_id: Some("healthCheck".to_string()),
            tags: vec!["Health".to_string()],
            request_schema: None,
            response_schemas: std::collections::HashMap::new(),
            parameters: Vec::new(),
            security: Vec::new(),
            deprecated: false,
        }
    ];
    
    println!("⚙️  Generating OpenAPI specification...");
    let spec = generator.generate(&routes)
        .map_err(|e| ElifError::Codegen(format!("OpenAPI generation failed: {}", e)))?;
    
    // Determine output format and path
    let output_format = match format.as_deref() {
        Some("yaml") | Some("yml") => OutputFormat::Yaml,
        Some("json") | _ => OutputFormat::Json,
    };
    
    let default_filename = match output_format {
        OutputFormat::Json => "target/_openapi.json",
        OutputFormat::Yaml => "target/_openapi.yaml",
    };
    
    let final_path = output_path.as_deref().unwrap_or(default_filename);
    
    // Ensure target directory exists
    if let Some(parent) = Path::new(final_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Save the specification
    OpenApiUtils::save_spec_to_file(spec, final_path, output_format, true)
        .map_err(|e| ElifError::Codegen(format!("Failed to save specification: {}", e)))?;
    
    println!("✅ OpenAPI specification generated: {}", final_path);
    
    // Validate the specification
    let warnings = OpenApiUtils::validate_spec(spec)
        .map_err(|e| ElifError::Validation(format!("Validation failed: {}", e)))?;
    
    if !warnings.is_empty() {
        println!("⚠️  Validation warnings:");
        for warning in warnings {
            match warning.level {
                elif_openapi::utils::ValidationLevel::Error => println!("   ❌ {}", warning.message),
                elif_openapi::utils::ValidationLevel::Warning => println!("   ⚠️  {}", warning.message),
                elif_openapi::utils::ValidationLevel::Info => println!("   ℹ️  {}", warning.message),
            }
        }
    } else {
        println!("✅ Specification validation passed");
    }
    
    Ok(())
}

/// Export OpenAPI specification to different formats
pub async fn export(format: String, output: String) -> Result<(), ElifError> {
    println!("📤 Exporting OpenAPI specification...");
    
    // First, generate the specification if it doesn't exist
    if !Path::new("target/_openapi.json").exists() && !Path::new("target/_openapi.yaml").exists() {
        println!("🔍 OpenAPI specification not found, generating...");
        generate(None, None).await?;
    }
    
    // Load existing specification  
    let spec_path = if Path::new("target/_openapi.json").exists() {
        "target/_openapi.json"
    } else {
        "target/_openapi.yaml"
    };
    
    let spec = OpenApiUtils::load_spec_from_file(spec_path)
        .map_err(|e| ElifError::Codegen(format!("Failed to load specification: {}", e)))?;
    
    match format.to_lowercase().as_str() {
        "postman" => {
            let collection = elif_openapi::export::OpenApiExporter::export_postman(&spec)
                .map_err(|e| ElifError::Codegen(format!("Postman export failed: {}", e)))?;
            
            let json = serde_json::to_string_pretty(&collection)?;
            
            std::fs::write(&output, json)?;
            
            println!("✅ Postman collection exported: {}", output);
        },
        "insomnia" => {
            let workspace = elif_openapi::export::OpenApiExporter::export_insomnia(&spec)
                .map_err(|e| ElifError::Codegen(format!("Insomnia export failed: {}", e)))?;
            
            let json = serde_json::to_string_pretty(&workspace)?;
            
            std::fs::write(&output, json)?;
            
            println!("✅ Insomnia workspace exported: {}", output);
        },
        _ => {
            return Err(ElifError::Validation(format!("Unsupported export format: {}", format)));
        }
    }
    
    Ok(())
}

/// Serve interactive Swagger UI documentation
pub async fn serve(port: Option<u16>) -> Result<(), ElifError> {
    println!("🚀 Starting Swagger UI server...");
    
    // Generate specification if it doesn't exist
    if !Path::new("target/_openapi.json").exists() && !Path::new("target/_openapi.yaml").exists() {
        println!("🔍 OpenAPI specification not found, generating...");
        generate(None, None).await?;
    }
    
    // Load the specification
    let spec_path = if Path::new("target/_openapi.json").exists() {
        "target/_openapi.json"  
    } else {
        "target/_openapi.yaml"
    };
    
    let spec = OpenApiUtils::load_spec_from_file(spec_path)
        .map_err(|e| ElifError::Codegen(format!("Failed to load specification: {}", e)))?;
    
    // Configure Swagger UI
    let config = elif_openapi::swagger::SwaggerConfig::new()
        .with_server("127.0.0.1", port.unwrap_or(8080))
        .with_title(&format!("{} - API Documentation", spec.info.title));
    
    let swagger_ui = elif_openapi::swagger::SwaggerUi::new(spec, config);
    
    // This would start the server in a real implementation
    // For now, just generate static HTML
    let html = elif_openapi::swagger::SwaggerUi::generate_static_html(
        swagger_ui.specification().unwrap(),
        swagger_ui.config()
    ).map_err(|e| ElifError::Codegen(format!("HTML generation failed: {}", e)))?;
    
    std::fs::create_dir_all("target")?;
    std::fs::write("target/_swagger.html", html)?;
    
    println!("📖 Static Swagger UI generated: target/_swagger.html");
    println!("💡 Open target/_swagger.html in your browser to view the API documentation");
    
    Ok(())
}