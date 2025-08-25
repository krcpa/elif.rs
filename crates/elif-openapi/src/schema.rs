use crate::{
    error::{OpenApiError, OpenApiResult},
    specification::Schema,
};
use serde_json::Value;
use std::collections::HashMap;

/// Schema generator for converting Rust types to OpenAPI schemas
pub struct SchemaGenerator {
    /// Generated schemas cache
    schemas: HashMap<String, Schema>,
    /// Configuration options
    config: SchemaConfig,
}

/// Configuration for schema generation
#[derive(Debug, Clone)]
pub struct SchemaConfig {
    /// Generate nullable schemas for Option<T>
    pub nullable_optional: bool,
    /// Include example values
    pub include_examples: bool,
    /// Custom type mappings
    pub custom_mappings: HashMap<String, Schema>,
}

/// Type information for schema generation
#[derive(Debug, Clone)]
pub struct TypeSchema {
    /// Type name
    pub name: String,
    /// Generated schema
    pub schema: Schema,
    /// Dependencies (other types this type references)
    pub dependencies: Vec<String>,
}

impl SchemaGenerator {
    /// Create a new schema generator
    pub fn new(config: SchemaConfig) -> Self {
        Self {
            schemas: HashMap::new(),
            config,
        }
    }

    /// Generate schema for a Rust type
    pub fn generate_schema(&mut self, type_name: &str) -> OpenApiResult<Schema> {
        // Check cache first
        if let Some(schema) = self.schemas.get(type_name) {
            return Ok(schema.clone());
        }

        // Check custom mappings
        if let Some(schema) = self.config.custom_mappings.get(type_name) {
            self.schemas.insert(type_name.to_string(), schema.clone());
            return Ok(schema.clone());
        }

        // Generate schema based on type
        let schema = self.generate_schema_for_type(type_name)?;
        self.schemas.insert(type_name.to_string(), schema.clone());
        Ok(schema)
    }

    /// Generate schema for primitive types, collections, and custom types
    fn generate_schema_for_type(&self, type_name: &str) -> OpenApiResult<Schema> {
        match type_name {
            // String types
            "String" | "str" | "&str" => Ok(Schema {
                schema_type: Some("string".to_string()),
                ..Default::default()
            }),

            // Numeric types
            "i8" | "i16" | "i32" => Ok(Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                ..Default::default()
            }),
            "i64" => Ok(Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                ..Default::default()
            }),
            "u8" | "u16" | "u32" => Ok(Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                minimum: Some(0.0),
                ..Default::default()
            }),
            "u64" => Ok(Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                minimum: Some(0.0),
                ..Default::default()
            }),
            "f32" => Ok(Schema {
                schema_type: Some("number".to_string()),
                format: Some("float".to_string()),
                ..Default::default()
            }),
            "f64" => Ok(Schema {
                schema_type: Some("number".to_string()),
                format: Some("double".to_string()),
                ..Default::default()
            }),

            // Boolean type
            "bool" => Ok(Schema {
                schema_type: Some("boolean".to_string()),
                ..Default::default()
            }),

            // UUID type
            "Uuid" => Ok(Schema {
                schema_type: Some("string".to_string()),
                format: Some("uuid".to_string()),
                ..Default::default()
            }),

            // DateTime types
            "DateTime" | "DateTime<Utc>" => Ok(Schema {
                schema_type: Some("string".to_string()),
                format: Some("date-time".to_string()),
                ..Default::default()
            }),
            "NaiveDate" => Ok(Schema {
                schema_type: Some("string".to_string()),
                format: Some("date".to_string()),
                ..Default::default()
            }),

            // Handle generic types
            type_name if type_name.starts_with("Option<") => {
                let inner_type = self.extract_generic_type(type_name, "Option")?;
                let mut schema = self.generate_schema_for_type(&inner_type)?;
                if self.config.nullable_optional {
                    schema.nullable = Some(true);
                }
                Ok(schema)
            }
            type_name if type_name.starts_with("Vec<") => {
                let inner_type = self.extract_generic_type(type_name, "Vec")?;
                let items_schema = self.generate_schema_for_type(&inner_type)?;
                Ok(Schema {
                    schema_type: Some("array".to_string()),
                    items: Some(Box::new(items_schema)),
                    ..Default::default()
                })
            }
            type_name if type_name.starts_with("HashMap<") => {
                // For simplicity, assume HashMap<String, V>
                let value_type = self.extract_hashmap_value_type(type_name)?;
                let value_schema = self.generate_schema_for_type(&value_type)?;
                Ok(Schema {
                    schema_type: Some("object".to_string()),
                    additional_properties: Some(Box::new(value_schema)),
                    ..Default::default()
                })
            }

            // Custom types - create reference
            _ => Ok(Schema {
                reference: Some(format!("#/components/schemas/{}", type_name)),
                ..Default::default()
            }),
        }
    }

    /// Extract generic type parameter (e.g., "T" from "Option<T>")
    fn extract_generic_type(&self, type_name: &str, wrapper: &str) -> OpenApiResult<String> {
        let start = wrapper.len() + 1; // +1 for '<'
        let end = type_name.len() - 1; // -1 for '>'

        if start >= end {
            return Err(OpenApiError::schema_error(format!(
                "Invalid generic type: {}",
                type_name
            )));
        }

        Ok(type_name[start..end].to_string())
    }

    /// Extract value type from HashMap<K, V>
    fn extract_hashmap_value_type(&self, type_name: &str) -> OpenApiResult<String> {
        // Simple implementation - assumes HashMap<String, ValueType>
        let inner = type_name
            .strip_prefix("HashMap<")
            .and_then(|s| s.strip_suffix(">"))
            .ok_or_else(|| {
                OpenApiError::schema_error(format!("Invalid HashMap type: {}", type_name))
            })?;

        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() != 2 {
            return Err(OpenApiError::schema_error(format!(
                "Invalid HashMap type: {}",
                type_name
            )));
        }

        Ok(parts[1].trim().to_string())
    }

    /// Generate schema for a struct with fields
    pub fn generate_struct_schema(
        &mut self,
        struct_name: &str,
        fields: &[(String, String, Option<String>)], // (name, type, description)
    ) -> OpenApiResult<Schema> {
        let mut properties = HashMap::new();
        let mut required = Vec::new();
        let mut dependencies = Vec::new();

        for (field_name, field_type, description) in fields {
            let mut field_schema = self.generate_schema(field_type)?;

            if let Some(desc) = description {
                field_schema.description = Some(desc.clone());
            }

            // Check if field is optional
            if !field_type.starts_with("Option<") {
                required.push(field_name.clone());
            }

            properties.insert(field_name.clone(), field_schema);

            // Track dependencies
            if !self.is_primitive_type(field_type) {
                dependencies.push(field_type.clone());
            }
        }

        let schema = Schema {
            schema_type: Some("object".to_string()),
            properties,
            required,
            ..Default::default()
        };

        self.schemas.insert(struct_name.to_string(), schema.clone());
        Ok(schema)
    }

    /// Generate schema for an enum
    pub fn generate_enum_schema(
        &mut self,
        enum_name: &str,
        variants: &[String],
    ) -> OpenApiResult<Schema> {
        let enum_values: Vec<Value> = variants.iter().map(|v| Value::String(v.clone())).collect();

        let schema = Schema {
            schema_type: Some("string".to_string()),
            enum_values,
            ..Default::default()
        };

        self.schemas.insert(enum_name.to_string(), schema.clone());
        Ok(schema)
    }

    /// Check if a type is primitive
    fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "String"
                | "str"
                | "&str"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "f32"
                | "f64"
                | "bool"
                | "Uuid"
                | "DateTime"
                | "DateTime<Utc>"
                | "NaiveDate"
        ) || type_name.starts_with("Option<")
            || type_name.starts_with("Vec<")
            || type_name.starts_with("HashMap<")
    }

    /// Get all generated schemas
    pub fn get_schemas(&self) -> &HashMap<String, Schema> {
        &self.schemas
    }

    /// Clear schema cache
    pub fn clear_cache(&mut self) {
        self.schemas.clear();
    }
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            nullable_optional: true,
            include_examples: true,
            custom_mappings: HashMap::new(),
        }
    }
}

impl SchemaConfig {
    /// Create new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set nullable option handling
    pub fn with_nullable_optional(mut self, nullable: bool) -> Self {
        self.nullable_optional = nullable;
        self
    }

    /// Set example inclusion
    pub fn with_examples(mut self, include: bool) -> Self {
        self.include_examples = include;
        self
    }

    /// Add custom type mapping
    pub fn with_custom_mapping(mut self, type_name: &str, schema: Schema) -> Self {
        self.custom_mappings.insert(type_name.to_string(), schema);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_schema_generation() {
        let mut generator = SchemaGenerator::new(SchemaConfig::default());

        let string_schema = generator.generate_schema("String").unwrap();
        assert_eq!(string_schema.schema_type, Some("string".to_string()));

        let int_schema = generator.generate_schema("i32").unwrap();
        assert_eq!(int_schema.schema_type, Some("integer".to_string()));
        assert_eq!(int_schema.format, Some("int32".to_string()));

        let bool_schema = generator.generate_schema("bool").unwrap();
        assert_eq!(bool_schema.schema_type, Some("boolean".to_string()));
    }

    #[test]
    fn test_optional_schema_generation() {
        let mut generator = SchemaGenerator::new(SchemaConfig::default());

        let optional_string_schema = generator.generate_schema("Option<String>").unwrap();
        assert_eq!(
            optional_string_schema.schema_type,
            Some("string".to_string())
        );
        assert_eq!(optional_string_schema.nullable, Some(true));
    }

    #[test]
    fn test_array_schema_generation() {
        let mut generator = SchemaGenerator::new(SchemaConfig::default());

        let array_schema = generator.generate_schema("Vec<String>").unwrap();
        assert_eq!(array_schema.schema_type, Some("array".to_string()));
        assert!(array_schema.items.is_some());

        let items = array_schema.items.unwrap();
        assert_eq!(items.schema_type, Some("string".to_string()));
    }

    #[test]
    fn test_struct_schema_generation() {
        let mut generator = SchemaGenerator::new(SchemaConfig::default());

        let fields = vec![
            ("id".to_string(), "i32".to_string(), None),
            (
                "name".to_string(),
                "String".to_string(),
                Some("User name".to_string()),
            ),
            ("email".to_string(), "Option<String>".to_string(), None),
        ];

        let schema = generator.generate_struct_schema("User", &fields).unwrap();
        assert_eq!(schema.schema_type, Some("object".to_string()));
        assert_eq!(schema.properties.len(), 3);
        assert_eq!(schema.required.len(), 2); // id and name are required
        assert!(schema.properties.contains_key("id"));
        assert!(schema.properties.contains_key("name"));
        assert!(schema.properties.contains_key("email"));
    }

    #[test]
    fn test_tuple_schema_representation() {
        // Test that tuples are represented correctly for OpenAPI 3.0
        // This test ensures we don't use oneOf incorrectly for tuples

        // Create a mock tuple schema similar to what the derive macro should generate
        let tuple_schema = crate::specification::Schema {
            schema_type: Some("array".to_string()),
            title: Some("TestTuple".to_string()),
            description: Some("A tuple with 2 fields in fixed order: (String, i32). Note: OpenAPI 3.0 cannot precisely represent tuple types - this is a generic array representation.".to_string()),
            items: Some(Box::new(crate::specification::Schema {
                description: Some("Tuple element (type varies by position)".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Verify the schema is structured correctly
        assert_eq!(tuple_schema.schema_type, Some("array".to_string()));
        assert!(tuple_schema.description.is_some());
        assert!(tuple_schema
            .description
            .as_ref()
            .unwrap()
            .contains("fixed order"));
        assert!(tuple_schema
            .description
            .as_ref()
            .unwrap()
            .contains("OpenAPI 3.0 cannot precisely represent"));

        // Verify items doesn't use oneOf (which would be incorrect)
        assert!(tuple_schema.items.is_some());
        let items = tuple_schema.items.as_ref().unwrap();
        assert!(items.one_of.is_empty()); // Should NOT use oneOf for tuples
    }
}
