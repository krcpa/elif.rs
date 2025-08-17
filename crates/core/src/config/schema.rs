use crate::config::{ConfigField, ConfigError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSchema {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub sections: Vec<ConfigSection>,
}

impl ConfigurationSchema {
    /// Create a new configuration schema
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: None,
            sections: Vec::new(),
        }
    }
    
    /// Set schema description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add a configuration section
    pub fn add_section(mut self, section: ConfigSection) -> Self {
        self.sections.push(section);
        self
    }
    
    /// Get section by name
    pub fn get_section(&self, name: &str) -> Option<&ConfigSection> {
        self.sections.iter().find(|s| s.name == name)
    }
    
    /// Get all fields from all sections
    pub fn all_fields(&self) -> Vec<&ConfigField> {
        self.sections.iter().flat_map(|s| &s.fields).collect()
    }
    
    /// Validate configuration against this schema
    pub fn validate(&self, config: &HashMap<String, serde_json::Value>) -> Result<(), ConfigError> {
        for section in &self.sections {
            section.validate_fields(config)?;
        }
        Ok(())
    }
    
    /// Generate OpenAPI schema
    pub fn to_openapi_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        for section in &self.sections {
            for field in &section.fields {
                properties.insert(
                    field.name.clone(),
                    field.to_openapi_property()
                );
                
                if field.required {
                    required.push(field.name.clone());
                }
            }
        }
        
        serde_json::json!({
            "type": "object",
            "title": self.name,
            "description": self.description.as_deref().unwrap_or("Configuration schema"),
            "properties": properties,
            "required": required
        })
    }
}

/// Configuration section for grouping related fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSection {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<ConfigField>,
}

impl ConfigSection {
    /// Create a new configuration section
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            fields: Vec::new(),
        }
    }
    
    /// Set section description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add a field to this section
    pub fn add_field(mut self, field: ConfigField) -> Self {
        self.fields.push(field);
        self
    }
    
    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<&ConfigField> {
        self.fields.iter().find(|f| f.name == name)
    }
    
    /// Validate fields in this section
    pub fn validate_fields(&self, config: &HashMap<String, serde_json::Value>) -> Result<(), ConfigError> {
        for field in &self.fields {
            if field.required && !config.contains_key(&field.name) {
                return Err(ConfigError::missing_required(
                    &field.name,
                    field.description.as_deref().unwrap_or("This field is required"),
                ));
            }
            
            if let Some(value) = config.get(&field.name) {
                self.validate_field_value(field, value)?;
            }
        }
        Ok(())
    }
    
    /// Validate a single field value
    fn validate_field_value(&self, field: &ConfigField, value: &serde_json::Value) -> Result<(), ConfigError> {
        match field.field_type.as_str() {
            "string" => {
                if !value.is_string() {
                    return Err(ConfigError::invalid_value(&field.name, value.to_string(), "string"));
                }
            }
            "integer" | "int" => {
                if !value.is_i64() {
                    return Err(ConfigError::invalid_value(&field.name, value.to_string(), "integer"));
                }
            }
            "number" | "float" => {
                if !value.is_f64() && !value.is_i64() {
                    return Err(ConfigError::invalid_value(&field.name, value.to_string(), "number"));
                }
            }
            "boolean" | "bool" => {
                if !value.is_boolean() {
                    return Err(ConfigError::invalid_value(&field.name, value.to_string(), "boolean"));
                }
            }
            "array" => {
                if !value.is_array() {
                    return Err(ConfigError::invalid_value(&field.name, value.to_string(), "array"));
                }
            }
            "object" => {
                if !value.is_object() {
                    return Err(ConfigError::invalid_value(&field.name, value.to_string(), "object"));
                }
            }
            _ => {
                // Custom type - no validation for now
            }
        }
        
        Ok(())
    }
}

impl ConfigField {
    /// Convert field to OpenAPI property definition
    pub fn to_openapi_property(&self) -> serde_json::Value {
        let mut property = serde_json::Map::new();
        
        // Set type
        let openapi_type = match self.field_type.as_str() {
            "integer" | "int" => "integer",
            "number" | "float" => "number",
            "boolean" | "bool" => "boolean",
            "array" => "array",
            "object" => "object",
            _ => "string",
        };
        property.insert("type".to_string(), serde_json::Value::String(openapi_type.to_string()));
        
        // Set description
        if let Some(desc) = &self.description {
            property.insert("description".to_string(), serde_json::Value::String(desc.clone()));
        }
        
        // Set default value
        if let Some(default) = &self.default_value {
            let default_value = match openapi_type {
                "integer" => serde_json::Value::Number(
                    serde_json::Number::from(default.parse::<i64>().unwrap_or(0))
                ),
                "number" => serde_json::Value::Number(
                    serde_json::Number::from_f64(default.parse::<f64>().unwrap_or(0.0)).unwrap()
                ),
                "boolean" => serde_json::Value::Bool(default.parse::<bool>().unwrap_or(false)),
                _ => serde_json::Value::String(default.clone()),
            };
            property.insert("default".to_string(), default_value);
        }
        
        serde_json::Value::Object(property)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_configuration_schema() {
        let schema = ConfigurationSchema::new("app_config", "1.0")
            .with_description("Application configuration")
            .add_section(
                ConfigSection::new("database")
                    .with_description("Database configuration")
                    .add_field(ConfigField::new("url", "string").required())
                    .add_field(ConfigField::new("max_connections", "integer").with_default("10"))
            );
        
        assert_eq!(schema.name, "app_config");
        assert_eq!(schema.sections.len(), 1);
        assert_eq!(schema.all_fields().len(), 2);
    }
    
    #[test]
    fn test_openapi_schema_generation() {
        let schema = ConfigurationSchema::new("test_config", "1.0")
            .add_section(
                ConfigSection::new("general")
                    .add_field(
                        ConfigField::new("name", "string")
                            .required()
                            .with_description("Application name")
                    )
                    .add_field(
                        ConfigField::new("port", "integer")
                            .with_default("3000")
                            .with_description("Server port")
                    )
            );
        
        let openapi_schema = schema.to_openapi_schema();
        
        assert_eq!(openapi_schema["type"], "object");
        assert_eq!(openapi_schema["title"], "test_config");
        assert!(openapi_schema["properties"]["name"].is_object());
        assert!(openapi_schema["properties"]["port"].is_object());
        assert_eq!(openapi_schema["required"].as_array().unwrap().len(), 1);
    }
}