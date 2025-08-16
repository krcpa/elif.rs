use crate::{
    config::OpenApiConfig,
    error::{OpenApiError, OpenApiResult},
    schema::{SchemaConfig, SchemaGenerator},
    specification::*,
};
use elif_http::routing::{ElifRouter, RouteInfo};
use std::collections::HashMap;

/// Main OpenAPI specification generator
pub struct OpenApiGenerator {
    /// Configuration
    config: OpenApiConfig,
    /// Schema generator
    schema_generator: SchemaGenerator,
    /// Generated specification
    spec: Option<OpenApiSpec>,
}

/// Route information for OpenAPI generation
#[derive(Debug, Clone)]
pub struct RouteMetadata {
    /// HTTP method
    pub method: String,
    /// Path pattern
    pub path: String,
    /// Operation summary
    pub summary: Option<String>,
    /// Operation description  
    pub description: Option<String>,
    /// Operation ID
    pub operation_id: Option<String>,
    /// Tags for grouping
    pub tags: Vec<String>,
    /// Request body schema
    pub request_schema: Option<String>,
    /// Response schemas by status code
    pub response_schemas: HashMap<String, String>,
    /// Parameters
    pub parameters: Vec<ParameterInfo>,
    /// Security requirements
    pub security: Vec<String>,
    /// Deprecated flag
    pub deprecated: bool,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Parameter location (path, query, header, cookie)
    pub location: String,
    /// Parameter type
    pub param_type: String,
    /// Description
    pub description: Option<String>,
    /// Required flag
    pub required: bool,
    /// Example value
    pub example: Option<serde_json::Value>,
}

impl OpenApiGenerator {
    /// Create a new OpenAPI generator
    pub fn new(config: OpenApiConfig) -> Self {
        let schema_config = SchemaConfig::new()
            .with_nullable_optional(config.nullable_optional)
            .with_examples(config.include_examples);

        Self {
            schema_generator: SchemaGenerator::new(schema_config),
            spec: None,
            config,
        }
    }

    /// Generate OpenAPI specification from route metadata
    pub fn generate(&mut self, routes: &[RouteMetadata]) -> OpenApiResult<&OpenApiSpec> {
        // Initialize specification
        let mut spec = OpenApiSpec {
            openapi: self.config.openapi_version.clone(),
            info: self.convert_api_info(),
            servers: self.convert_servers(),
            paths: HashMap::new(),
            components: None,
            security: self.convert_security_requirements(),
            tags: self.convert_tags(),
            external_docs: self.config.external_docs.as_ref().map(|ed| ExternalDocumentation {
                url: ed.url.clone(),
                description: ed.description.clone(),
            }),
        };

        // Generate paths from routes
        for route in routes {
            self.process_route(&mut spec, route)?;
        }

        // Generate components with schemas
        self.generate_components(&mut spec)?;

        self.spec = Some(spec);
        Ok(self.spec.as_ref().unwrap())
    }

    /// Process a single route and add to specification
    fn process_route(&mut self, spec: &mut OpenApiSpec, route: &RouteMetadata) -> OpenApiResult<()> {
        // Get or create path item
        let path_item = spec.paths.entry(route.path.clone()).or_insert_with(|| PathItem {
            summary: None,
            description: None,
            get: None,
            put: None,
            post: None,
            delete: None,
            options: None,
            head: None,
            patch: None,
            trace: None,
            parameters: Vec::new(),
        });

        // Create operation
        let operation = self.create_operation(route)?;

        // Add operation to appropriate method
        match route.method.to_uppercase().as_str() {
            "GET" => path_item.get = Some(operation),
            "POST" => path_item.post = Some(operation),
            "PUT" => path_item.put = Some(operation),
            "DELETE" => path_item.delete = Some(operation),
            "PATCH" => path_item.patch = Some(operation),
            "OPTIONS" => path_item.options = Some(operation),
            "HEAD" => path_item.head = Some(operation),
            "TRACE" => path_item.trace = Some(operation),
            _ => {
                return Err(OpenApiError::route_discovery_error(
                    format!("Unsupported HTTP method: {}", route.method)
                ));
            }
        }

        Ok(())
    }

    /// Create operation from route metadata
    fn create_operation(&mut self, route: &RouteMetadata) -> OpenApiResult<Operation> {
        // Generate parameters
        let parameters = route.parameters
            .iter()
            .map(|param| self.create_parameter(param))
            .collect::<OpenApiResult<Vec<_>>>()?;

        // Generate request body
        let request_body = if let Some(request_schema) = &route.request_schema {
            Some(self.create_request_body(request_schema)?)
        } else {
            None
        };

        // Generate responses
        let responses = self.create_responses(&route.response_schemas)?;

        // Generate security requirements
        let security = if route.security.is_empty() {
            Vec::new()
        } else {
            route.security
                .iter()
                .map(|scheme| {
                    let mut req = HashMap::new();
                    req.insert(scheme.clone(), Vec::new());
                    req
                })
                .collect()
        };

        Ok(Operation {
            tags: route.tags.clone(),
            summary: route.summary.clone(),
            description: route.description.clone(),
            external_docs: None,
            operation_id: route.operation_id.clone(),
            parameters,
            request_body,
            responses,
            security,
            servers: Vec::new(),
            deprecated: if route.deprecated { Some(true) } else { None },
        })
    }

    /// Create parameter from parameter info
    fn create_parameter(&mut self, param: &ParameterInfo) -> OpenApiResult<Parameter> {
        // Generate schema for parameter type
        let schema = self.schema_generator.generate_schema(&param.param_type)?;

        Ok(Parameter {
            name: param.name.clone(),
            location: param.location.clone(),
            description: param.description.clone(),
            required: Some(param.required),
            deprecated: None,
            schema: Some(schema),
            example: param.example.clone(),
        })
    }

    /// Create request body from schema name
    fn create_request_body(&mut self, schema_name: &str) -> OpenApiResult<RequestBody> {
        let schema = Schema {
            reference: Some(format!("#/components/schemas/{}", schema_name)),
            ..Default::default()
        };

        let mut content = HashMap::new();
        content.insert("application/json".to_string(), MediaType {
            schema: Some(schema),
            example: None,
            examples: HashMap::new(),
        });

        Ok(RequestBody {
            description: Some(format!("Request payload for {}", schema_name)),
            content,
            required: Some(true),
        })
    }

    /// Create responses from schema mappings
    fn create_responses(&mut self, response_schemas: &HashMap<String, String>) -> OpenApiResult<HashMap<String, Response>> {
        let mut responses = HashMap::new();

        // Default success response if none specified
        if response_schemas.is_empty() {
            responses.insert("200".to_string(), Response {
                description: "Successful operation".to_string(),
                headers: HashMap::new(),
                content: HashMap::new(),
                links: HashMap::new(),
            });
        } else {
            for (status_code, schema_name) in response_schemas {
                let schema = Schema {
                    reference: Some(format!("#/components/schemas/{}", schema_name)),
                    ..Default::default()
                };

                let mut content = HashMap::new();
                content.insert("application/json".to_string(), MediaType {
                    schema: Some(schema),
                    example: None,
                    examples: HashMap::new(),
                });

                let description = match status_code.as_str() {
                    "200" => "OK",
                    "201" => "Created",
                    "204" => "No Content",
                    "400" => "Bad Request",
                    "401" => "Unauthorized",
                    "403" => "Forbidden",
                    "404" => "Not Found",
                    "422" => "Unprocessable Entity",
                    "500" => "Internal Server Error",
                    _ => "Response",
                };

                responses.insert(status_code.clone(), Response {
                    description: description.to_string(),
                    headers: HashMap::new(),
                    content,
                    links: HashMap::new(),
                });
            }
        }

        Ok(responses)
    }

    /// Generate components section with schemas
    fn generate_components(&mut self, spec: &mut OpenApiSpec) -> OpenApiResult<()> {
        let schemas = self.schema_generator.get_schemas().clone();
        let security_schemes = self.convert_security_schemes();

        if !schemas.is_empty() || !security_schemes.is_empty() {
            spec.components = Some(Components {
                schemas,
                responses: HashMap::new(),
                parameters: HashMap::new(),
                examples: HashMap::new(),
                request_bodies: HashMap::new(),
                headers: HashMap::new(),
                security_schemes,
                links: HashMap::new(),
            });
        }

        Ok(())
    }

    /// Convert configuration info to specification info
    fn convert_api_info(&self) -> ApiInfo {
        ApiInfo {
            title: self.config.info.title.clone(),
            description: self.config.info.description.clone(),
            terms_of_service: self.config.info.terms_of_service.clone(),
            contact: self.config.info.contact.as_ref().map(|c| Contact {
                name: c.name.clone(),
                url: c.url.clone(),
                email: c.email.clone(),
            }),
            license: self.config.info.license.as_ref().map(|l| License {
                name: l.name.clone(),
                url: l.url.clone(),
            }),
            version: self.config.info.version.clone(),
        }
    }

    /// Convert server configurations
    fn convert_servers(&self) -> Vec<Server> {
        self.config.servers
            .iter()
            .map(|s| Server {
                url: s.url.clone(),
                description: s.description.clone(),
                variables: s.variables.as_ref().map(|vars| {
                    vars.iter()
                        .map(|(k, v)| (k.clone(), ServerVariable {
                            default: v.default.clone(),
                            enum_values: v.r#enum.clone(),
                            description: v.description.clone(),
                        }))
                        .collect()
                }),
            })
            .collect()
    }

    /// Convert security schemes
    fn convert_security_schemes(&self) -> HashMap<String, SecurityScheme> {
        self.config.security_schemes
            .iter()
            .map(|(name, scheme)| {
                let security_scheme = match scheme {
                    crate::config::SecurityScheme::Http { scheme, bearer_format } => {
                        SecurityScheme::Http {
                            scheme: scheme.clone(),
                            bearer_format: bearer_format.clone(),
                        }
                    },
                    crate::config::SecurityScheme::ApiKey { name, r#in } => {
                        SecurityScheme::ApiKey {
                            name: name.clone(),
                            location: r#in.clone(),
                        }
                    },
                    crate::config::SecurityScheme::OAuth2 { flows } => {
                        SecurityScheme::OAuth2 {
                            flows: OAuth2Flows {
                                implicit: flows.implicit.as_ref().map(|f| OAuth2Flow {
                                    authorization_url: f.authorization_url.clone(),
                                    token_url: f.token_url.clone(),
                                    refresh_url: f.refresh_url.clone(),
                                    scopes: f.scopes.clone(),
                                }),
                                password: flows.password.as_ref().map(|f| OAuth2Flow {
                                    authorization_url: f.authorization_url.clone(),
                                    token_url: f.token_url.clone(),
                                    refresh_url: f.refresh_url.clone(),
                                    scopes: f.scopes.clone(),
                                }),
                                client_credentials: flows.client_credentials.as_ref().map(|f| OAuth2Flow {
                                    authorization_url: f.authorization_url.clone(),
                                    token_url: f.token_url.clone(),
                                    refresh_url: f.refresh_url.clone(),
                                    scopes: f.scopes.clone(),
                                }),
                                authorization_code: flows.authorization_code.as_ref().map(|f| OAuth2Flow {
                                    authorization_url: f.authorization_url.clone(),
                                    token_url: f.token_url.clone(),
                                    refresh_url: f.refresh_url.clone(),
                                    scopes: f.scopes.clone(),
                                }),
                            },
                        }
                    },
                    crate::config::SecurityScheme::OpenIdConnect { open_id_connect_url } => {
                        SecurityScheme::OpenIdConnect {
                            open_id_connect_url: open_id_connect_url.clone(),
                        }
                    },
                };
                (name.clone(), security_scheme)
            })
            .collect()
    }

    /// Convert global security requirements
    fn convert_security_requirements(&self) -> Vec<SecurityRequirement> {
        // Default to no global security (will be set per operation)
        Vec::new()
    }

    /// Convert tags
    fn convert_tags(&self) -> Vec<Tag> {
        self.config.tags
            .iter()
            .map(|t| Tag {
                name: t.name.clone(),
                description: t.description.clone(),
                external_docs: t.external_docs.as_ref().map(|ed| ExternalDocumentation {
                    url: ed.url.clone(),
                    description: ed.description.clone(),
                }),
            })
            .collect()
    }

    /// Export specification as JSON
    pub fn export_json(&self, pretty: bool) -> OpenApiResult<String> {
        let spec = self.spec.as_ref().ok_or_else(|| {
            OpenApiError::generic("No specification generated yet. Call generate() first.")
        })?;

        if pretty {
            serde_json::to_string_pretty(spec).map_err(OpenApiError::from)
        } else {
            serde_json::to_string(spec).map_err(OpenApiError::from)
        }
    }

    /// Export specification as YAML
    pub fn export_yaml(&self) -> OpenApiResult<String> {
        let spec = self.spec.as_ref().ok_or_else(|| {
            OpenApiError::generic("No specification generated yet. Call generate() first.")
        })?;

        serde_yaml::to_string(spec).map_err(OpenApiError::from)
    }

    /// Get the generated specification
    pub fn specification(&self) -> Option<&OpenApiSpec> {
        self.spec.as_ref()
    }

    /// Validate the generated specification
    pub fn validate(&self) -> OpenApiResult<()> {
        let _spec = self.spec.as_ref().ok_or_else(|| {
            OpenApiError::validation_error("No specification generated yet. Call generate() first.")
        })?;

        // Basic validation
        // TODO: Add more comprehensive validation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OpenApiConfig;

    #[test]
    fn test_generator_creation() {
        let config = OpenApiConfig::default();
        let generator = OpenApiGenerator::new(config);
        assert!(generator.spec.is_none());
    }

    #[test]
    fn test_empty_routes_generation() {
        let config = OpenApiConfig::new("Test API", "1.0.0");
        let mut generator = OpenApiGenerator::new(config);
        
        let routes = vec![];
        let spec = generator.generate(&routes).unwrap();
        
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
        assert!(spec.paths.is_empty());
    }

    #[test]
    fn test_basic_route_generation() {
        let config = OpenApiConfig::new("Test API", "1.0.0");
        let mut generator = OpenApiGenerator::new(config);
        
        let routes = vec![RouteMetadata {
            method: "GET".to_string(),
            path: "/users".to_string(),
            summary: Some("List users".to_string()),
            description: Some("Get all users".to_string()),
            operation_id: Some("listUsers".to_string()),
            tags: vec!["Users".to_string()],
            request_schema: None,
            response_schemas: HashMap::new(),
            parameters: Vec::new(),
            security: Vec::new(),
            deprecated: false,
        }];
        
        let spec = generator.generate(&routes).unwrap();
        
        assert_eq!(spec.paths.len(), 1);
        assert!(spec.paths.contains_key("/users"));
        
        let path_item = &spec.paths["/users"];
        assert!(path_item.get.is_some());
        
        let operation = path_item.get.as_ref().unwrap();
        assert_eq!(operation.summary, Some("List users".to_string()));
        assert_eq!(operation.tags, vec!["Users".to_string()]);
    }
}