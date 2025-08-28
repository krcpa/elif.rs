use elif_core::{ElifError, specs::FieldSpec};
use crate::generators::{
    resource_generator::{ResourceGenerator, ResourceRelationship, GenerationOptions, GeneratedFile},
    auth_generator::{AuthGenerator, AuthOptions},
    api_generator::{ApiGenerator, ApiOptions, ApiResource},
    project_analyzer::ProjectAnalyzer,
};

#[allow(dead_code)]
pub async fn resource(
    name: &str,
    fields_str: &str,
    relationships_str: Option<&str>,
    api: bool,
    tests: bool,
    policy: bool,
    requests: bool,
    resources: bool,
    auth: bool,
    soft_delete: bool,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    let generator = ResourceGenerator::new(project_root.clone())?;
    
    // Parse fields
    let fields = parse_fields(fields_str)?;
    
    // Parse relationships
    let relationships = if let Some(rel_str) = relationships_str {
        parse_relationships(rel_str)?
    } else {
        vec![]
    };
    
    // Set up generation options
    let options = GenerationOptions {
        generate_model: true,
        generate_controller: true,
        generate_migration: true,
        generate_tests: tests,
        generate_policy: policy,
        generate_requests: requests,
        generate_resources: resources,
        with_auth: auth,
        timestamps: true,
        soft_delete,
    };
    
    // Generate all files
    let generated_files = generator.generate_resource(name, &fields, &relationships, &options)?;
    
    // Write generated files
    write_generated_files(&generated_files)?;
    
    println!("✓ Generated {} for {} with {} files", 
        if api { "API resource" } else { "resource" }, 
        name, 
        generated_files.len()
    );
    
    // Show generated files
    for file in &generated_files {
        println!("  - {}", file.path.display());
    }
    
    Ok(())
}

#[allow(dead_code)]
pub async fn auth(
    jwt: bool,
    session: bool,
    mfa: bool,
    password_reset: bool,
    registration: bool,
    rbac: bool,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    let generator = AuthGenerator::new(project_root.clone())?;
    
    let options = AuthOptions {
        jwt,
        session,
        mfa,
        password_reset,
        registration,
        rbac,
    };
    
    let generated_files = generator.generate_auth_system(&options)?;
    
    // Write generated files
    write_generated_files(&generated_files)?;
    
    println!("✓ Generated authentication system with {} files", generated_files.len());
    
    // Show generated files
    for file in &generated_files {
        println!("  - {}", file.path.display());
    }
    
    Ok(())
}


fn parse_fields(fields_str: &str) -> Result<Vec<FieldSpec>, ElifError> {
    let mut fields = vec![
        FieldSpec {
            name: "id".to_string(),
            field_type: "uuid".to_string(),
            pk: true,
            required: true,
            index: false,
            default: Some("gen_random_uuid()".to_string()),
            validate: None,
        }
    ];
    
    for field_def in fields_str.split(',') {
        let parts: Vec<&str> = field_def.trim().split(':').collect();
        if parts.len() != 2 {
            return Err(ElifError::Validation { message: format!("Invalid field definition: {}. Expected name:type format", field_def) });
        }
        
        fields.push(FieldSpec {
            name: parts[0].trim().to_string(),
            field_type: parts[1].trim().to_string(),
            pk: false,
            required: true,
            index: false,
            default: None,
            validate: None,
        });
    }
    
    Ok(fields)
}

fn parse_relationships(relationships_str: &str) -> Result<Vec<ResourceRelationship>, ElifError> {
    let mut relationships = Vec::new();
    
    for rel_def in relationships_str.split(',') {
        let parts: Vec<&str> = rel_def.trim().split(':').collect();
        if parts.len() != 2 {
            return Err(ElifError::Validation { message: format!("Invalid relationship definition: {}. Expected name:type format", rel_def) });
        }
        
        let related_model = parts[0].trim().to_string();
        let rel_type = parts[1].trim().to_string();
        
        // Validate relationship type
        match rel_type.as_str() {
            "belongs_to" | "has_one" | "has_many" | "belongs_to_many" => {},
            _ => {
                return Err(ElifError::Validation { message: format!("Invalid relationship type: {}. Valid types: belongs_to, has_one, has_many, belongs_to_many", rel_type) });
            }
        }
        
        relationships.push(ResourceRelationship {
            relationship_type: rel_type,
            related_model,
            foreign_key: None, // Will be inferred
            pivot_table: None, // Will be inferred for belongs_to_many
        });
    }
    
    Ok(relationships)
}

fn write_generated_files(files: &[GeneratedFile]) -> Result<(), ElifError> {
    for file in files {
        // Create directory if it doesn't exist
        if let Some(parent) = file.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ElifError::Io(e))?;
        }
        
        // Write file content
        std::fs::write(&file.path, &file.content)
            .map_err(|e| ElifError::Io(e))?;
    }
    
    Ok(())
}

pub async fn api(
    resource: &str,
    version: &str,
    _module: Option<&str>,
    _auth: bool,
    _validation: bool,
    docs: bool,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    let generator = ApiGenerator::new(project_root.clone());
    
    // Create API resource for single resource
    let api_resource = ApiResource {
        name: resource.to_string(),
        endpoints: vec![], // Standard CRUD endpoints will be generated
    };
    
    let options = ApiOptions {
        version: version.to_string(),
        prefix: "api".to_string(),
        with_openapi: docs,
        with_versioning: true,
    };
    
    let generated_files = generator.generate_api(&[api_resource], &options)?;
    
    // Write generated files
    write_generated_files(&generated_files)?;
    
    println!("✓ Generated API {} for {} with {} files", version, resource, generated_files.len());
    
    // Show generated files
    for file in &generated_files {
        println!("  - {}", file.path.display());
    }
    
    Ok(())
}

pub async fn crud(
    resource: &str,
    fields: Option<&str>,
    relationships: Option<&str>,
    _module: Option<&str>,
    migration: bool,
    tests: bool,
    factory: bool,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    let generator = ResourceGenerator::new(project_root.clone())?;
    
    // Parse fields - default to minimal if not provided
    let fields = if let Some(fields_str) = fields {
        parse_fields(fields_str)?
    } else {
        vec![
            FieldSpec {
                name: "id".to_string(),
                field_type: "uuid".to_string(),
                pk: true,
                required: true,
                index: false,
                default: Some("gen_random_uuid()".to_string()),
                validate: None,
            },
            FieldSpec {
                name: "name".to_string(),
                field_type: "string".to_string(),
                pk: false,
                required: true,
                index: false,
                default: None,
                validate: None,
            },
        ]
    };
    
    // Parse relationships
    let relationships = if let Some(rel_str) = relationships {
        parse_relationships(rel_str)?
    } else {
        vec![]
    };
    
    // Set up generation options
    let options = GenerationOptions {
        generate_model: true,
        generate_controller: true,
        generate_migration: migration,
        generate_tests: tests,
        generate_policy: true,
        generate_requests: true,
        generate_resources: true,
        with_auth: false,
        timestamps: true,
        soft_delete: true,
    };
    
    // Generate all files
    let generated_files = generator.generate_resource(resource, &fields, &relationships, &options)?;
    
    // Write generated files
    write_generated_files(&generated_files)?;
    
    println!("✓ Generated CRUD system for {} with {} files", resource, generated_files.len());
    
    // Show generated files
    for file in &generated_files {
        println!("  - {}", file.path.display());
    }
    
    // Generate factory if requested
    if factory {
        factory_for_model(resource, 10, None, None).await?;
    }
    
    Ok(())
}

pub async fn service(
    name: &str,
    module: Option<&str>,
    trait_impl: Option<&str>,
    dependencies: Option<&str>,
    async_methods: bool,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    // Use project analyzer to understand the current structure
    let analyzer = ProjectAnalyzer::new(project_root.clone());
    let project_structure = analyzer.analyze_project_structure()?;
    
    // Parse service name to generate clean names
    let service_name = if name.ends_with("Service") {
        name.to_string()
    } else {
        format!("{}Service", name)
    };
    
    // Parse dependencies
    let deps: Vec<String> = if let Some(deps_str) = dependencies {
        deps_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };
    
    // Determine target module - use provided module or suggest based on project structure
    let target_module = if let Some(module_name) = module {
        Some(module_name.to_string())
    } else if !project_structure.modules.is_empty() {
        // Try to suggest a module based on service name
        analyzer.suggest_module_for_resource(&name.replace("Service", ""))?
    } else {
        None
    };
    
    // Generate service file path
    let module_path = if let Some(module_name) = &target_module {
        format!("src/modules/{}/services", module_name.to_lowercase())
    } else {
        "src/services".to_string()
    };
    
    let service_file_path = project_root.join(&module_path).join(format!("{}.rs", service_name.to_lowercase()));
    
    // Create directory if it doesn't exist
    if let Some(parent) = service_file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ElifError::Io(e))?;
    }
    
    // Generate service content with project context
    let mut context = analyzer.generate_project_context()?;
    context.insert("service_name".to_string(), serde_json::to_value(&service_name)?);
    context.insert("dependencies".to_string(), serde_json::to_value(&deps)?);
    context.insert("async_methods".to_string(), serde_json::to_value(async_methods)?);
    context.insert("trait_impl".to_string(), serde_json::to_value(trait_impl)?);
    
    let service_content = generate_enhanced_service_content(&service_name, trait_impl, &deps, async_methods, &context);
    
    // Write service file
    std::fs::write(&service_file_path, service_content)
        .map_err(|e| ElifError::Io(e))?;
    
    if let Some(ref module_name) = target_module {
        println!("✓ Generated service {} in module {} at {}", service_name, module_name, service_file_path.display());
        update_module_services(&project_root, module_name, &service_name).await?;
    } else {
        println!("✓ Generated service {} at {}", service_name, service_file_path.display());
    }
    
    Ok(())
}

pub async fn factory(
    model: &str,
    count: u32,
    relationships: Option<&str>,
    traits: Option<&str>,
) -> Result<(), ElifError> {
    factory_for_model(model, count, relationships, traits).await
}

async fn factory_for_model(
    model: &str,
    count: u32,
    relationships: Option<&str>,
    traits: Option<&str>,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;
    
    // Generate factory file path
    let factory_file_path = project_root
        .join("src/database/factories")
        .join(format!("{}_factory.rs", model.to_lowercase()));
    
    // Create directory if it doesn't exist
    if let Some(parent) = factory_file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ElifError::Io(e))?;
    }
    
    // Parse relationships
    let rels: Vec<String> = if let Some(rel_str) = relationships {
        rel_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };
    
    // Parse traits
    let factory_traits: Vec<String> = if let Some(traits_str) = traits {
        traits_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec!["Faker".to_string()]
    };
    
    // Generate factory content
    let factory_content = generate_factory_content(model, count, &rels, &factory_traits);
    
    // Write factory file
    std::fs::write(&factory_file_path, factory_content)
        .map_err(|e| ElifError::Io(e))?;
    
    println!("✓ Generated factory for {} at {}", model, factory_file_path.display());
    
    Ok(())
}

#[allow(dead_code)]
fn generate_service_content(
    name: &str,
    trait_impl: Option<&str>,
    dependencies: &[String],
    async_methods: bool,
) -> String {
    let mut content = String::new();
    
    // Imports
    content.push_str("use std::sync::Arc;\n");
    content.push_str("use elif_core::{ElifResult, ElifError};\n");
    content.push_str("use async_trait::async_trait;\n");
    
    if !dependencies.is_empty() {
        content.push('\n');
        for dep in dependencies {
            content.push_str(&format!("use crate::services::{};\n", dep));
        }
    }
    
    content.push('\n');
    
    // Trait definition if specified
    if let Some(trait_name) = trait_impl {
        let async_keyword = if async_methods { "#[async_trait]" } else { "" };
        content.push_str(&format!("{}\n", async_keyword));
        content.push_str(&format!("pub trait {} {{\n", trait_name));
        
        if async_methods {
            content.push_str("    async fn execute(&self) -> ElifResult<()>;\n");
        } else {
            content.push_str("    fn execute(&self) -> ElifResult<()>;\n");
        }
        
        content.push_str("}\n\n");
    }
    
    // Service struct
    content.push_str("// <<<ELIF:BEGIN agent-editable:service-fields>>>\n");
    content.push_str("#[derive(Debug, Clone)]\n");
    content.push_str(&format!("pub struct {} {{\n", name));
    
    for dep in dependencies {
        content.push_str(&format!("    {}: Arc<{}>,\n", dep.to_lowercase(), dep));
    }
    
    content.push_str("}\n");
    content.push_str("// <<<ELIF:END agent-editable:service-fields>>>\n\n");
    
    // Implementation
    content.push_str(&format!("impl {} {{\n", name));
    
    // Constructor
    content.push_str("    pub fn new(");
    for (i, dep) in dependencies.iter().enumerate() {
        if i > 0 {
            content.push_str(", ");
        }
        content.push_str(&format!("{}: Arc<{}>", dep.to_lowercase(), dep));
    }
    content.push_str(") -> Self {\n");
    content.push_str("        Self {\n");
    
    for dep in dependencies {
        content.push_str(&format!("            {},\n", dep.to_lowercase()));
    }
    
    content.push_str("        }\n    }\n\n");
    
    // Methods section
    content.push_str("    // <<<ELIF:BEGIN agent-editable:service-methods>>>\n");
    
    if async_methods {
        content.push_str("    pub async fn perform_operation(&self) -> ElifResult<()> {\n");
        content.push_str("        // TODO: Implement service logic\n");
        content.push_str("        Ok(())\n");
        content.push_str("    }\n");
    } else {
        content.push_str("    pub fn perform_operation(&self) -> ElifResult<()> {\n");
        content.push_str("        // TODO: Implement service logic\n");
        content.push_str("        Ok(())\n");
        content.push_str("    }\n");
    }
    
    content.push_str("    // <<<ELIF:END agent-editable:service-methods>>>\n");
    content.push_str("}\n\n");
    
    // Trait implementation if specified
    if let Some(trait_name) = trait_impl {
        let async_keyword = if async_methods { "#[async_trait]" } else { "" };
        content.push_str(&format!("{}\n", async_keyword));
        content.push_str(&format!("impl {} for {} {{\n", trait_name, name));
        
        if async_methods {
            content.push_str("    async fn execute(&self) -> ElifResult<()> {\n");
            content.push_str("        self.perform_operation().await\n");
        } else {
            content.push_str("    fn execute(&self) -> ElifResult<()> {\n");
            content.push_str("        self.perform_operation()\n");
        }
        
        content.push_str("    }\n");
        content.push_str("}\n");
    }
    
    content
}

fn generate_enhanced_service_content(
    name: &str,
    trait_impl: Option<&str>,
    dependencies: &[String],
    async_methods: bool,
    context: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    let mut content = String::new();
    
    // Enhanced imports based on project structure
    content.push_str("use std::sync::Arc;\n");
    content.push_str("use elif_core::{ElifResult, ElifError};\n");
    content.push_str("use async_trait::async_trait;\n");
    content.push_str("use tracing::{debug, info, warn, error};\n");
    
    // Check if project has database models
    if let Some(models) = context.get("models") {
        if let Some(models_array) = models.as_array() {
            if !models_array.is_empty() {
                content.push_str("use crate::models::*;\n");
            }
        }
    }
    
    // Add module-specific imports
    if let Some(modules) = context.get("modules") {
        if let Some(modules_obj) = modules.as_object() {
            if !modules_obj.is_empty() {
                content.push_str("// Module imports\n");
                for (module_name, _) in modules_obj {
                    content.push_str(&format!("// use crate::modules::{}::*;\n", module_name));
                }
            }
        }
    }
    
    if !dependencies.is_empty() {
        content.push('\n');
        for dep in dependencies {
            content.push_str(&format!("use crate::services::{};\n", dep));
        }
    }
    
    content.push('\n');
    
    // Enhanced trait definition if specified
    if let Some(trait_name) = trait_impl {
        let async_keyword = if async_methods { "#[async_trait]" } else { "" };
        content.push_str(&format!("{}\n", async_keyword));
        content.push_str(&format!("pub trait {} {{\n", trait_name));
        
        if async_methods {
            content.push_str("    /// Execute the main operation of this service\n");
            content.push_str("    async fn execute(&self) -> ElifResult<()>;\n");
            content.push_str("    \n");
            content.push_str("    /// Health check for this service\n");
            content.push_str("    async fn health_check(&self) -> ElifResult<bool> {\n");
            content.push_str("        Ok(true)\n");
            content.push_str("    }\n");
        } else {
            content.push_str("    /// Execute the main operation of this service\n");
            content.push_str("    fn execute(&self) -> ElifResult<()>;\n");
            content.push_str("    \n");
            content.push_str("    /// Health check for this service\n");
            content.push_str("    fn health_check(&self) -> ElifResult<bool> {\n");
            content.push_str("        Ok(true)\n");
            content.push_str("    }\n");
        }
        
        content.push_str("}\n\n");
    }
    
    // Enhanced service struct with documentation
    content.push_str("// <<<ELIF:BEGIN agent-editable:service-fields>>>\n");
    content.push_str(&format!("/// {} provides business logic operations\n", name));
    content.push_str("///\n");
    content.push_str("/// This service is auto-generated and should be customized\n");
    content.push_str("/// for your specific business requirements.\n");
    content.push_str("#[derive(Debug, Clone)]\n");
    content.push_str(&format!("pub struct {} {{\n", name));
    
    for dep in dependencies {
        content.push_str(&format!("    /// Dependency: {}\n", dep));
        content.push_str(&format!("    {}: Arc<{}>,\n", dep.to_lowercase(), dep));
    }
    
    content.push_str("}\n");
    content.push_str("// <<<ELIF:END agent-editable:service-fields>>>\n\n");
    
    // Enhanced implementation with better error handling and logging
    content.push_str(&format!("impl {} {{\n", name));
    
    // Enhanced constructor
    content.push_str("    /// Creates a new instance of this service\n");
    content.push_str("    pub fn new(");
    for (i, dep) in dependencies.iter().enumerate() {
        if i > 0 {
            content.push_str(", ");
        }
        content.push_str(&format!("{}: Arc<{}>", dep.to_lowercase(), dep));
    }
    content.push_str(") -> Self {\n");
    content.push_str(&format!("        debug!(\"Creating new {} instance\");\n", name));
    content.push_str("        Self {\n");
    
    for dep in dependencies {
        content.push_str(&format!("            {},\n", dep.to_lowercase()));
    }
    
    content.push_str("        }\n    }\n\n");
    
    // Methods section with enhanced functionality
    content.push_str("    // <<<ELIF:BEGIN agent-editable:service-methods>>>\n");
    
    if async_methods {
        content.push_str("    /// Performs the main operation of this service\n");
        content.push_str("    pub async fn perform_operation(&self) -> ElifResult<()> {\n");
        content.push_str(&format!("        info!(\"Executing {} operation\");\n", name));
        content.push_str("        \n");
        content.push_str("        // TODO: Implement service logic here\n");
        content.push_str("        // Example: database operations, external API calls, etc.\n");
        content.push_str("        \n");
        content.push_str(&format!("        debug!(\"Completed {} operation successfully\");\n", name));
        content.push_str("        Ok(())\n");
        content.push_str("    }\n\n");
        
        // Add validation method
        content.push_str("    /// Validates input data for this service\n");
        content.push_str("    pub async fn validate_input(&self, _data: &str) -> ElifResult<bool> {\n");
        content.push_str("        // TODO: Add input validation logic\n");
        content.push_str("        Ok(true)\n");
        content.push_str("    }\n");
    } else {
        content.push_str("    /// Performs the main operation of this service\n");
        content.push_str("    pub fn perform_operation(&self) -> ElifResult<()> {\n");
        content.push_str(&format!("        info!(\"Executing {} operation\");\n", name));
        content.push_str("        \n");
        content.push_str("        // TODO: Implement service logic here\n");
        content.push_str("        // Example: database operations, data processing, etc.\n");
        content.push_str("        \n");
        content.push_str(&format!("        debug!(\"Completed {} operation successfully\");\n", name));
        content.push_str("        Ok(())\n");
        content.push_str("    }\n\n");
        
        // Add validation method
        content.push_str("    /// Validates input data for this service\n");
        content.push_str("    pub fn validate_input(&self, _data: &str) -> ElifResult<bool> {\n");
        content.push_str("        // TODO: Add input validation logic\n");
        content.push_str("        Ok(true)\n");
        content.push_str("    }\n");
    }
    
    content.push_str("    // <<<ELIF:END agent-editable:service-methods>>>\n");
    content.push_str("}\n\n");
    
    // Enhanced trait implementation with better error handling
    if let Some(trait_name) = trait_impl {
        let async_keyword = if async_methods { "#[async_trait]" } else { "" };
        content.push_str(&format!("{}\n", async_keyword));
        content.push_str(&format!("impl {} for {} {{\n", trait_name, name));
        
        if async_methods {
            content.push_str("    async fn execute(&self) -> ElifResult<()> {\n");
            content.push_str("        self.perform_operation().await\n");
            content.push_str("    }\n\n");
            content.push_str("    async fn health_check(&self) -> ElifResult<bool> {\n");
            content.push_str("        // TODO: Implement actual health check logic\n");
            content.push_str("        Ok(true)\n");
            content.push_str("    }\n");
        } else {
            content.push_str("    fn execute(&self) -> ElifResult<()> {\n");
            content.push_str("        self.perform_operation()\n");
            content.push_str("    }\n\n");
            content.push_str("    fn health_check(&self) -> ElifResult<bool> {\n");
            content.push_str("        // TODO: Implement actual health check logic\n");
            content.push_str("        Ok(true)\n");
            content.push_str("    }\n");
        }
        
        content.push_str("}\n\n");
    }
    
    // Add tests module
    content.push_str("#[cfg(test)]\n");
    content.push_str("mod tests {\n");
    content.push_str("    use super::*;\n");
    content.push_str("    \n");
    content.push_str("    #[tokio::test]\n");
    content.push_str(&format!("    async fn test_{}_creation() {{\n", name.to_lowercase()));
    content.push_str(&format!("        let service = {}::new(", name));
    for (i, dep) in dependencies.iter().enumerate() {
        if i > 0 {
            content.push_str(", ");
        }
        content.push_str(&format!("Arc::new({}::default())", dep));
    }
    content.push_str(");\n");
    content.push_str("        // Add assertions here\n");
    content.push_str("    }\n");
    content.push_str("}\n");
    
    content
}

fn generate_factory_content(
    model: &str,
    count: u32,
    relationships: &[String],
    traits: &[String],
) -> String {
    let mut content = String::new();
    
    // Imports
    content.push_str("use fake::{Fake, Faker};\n");
    content.push_str("use serde::{Deserialize, Serialize};\n");
    content.push_str("use uuid::Uuid;\n");
    content.push_str(&format!("use crate::models::{};\n", model));
    
    if !relationships.is_empty() {
        for rel in relationships {
            content.push_str(&format!("use crate::models::{};\n", rel));
        }
    }
    
    content.push('\n');
    
    // Factory struct
    content.push_str("#[derive(Debug, Clone)]\n");
    content.push_str(&format!("pub struct {}Factory {{\n", model));
    content.push_str(&format!("    pub count: u32,\n"));
    
    for trait_name in traits {
        if trait_name != "Faker" {
            content.push_str(&format!("    pub {}: bool,\n", trait_name.to_lowercase()));
        }
    }
    
    content.push_str("}\n\n");
    
    // Factory implementation
    content.push_str(&format!("impl {}Factory {{\n", model));
    
    // Constructor
    content.push_str("    pub fn new() -> Self {\n");
    content.push_str("        Self {\n");
    content.push_str(&format!("            count: {},\n", count));
    
    for trait_name in traits {
        if trait_name != "Faker" {
            content.push_str(&format!("            {}: false,\n", trait_name.to_lowercase()));
        }
    }
    
    content.push_str("        }\n");
    content.push_str("    }\n\n");
    
    // Count setter
    content.push_str("    pub fn count(mut self, count: u32) -> Self {\n");
    content.push_str("        self.count = count;\n");
    content.push_str("        self\n");
    content.push_str("    }\n\n");
    
    // Trait setters
    for trait_name in traits {
        if trait_name != "Faker" {
            content.push_str(&format!("    pub fn {}(mut self) -> Self {{\n", trait_name.to_lowercase()));
            content.push_str(&format!("        self.{} = true;\n", trait_name.to_lowercase()));
            content.push_str("        self\n");
            content.push_str("    }\n\n");
        }
    }
    
    // Generate method
    content.push_str("    // <<<ELIF:BEGIN agent-editable:factory-generation>>>\n");
    content.push_str(&format!("    pub fn make(&self) -> {} {{\n", model));
    content.push_str(&format!("        {} {{\n", model));
    content.push_str("            id: Uuid::new_v4(),\n");
    content.push_str("            name: Faker.fake(),\n");
    content.push_str("            created_at: chrono::Utc::now(),\n");
    content.push_str("            updated_at: chrono::Utc::now(),\n");
    content.push_str("        }\n");
    content.push_str("    }\n\n");
    
    content.push_str(&format!("    pub fn make_many(&self, count: u32) -> Vec<{}> {{\n", model));
    content.push_str("        (0..count).map(|_| self.make()).collect()\n");
    content.push_str("    }\n");
    content.push_str("    // <<<ELIF:END agent-editable:factory-generation>>>\n");
    
    content.push_str("}\n\n");
    
    // Default implementation
    content.push_str(&format!("impl Default for {}Factory {{\n", model));
    content.push_str("    fn default() -> Self {\n");
    content.push_str("        Self::new()\n");
    content.push_str("    }\n");
    content.push_str("}\n");
    
    content
}

async fn update_module_services(
    project_root: &std::path::Path,
    module_name: &str,
    service_name: &str,
) -> Result<(), ElifError> {
    let mod_file_path = project_root
        .join("src/modules")
        .join(module_name.to_lowercase())
        .join("services")
        .join("mod.rs");
    
    // Read existing mod.rs or create if it doesn't exist
    let mut content = if mod_file_path.exists() {
        std::fs::read_to_string(&mod_file_path)
            .map_err(|e| ElifError::Io(e))?
    } else {
        String::new()
    };
    
    // Add module declaration if not already present
    let module_line = format!("pub mod {};", service_name.to_lowercase());
    if !content.contains(&module_line) {
        content.push_str(&format!("\n{}\n", module_line));
    }
    
    // Add re-export if not already present
    let export_line = format!("pub use {}::{};", service_name.to_lowercase(), service_name);
    if !content.contains(&export_line) {
        content.push_str(&format!("{}\n", export_line));
    }
    
    // Create directory if it doesn't exist
    if let Some(parent) = mod_file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ElifError::Io(e))?;
    }
    
    // Write updated mod.rs
    std::fs::write(&mod_file_path, content.trim())
        .map_err(|e| ElifError::Io(e))?;
    
    Ok(())
}