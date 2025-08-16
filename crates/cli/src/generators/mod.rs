pub mod templates;
pub mod resource_generator;
pub mod auth_generator;
pub mod api_generator;

use handlebars::Handlebars;
use std::collections::HashMap;
use elif_core::ElifError;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Result<Self, ElifError> {
        let mut handlebars = Handlebars::new();
        
        // Register built-in templates
        handlebars.register_template_string("model", templates::MODEL_TEMPLATE)
            .map_err(|e| ElifError::Validation(format!("Failed to register model template: {}", e)))?;
        handlebars.register_template_string("controller", templates::CONTROLLER_TEMPLATE)
            .map_err(|e| ElifError::Validation(format!("Failed to register controller template: {}", e)))?;
        handlebars.register_template_string("migration", templates::MIGRATION_TEMPLATE)
            .map_err(|e| ElifError::Validation(format!("Failed to register migration template: {}", e)))?;
        handlebars.register_template_string("test", templates::TEST_TEMPLATE)
            .map_err(|e| ElifError::Validation(format!("Failed to register test template: {}", e)))?;
        handlebars.register_template_string("policy", templates::POLICY_TEMPLATE)
            .map_err(|e| ElifError::Validation(format!("Failed to register policy template: {}", e)))?;
        
        // Register helpers
        handlebars.register_helper("pluralize", Box::new(pluralize_helper));
        handlebars.register_helper("snake_case", Box::new(snake_case_helper));
        handlebars.register_helper("pascal_case", Box::new(pascal_case_helper));
        handlebars.register_helper("camel_case", Box::new(camel_case_helper));
        handlebars.register_helper("upper_case", Box::new(upper_case_helper));
        handlebars.register_helper("sql_type", Box::new(sql_type_helper));
        
        Ok(TemplateEngine { handlebars })
    }
    
    pub fn render(&self, template: &str, data: &HashMap<String, serde_json::Value>) -> Result<String, ElifError> {
        self.handlebars.render(template, data)
            .map_err(|e| ElifError::Validation(format!("Template rendering error: {}", e)))
    }
}

// Helper functions for templates
fn pluralize_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(word) = param.value().as_str() {
            let pluralized = pluralize_word(word);
            out.write(&pluralized)?;
        }
    }
    Ok(())
}

fn snake_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            let snake_case = to_snake_case(s);
            out.write(&snake_case)?;
        }
    }
    Ok(())
}

fn pascal_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            let pascal_case = to_pascal_case(s);
            out.write(&pascal_case)?;
        }
    }
    Ok(())
}

fn camel_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            let camel_case = to_camel_case(s);
            out.write(&camel_case)?;
        }
    }
    Ok(())
}

fn upper_case_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            let upper_case = s.to_uppercase();
            out.write(&upper_case)?;
        }
    }
    Ok(())
}

fn sql_type_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(field_type) = param.value().as_str() {
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
            out.write(sql_type)?;
        }
    }
    Ok(())
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