pub mod resource_generator;
pub mod auth_generator;
pub mod api_generator;
pub mod project_analyzer;

use tera::{Tera, Context, Value, to_value, Result as TeraResult};
use std::collections::HashMap;
use elif_core::ElifError;

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self, ElifError> {
        let mut tera = Tera::default();
        
        // Load stub templates from files
        let template_dir = std::env::current_dir()
            .map_err(|e| ElifError::Validation { message: format!("Failed to get current directory: {}", e) })?
            .join("crates/cli/templates");
            
        if template_dir.exists() {
            // Combine all templates into a single collection for unified loading
            let all_templates = [
                // Root-level templates
                "cargo_toml.stub",
                "main_api.stub", 
                "main_minimal.stub",
                "main_bootstrap.stub", // Laravel-style bootstrap template
                "main_modular.stub", // New modular structure template
                "app_module.stub",
                "app_module_bootstrap.stub", // Bootstrap-ready module template
                "app_controller.stub",
                "app_service.stub",
                "user_controller.stub",
                "user_service.stub",
                "controllers_mod.stub",
                "services_mod.stub",
                "module_services.stub",
                // Modular templates from modules/ directory
                "modules/app_module.stub",
                "modules/app_controller.stub", 
                "modules/app_service.stub",
                "modules/feature_module.stub",
                "modules/module_controller.stub",
                "modules/module_service.stub",
                "modules/dto/create_dto.stub",
                "modules/dto/update_dto.stub", 
                "modules/dto/mod_dto.stub",
            ];
            
            // Single loop to load all templates
            for template_file in &all_templates {
                let template_path = template_dir.join(template_file);
                if template_path.exists() {
                    let content = std::fs::read_to_string(&template_path)
                        .map_err(|e| ElifError::Validation { message: format!("Failed to read template {}: {}", template_file, e) })?;
                    tera.add_raw_template(template_file, &content)
                        .map_err(|e| ElifError::Validation { message: format!("Failed to register template {}: {}", template_file, e) })?;
                }
            }
        } else {
            // Fallback to embedded templates for backwards compatibility
            tera.add_raw_template("model.stub", include_str!("../../templates/model.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register model template: {}", e) })?;
            tera.add_raw_template("controller.stub", include_str!("../../templates/controller.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register controller template: {}", e) })?;
            tera.add_raw_template("migration.stub", include_str!("../../templates/migration.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register migration template: {}", e) })?;
            tera.add_raw_template("test.stub", include_str!("../../templates/test.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register test template: {}", e) })?;
            tera.add_raw_template("policy.stub", include_str!("../../templates/policy.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register policy template: {}", e) })?;
            tera.add_raw_template("cargo_toml.stub", include_str!("../../templates/cargo_toml.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register cargo_toml template: {}", e) })?;
            tera.add_raw_template("main_api.stub", include_str!("../../templates/main_api.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register main_api template: {}", e) })?;
            // Add the missing critical templates
            tera.add_raw_template("controllers_mod.stub", include_str!("../../templates/controllers_mod.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register controllers_mod template: {}", e) })?;
            tera.add_raw_template("services_mod.stub", include_str!("../../templates/services_mod.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register services_mod template: {}", e) })?;
            tera.add_raw_template("user_controller.stub", include_str!("../../templates/user_controller.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register user_controller template: {}", e) })?;
            tera.add_raw_template("user_service.stub", include_str!("../../templates/user_service.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register user_service template: {}", e) })?;
            tera.add_raw_template("module_services.stub", include_str!("../../templates/module_services.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register module_services template: {}", e) })?;
            // Add NestJS-style minimal templates
            tera.add_raw_template("main_minimal.stub", include_str!("../../templates/main_minimal.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register main_minimal template: {}", e) })?;
            tera.add_raw_template("app_module.stub", include_str!("../../templates/app_module.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register app_module template: {}", e) })?;
            tera.add_raw_template("app_controller.stub", include_str!("../../templates/app_controller.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register app_controller template: {}", e) })?;
            tera.add_raw_template("app_service.stub", include_str!("../../templates/app_service.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register app_service template: {}", e) })?;
            // Add bootstrap templates
            tera.add_raw_template("main_bootstrap.stub", include_str!("../../templates/main_bootstrap.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register main_bootstrap template: {}", e) })?;
            tera.add_raw_template("app_module_bootstrap.stub", include_str!("../../templates/app_module_bootstrap.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register app_module_bootstrap template: {}", e) })?;
            // Add modular template
            tera.add_raw_template("main_modular.stub", include_str!("../../templates/main_modular.stub"))
                .map_err(|e| ElifError::Validation { message: format!("Failed to register main_modular template: {}", e) })?;
        }
        
        // Register filters (Tera equivalent of Handlebars helpers)
        register_custom_filters(&mut tera);
        
        Ok(TemplateEngine { tera })
    }
    
    pub fn render(&self, template: &str, data: &HashMap<String, serde_json::Value>) -> Result<String, ElifError> {
        let mut context = Context::new();
        for (key, value) in data {
            context.insert(key, value);
        }
        
        self.render_with_context(template, &context)
    }
    
    pub fn render_with_context(&self, template: &str, context: &Context) -> Result<String, ElifError> {
        // Handle both old template names and new .stub names
        let template_name = if template.ends_with(".stub") {
            template
        } else {
            &format!("{}.stub", template)
        };
        
        self.tera.render(template_name, context)
            .map_err(|e| ElifError::Validation { message: format!("Template rendering error: {}", e) })
    }
}

// Register custom filters for Tera
fn register_custom_filters(tera: &mut Tera) {
    tera.register_filter("pluralize", pluralize_filter);
    tera.register_filter("snake_case", snake_case_filter);
    tera.register_filter("pascal_case", pascal_case_filter);
    tera.register_filter("camel_case", camel_case_filter);
    tera.register_filter("upper_case", upper_case_filter);
    tera.register_filter("lower", lower_filter);
    tera.register_filter("sql_type", sql_type_filter);
}

// Tera filter functions
fn pluralize_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let word = value.as_str().ok_or_else(|| tera::Error::msg("pluralize filter requires string input"))?;
    let pluralized = pluralize_word(word);
    Ok(to_value(pluralized)?)
}

fn snake_case_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| tera::Error::msg("snake_case filter requires string input"))?;
    let snake_case = to_snake_case(s);
    Ok(to_value(snake_case)?)
}

fn pascal_case_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| tera::Error::msg("pascal_case filter requires string input"))?;
    let pascal_case = to_pascal_case(s);
    Ok(to_value(pascal_case)?)
}

fn camel_case_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| tera::Error::msg("camel_case filter requires string input"))?;
    let camel_case = to_camel_case(s);
    Ok(to_value(camel_case)?)
}

fn upper_case_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| tera::Error::msg("upper_case filter requires string input"))?;
    let upper_case = s.to_uppercase();
    Ok(to_value(upper_case)?)
}

fn lower_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| tera::Error::msg("lower filter requires string input"))?;
    let lower_case = s.to_lowercase();
    Ok(to_value(lower_case)?)
}

fn sql_type_filter(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let field_type = value.as_str().ok_or_else(|| tera::Error::msg("sql_type filter requires string input"))?;
    let sql_type = match field_type {
        "Uuid" | "uuid" => "UUID",
        "String" | "string" => "VARCHAR(255)",
        "text" => "TEXT",
        "i32" | "integer" => "INTEGER",
        "i64" | "bigint" => "BIGINT",
        "bool" | "boolean" => "BOOLEAN",
        "DateTime<Utc>" | "timestamp" => "TIMESTAMPTZ",
        "serde_json::Value" | "json" => "JSONB",
        _ => field_type,
    };
    Ok(to_value(sql_type)?)
}

// Utility functions for string transformation
pub fn pluralize_word(word: &str) -> String {
    if word.ends_with('y') && word.len() > 1 {
        format!("{}ies", &word[..word.len()-1])
    } else if word.ends_with('s') || word.ends_with("sh") || word.ends_with("ch") || word.ends_with('x') {
        format!("{}es", word)
    } else {
        format!("{}s", word)
    }
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.collect::<String>()
            }
        })
        .collect()
}

pub fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    if let Some(first) = pascal.chars().next() {
        first.to_lowercase().collect::<String>() + &pascal[1..]
    } else {
        pascal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pluralize_word() {
        assert_eq!(pluralize_word("user"), "users");
        assert_eq!(pluralize_word("category"), "categories");
        assert_eq!(pluralize_word("box"), "boxes");
        assert_eq!(pluralize_word("class"), "classes");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_snake_case("APIKey"), "a_p_i_key");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_pascal_case("api_key"), "ApiKey");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("user_profile"), "userProfile");
        assert_eq!(to_camel_case("api_key"), "apiKey");
    }
}