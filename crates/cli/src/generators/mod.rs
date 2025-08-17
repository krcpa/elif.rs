pub mod resource_generator;
pub mod auth_generator;
pub mod api_generator;

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
            // Load from stub files if available
            tera = Tera::new(&format!("{}/*.stub", template_dir.display()))
                .map_err(|e| ElifError::Validation { message: format!("Failed to load stub templates: {}", e) })?;
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
        
        // Handle both old template names and new .stub names
        let template_name = if template.ends_with(".stub") {
            template
        } else {
            &format!("{}.stub", template)
        };
        
        self.tera.render(template_name, &context)
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