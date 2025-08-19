use elif_core::{ElifError, specs::FieldSpec};
use crate::generators::{
    resource_generator::{ResourceGenerator, ResourceRelationship, GenerationOptions, GeneratedFile},
    auth_generator::{AuthGenerator, AuthOptions},
    api_generator::{ApiGenerator, ApiOptions, ApiResource},
};

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

pub async fn api(
    version: &str,
    resources_str: &str,
    openapi: bool,
    versioning: bool,
) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    let generator = ApiGenerator::new(project_root.clone());
    
    // Parse resources
    let resource_names: Vec<&str> = resources_str.split(',').map(|s| s.trim()).collect();
    let resources: Vec<ApiResource> = resource_names.iter().map(|name| {
        ApiResource {
            name: name.to_string(),
            endpoints: vec![], // Standard CRUD endpoints will be generated
        }
    }).collect();
    
    let options = ApiOptions {
        version: version.to_string(),
        prefix: "api".to_string(),
        with_openapi: openapi,
        with_versioning: versioning,
    };
    
    let generated_files = generator.generate_api(&resources, &options)?;
    
    // Write generated files
    write_generated_files(&generated_files)?;
    
    println!("✓ Generated API {} with {} files", version, generated_files.len());
    
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