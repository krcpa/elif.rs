use super::{TemplateEngine, pluralize_word, to_snake_case, to_pascal_case};
use elif_core::{ElifError, specs::FieldSpec};
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::{json, Value};
use chrono::Utc;

pub struct ResourceGenerator {
    template_engine: TemplateEngine,
    project_root: PathBuf,
}

impl ResourceGenerator {
    pub fn new(project_root: PathBuf) -> Result<Self, ElifError> {
        Ok(Self {
            template_engine: TemplateEngine::new()?,
            project_root,
        })
    }

    pub fn generate_resource(
        &self,
        name: &str,
        fields: &[FieldSpec],
        relationships: &[ResourceRelationship],
        options: &GenerationOptions,
    ) -> Result<Vec<GeneratedFile>, ElifError> {
        let mut generated_files = Vec::new();
        let context = self.build_template_context(name, fields, relationships, options)?;

        if options.generate_model {
            let model_file = self.generate_model(&context)?;
            generated_files.push(model_file);
        }

        if options.generate_controller {
            let controller_file = self.generate_controller(&context)?;
            generated_files.push(controller_file);
        }

        if options.generate_migration {
            let migration_file = self.generate_migration(&context)?;
            generated_files.push(migration_file);
        }

        if options.generate_tests {
            let test_file = self.generate_test(&context)?;
            generated_files.push(test_file);
        }

        if options.generate_policy {
            let policy_file = self.generate_policy(&context)?;
            generated_files.push(policy_file);
        }

        if options.generate_requests {
            let request_files = self.generate_request_classes(&context)?;
            generated_files.extend(request_files);
        }

        if options.generate_resources {
            let resource_files = self.generate_resource_classes(&context)?;
            generated_files.extend(resource_files);
        }

        Ok(generated_files)
    }

    fn build_template_context(
        &self,
        name: &str,
        fields: &[FieldSpec],
        relationships: &[ResourceRelationship],
        options: &GenerationOptions,
    ) -> Result<HashMap<String, Value>, ElifError> {
        let mut context = HashMap::new();

        context.insert("name".to_string(), json!(name));
        context.insert("table_name".to_string(), json!(pluralize_word(&to_snake_case(name))));
        context.insert("snake_case_name".to_string(), json!(to_snake_case(name)));
        context.insert("pascal_case_name".to_string(), json!(to_pascal_case(name)));

        // Process fields
        let processed_fields: Vec<Value> = fields.iter().map(|field| {
            json!({
                "name": field.name,
                "field_type": self.rust_type_from_field_type(&field.field_type),
                "sql_type": self.sql_type_from_field_type(&field.field_type),
                "pk": field.pk,
                "required": field.required,
                "index": field.index,
                "default": field.default,
            })
        }).collect();

        context.insert("fields".to_string(), json!(processed_fields));

        // Check for special field types
        context.insert("has_uuid".to_string(), json!(
            fields.iter().any(|f| f.field_type == "uuid")
        ));

        // Add timestamps and soft delete flags
        context.insert("timestamps".to_string(), json!(options.timestamps));
        context.insert("soft_delete".to_string(), json!(options.soft_delete));

        // Process relationships
        let processed_relationships: Vec<Value> = relationships.iter().map(|rel| {
            json!({
                "type": rel.relationship_type,
                "related_model": rel.related_model,
                "foreign_key": rel.foreign_key,
                "pivot_table": rel.pivot_table,
            })
        }).collect();

        context.insert("relationships".to_string(), json!(processed_relationships));
        context.insert("has_relationships".to_string(), json!(!relationships.is_empty()));

        // Add generation options
        context.insert("validation".to_string(), json!(options.generate_requests));
        context.insert("auth".to_string(), json!(options.with_auth));
        context.insert("policy".to_string(), json!(options.generate_policy));
        context.insert("user_owned".to_string(), json!(
            fields.iter().any(|f| f.name == "user_id")
        ));

        // Add timestamp for migration
        context.insert("timestamp".to_string(), json!(
            Utc::now().format("%Y%m%d%H%M%S").to_string()
        ));
        context.insert("created_at".to_string(), json!(
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
        ));

        // Add has_controller flag
        context.insert("has_controller".to_string(), json!(options.generate_controller));

        Ok(context)
    }

    fn generate_model(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let content = self.template_engine.render("model", context)?;
        let name = context.get("snake_case_name").unwrap().as_str().unwrap();
        let path = self.project_root.join("src").join("models").join(format!("{}.rs", name));

        Ok(GeneratedFile {
            path,
            content,
            file_type: GeneratedFileType::Model,
        })
    }

    fn generate_controller(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let content = self.template_engine.render("controller", context)?;
        let name = context.get("snake_case_name").unwrap().as_str().unwrap();
        let path = self.project_root.join("src").join("controllers").join(format!("{}_controller.rs", name));

        Ok(GeneratedFile {
            path,
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

    fn generate_migration(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let content = self.template_engine.render("migration", context)?;
        let timestamp = context.get("timestamp").unwrap().as_str().unwrap();
        let name = context.get("snake_case_name").unwrap().as_str().unwrap();
        let filename = format!("{}_create_{}_table.sql", timestamp, pluralize_word(name));
        let path = self.project_root.join("migrations").join(filename);

        Ok(GeneratedFile {
            path,
            content,
            file_type: GeneratedFileType::Migration,
        })
    }

    fn generate_test(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let content = self.template_engine.render("test", context)?;
        let name = context.get("snake_case_name").unwrap().as_str().unwrap();
        let path = self.project_root.join("tests").join("models").join(format!("{}_test.rs", name));

        Ok(GeneratedFile {
            path,
            content,
            file_type: GeneratedFileType::Test,
        })
    }

    fn generate_policy(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let content = self.template_engine.render("policy", context)?;
        let name = context.get("snake_case_name").unwrap().as_str().unwrap();
        let path = self.project_root.join("src").join("policies").join(format!("{}_policy.rs", name));

        Ok(GeneratedFile {
            path,
            content,
            file_type: GeneratedFileType::Policy,
        })
    }

    fn generate_request_classes(&self, context: &HashMap<String, Value>) -> Result<Vec<GeneratedFile>, ElifError> {
        let mut files = Vec::new();
        let _name = context.get("pascal_case_name").unwrap().as_str().unwrap();
        let snake_name = context.get("snake_case_name").unwrap().as_str().unwrap();

        // Create request template
        let create_request_content = self.generate_create_request_content(context)?;
        let create_request_path = self.project_root
            .join("src")
            .join("requests")
            .join(format!("create_{}_request.rs", snake_name));

        files.push(GeneratedFile {
            path: create_request_path,
            content: create_request_content,
            file_type: GeneratedFileType::Request,
        });

        // Update request template
        let update_request_content = self.generate_update_request_content(context)?;
        let update_request_path = self.project_root
            .join("src")
            .join("requests")
            .join(format!("update_{}_request.rs", snake_name));

        files.push(GeneratedFile {
            path: update_request_path,
            content: update_request_content,
            file_type: GeneratedFileType::Request,
        });

        Ok(files)
    }

    fn generate_resource_classes(&self, context: &HashMap<String, Value>) -> Result<Vec<GeneratedFile>, ElifError> {
        let mut files = Vec::new();
        let _name = context.get("pascal_case_name").unwrap().as_str().unwrap();
        let snake_name = context.get("snake_case_name").unwrap().as_str().unwrap();

        // Resource template
        let resource_content = self.generate_resource_content(context)?;
        let resource_path = self.project_root
            .join("src")
            .join("resources")
            .join(format!("{}_resource.rs", snake_name));

        files.push(GeneratedFile {
            path: resource_path,
            content: resource_content,
            file_type: GeneratedFileType::Resource,
        });

        // Collection template
        let collection_content = self.generate_collection_content(context)?;
        let collection_path = self.project_root
            .join("src")
            .join("resources")
            .join(format!("{}_collection.rs", snake_name));

        files.push(GeneratedFile {
            path: collection_path,
            content: collection_content,
            file_type: GeneratedFileType::Resource,
        });

        Ok(files)
    }

    fn generate_create_request_content(&self, context: &HashMap<String, Value>) -> Result<String, ElifError> {
        let name = context.get("pascal_case_name").unwrap().as_str().unwrap();
        let fields = context.get("fields").unwrap().as_array().unwrap();

        let mut content = String::new();
        content.push_str("use serde::{Serialize, Deserialize};\n");
        content.push_str("use elif_validation::prelude::*;\n\n");
        content.push_str(&format!("#[derive(Debug, Serialize, Deserialize, Validate)]\npub struct Create{}Request {{\n", name));

        for field in fields {
            let field_obj = field.as_object().unwrap();
            let field_name = field_obj.get("name").unwrap().as_str().unwrap();
            let field_type = field_obj.get("field_type").unwrap().as_str().unwrap();
            let is_pk = field_obj.get("pk").unwrap().as_bool().unwrap_or(false);
            let is_required = field_obj.get("required").unwrap().as_bool().unwrap_or(false);

            if !is_pk && field_name != "created_at" && field_name != "updated_at" && field_name != "deleted_at" {
                if is_required {
                    content.push_str(&format!("    #[validate(required)]\n"));
                }
                if field_type == "String" {
                    content.push_str(&format!("    #[validate(length(min = 1, max = 255))]\n"));
                }
                content.push_str(&format!("    pub {}: {},\n", to_snake_case(field_name), field_type));
            }
        }

        content.push_str("}\n");
        Ok(content)
    }

    fn generate_update_request_content(&self, context: &HashMap<String, Value>) -> Result<String, ElifError> {
        let name = context.get("pascal_case_name").unwrap().as_str().unwrap();
        let fields = context.get("fields").unwrap().as_array().unwrap();

        let mut content = String::new();
        content.push_str("use serde::{Serialize, Deserialize};\n");
        content.push_str("use elif_validation::prelude::*;\n\n");
        content.push_str(&format!("#[derive(Debug, Serialize, Deserialize, Validate)]\npub struct Update{}Request {{\n", name));

        for field in fields {
            let field_obj = field.as_object().unwrap();
            let field_name = field_obj.get("name").unwrap().as_str().unwrap();
            let field_type = field_obj.get("field_type").unwrap().as_str().unwrap();
            let is_pk = field_obj.get("pk").unwrap().as_bool().unwrap_or(false);

            if !is_pk && field_name != "created_at" && field_name != "updated_at" && field_name != "deleted_at" {
                if field_type == "String" {
                    content.push_str(&format!("    #[validate(length(max = 255))]\n"));
                }
                content.push_str(&format!("    pub {}: Option<{}>,\n", to_snake_case(field_name), field_type));
            }
        }

        content.push_str("}\n");
        Ok(content)
    }

    fn generate_resource_content(&self, context: &HashMap<String, Value>) -> Result<String, ElifError> {
        let name = context.get("pascal_case_name").unwrap().as_str().unwrap();
        let snake_name = context.get("snake_case_name").unwrap().as_str().unwrap();

        let content = format!(
            "use serde::{{Serialize, Deserialize}};\nuse crate::models::{}::{};\nuse chrono::{{DateTime, Utc}};\nuse uuid::Uuid;\n\n#[derive(Debug, Serialize, Deserialize)]\npub struct {}Resource {{\n    pub id: Uuid,\n    // Add other fields as needed\n    pub created_at: DateTime<Utc>,\n    pub updated_at: DateTime<Utc>,\n}}\n\nimpl {}Resource {{\n    pub fn new({}: {}) -> Self {{\n        Self {{\n            id: {}.id,\n            created_at: {}.created_at,\n            updated_at: {}.updated_at,\n        }}\n    }}\n}}\n",
            snake_name, name, name, name, snake_name, name, snake_name, snake_name, snake_name
        );

        Ok(content)
    }

    fn generate_collection_content(&self, context: &HashMap<String, Value>) -> Result<String, ElifError> {
        let name = context.get("pascal_case_name").unwrap().as_str().unwrap();
        let snake_name = context.get("snake_case_name").unwrap().as_str().unwrap();
        let _plural_name = pluralize_word(name);

        let content = format!(
            "use serde::{{Serialize, Deserialize}};\nuse crate::models::{}::{};\nuse crate::resources::{}_resource::{}Resource;\n\n#[derive(Debug, Serialize, Deserialize)]\npub struct {}Collection {{\n    pub data: Vec<{}Resource>,\n    pub meta: CollectionMeta,\n}}\n\nimpl {}Collection {{\n    pub fn new({}: Vec<{}>) -> Self {{\n        let data = {}.into_iter()\n            .map({}Resource::new)\n            .collect();\n\n        Self {{\n            data,\n            meta: CollectionMeta {{\n                total: data.len(),\n            }},\n        }}\n    }}\n}}\n\n#[derive(Debug, Serialize, Deserialize)]\npub struct CollectionMeta {{\n    pub total: usize,\n}}\n",
            snake_name, name, snake_name, name, name, name, name, 
            pluralize_word(&to_snake_case(name)), name, pluralize_word(&to_snake_case(name)), name
        );

        Ok(content)
    }

    fn rust_type_from_field_type(&self, field_type: &str) -> String {
        match field_type {
            "uuid" => "Uuid".to_string(),
            "string" | "text" => "String".to_string(),
            "integer" => "i32".to_string(),
            "bigint" => "i64".to_string(),
            "boolean" => "bool".to_string(),
            "timestamp" => "DateTime<Utc>".to_string(),
            "json" => "serde_json::Value".to_string(),
            _ => field_type.to_string(),
        }
    }

    fn sql_type_from_field_type(&self, field_type: &str) -> String {
        match field_type {
            "uuid" => "UUID".to_string(),
            "string" => "VARCHAR(255)".to_string(),
            "text" => "TEXT".to_string(),
            "integer" => "INTEGER".to_string(),
            "bigint" => "BIGINT".to_string(),
            "boolean" => "BOOLEAN".to_string(),
            "timestamp" => "TIMESTAMPTZ".to_string(),
            "json" => "JSONB".to_string(),
            _ => field_type.to_uppercase(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceRelationship {
    pub relationship_type: String, // "belongs_to", "has_one", "has_many", "belongs_to_many"
    pub related_model: String,
    pub foreign_key: Option<String>,
    pub pivot_table: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GenerationOptions {
    pub generate_model: bool,
    pub generate_controller: bool,
    pub generate_migration: bool,
    pub generate_tests: bool,
    pub generate_policy: bool,
    pub generate_requests: bool,
    pub generate_resources: bool,
    pub with_auth: bool,
    pub timestamps: bool,
    pub soft_delete: bool,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            generate_model: true,
            generate_controller: true,
            generate_migration: true,
            generate_tests: false,
            generate_policy: false,
            generate_requests: false,
            generate_resources: false,
            with_auth: false,
            timestamps: true,
            soft_delete: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
    #[allow(dead_code)]
    pub file_type: GeneratedFileType,
}

#[derive(Debug, Clone)]
pub enum GeneratedFileType {
    Model,
    Controller,
    Migration,
    Test,
    Policy,
    Request,
    Resource,
}