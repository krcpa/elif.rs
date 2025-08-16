use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for OpenAPI specification generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiConfig {
    /// OpenAPI specification version (should be "3.0.3")
    pub openapi_version: String,
    
    /// API information
    pub info: ApiInfo,
    
    /// Server configurations
    pub servers: Vec<ServerConfig>,
    
    /// Global security schemes
    pub security_schemes: HashMap<String, SecurityScheme>,
    
    /// Global tags for grouping operations
    pub tags: Vec<TagConfig>,
    
    /// External documentation
    pub external_docs: Option<ExternalDocs>,
    
    /// Whether to include example values in schemas
    pub include_examples: bool,
    
    /// Whether to generate nullable fields for Option<T>
    pub nullable_optional: bool,
    
    /// Custom schema mappings for specific types
    pub custom_schemas: HashMap<String, String>,
    
    /// Export settings
    pub export: ExportConfig,
}

/// API information section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    /// API title
    pub title: String,
    
    /// API description
    pub description: Option<String>,
    
    /// API version
    pub version: String,
    
    /// Terms of service URL
    pub terms_of_service: Option<String>,
    
    /// Contact information
    pub contact: Option<Contact>,
    
    /// License information
    pub license: Option<License>,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: String,
    pub description: Option<String>,
    pub variables: Option<HashMap<String, ServerVariable>>,
}

/// Server variable for parameterized server URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    pub default: String,
    pub description: Option<String>,
    pub r#enum: Option<Vec<String>>,
}

/// Security scheme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    #[serde(rename = "http")]
    Http {
        scheme: String,
        bearer_format: Option<String>,
    },
    #[serde(rename = "apiKey")]
    ApiKey {
        name: String,
        r#in: String,
    },
    #[serde(rename = "oauth2")]
    OAuth2 {
        flows: OAuth2Flows,
    },
    #[serde(rename = "openIdConnect")]
    OpenIdConnect {
        open_id_connect_url: String,
    },
}

/// OAuth2 flows configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Flows {
    pub authorization_code: Option<OAuth2Flow>,
    pub implicit: Option<OAuth2Flow>,
    pub password: Option<OAuth2Flow>,
    pub client_credentials: Option<OAuth2Flow>,
}

/// Individual OAuth2 flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Flow {
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub refresh_url: Option<String>,
    pub scopes: HashMap<String, String>,
}

/// Tag configuration for grouping operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagConfig {
    pub name: String,
    pub description: Option<String>,
    pub external_docs: Option<ExternalDocs>,
}

/// External documentation reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    pub url: String,
    pub description: Option<String>,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Output formats to generate
    pub formats: Vec<ExportFormat>,
    
    /// Whether to validate generated specifications
    pub validate: bool,
    
    /// Pretty print JSON output
    pub pretty_print: bool,
}

/// Available export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Yaml,
    Postman,
    Insomnia,
}

impl Default for OpenApiConfig {
    fn default() -> Self {
        Self {
            openapi_version: "3.0.3".to_string(),
            info: ApiInfo {
                title: "API Documentation".to_string(),
                description: Some("Auto-generated API documentation".to_string()),
                version: "1.0.0".to_string(),
                terms_of_service: None,
                contact: None,
                license: Some(License {
                    name: "MIT".to_string(),
                    url: Some("https://opensource.org/licenses/MIT".to_string()),
                }),
            },
            servers: vec![ServerConfig {
                url: "http://localhost:3000".to_string(),
                description: Some("Development server".to_string()),
                variables: None,
            }],
            security_schemes: {
                let mut schemes = HashMap::new();
                schemes.insert("bearerAuth".to_string(), SecurityScheme::Http {
                    scheme: "bearer".to_string(),
                    bearer_format: Some("JWT".to_string()),
                });
                schemes
            },
            tags: Vec::new(),
            external_docs: None,
            include_examples: true,
            nullable_optional: true,
            custom_schemas: HashMap::new(),
            export: ExportConfig {
                formats: vec![ExportFormat::Json, ExportFormat::Yaml],
                validate: true,
                pretty_print: true,
            },
        }
    }
}

impl OpenApiConfig {
    /// Create a new configuration with custom API info
    pub fn new(title: &str, version: &str) -> Self {
        let mut config = Self::default();
        config.info.title = title.to_string();
        config.info.version = version.to_string();
        config
    }

    /// Add a server configuration
    pub fn add_server(mut self, url: &str, description: Option<&str>) -> Self {
        self.servers.push(ServerConfig {
            url: url.to_string(),
            description: description.map(|s| s.to_string()),
            variables: None,
        });
        self
    }

    /// Add a security scheme
    pub fn add_security_scheme(mut self, name: &str, scheme: SecurityScheme) -> Self {
        self.security_schemes.insert(name.to_string(), scheme);
        self
    }

    /// Add a tag
    pub fn add_tag(mut self, name: &str, description: Option<&str>) -> Self {
        self.tags.push(TagConfig {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            external_docs: None,
        });
        self
    }
}