use crate::config::ConfigError;
use service_builder::builder;
use std::collections::HashMap;

/// Configuration field definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub validation_rules: Vec<String>,
}

impl ConfigField {
    /// Create a new configuration field
    pub fn new(name: impl Into<String>, field_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            required: false,
            default_value: None,
            description: None,
            validation_rules: Vec::new(),
        }
    }
    
    /// Make field required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    
    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }
    
    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add validation rule
    pub fn add_validation(mut self, rule: impl Into<String>) -> Self {
        self.validation_rules.push(rule.into());
        self
    }
}

/// Configuration schema for validation and generation
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    pub name: String,
    pub fields: Vec<ConfigField>,
}

impl ConfigSchema {
    /// Create a new configuration schema
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }
    
    /// Add a field to the schema
    pub fn add_field(mut self, field: ConfigField) -> Self {
        self.fields.push(field);
        self
    }
    
    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<&ConfigField> {
        self.fields.iter().find(|f| f.name == name)
    }
    
    /// Get all required fields
    pub fn required_fields(&self) -> Vec<&ConfigField> {
        self.fields.iter().filter(|f| f.required).collect()
    }
    
    /// Validate a configuration map against this schema
    pub fn validate_config(&self, config: &HashMap<String, String>) -> Result<(), ConfigError> {
        // Check required fields
        for field in &self.fields {
            if field.required && !config.contains_key(&field.name) {
                return Err(ConfigError::missing_required(
                    &field.name,
                    field.description.as_deref().unwrap_or("This field is required"),
                ));
            }
        }
        
        // Validate field values
        for (key, value) in config {
            if let Some(field) = self.get_field(key) {
                self.validate_field_value(field, value)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate a single field value
    fn validate_field_value(&self, field: &ConfigField, value: &str) -> Result<(), ConfigError> {
        // Basic type validation
        match field.field_type.as_str() {
            "integer" | "int" => {
                value.parse::<i64>().map_err(|_| {
                    ConfigError::invalid_value(&field.name, value, "valid integer")
                })?;
            }
            "float" | "number" => {
                value.parse::<f64>().map_err(|_| {
                    ConfigError::invalid_value(&field.name, value, "valid number")
                })?;
            }
            "boolean" | "bool" => {
                value.parse::<bool>().map_err(|_| {
                    ConfigError::invalid_value(&field.name, value, "true or false")
                })?;
            }
            "url" => {
                if !value.starts_with("http://") && !value.starts_with("https://") {
                    return Err(ConfigError::invalid_value(&field.name, value, "valid URL"));
                }
            }
            _ => {
                // String or custom types - no validation for now
            }
        }
        
        // Apply validation rules
        for rule in &field.validation_rules {
            self.apply_validation_rule(field, value, rule)?;
        }
        
        Ok(())
    }
    
    /// Apply a validation rule to a field value
    fn apply_validation_rule(&self, field: &ConfigField, value: &str, rule: &str) -> Result<(), ConfigError> {
        if rule.starts_with("min_length:") {
            let min_len: usize = rule.strip_prefix("min_length:").unwrap().parse()
                .map_err(|_| ConfigError::validation_failed("Invalid min_length rule"))?;
            if value.len() < min_len {
                return Err(ConfigError::invalid_value(
                    &field.name, 
                    value, 
                    format!("at least {} characters", min_len)
                ));
            }
        } else if rule.starts_with("max_length:") {
            let max_len: usize = rule.strip_prefix("max_length:").unwrap().parse()
                .map_err(|_| ConfigError::validation_failed("Invalid max_length rule"))?;
            if value.len() > max_len {
                return Err(ConfigError::invalid_value(
                    &field.name, 
                    value, 
                    format!("at most {} characters", max_len)
                ));
            }
        } else if rule.starts_with("pattern:") {
            let pattern = rule.strip_prefix("pattern:").unwrap();
            // In a real implementation, you would use a regex library
            if !value.contains(pattern) {
                return Err(ConfigError::invalid_value(
                    &field.name, 
                    value, 
                    format!("matching pattern: {}", pattern)
                ));
            }
        }
        
        Ok(())
    }
}

/// Configuration builder for creating configurations programmatically
#[builder]
pub struct ConfigBuilder<T> {
    #[builder(default)]
    pub fields: Vec<ConfigField>,
    
    #[builder(optional)]
    pub name: Option<String>,
    
    #[builder(default)]
    pub _phantom: std::marker::PhantomData<T>,
}

impl<T> std::fmt::Debug for ConfigBuilder<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigBuilder")
            .field("fields_count", &self.fields.len())
            .field("name", &self.name)
            .finish()
    }
}

impl<T> ConfigBuilder<T> {
    /// Build a ConfigSchema from this builder
    pub fn build_schema(self) -> ConfigSchema {
        ConfigSchema {
            name: self.name.unwrap_or_else(|| "DefaultConfig".to_string()),
            fields: self.fields,
        }
    }
}

// Add convenience methods to the generated builder
impl<T> ConfigBuilderBuilder<T> {
    /// Add a configuration field
    pub fn add_field(self, field: ConfigField) -> Self {
        let mut fields_vec = self.fields.unwrap_or_default();
        fields_vec.push(field);
        ConfigBuilderBuilder {
            fields: Some(fields_vec),
            name: self.name,
            _phantom: self._phantom,
        }
    }
    
    /// Add a string field
    pub fn add_string_field(self, name: impl Into<String>) -> Self {
        self.add_field(ConfigField::new(name, "string"))
    }
    
    /// Add a required string field
    pub fn add_required_string_field(self, name: impl Into<String>) -> Self {
        self.add_field(ConfigField::new(name, "string").required())
    }
    
    /// Add an integer field
    pub fn add_int_field(self, name: impl Into<String>) -> Self {
        self.add_field(ConfigField::new(name, "integer"))
    }
    
    /// Add a boolean field
    pub fn add_bool_field(self, name: impl Into<String>) -> Self {
        self.add_field(ConfigField::new(name, "boolean"))
    }
    
    /// Add a URL field
    pub fn add_url_field(self, name: impl Into<String>) -> Self {
        self.add_field(ConfigField::new(name, "url"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_schema() {
        let schema = ConfigSchema::new("test_config")
            .add_field(ConfigField::new("name", "string").required())
            .add_field(ConfigField::new("port", "integer").with_default("3000"))
            .add_field(ConfigField::new("debug", "boolean").with_default("false"));
        
        assert_eq!(schema.name, "test_config");
        assert_eq!(schema.fields.len(), 3);
        assert_eq!(schema.required_fields().len(), 1);
    }
    
    #[test]
    fn test_config_validation() {
        let schema = ConfigSchema::new("test_config")
            .add_field(ConfigField::new("name", "string").required())
            .add_field(ConfigField::new("port", "integer"));
        
        let mut config = HashMap::new();
        config.insert("name".to_string(), "test".to_string());
        config.insert("port".to_string(), "3000".to_string());
        
        assert!(schema.validate_config(&config).is_ok());
        
        config.remove("name");
        assert!(schema.validate_config(&config).is_err());
    }
}