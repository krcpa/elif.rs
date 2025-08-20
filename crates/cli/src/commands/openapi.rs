use elif_core::ElifError;
use elif_openapi::{
    OpenApiGenerator, OpenApiConfig,
    discovery::{ProjectDiscovery, ProjectStructure},
    generator::{RouteMetadata, ParameterInfo},
    utils::{OpenApiUtils, OutputFormat},
};
use std::path::Path;

/// Generate OpenAPI specification from project
pub async fn generate(output_path: Option<String>, format: Option<String>) -> Result<(), ElifError> {
    println!("🔍 Discovering project structure...");
    
    let project_root = std::env::current_dir().map_err(|e| ElifError::Io(e))?;
    
    let discovery = ProjectDiscovery::new(&project_root);
    let project_structure = discovery.discover()
        .map_err(|e| ElifError::Codegen { message: format!("Project discovery failed: {}", e) })?;
    
    println!("📊 Found {} controllers, {} models", 
        project_structure.controllers.len(), 
        project_structure.models.len());
    
    // Create OpenAPI configuration  
    let config = OpenApiConfig::new(&project_structure.metadata.name, &project_structure.metadata.version)
        .add_server("http://localhost:3000", Some("Development server"))
        .add_tag("API", Some("Generated API endpoints"));
    
    let mut generator = OpenApiGenerator::new(config);
    
    // Convert discovered routes to OpenAPI routes
    let routes = convert_discovered_routes_to_metadata(&project_structure)?;
    
    println!("⚙️  Generating OpenAPI specification...");
    let spec = generator.generate(&routes)
        .map_err(|e| ElifError::Codegen { message: format!("OpenAPI generation failed: {}", e) })?;
    
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
        std::fs::create_dir_all(parent).map_err(|e| ElifError::Io(e))?;
    }
    
    // Save the specification
    OpenApiUtils::save_spec_to_file(spec, final_path, output_format, true)
        .map_err(|e| ElifError::Codegen { message: format!("Failed to save specification: {}", e) })?;
    
    println!("✅ OpenAPI specification generated: {}", final_path);
    
    // Validate the specification
    let warnings = OpenApiUtils::validate_spec(spec)
        .map_err(|e| ElifError::Validation { message: format!("Validation failed: {}", e) })?;
    
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
        .map_err(|e| ElifError::Codegen { message: format!("Failed to load specification: {}", e) })?;
    
    match format.to_lowercase().as_str() {
        "postman" => {
            // TODO: Implement Postman export functionality
            // let collection = elif_openapi::export::OpenApiExporter::export_postman(&spec)
            //     .map_err(|e| ElifError::Codegen { message: format!("Postman export failed: {}", e) })?;
            // 
            // let json = serde_json::to_string_pretty(&collection)?;
            // std::fs::write(&output, json)?;
            
            // For now, create a simple mock collection
            let mock_collection = serde_json::json!({
                "info": {
                    "name": spec.info.title,
                    "version": spec.info.version,
                    "description": "Generated Postman collection (placeholder)"
                },
                "item": []
            });
            
            std::fs::write(&output, serde_json::to_string_pretty(&mock_collection)?)?;
            println!("✅ Postman collection exported: {}", output);
        },
        "insomnia" => {
            // TODO: Implement Insomnia export functionality
            // let workspace = elif_openapi::export::OpenApiExporter::export_insomnia(&spec)
            //     .map_err(|e| ElifError::Codegen { message: format!("Insomnia export failed: {}", e) })?;
            // 
            // let json = serde_json::to_string_pretty(&workspace)?;
            // std::fs::write(&output, json)?;
            
            // For now, create a simple mock workspace
            let mock_workspace = serde_json::json!({
                "_type": "export",
                "__export_format": 4,
                "resources": [{
                    "_type": "workspace",
                    "name": spec.info.title,
                    "description": "Generated Insomnia workspace (placeholder)"
                }]
            });
            
            std::fs::write(&output, serde_json::to_string_pretty(&mock_workspace)?)?;
            println!("✅ Insomnia workspace exported: {}", output);
        },
        _ => {
            return Err(ElifError::Validation { message: format!("Unsupported export format: {}", format) });
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
        .map_err(|e| ElifError::Codegen { message: format!("Failed to load specification: {}", e) })?;
    
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
    ).map_err(|e| ElifError::Codegen { message: format!("HTML generation failed: {}", e) })?;
    
    std::fs::create_dir_all("target").map_err(|e| ElifError::Io(e))?;
    std::fs::write("target/_swagger.html", html).map_err(|e| ElifError::Io(e))?;
    
    println!("📖 Static Swagger UI generated: target/_swagger.html");
    println!("💡 Open target/_swagger.html in your browser to view the API documentation");
    
    Ok(())
}

/// Convert discovered project structure to RouteMetadata for OpenAPI generation
fn convert_discovered_routes_to_metadata(project_structure: &ProjectStructure) -> Result<Vec<RouteMetadata>, ElifError> {
    let mut routes = Vec::new();
    
    // Always include a health check route as a fallback
    routes.push(RouteMetadata {
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
    });
    
    // Convert discovered controllers to routes
    for controller in &project_structure.controllers {
        let controller_tag = controller.name.clone();
        
        for endpoint in &controller.endpoints {
            let full_path = if let Some(base_path) = &controller.base_path {
                format!("{}{}", base_path, endpoint.path)
            } else {
                endpoint.path.clone()
            };
            
            // Convert endpoint parameters to OpenAPI parameters
            let parameters = endpoint.parameters.iter().map(|param| {
                ParameterInfo {
                    name: param.name.clone(),
                    param_type: param.param_type.clone(),
                    location: match param.source {
                        elif_openapi::endpoints::ParameterSource::Path => "path".to_string(),
                        elif_openapi::endpoints::ParameterSource::Query => "query".to_string(),
                        elif_openapi::endpoints::ParameterSource::Header => "header".to_string(),
                        elif_openapi::endpoints::ParameterSource::Body => "body".to_string(),
                        elif_openapi::endpoints::ParameterSource::Cookie => "cookie".to_string(),
                    },
                    required: !param.optional,
                    description: param.documentation.clone(),
                    example: None, // Could be enhanced later
                }
            }).collect();
            
            routes.push(RouteMetadata {
                method: endpoint.verb.clone(),
                path: full_path,
                summary: Some(format!("{} - {}", controller.name, endpoint.method)),
                description: endpoint.documentation.clone(),
                operation_id: Some(format!("{}_{}", 
                    controller.name.to_lowercase().replace(' ', "_"),
                    endpoint.method.to_lowercase().replace(' ', "_")
                )),
                tags: vec![controller_tag.clone()],
                request_schema: None, // Could be enhanced by analyzing endpoint.return_type
                response_schemas: std::collections::HashMap::new(), // Could be enhanced later
                parameters,
                security: Vec::new(), // Could be enhanced by analyzing controller.attributes
                deprecated: false,
            });
        }
    }
    
    Ok(routes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};
    use std::fs;
    use std::path::PathBuf;
    use serial_test::serial;

    /// Test utilities for OpenAPI functionality
    pub struct OpenApiTestUtils;

    impl OpenApiTestUtils {
        /// Create a temporary test project structure
        pub fn create_test_project() -> (TempDir, PathBuf) {
            let temp_dir = tempdir().expect("Failed to create temp directory");
            let project_root = temp_dir.path().to_path_buf();

            // Create basic project structure
            fs::create_dir_all(project_root.join("src/controllers")).unwrap();
            fs::create_dir_all(project_root.join("src/models")).unwrap();
            fs::create_dir_all(project_root.join("target")).unwrap();

            // Create a mock Cargo.toml
            fs::write(
                project_root.join("Cargo.toml"),
                r#"[package]
name = "test-project"
version = "1.0.0"
edition = "2021"

[dependencies]
elif-core = "0.5.0""#
            ).unwrap();

            // Create a basic controller
            fs::write(
                project_root.join("src/controllers/user_controller.rs"),
                r#"use elif_http::{Request, Response, Controller};

pub struct UserController;

impl Controller for UserController {
    /// Get all users
    pub async fn index(req: Request) -> Response {
        todo!()
    }
    
    /// Get user by ID
    pub async fn show(req: Request) -> Response {
        todo!()
    }
    
    /// Create new user
    pub async fn create(req: Request) -> Response {
        todo!()
    }
    
    /// Update existing user
    pub async fn update(req: Request) -> Response {
        todo!()
    }
    
    /// Delete user
    pub async fn destroy(req: Request) -> Response {
        todo!()
    }
}"#
            ).unwrap();

            // Create a basic model
            fs::write(
                project_root.join("src/models/user.rs"),
                r#"use elif_orm::Model;
use serde::{Serialize, Deserialize};

#[derive(Model, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: String,
}"#
            ).unwrap();

            (temp_dir, project_root)
        }

        /// Create a mock project structure for testing
        pub fn create_mock_project_structure() -> ProjectStructure {
            use elif_openapi::endpoints::{ControllerInfo, EndpointMetadata, EndpointParameter, ParameterSource};

            ProjectStructure {
                metadata: elif_openapi::discovery::ProjectMetadata {
                    name: "test-api".to_string(),
                    version: "1.0.0".to_string(),
                    description: Some("Test API".to_string()),
                    authors: vec!["Test Author".to_string()],
                },
                controllers: vec![
                    ControllerInfo::new("UserController")
                        .with_base_path("/api/users")
                        .add_endpoint(
                            EndpointMetadata::new("index", "GET", "/")
                                .with_documentation("List all users")
                        )
                        .add_endpoint(
                            EndpointMetadata::new("show", "GET", "/:id")
                                .with_documentation("Get user by ID")
                                .with_parameter(EndpointParameter {
                                    name: "id".to_string(),
                                    param_type: "i32".to_string(),
                                    source: ParameterSource::Path,
                                    optional: false,
                                    documentation: Some("User ID".to_string()),
                                })
                        )
                ],
                models: vec![
                    elif_openapi::discovery::ModelInfo {
                        name: "User".to_string(),
                        fields: vec![
                            elif_openapi::discovery::ModelField {
                                name: "id".to_string(),
                                field_type: "i32".to_string(),
                                optional: false,
                                documentation: Some("Unique user identifier".to_string()),
                            },
                            elif_openapi::discovery::ModelField {
                                name: "name".to_string(),
                                field_type: "String".to_string(),
                                optional: false,
                                documentation: Some("User's full name".to_string()),
                            },
                            elif_openapi::discovery::ModelField {
                                name: "email".to_string(),
                                field_type: "String".to_string(),
                                optional: false,
                                documentation: Some("User's email address".to_string()),
                            },
                        ],
                        documentation: Some("User model".to_string()),
                        derives: vec!["Debug".to_string(), "Clone".to_string(), "Serialize".to_string()],
                    }
                ],
            }
        }

        /// Validate that a generated OpenAPI spec contains expected routes
        pub fn validate_openapi_spec(spec_path: &str, expected_paths: &[&str]) -> bool {
            if !Path::new(spec_path).exists() {
                return false;
            }

            let content = fs::read_to_string(spec_path).unwrap();
            
            // For JSON specs
            if spec_path.ends_with(".json") {
                let spec: serde_json::Value = serde_json::from_str(&content).unwrap();
                if let Some(paths) = spec["paths"].as_object() {
                    for expected_path in expected_paths {
                        if !paths.contains_key(*expected_path) {
                            return false;
                        }
                    }
                    return true;
                }
            }
            
            // For YAML specs, do simple string matching
            if spec_path.ends_with(".yaml") || spec_path.ends_with(".yml") {
                for expected_path in expected_paths {
                    if !content.contains(expected_path) {
                        return false;
                    }
                }
                return true;
            }
            
            false
        }

        /// Clean up test files
        pub fn cleanup_test_files(files: &[&str]) {
            for file in files {
                let _ = fs::remove_file(file);
            }
            let _ = fs::remove_dir_all("target");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_openapi_generation_with_default_settings() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        // Change to project directory for the test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        // Test generation with explicit path to /tmp
        let output_path = format!("/tmp/openapi-test-{}.json", std::process::id());
        let result = generate(Some(output_path.clone()), Some("json".to_string())).await;
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "OpenAPI generation should succeed: {:?}", result);
        
        let expected_file = std::path::PathBuf::from(&output_path);
        assert!(expected_file.exists(), "JSON file should be created");
        
        // Validate the generated specification contains basic structure
        let content = fs::read_to_string(&expected_file).unwrap();
        assert!(content.contains("openapi"), "Should contain OpenAPI version");
        assert!(content.contains("/health"), "Should contain health check endpoint");
        
        // Clean up
        let _ = fs::remove_file(&expected_file);
    }

    #[tokio::test] 
    #[serial]
    async fn test_openapi_generation_yaml_format() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        let output_path = format!("/tmp/openapi-yaml-{}.yaml", std::process::id());
        let result = generate(Some(output_path.clone()), Some("yaml".to_string())).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "YAML generation should succeed");
        
        let expected_file = std::path::PathBuf::from(&output_path);
        assert!(expected_file.exists(), "YAML file should be created");
        
        let content = fs::read_to_string(&expected_file).unwrap();
        assert!(content.contains("openapi:"), "Should contain YAML OpenAPI header");
        assert!(content.contains("/health"), "Should contain health check endpoint");
        
        // Clean up
        let _ = fs::remove_file(&expected_file);
    }

    #[tokio::test]
    #[serial]
    async fn test_openapi_generation_custom_output_path() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        let custom_path = format!("/tmp/api-{}.json", std::process::id());
        let result = generate(Some(custom_path.clone()), Some("json".to_string())).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "Custom path generation should succeed");
        
        let expected_file = std::path::PathBuf::from(&custom_path);
        assert!(expected_file.exists(), "Custom path file should be created");
        
        // Clean up
        let _ = fs::remove_file(&expected_file);
    }

    #[tokio::test]
    async fn test_route_metadata_conversion() {
        let project_structure = OpenApiTestUtils::create_mock_project_structure();
        let routes = convert_discovered_routes_to_metadata(&project_structure).unwrap();
        
        // Should have health check route plus discovered routes
        assert!(routes.len() >= 3, "Should have at least health + 2 user routes");
        
        // Find the health check route
        let health_route = routes.iter().find(|r| r.path == "/health").unwrap();
        assert_eq!(health_route.method, "GET");
        assert_eq!(health_route.operation_id, Some("healthCheck".to_string()));
        
        // Find user routes
        let user_index = routes.iter().find(|r| r.path == "/api/users/").unwrap();
        assert_eq!(user_index.method, "GET");
        assert!(user_index.tags.contains(&"UserController".to_string()));
        
        let user_show = routes.iter().find(|r| r.path == "/api/users/:id").unwrap();
        assert_eq!(user_show.method, "GET");
        assert!(!user_show.parameters.is_empty(), "Show route should have ID parameter");
    }

    #[tokio::test]
    #[serial]
    async fn test_export_postman_format() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        // Generate OpenAPI spec first
        let _ = generate(None, None).await;
        
        // Test Postman export - use absolute path in /tmp
        let output_path = format!("/tmp/test-collection-{}.json", std::process::id());
        let result = export("postman".to_string(), output_path.clone()).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "Postman export should succeed");
        
        let exported_file = std::path::PathBuf::from(&output_path);
        assert!(exported_file.exists(), "Postman collection file should be created");
        
        // Validate it's valid JSON
        let content = fs::read_to_string(&exported_file).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
        
        // Clean up
        let _ = fs::remove_file(&exported_file);
    }

    #[tokio::test]
    #[serial]
    async fn test_export_insomnia_format() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        // Generate OpenAPI spec first  
        let _ = generate(None, None).await;
        
        // Test Insomnia export - use absolute path in /tmp
        let output_path = format!("/tmp/test-workspace-{}.json", std::process::id());
        let result = export("insomnia".to_string(), output_path.clone()).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "Insomnia export should succeed");
        
        let exported_file = std::path::PathBuf::from(&output_path);
        assert!(exported_file.exists(), "Insomnia workspace file should be created");
        
        // Validate it's valid JSON
        let content = fs::read_to_string(&exported_file).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
        
        // Clean up
        let _ = fs::remove_file(&exported_file);
    }

    #[tokio::test]
    #[serial]
    async fn test_export_unsupported_format() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        // Generate OpenAPI spec first
        let _ = generate(None, None).await;
        
        // Ensure spec file exists before testing unsupported format
        let spec_file = project_root.join("target/_openapi.json");
        assert!(spec_file.exists(), "OpenAPI spec should be generated for unsupported format test");
        
        // Test unsupported format - this should fail with format error, not file error
        let result = export("unsupported".to_string(), "output.json".to_string()).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err(), "Unsupported format should fail");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported export format"), "Expected 'Unsupported export format' in error: {}", error_msg);
    }

    #[tokio::test]
    #[serial]
    async fn test_swagger_ui_generation() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        // Generate OpenAPI spec first so serve can find it  
        let gen_result = generate(None, None).await;
        assert!(gen_result.is_ok(), "OpenAPI generation should succeed for swagger test");
        
        // Verify spec file exists before serving
        let spec_file = project_root.join("target/_openapi.json");
        assert!(spec_file.exists(), "OpenAPI spec should exist before serving Swagger UI");
        
        let result = serve(Some(8080)).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "Swagger UI generation should succeed: {:?}", result.as_ref().err());
        
        let swagger_file = project_root.join("target/_swagger.html");
        assert!(swagger_file.exists(), "Swagger HTML file should be created");
        
        let content = fs::read_to_string(&swagger_file).unwrap();
        assert!(content.contains("<html"), "Should be valid HTML");
        assert!(content.contains("swagger"), "Should contain Swagger UI content");
    }

    #[tokio::test]
    #[serial]
    async fn test_auto_generation_on_export() {
        let (_temp_dir, project_root) = OpenApiTestUtils::create_test_project();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        // Ensure no existing spec file to test auto-generation
        let spec_file = project_root.join("target/_openapi.json");
        let spec_yaml_file = project_root.join("target/_openapi.yaml");
        let _ = fs::remove_file(&spec_file);
        let _ = fs::remove_file(&spec_yaml_file);
        
        // Export without generating first - should auto-generate
        let output_path = format!("/tmp/auto-gen-{}.json", std::process::id());
        let result = export("postman".to_string(), output_path.clone()).await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), "Auto-generation during export should work: {:?}", result.as_ref().err());
        
        // Both OpenAPI spec and export should exist
        let export_file = std::path::PathBuf::from(&output_path);
        assert!(spec_file.exists(), "OpenAPI spec should be auto-generated");
        assert!(export_file.exists(), "Export file should be created");
        
        // Clean up
        let _ = fs::remove_file(&export_file);
    }

    #[test]
    fn test_openapi_utils() {
        // Test the utility functions
        let test_paths = vec!["/health", "/users", "/users/:id"];
        
        // These would normally be tested with real files, but we'll test the logic
        assert!(OpenApiTestUtils::validate_openapi_spec(
            "nonexistent.json", 
            &test_paths
        ) == false, "Should return false for non-existent file");
        
        // Test cleanup utility
        OpenApiTestUtils::cleanup_test_files(&["test1.json", "test2.yaml"]);
        // No assertions needed - just ensuring it doesn't panic
    }

    #[test]
    fn test_mock_project_structure() {
        let project = OpenApiTestUtils::create_mock_project_structure();
        
        assert_eq!(project.metadata.name, "test-api");
        assert_eq!(project.metadata.version, "1.0.0");
        assert!(!project.controllers.is_empty());
        assert!(!project.models.is_empty());
        
        let user_controller = &project.controllers[0];
        assert_eq!(user_controller.name, "UserController");
        assert!(user_controller.base_path.is_some());
        assert!(!user_controller.endpoints.is_empty());
    }
}