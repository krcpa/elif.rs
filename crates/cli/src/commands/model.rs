use elif_core::ElifError;
use std::fs;

pub async fn add_model(name: &str, fields_str: &str) -> Result<(), ElifError> {
    println!("📦 Adding model: {} with fields: {}", name, fields_str);
    
    // Parse fields
    let fields = parse_fields(fields_str)?;
    
    // Create model file
    create_model_file(name, &fields).await?;
    
    // Create migration
    create_migration(name, &fields).await?;
    
    // Update models/mod.rs
    update_models_mod(name)?;
    
    println!("✅ Model '{}' created successfully!", name);
    println!("📝 Model: src/models/{}.rs", name.to_lowercase());
    println!("📊 Migration: migrations/TIMESTAMP_create_{}.sql", name.to_lowercase());
    
    Ok(())
}

#[derive(Debug)]
struct Field {
    name: String,
    field_type: String,
    rust_type: String,
    sql_type: String,
    required: bool,
}

fn parse_fields(fields_str: &str) -> Result<Vec<Field>, ElifError> {
    let mut fields = vec![
        Field {
            name: "id".to_string(),
            field_type: "uuid".to_string(),
            rust_type: "Uuid".to_string(),
            sql_type: "UUID PRIMARY KEY DEFAULT gen_random_uuid()".to_string(),
            required: true,
        }
    ];
    
    for field_def in fields_str.split(',') {
        let parts: Vec<&str> = field_def.trim().split(':').collect();
        if parts.len() != 2 {
            return Err(ElifError::Validation(
                format!("Invalid field definition: {}. Expected name:type format", field_def)
            ));
        }
        
        let field_name = parts[0].trim();
        let field_type = parts[1].trim();
        
        let (rust_type, sql_type) = map_field_types(field_type);
        
        fields.push(Field {
            name: field_name.to_string(),
            field_type: field_type.to_string(),
            rust_type,
            sql_type: format!("{} NOT NULL", sql_type),
            required: true,
        });
    }
    
    // Add timestamps
    fields.push(Field {
        name: "created_at".to_string(),
        field_type: "timestamp".to_string(), 
        rust_type: "DateTime<Utc>".to_string(),
        sql_type: "TIMESTAMPTZ NOT NULL DEFAULT NOW()".to_string(),
        required: true,
    });
    
    fields.push(Field {
        name: "updated_at".to_string(),
        field_type: "timestamp".to_string(),
        rust_type: "DateTime<Utc>".to_string(), 
        sql_type: "TIMESTAMPTZ NOT NULL DEFAULT NOW()".to_string(),
        required: true,
    });
    
    Ok(fields)
}

fn map_field_types(field_type: &str) -> (String, String) {
    match field_type {
        "string" | "text" => ("String".to_string(), "TEXT".to_string()),
        "int" | "integer" => ("i32".to_string(), "INTEGER".to_string()),
        "bigint" => ("i64".to_string(), "BIGINT".to_string()),
        "bool" | "boolean" => ("bool".to_string(), "BOOLEAN".to_string()),
        "float" | "decimal" => ("f64".to_string(), "DOUBLE PRECISION".to_string()),
        "uuid" => ("Uuid".to_string(), "UUID".to_string()),
        "timestamp" | "datetime" => ("DateTime<Utc>".to_string(), "TIMESTAMPTZ".to_string()),
        "json" => ("Value".to_string(), "JSONB".to_string()),
        _ => ("String".to_string(), "TEXT".to_string()), // Default to string
    }
}

async fn create_model_file(name: &str, fields: &Vec<Field>) -> Result<(), ElifError> {
    let model_path = format!("src/models/{}.rs", name.to_lowercase());
    
    let rust_fields = fields.iter()
        .map(|f| {
            if f.name == "id" {
                format!("    pub {}: {},", f.name, f.rust_type)
            } else {
                format!("    pub {}: {},", f.name, f.rust_type)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    let model_content = format!(r#"use serde::{{Deserialize, Serialize}};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{{DateTime, Utc}};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {} {{
{}
}}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Create{} {{
{}
}}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update{} {{
{}
}}

impl {} {{
    // <<<ELIF:BEGIN agent-editable:{}_methods>>>
    // Add your model methods here
    pub fn new() -> Self {{
        Self {{
            id: Uuid::new_v4(),
            // Initialize other fields as needed
        }}
    }}
    // <<<ELIF:END agent-editable:{}_methods>>>
}}
"#, 
        name, rust_fields,
        name, rust_fields, 
        name, rust_fields,
        name, name.to_lowercase(), name.to_lowercase()
    );
    
    fs::write(&model_path, model_content)?;
    
    Ok(())
}

async fn create_migration(name: &str, fields: &Vec<Field>) -> Result<(), ElifError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| ElifError::Codegen(format!("Time error: {}", e)))?
        .as_secs();
    
    let migration_path = format!("migrations/{}_create_{}.sql", timestamp, name.to_lowercase());
    
    let sql_fields = fields.iter()
        .map(|f| format!("    {} {},", f.name, f.sql_type))
        .collect::<Vec<_>>()
        .join("\n");
    
    let migration_content = format!(r#"CREATE TABLE {} (
{}
);

-- Add indexes as needed
-- CREATE INDEX idx_{}_{} ON {} ({});
"#, 
        name.to_lowercase(), 
        sql_fields.trim_end_matches(','),
        name.to_lowercase(), 
        "field_name",
        name.to_lowercase(),
        "field_name"
    );
    
    fs::create_dir_all("migrations")?;
    fs::write(&migration_path, migration_content)?;
    
    Ok(())
}

fn update_models_mod(name: &str) -> Result<(), ElifError> {
    let mod_path = "src/models/mod.rs";
    let mut content = fs::read_to_string(mod_path)?;
    
    let mod_declaration = format!("pub mod {};\npub use {}::*;", 
                                 name.to_lowercase(), name.to_lowercase());
    
    if !content.contains(&format!("pub mod {};", name.to_lowercase())) {
        content = format!("{}\n{}", content.trim(), mod_declaration);
        fs::write(mod_path, content)?;
    }
    
    Ok(())
}