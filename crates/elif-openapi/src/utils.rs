/*!
Utility functions for OpenAPI generation.
*/

use crate::{
    error::{OpenApiError, OpenApiResult},
    specification::OpenApiSpec,
};
use std::fs;
use std::path::Path;

/// Utility functions for OpenAPI operations
pub struct OpenApiUtils;

impl OpenApiUtils {
    /// Validate an OpenAPI specification
    pub fn validate_spec(spec: &OpenApiSpec) -> OpenApiResult<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        // Check required fields
        if spec.info.title.is_empty() {
            warnings.push(ValidationWarning::new(
                "info.title is required but empty",
                ValidationLevel::Error,
            ));
        }

        if spec.info.version.is_empty() {
            warnings.push(ValidationWarning::new(
                "info.version is required but empty", 
                ValidationLevel::Error,
            ));
        }

        // Check OpenAPI version
        if spec.openapi != "3.0.3" && !spec.openapi.starts_with("3.0") {
            warnings.push(ValidationWarning::new(
                &format!("OpenAPI version {} may not be fully supported", spec.openapi),
                ValidationLevel::Warning,
            ));
        }

        // Check paths
        if spec.paths.is_empty() {
            warnings.push(ValidationWarning::new(
                "No paths defined in specification",
                ValidationLevel::Warning,
            ));
        }

        // Validate path operations
        for (path, path_item) in &spec.paths {
            if !path.starts_with('/') {
                warnings.push(ValidationWarning::new(
                    &format!("Path '{}' should start with '/'", path),
                    ValidationLevel::Warning,
                ));
            }

            // Check if path has at least one operation
            let has_operations = path_item.get.is_some()
                || path_item.post.is_some()
                || path_item.put.is_some()
                || path_item.delete.is_some()
                || path_item.patch.is_some()
                || path_item.options.is_some()
                || path_item.head.is_some()
                || path_item.trace.is_some();

            if !has_operations {
                warnings.push(ValidationWarning::new(
                    &format!("Path '{}' has no operations defined", path),
                    ValidationLevel::Warning,
                ));
            }

            // Validate operations
            let operations = vec![
                ("GET", &path_item.get),
                ("POST", &path_item.post),
                ("PUT", &path_item.put),
                ("DELETE", &path_item.delete),
                ("PATCH", &path_item.patch),
                ("OPTIONS", &path_item.options),
                ("HEAD", &path_item.head),
                ("TRACE", &path_item.trace),
            ];

            for (method, operation) in operations {
                if let Some(op) = operation {
                    if op.responses.is_empty() {
                        warnings.push(ValidationWarning::new(
                            &format!("{} {} has no responses defined", method, path),
                            ValidationLevel::Error,
                        ));
                    }

                    // Check for operation ID uniqueness would require global tracking
                    if let Some(op_id) = &op.operation_id {
                        if op_id.is_empty() {
                            warnings.push(ValidationWarning::new(
                                &format!("{} {} has empty operationId", method, path),
                                ValidationLevel::Warning,
                            ));
                        }
                    }
                }
            }
        }

        // Validate components
        if let Some(components) = &spec.components {
            // Check for unused schemas
            for schema_name in components.schemas.keys() {
                let reference = format!("#/components/schemas/{}", schema_name);
                let is_used = Self::is_schema_referenced(spec, &reference);
                if !is_used {
                    warnings.push(ValidationWarning::new(
                        &format!("Schema '{}' is defined but never referenced", schema_name),
                        ValidationLevel::Info,
                    ));
                }
            }
        }

        Ok(warnings)
    }

    /// Check if a schema is referenced anywhere in the spec using proper recursive traversal
    fn is_schema_referenced(spec: &OpenApiSpec, reference: &str) -> bool {

        // Check in paths and operations
        for path_item in spec.paths.values() {
            if Self::is_schema_in_path_item(path_item, reference) {
                return true;
            }
        }

        // Check in components
        if let Some(components) = &spec.components {
            // Check in schema definitions themselves (for nested references)
            for schema in components.schemas.values() {
                if Self::is_schema_in_schema(schema, reference) {
                    return true;
                }
            }

            // Check in responses
            for response in components.responses.values() {
                if Self::is_schema_in_response(response, reference) {
                    return true;
                }
            }

            // Check in request bodies
            for request_body in components.request_bodies.values() {
                if Self::is_schema_in_request_body(request_body, reference) {
                    return true;
                }
            }

            // Check in parameters
            for parameter in components.parameters.values() {
                if Self::is_schema_in_parameter(parameter, reference) {
                    return true;
                }
            }

            // Check in headers
            for header in components.headers.values() {
                if Self::is_schema_in_header(header, reference) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if schema is referenced in a path item
    fn is_schema_in_path_item(path_item: &crate::specification::PathItem, reference: &str) -> bool {
        let operations = vec![
            &path_item.get, &path_item.post, &path_item.put, &path_item.delete,
            &path_item.patch, &path_item.options, &path_item.head, &path_item.trace,
        ];

        for operation_opt in operations {
            if let Some(operation) = operation_opt {
                if Self::is_schema_in_operation(operation, reference) {
                    return true;
                }
            }
        }

        // Check path-level parameters
        for parameter in &path_item.parameters {
            if Self::is_schema_in_parameter(parameter, reference) {
                return true;
            }
        }

        false
    }

    /// Check if schema is referenced in an operation
    fn is_schema_in_operation(operation: &crate::specification::Operation, reference: &str) -> bool {
        // Check parameters
        for parameter in &operation.parameters {
            if Self::is_schema_in_parameter(parameter, reference) {
                return true;
            }
        }

        // Check request body
        if let Some(request_body) = &operation.request_body {
            if Self::is_schema_in_request_body(request_body, reference) {
                return true;
            }
        }

        // Check responses
        for response in operation.responses.values() {
            if Self::is_schema_in_response(response, reference) {
                return true;
            }
        }

        false
    }

    /// Check if schema is referenced in a parameter
    fn is_schema_in_parameter(parameter: &crate::specification::Parameter, reference: &str) -> bool {
        if let Some(schema) = &parameter.schema {
            Self::is_schema_in_schema(schema, reference)
        } else {
            false
        }
    }

    /// Check if schema is referenced in a request body
    fn is_schema_in_request_body(request_body: &crate::specification::RequestBody, reference: &str) -> bool {
        for media_type in request_body.content.values() {
            if let Some(schema) = &media_type.schema {
                if Self::is_schema_in_schema(schema, reference) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if schema is referenced in a response
    fn is_schema_in_response(response: &crate::specification::Response, reference: &str) -> bool {
        // Check response content
        for media_type in response.content.values() {
            if let Some(schema) = &media_type.schema {
                if Self::is_schema_in_schema(schema, reference) {
                    return true;
                }
            }
        }

        // Check response headers
        for header in response.headers.values() {
            if Self::is_schema_in_header(header, reference) {
                return true;
            }
        }

        false
    }

    /// Check if schema is referenced in a header
    fn is_schema_in_header(header: &crate::specification::Header, reference: &str) -> bool {
        if let Some(schema) = &header.schema {
            Self::is_schema_in_schema(schema, reference)
        } else {
            false
        }
    }

    /// Check if schema is referenced within another schema (recursive)
    fn is_schema_in_schema(schema: &crate::specification::Schema, reference: &str) -> bool {
        // Check direct reference
        if let Some(ref_str) = &schema.reference {
            if ref_str == reference {
                return true;
            }
        }

        // Check properties (for object schemas)
        for property_schema in schema.properties.values() {
            if Self::is_schema_in_schema(property_schema, reference) {
                return true;
            }
        }

        // Check additional properties
        if let Some(additional_properties) = &schema.additional_properties {
            if Self::is_schema_in_schema(additional_properties, reference) {
                return true;
            }
        }

        // Check items (for array schemas)
        if let Some(items_schema) = &schema.items {
            if Self::is_schema_in_schema(items_schema, reference) {
                return true;
            }
        }

        // Check composition schemas (allOf, anyOf, oneOf)
        for composed_schema in &schema.all_of {
            if Self::is_schema_in_schema(composed_schema, reference) {
                return true;
            }
        }

        for composed_schema in &schema.any_of {
            if Self::is_schema_in_schema(composed_schema, reference) {
                return true;
            }
        }

        for composed_schema in &schema.one_of {
            if Self::is_schema_in_schema(composed_schema, reference) {
                return true;
            }
        }

        false
    }

    /// Save OpenAPI specification to file
    pub fn save_spec_to_file<P: AsRef<Path>>(
        spec: &OpenApiSpec,
        path: P,
        format: OutputFormat,
        pretty: bool,
    ) -> OpenApiResult<()> {
        let content = match format {
            OutputFormat::Json => {
                if pretty {
                    serde_json::to_string_pretty(spec)?
                } else {
                    serde_json::to_string(spec)?
                }
            }
            OutputFormat::Yaml => {
                serde_yaml::to_string(spec)?
            }
        };

        fs::write(path.as_ref(), content)
            .map_err(|e| OpenApiError::Io(e))?;

        Ok(())
    }

    /// Load OpenAPI specification from file
    pub fn load_spec_from_file<P: AsRef<Path>>(path: P) -> OpenApiResult<OpenApiSpec> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| OpenApiError::Io(e))?;

        let extension = path
            .as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "json" => {
                serde_json::from_str(&content).map_err(OpenApiError::from)
            }
            "yaml" | "yml" => {
                serde_yaml::from_str(&content).map_err(OpenApiError::from)
            }
            _ => {
                // Try to detect format from content
                if content.trim_start().starts_with('{') {
                    serde_json::from_str(&content).map_err(OpenApiError::from)
                } else {
                    serde_yaml::from_str(&content).map_err(OpenApiError::from)
                }
            }
        }
    }

    /// Merge two OpenAPI specifications
    pub fn merge_specs(base: &mut OpenApiSpec, other: &OpenApiSpec) -> OpenApiResult<()> {
        // Merge paths
        for (path, path_item) in &other.paths {
            if base.paths.contains_key(path) {
                return Err(OpenApiError::validation_error(
                    format!("Path '{}' already exists in base specification", path)
                ));
            }
            base.paths.insert(path.clone(), path_item.clone());
        }

        // Merge components
        if let Some(other_components) = &other.components {
            let base_components = base.components.get_or_insert_with(Default::default);

            // Merge schemas
            for (name, schema) in &other_components.schemas {
                if base_components.schemas.contains_key(name) {
                    return Err(OpenApiError::validation_error(
                        format!("Schema '{}' already exists in base specification", name)
                    ));
                }
                base_components.schemas.insert(name.clone(), schema.clone());
            }

            // Merge other components...
            for (name, response) in &other_components.responses {
                base_components.responses.insert(name.clone(), response.clone());
            }
        }

        // Merge tags
        for tag in &other.tags {
            if !base.tags.iter().any(|t| t.name == tag.name) {
                base.tags.push(tag.clone());
            }
        }

        Ok(())
    }

    /// Generate example request/response from schema
    pub fn generate_example_from_schema(
        schema: &crate::specification::Schema,
    ) -> OpenApiResult<serde_json::Value> {
        use serde_json::{Value, Map};

        match schema.schema_type.as_deref() {
            Some("object") => {
                let mut obj = Map::new();
                for (prop_name, prop_schema) in &schema.properties {
                    let example = Self::generate_example_from_schema(prop_schema)?;
                    obj.insert(prop_name.clone(), example);
                }
                Ok(Value::Object(obj))
            }
            Some("array") => {
                if let Some(items_schema) = &schema.items {
                    let item_example = Self::generate_example_from_schema(items_schema)?;
                    Ok(Value::Array(vec![item_example]))
                } else {
                    Ok(Value::Array(vec![]))
                }
            }
            Some("string") => {
                if !schema.enum_values.is_empty() {
                    Ok(schema.enum_values[0].clone())
                } else {
                    match schema.format.as_deref() {
                        Some("email") => Ok(Value::String("user@example.com".to_string())),
                        Some("uri") => Ok(Value::String("https://example.com".to_string())),
                        Some("date") => Ok(Value::String("2023-12-01".to_string())),
                        Some("date-time") => Ok(Value::String("2023-12-01T12:00:00Z".to_string())),
                        Some("uuid") => Ok(Value::String("123e4567-e89b-12d3-a456-426614174000".to_string())),
                        _ => Ok(Value::String("string".to_string())),
                    }
                }
            }
            Some("integer") => {
                match schema.format.as_deref() {
                    Some("int64") => Ok(Value::Number(serde_json::Number::from(42i64))),
                    _ => Ok(Value::Number(serde_json::Number::from(42i32))),
                }
            }
            Some("number") => {
                Ok(Value::Number(serde_json::Number::from_f64(std::f64::consts::PI).unwrap()))
            }
            Some("boolean") => Ok(Value::Bool(true)),
            _ => {
                if let Some(example) = &schema.example {
                    Ok(example.clone())
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    /// Extract operation summary from function name
    pub fn generate_operation_summary(method: &str, path: &str) -> String {
        let verb = method.to_lowercase();
        let resource = Self::extract_resource_from_path(path);

        match verb.as_str() {
            "get" => {
                if path.contains('{') {
                    format!("Get {}", resource)
                } else {
                    format!("List {}", Self::pluralize(&resource))
                }
            }
            "post" => format!("Create {}", resource),
            "put" => format!("Update {}", resource),
            "patch" => format!("Partially update {}", resource),
            "delete" => format!("Delete {}", resource),
            _ => format!("{} {}", verb, resource),
        }
    }

    /// Extract resource name from path
    fn extract_resource_from_path(path: &str) -> String {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        
        if let Some(last_part) = parts.last() {
            if last_part.starts_with('{') {
                // Path parameter, use previous part
                if parts.len() > 1 {
                    Self::singularize(parts[parts.len() - 2])
                } else {
                    "resource".to_string()
                }
            } else {
                Self::singularize(last_part)
            }
        } else {
            "resource".to_string()
        }
    }

    /// Simple singularization
    fn singularize(word: &str) -> String {
        if word.ends_with("ies") {
            word.trim_end_matches("ies").to_string() + "y"
        } else if word.ends_with('s') && !word.ends_with("ss") {
            word.trim_end_matches('s').to_string()
        } else {
            word.to_string()
        }
    }

    /// Simple pluralization
    fn pluralize(word: &str) -> String {
        if word.ends_with('y') {
            word.trim_end_matches('y').to_string() + "ies"
        } else if word.ends_with("s") || word.ends_with("sh") || word.ends_with("ch") {
            word.to_string() + "es"
        } else {
            word.to_string() + "s"
        }
    }
}

/// Output format for saving specifications
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json,
    Yaml,
}

/// Validation warning levels
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationLevel {
    Error,
    Warning,
    Info,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub message: String,
    pub level: ValidationLevel,
}

impl ValidationWarning {
    pub fn new(message: &str, level: ValidationLevel) -> Self {
        Self {
            message: message.to_string(),
            level,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specification::{ApiInfo, Schema};
    use std::collections::HashMap;

    #[test]
    fn test_operation_summary_generation() {
        assert_eq!(OpenApiUtils::generate_operation_summary("GET", "/users"), "List users");
        assert_eq!(OpenApiUtils::generate_operation_summary("GET", "/users/{id}"), "Get user");
        assert_eq!(OpenApiUtils::generate_operation_summary("POST", "/users"), "Create user");
        assert_eq!(OpenApiUtils::generate_operation_summary("PUT", "/users/{id}"), "Update user");
        assert_eq!(OpenApiUtils::generate_operation_summary("DELETE", "/users/{id}"), "Delete user");
    }

    #[test]
    fn test_resource_extraction() {
        assert_eq!(OpenApiUtils::extract_resource_from_path("/users"), "user");
        assert_eq!(OpenApiUtils::extract_resource_from_path("/users/{id}"), "user");
        assert_eq!(OpenApiUtils::extract_resource_from_path("/api/v1/posts/{id}/comments"), "comment");
        assert_eq!(OpenApiUtils::extract_resource_from_path("/"), "resource");
    }

    #[test]
    fn test_singularization() {
        assert_eq!(OpenApiUtils::singularize("users"), "user");
        assert_eq!(OpenApiUtils::singularize("posts"), "post");
        assert_eq!(OpenApiUtils::singularize("categories"), "category");
        assert_eq!(OpenApiUtils::singularize("companies"), "company");
        assert_eq!(OpenApiUtils::singularize("class"), "class"); // shouldn't change
    }

    #[test]
    fn test_pluralization() {
        assert_eq!(OpenApiUtils::pluralize("user"), "users");
        assert_eq!(OpenApiUtils::pluralize("post"), "posts");
        assert_eq!(OpenApiUtils::pluralize("category"), "categories");
        assert_eq!(OpenApiUtils::pluralize("company"), "companies");
        assert_eq!(OpenApiUtils::pluralize("class"), "classes");
    }

    #[test]
    fn test_example_generation() {
        let string_schema = Schema {
            schema_type: Some("string".to_string()),
            ..Default::default()
        };
        let example = OpenApiUtils::generate_example_from_schema(&string_schema).unwrap();
        assert_eq!(example, serde_json::Value::String("string".to_string()));

        let integer_schema = Schema {
            schema_type: Some("integer".to_string()),
            ..Default::default()
        };
        let example = OpenApiUtils::generate_example_from_schema(&integer_schema).unwrap();
        assert_eq!(example, serde_json::Value::Number(serde_json::Number::from(42)));
    }

    #[test]
    fn test_spec_validation() {
        let mut spec = OpenApiSpec::new("Test API", "1.0.0");
        spec.paths = HashMap::new();

        let warnings = OpenApiUtils::validate_spec(&spec).unwrap();
        
        // Should have warning about no paths
        assert!(warnings.iter().any(|w| w.message.contains("No paths defined")));
    }

    #[test]
    fn test_schema_reference_detection_accurate() {
        use crate::specification::*;

        // Create a spec with schemas and verify accurate detection
        let mut spec = OpenApiSpec::new("Test API", "1.0.0");
        
        // Add a User schema
        let user_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert("id".to_string(), Schema {
                    schema_type: Some("integer".to_string()),
                    ..Default::default()
                });
                props.insert("name".to_string(), Schema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                });
                props
            },
            required: vec!["id".to_string(), "name".to_string()],
            ..Default::default()
        };

        // Add an Address schema that references User
        let address_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert("street".to_string(), Schema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                });
                props.insert("owner".to_string(), Schema {
                    reference: Some("#/components/schemas/User".to_string()),
                    ..Default::default()
                });
                props
            },
            ..Default::default()
        };

        // Add an unused schema for testing
        let unused_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert("value".to_string(), Schema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                });
                props
            },
            ..Default::default()
        };

        // Set up components
        let mut components = Components::default();
        components.schemas.insert("User".to_string(), user_schema);
        components.schemas.insert("Address".to_string(), address_schema);
        components.schemas.insert("UnusedSchema".to_string(), unused_schema);
        spec.components = Some(components);

        // Test reference detection
        assert!(OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/User"));
        assert!(!OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/UnusedSchema"));
        assert!(!OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/NonExistent"));
    }

    #[test] 
    fn test_schema_reference_false_positive_prevention() {
        use crate::specification::*;

        // Create a spec where schema reference appears in description but not as actual reference
        let mut spec = OpenApiSpec::new("Test API", "1.0.0");

        // Add a schema with reference string in description (should NOT be detected as reference)
        let user_schema = Schema {
            schema_type: Some("object".to_string()),
            description: Some("This schema represents a user. See also #/components/schemas/User for details.".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert("name".to_string(), Schema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                });
                props
            },
            ..Default::default()
        };

        // Add an example with schema reference in the example value
        let example_schema = Schema {
            schema_type: Some("string".to_string()),
            example: Some(serde_json::Value::String("#/components/schemas/User".to_string())),
            ..Default::default()
        };

        let mut components = Components::default();
        components.schemas.insert("User".to_string(), user_schema);
        components.schemas.insert("Example".to_string(), example_schema);
        spec.components = Some(components);

        // The old string-based approach would incorrectly detect these as references
        // The new approach should correctly identify that User is not actually referenced
        assert!(!OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/User"));
        assert!(!OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/Example"));
    }

    #[test]
    fn test_schema_reference_in_operations() {
        use crate::specification::*;

        let mut spec = OpenApiSpec::new("Test API", "1.0.0");

        // Create a schema
        let user_schema = Schema {
            schema_type: Some("object".to_string()),
            ..Default::default()
        };

        // Create an operation that uses the schema in request body
        let request_body = RequestBody {
            description: Some("User data".to_string()),
            content: {
                let mut content = HashMap::new();
                content.insert("application/json".to_string(), MediaType {
                    schema: Some(Schema {
                        reference: Some("#/components/schemas/User".to_string()),
                        ..Default::default()
                    }),
                    example: None,
                    examples: HashMap::new(),
                });
                content
            },
            required: Some(true),
        };

        let operation = Operation {
            request_body: Some(request_body),
            responses: {
                let mut responses = HashMap::new();
                responses.insert("200".to_string(), Response {
                    description: "Success".to_string(),
                    content: {
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), MediaType {
                            schema: Some(Schema {
                                reference: Some("#/components/schemas/User".to_string()),
                                ..Default::default()
                            }),
                            example: None,
                            examples: HashMap::new(),
                        });
                        content
                    },
                    headers: HashMap::new(),
                    links: HashMap::new(),
                });
                responses
            },
            ..Default::default()
        };

        let path_item = PathItem {
            post: Some(operation),
            ..Default::default()
        };

        spec.paths.insert("/users".to_string(), path_item);

        let mut components = Components::default();
        components.schemas.insert("User".to_string(), user_schema);
        spec.components = Some(components);

        // User schema should be detected as referenced in the operation
        assert!(OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/User"));
    }

    #[test]
    fn test_schema_reference_in_nested_schemas() {
        use crate::specification::*;

        let mut spec = OpenApiSpec::new("Test API", "1.0.0");

        // Create deeply nested schema structure
        let user_schema = Schema {
            schema_type: Some("object".to_string()),
            ..Default::default()
        };

        let profile_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert("user".to_string(), Schema {
                    reference: Some("#/components/schemas/User".to_string()),
                    ..Default::default()
                });
                props
            },
            ..Default::default()
        };

        let response_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: {
                let mut props = HashMap::new();
                props.insert("data".to_string(), Schema {
                    schema_type: Some("array".to_string()),
                    items: Some(Box::new(Schema {
                        reference: Some("#/components/schemas/Profile".to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                });
                props
            },
            ..Default::default()
        };

        let mut components = Components::default();
        components.schemas.insert("User".to_string(), user_schema);
        components.schemas.insert("Profile".to_string(), profile_schema);
        components.schemas.insert("Response".to_string(), response_schema);
        spec.components = Some(components);

        // Both User and Profile should be detected as referenced
        assert!(OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/User"));
        assert!(OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/Profile"));
    }

    #[test]
    fn test_schema_reference_in_composition() {
        use crate::specification::*;

        let mut spec = OpenApiSpec::new("Test API", "1.0.0");

        let base_schema = Schema {
            schema_type: Some("object".to_string()),
            ..Default::default()
        };

        let extended_schema = Schema {
            all_of: vec![
                Schema {
                    reference: Some("#/components/schemas/Base".to_string()),
                    ..Default::default()
                },
                Schema {
                    schema_type: Some("object".to_string()),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert("extra".to_string(), Schema {
                            schema_type: Some("string".to_string()),
                            ..Default::default()
                        });
                        props
                    },
                    ..Default::default()
                }
            ],
            ..Default::default()
        };

        let mut components = Components::default();
        components.schemas.insert("Base".to_string(), base_schema);
        components.schemas.insert("Extended".to_string(), extended_schema);
        spec.components = Some(components);

        // Base schema should be detected as referenced in allOf composition
        assert!(OpenApiUtils::is_schema_referenced(&spec, "#/components/schemas/Base"));
    }
}