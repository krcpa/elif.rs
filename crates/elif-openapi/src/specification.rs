use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete OpenAPI 3.0 specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    /// OpenAPI specification version
    pub openapi: String,
    
    /// API metadata
    pub info: ApiInfo,
    
    /// Server URLs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub servers: Vec<Server>,
    
    /// API paths and operations
    #[serde(default)]
    pub paths: HashMap<String, PathItem>,
    
    /// Reusable components
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    
    /// Global security requirements
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub security: Vec<SecurityRequirement>,
    
    /// Tags for grouping operations
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<Tag>,
    
    /// External documentation
    #[serde(rename = "externalDocs", skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
}

/// API metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    /// API title
    pub title: String,
    
    /// API description  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Terms of service URL
    #[serde(rename = "termsOfService", skip_serializing_if = "Option::is_none")]
    pub terms_of_service: Option<String>,
    
    /// Contact information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    
    /// License information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    
    /// API version
    pub version: String,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Server URL
    pub url: String,
    
    /// Server description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Variable substitutions for server URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, ServerVariable>>,
}

/// Server URL variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    /// Default value
    pub default: String,
    
    /// Allowed values
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Path item containing operations for a specific path
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathItem {
    /// Optional summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// GET operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    
    /// PUT operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    
    /// POST operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    
    /// DELETE operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    
    /// OPTIONS operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Operation>,
    
    /// HEAD operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<Operation>,
    
    /// PATCH operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    
    /// TRACE operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Operation>,
    
    /// Common parameters for all operations on this path
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<Parameter>,
}

/// HTTP operation (GET, POST, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Operation {
    /// Tags for grouping
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    
    /// Short summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    
    /// Long description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// External documentation
    #[serde(rename = "externalDocs", skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
    
    /// Unique operation ID
    #[serde(rename = "operationId", skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    
    /// Parameters
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<Parameter>,
    
    /// Request body
    #[serde(rename = "requestBody", skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    
    /// Possible responses
    #[serde(default)]
    pub responses: HashMap<String, Response>,
    
    /// Security requirements
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub security: Vec<SecurityRequirement>,
    
    /// Servers specific to this operation
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub servers: Vec<Server>,
    
    /// Deprecated flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
}

/// Parameter for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    
    /// Parameter location (query, header, path, cookie)
    #[serde(rename = "in")]
    pub location: String,
    
    /// Parameter description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Required flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    
    /// Deprecated flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    
    /// Schema defining the parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    
    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

/// Request body specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Media type content
    pub content: HashMap<String, MediaType>,
    
    /// Required flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Response specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Description
    pub description: String,
    
    /// Headers
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub headers: HashMap<String, Header>,
    
    /// Content
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub content: HashMap<String, MediaType>,
    
    /// Links to other operations
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub links: HashMap<String, Link>,
}

/// Header specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
}

/// Media type specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    /// Schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    
    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    
    /// Multiple examples
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub examples: HashMap<String, Example>,
}

/// Example specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(rename = "externalValue", skip_serializing_if = "Option::is_none")]
    pub external_value: Option<String>,
}

/// Link specification
#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct Link {
    #[serde(rename = "operationRef", skip_serializing_if = "Option::is_none")]
    pub operation_ref: Option<String>,
    #[serde(rename = "operationId", skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(rename = "requestBody", skip_serializing_if = "Option::is_none")]
    pub request_body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Schema for data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    /// Data type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    
    /// Format specifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    
    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    
    /// Nullable flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    
    /// Properties for object types
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, Schema>,
    
    /// Required properties
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    
    /// Additional properties schema
    #[serde(rename = "additionalProperties", skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<Schema>>,
    
    /// Items schema for arrays
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    
    /// Enum values
    #[serde(rename = "enum", skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<serde_json::Value>,
    
    /// Reference to another schema
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    
    /// AllOf composition
    #[serde(rename = "allOf", skip_serializing_if = "Vec::is_empty")]
    pub all_of: Vec<Schema>,
    
    /// AnyOf composition
    #[serde(rename = "anyOf", skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<Schema>,
    
    /// OneOf composition
    #[serde(rename = "oneOf", skip_serializing_if = "Vec::is_empty")]
    pub one_of: Vec<Schema>,
    
    /// Validation: minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    
    /// Validation: maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    
    /// Validation: minimum length
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    
    /// Validation: maximum length
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    
    /// Validation: pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// Reusable components
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Components {
    /// Reusable schemas
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub schemas: HashMap<String, Schema>,
    
    /// Reusable responses
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub responses: HashMap<String, Response>,
    
    /// Reusable parameters
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub parameters: HashMap<String, Parameter>,
    
    /// Reusable examples
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub examples: HashMap<String, Example>,
    
    /// Reusable request bodies
    #[serde(rename = "requestBodies", skip_serializing_if = "HashMap::is_empty", default)]
    pub request_bodies: HashMap<String, RequestBody>,
    
    /// Reusable headers
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub headers: HashMap<String, Header>,
    
    /// Security schemes
    #[serde(rename = "securitySchemes", skip_serializing_if = "HashMap::is_empty", default)]
    pub security_schemes: HashMap<String, SecurityScheme>,
    
    /// Reusable links
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub links: HashMap<String, Link>,
}

/// Security scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SecurityScheme {
    #[serde(rename = "apiKey")]
    ApiKey {
        name: String,
        #[serde(rename = "in")]
        location: String,
    },
    #[serde(rename = "http")]
    Http {
        scheme: String,
        #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
        bearer_format: Option<String>,
    },
    #[serde(rename = "oauth2")]
    OAuth2 {
        flows: OAuth2Flows,
    },
    #[serde(rename = "openIdConnect")]
    OpenIdConnect {
        #[serde(rename = "openIdConnectUrl")]
        open_id_connect_url: String,
    },
}

/// OAuth2 flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Flows {
    #[serde(rename = "implicit", skip_serializing_if = "Option::is_none")]
    pub implicit: Option<OAuth2Flow>,
    #[serde(rename = "password", skip_serializing_if = "Option::is_none")]
    pub password: Option<OAuth2Flow>,
    #[serde(rename = "clientCredentials", skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<OAuth2Flow>,
    #[serde(rename = "authorizationCode", skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<OAuth2Flow>,
}

/// OAuth2 flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Flow {
    #[serde(rename = "authorizationUrl", skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(rename = "tokenUrl", skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    pub scopes: HashMap<String, String>,
}

/// Security requirement
pub type SecurityRequirement = HashMap<String, Vec<String>>;

/// Tag for grouping operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "externalDocs", skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
}

/// External documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl OpenApiSpec {
    /// Create a new OpenAPI specification
    pub fn new(title: &str, version: &str) -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: ApiInfo {
                title: title.to_string(),
                description: None,
                terms_of_service: None,
                contact: None,
                license: None,
                version: version.to_string(),
            },
            servers: Vec::new(),
            paths: HashMap::new(),
            components: None,
            security: Vec::new(),
            tags: Vec::new(),
            external_docs: None,
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            title: None,
            schema_type: None,
            format: None,
            description: None,
            default: None,
            example: None,
            nullable: None,
            properties: HashMap::new(),
            required: Vec::new(),
            additional_properties: None,
            items: None,
            enum_values: Vec::new(),
            reference: None,
            all_of: Vec::new(),
            any_of: Vec::new(),
            one_of: Vec::new(),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
        }
    }
}