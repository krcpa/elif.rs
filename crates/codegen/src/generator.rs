use elif_core::{ElifError, ResourceSpec, FieldSpec};
use crate::templates::{render_template, MODEL_TEMPLATE, HANDLER_TEMPLATE, MIGRATION_TEMPLATE, TEST_TEMPLATE};
use crate::writer::CodeWriter;
use std::path::PathBuf;
use std::collections::HashMap;

pub struct ResourceGenerator<'a> {
    project_root: &'a PathBuf,
    spec: &'a ResourceSpec,
    writer: CodeWriter,
}

impl<'a> ResourceGenerator<'a> {
    pub fn new(project_root: &'a PathBuf, spec: &'a ResourceSpec) -> Self {
        Self {
            project_root,
            spec,
            writer: CodeWriter::new(),
        }
    }
    
    pub fn generate_model(&self) -> Result<(), ElifError> {
        let model_path = self.project_root
            .join("crates/orm/src/models")
            .join(format!("{}.rs", self.spec.name.to_lowercase()));
        
        let mut context = HashMap::new();
        context.insert("name", self.spec.name.clone());
        context.insert("table", self.spec.storage.table.clone());
        context.insert("fields", self.format_model_fields());
        
        let content = render_template(MODEL_TEMPLATE, &context)?;
        self.writer.write_if_changed(&model_path, &content)?;
        
        Ok(())
    }
    
    pub fn generate_handler(&self) -> Result<(), ElifError> {
        let handler_path = self.project_root
            .join("apps/api/src/routes")
            .join(format!("{}.rs", self.spec.name.to_lowercase()));
        
        let mut context = HashMap::new();
        context.insert("name", self.spec.name.clone());
        context.insert("route", self.spec.route.clone());
        context.insert("operations", self.format_operations());
        
        let content = render_template(HANDLER_TEMPLATE, &context)?;
        self.writer.write_preserving_markers(&handler_path, &content)?;
        
        Ok(())
    }
    
    pub fn generate_migration(&self) -> Result<(), ElifError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| ElifError::Codegen { message: format!("Time error: {}", e) })?
            .as_secs();
        
        let migration_path = self.project_root
            .join("migrations")
            .join(format!("{}_create_{}.sql", timestamp, self.spec.storage.table));
        
        let mut context = HashMap::new();
        context.insert("table", self.spec.storage.table.clone());
        context.insert("fields", self.format_migration_fields());
        context.insert("indexes", self.format_migration_indexes());
        
        let content = render_template(MIGRATION_TEMPLATE, &context)?;
        self.writer.write_if_changed(&migration_path, &content)?;
        
        Ok(())
    }
    
    pub fn generate_test(&self) -> Result<(), ElifError> {
        let test_path = self.project_root
            .join("tests")
            .join(format!("{}_http.rs", self.spec.name.to_lowercase()));
        
        let mut context = HashMap::new();
        context.insert("name", self.spec.name.clone());
        context.insert("route", self.spec.route.clone());
        
        let content = render_template(TEST_TEMPLATE, &context)?;
        self.writer.write_if_changed(&test_path, &content)?;
        
        Ok(())
    }
    
    fn format_model_fields(&self) -> String {
        self.spec.storage.fields.iter()
            .map(|field| self.format_model_field(field))
            .collect::<Vec<_>>()
            .join("\n    ")
    }
    
    fn format_model_field(&self, field: &FieldSpec) -> String {
        let rust_type = self.map_field_type(&field.field_type);
        let optional = if field.required { rust_type } else { format!("Option<{}>", rust_type) };
        
        format!("pub {}: {},", field.name, optional)
    }
    
    fn format_migration_fields(&self) -> String {
        let mut fields = self.spec.storage.fields.iter()
            .map(|field| self.format_migration_field(field))
            .collect::<Vec<_>>();
            
        if self.spec.storage.timestamps {
            fields.push("    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),".to_string());
            fields.push("    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()".to_string());
        }
        
        fields.join("\n")
    }
    
    fn format_migration_field(&self, field: &FieldSpec) -> String {
        let sql_type = self.map_field_type_to_sql(&field.field_type);
        let nullable = if field.required { "NOT NULL" } else { "" };
        let default = field.default.as_ref()
            .map(|d| format!("DEFAULT {}", d))
            .unwrap_or_default();
        let pk = if field.pk { "PRIMARY KEY" } else { "" };
        
        format!("    {} {} {} {} {},", field.name, sql_type, pk, nullable, default).trim().to_string()
    }
    
    fn format_migration_indexes(&self) -> String {
        self.spec.indexes.iter()
            .map(|idx| format!(
                "CREATE INDEX {} ON {} ({});",
                idx.name,
                self.spec.storage.table,
                idx.fields.join(", ")
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn format_operations(&self) -> String {
        self.spec.api.operations.iter()
            .map(|op| format!("{} {}", op.method, op.path))
            .collect::<Vec<_>>()
            .join(", ")
    }
    
    fn map_field_type(&self, field_type: &str) -> String {
        match field_type {
            "uuid" => "uuid::Uuid".to_string(),
            "text" | "string" => "String".to_string(),
            "bool" => "bool".to_string(),
            "int" => "i32".to_string(),
            "bigint" => "i64".to_string(),
            "float" => "f64".to_string(),
            "timestamp" => "chrono::DateTime<chrono::Utc>".to_string(),
            "json" => "serde_json::Value".to_string(),
            _ => "String".to_string(),
        }
    }
    
    fn map_field_type_to_sql(&self, field_type: &str) -> String {
        match field_type {
            "uuid" => "UUID".to_string(),
            "text" | "string" => "TEXT".to_string(),
            "bool" => "BOOLEAN".to_string(),
            "int" => "INTEGER".to_string(),
            "bigint" => "BIGINT".to_string(),
            "float" => "DOUBLE PRECISION".to_string(),
            "timestamp" => "TIMESTAMPTZ".to_string(),
            "json" => "JSONB".to_string(),
            _ => "TEXT".to_string(),
        }
    }
}