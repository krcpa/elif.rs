#[cfg(test)]
pub mod test_utils {
    use crate::{
        generator::{RouteMetadata, ParameterInfo},
        specification::*,
        endpoints::{ControllerInfo, EndpointMetadata},
    };
    use std::collections::HashMap;

    /// Create a test OpenAPI specification
    #[allow(dead_code)]
    pub fn create_test_spec() -> OpenApiSpec {
        let mut spec = OpenApiSpec::new("Test API", "1.0.0");
        
        spec.info.description = Some("A test API for OpenAPI generation".to_string());
        
        // Add a server
        spec.servers.push(Server {
            url: "http://localhost:3000".to_string(),
            description: Some("Development server".to_string()),
            variables: None,
        });

        // Add a tag
        spec.tags.push(Tag {
            name: "Users".to_string(),
            description: Some("User management operations".to_string()),
            external_docs: None,
        });

        spec
    }

    /// Create test route metadata
    #[allow(dead_code)]
    pub fn create_test_route() -> RouteMetadata {
        RouteMetadata {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: Some("Get user by ID".to_string()),
            description: Some("Retrieve a specific user by their ID".to_string()),
            operation_id: Some("getUserById".to_string()),
            tags: vec!["Users".to_string()],
            request_schema: None,
            response_schemas: {
                let mut schemas = HashMap::new();
                schemas.insert("200".to_string(), "User".to_string());
                schemas.insert("404".to_string(), "Error".to_string());
                schemas
            },
            parameters: vec![
                ParameterInfo {
                    name: "id".to_string(),
                    location: "path".to_string(),
                    param_type: "i32".to_string(),
                    description: Some("User ID".to_string()),
                    required: true,
                    example: Some(serde_json::json!(123)),
                }
            ],
            security: vec!["bearerAuth".to_string()],
            deprecated: false,
        }
    }

    /// Create test controller info
    #[allow(dead_code)]
    pub fn create_test_controller() -> ControllerInfo {
        let endpoint = EndpointMetadata::new("show", "GET", "/users/{id}")
            .with_return_type("User")
            .with_documentation("Get user by ID");

        ControllerInfo::new("Users")
            .with_base_path("/api/v1")
            .add_endpoint(endpoint)
    }

    /// Create test schema
    #[allow(dead_code)]
    pub fn create_test_schema() -> Schema {
        let mut properties = HashMap::new();
        
        properties.insert("id".to_string(), Schema {
            schema_type: Some("integer".to_string()),
            format: Some("int32".to_string()),
            description: Some("Unique user identifier".to_string()),
            ..Default::default()
        });

        properties.insert("name".to_string(), Schema {
            schema_type: Some("string".to_string()),
            description: Some("User's full name".to_string()),
            max_length: Some(100),
            ..Default::default()
        });

        properties.insert("email".to_string(), Schema {
            schema_type: Some("string".to_string()),
            format: Some("email".to_string()),
            description: Some("User's email address".to_string()),
            ..Default::default()
        });

        properties.insert("created_at".to_string(), Schema {
            schema_type: Some("string".to_string()),
            format: Some("date-time".to_string()),
            description: Some("Account creation timestamp".to_string()),
            ..Default::default()
        });

        Schema {
            title: Some("User".to_string()),
            schema_type: Some("object".to_string()),
            description: Some("User account information".to_string()),
            properties,
            required: vec!["id".to_string(), "name".to_string(), "email".to_string()],
            ..Default::default()
        }
    }

    /// Create test operation
    #[allow(dead_code)]
    pub fn create_test_operation() -> Operation {
        let mut responses = HashMap::new();
        
        responses.insert("200".to_string(), Response {
            description: "Successful response".to_string(),
            headers: HashMap::new(),
            content: {
                let mut content = HashMap::new();
                content.insert("application/json".to_string(), MediaType {
                    schema: Some(Schema {
                        reference: Some("#/components/schemas/User".to_string()),
                        ..Default::default()
                    }),
                    example: Some(serde_json::json!({
                        "id": 123,
                        "name": "John Doe",
                        "email": "john@example.com"
                    })),
                    examples: HashMap::new(),
                });
                content
            },
            links: HashMap::new(),
        });

        responses.insert("404".to_string(), Response {
            description: "User not found".to_string(),
            headers: HashMap::new(),
            content: HashMap::new(),
            links: HashMap::new(),
        });

        Operation {
            tags: vec!["Users".to_string()],
            summary: Some("Get user by ID".to_string()),
            description: Some("Retrieve a specific user by their unique identifier".to_string()),
            operation_id: Some("getUserById".to_string()),
            parameters: vec![
                Parameter {
                    name: "id".to_string(),
                    location: "path".to_string(),
                    description: Some("User ID".to_string()),
                    required: Some(true),
                    deprecated: None,
                    schema: Some(Schema {
                        schema_type: Some("integer".to_string()),
                        format: Some("int32".to_string()),
                        ..Default::default()
                    }),
                    example: Some(serde_json::json!(123)),
                }
            ],
            request_body: None,
            responses,
            security: vec![{
                let mut security = HashMap::new();
                security.insert("bearerAuth".to_string(), Vec::new());
                security
            }],
            servers: Vec::new(),
            external_docs: None,
            deprecated: None,
        }
    }

    /// Create test components
    #[allow(dead_code)]
    pub fn create_test_components() -> Components {
        let mut schemas = HashMap::new();
        schemas.insert("User".to_string(), create_test_schema());

        let mut security_schemes = HashMap::new();
        security_schemes.insert("bearerAuth".to_string(), SecurityScheme::Http {
            scheme: "bearer".to_string(),
            bearer_format: Some("JWT".to_string()),
        });

        Components {
            schemas,
            responses: HashMap::new(),
            parameters: HashMap::new(),
            examples: HashMap::new(),
            request_bodies: HashMap::new(),
            headers: HashMap::new(),
            security_schemes,
            links: HashMap::new(),
        }
    }

    /// Assert that an OpenAPI spec is valid
    #[allow(dead_code)]
    pub fn assert_valid_spec(spec: &OpenApiSpec) {
        assert!(!spec.info.title.is_empty());
        assert!(!spec.info.version.is_empty());
        assert_eq!(spec.openapi, "3.0.3");
    }

    /// Assert that a route metadata is valid
    #[allow(dead_code)]
    pub fn assert_valid_route(route: &RouteMetadata) {
        assert!(!route.method.is_empty());
        assert!(!route.path.is_empty());
        assert!(route.path.starts_with('/'));
    }
}