/*!
OpenAPI schema generation traits and utilities.

This module provides traits and utilities for OpenAPI schema generation from Rust types.
The derive macro implementation would be in a separate proc-macro crate.
*/

/// Trait for types that can generate OpenAPI schemas
pub trait OpenApiSchema {
    /// Generate OpenAPI schema for this type
    fn openapi_schema() -> crate::specification::Schema;
    
    /// Get the schema name/title
    fn schema_name() -> String;
}

// Implementation would go here for procedural macros
// This is simplified for the demo version

// Manual implementations for common types
impl OpenApiSchema for String {
    fn openapi_schema() -> crate::specification::Schema {
        crate::specification::Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        }
    }
    
    fn schema_name() -> String {
        "String".to_string()
    }
}

impl OpenApiSchema for i32 {
    fn openapi_schema() -> crate::specification::Schema {
        crate::specification::Schema {
            schema_type: Some("integer".to_string()),
            format: Some("int32".to_string()),
            ..Default::default()
        }
    }
    
    fn schema_name() -> String {
        "i32".to_string()
    }
}

impl OpenApiSchema for i64 {
    fn openapi_schema() -> crate::specification::Schema {
        crate::specification::Schema {
            schema_type: Some("integer".to_string()),
            format: Some("int64".to_string()),
            ..Default::default()
        }
    }
    
    fn schema_name() -> String {
        "i64".to_string()
    }
}

impl OpenApiSchema for bool {
    fn openapi_schema() -> crate::specification::Schema {
        crate::specification::Schema {
            schema_type: Some("boolean".to_string()),
            ..Default::default()
        }
    }
    
    fn schema_name() -> String {
        "bool".to_string()
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Option<T> {
    fn openapi_schema() -> crate::specification::Schema {
        let mut schema = T::openapi_schema();
        schema.nullable = Some(true);
        schema
    }
    
    fn schema_name() -> String {
        format!("Option<{}>", T::schema_name())
    }
}

impl<T: OpenApiSchema> OpenApiSchema for Vec<T> {
    fn openapi_schema() -> crate::specification::Schema {
        crate::specification::Schema {
            schema_type: Some("array".to_string()),
            items: Some(Box::new(T::openapi_schema())),
            ..Default::default()
        }
    }
    
    fn schema_name() -> String {
        format!("Vec<{}>", T::schema_name())
    }
}


