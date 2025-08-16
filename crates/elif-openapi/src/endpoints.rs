/*!
Endpoint discovery and analysis for OpenAPI generation.

This module provides functionality to discover API endpoints from elif.rs framework
components and extract their metadata for documentation generation.
*/

use crate::{
    error::{OpenApiError, OpenApiResult},
    generator::{RouteMetadata, ParameterInfo},
};
use regex::Regex;
use std::collections::HashMap;

/// Endpoint metadata extracted from framework components
#[derive(Debug, Clone)]
pub struct EndpointMetadata {
    /// Controller name
    pub controller: String,
    /// Method name
    pub method: String,
    /// HTTP verb
    pub verb: String,
    /// Path pattern
    pub path: String,
    /// Documentation comments
    pub documentation: Option<String>,
    /// Parameters
    pub parameters: Vec<EndpointParameter>,
    /// Return type
    pub return_type: Option<String>,
    /// Attributes/annotations
    pub attributes: HashMap<String, String>,
}

/// Parameter extracted from endpoint
#[derive(Debug, Clone)]
pub struct EndpointParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
    /// Parameter source (path, query, body, header)
    pub source: ParameterSource,
    /// Optional flag
    pub optional: bool,
    /// Documentation
    pub documentation: Option<String>,
}

/// Source of parameter data
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterSource {
    Path,
    Query,
    Body,
    Header,
    Cookie,
}

/// Endpoint discovery service
pub struct EndpointDiscovery {
    /// Path parameter regex
    path_param_regex: Regex,
}

impl EndpointDiscovery {
    /// Create new endpoint discovery service
    pub fn new() -> OpenApiResult<Self> {
        Ok(Self {
            path_param_regex: Regex::new(r"\{([^}]+)\}").map_err(|e| {
                OpenApiError::route_discovery_error(format!("Failed to compile regex: {}", e))
            })?,
        })
    }

    /// Discover endpoints from controller metadata
    pub fn discover_endpoints(&self, controllers: &[ControllerInfo]) -> OpenApiResult<Vec<RouteMetadata>> {
        let mut routes = Vec::new();

        for controller in controllers {
            for endpoint in &controller.endpoints {
                let route = self.convert_endpoint_to_route(controller, endpoint)?;
                routes.push(route);
            }
        }

        Ok(routes)
    }

    /// Convert endpoint metadata to route metadata
    fn convert_endpoint_to_route(
        &self,
        controller: &ControllerInfo,
        endpoint: &EndpointMetadata,
    ) -> OpenApiResult<RouteMetadata> {
        // Extract path parameters
        let path_params = self.extract_path_parameters(&endpoint.path)?;
        
        // Build parameters list
        let mut parameters = Vec::new();
        
        // Add path parameters
        for param_name in &path_params {
            if let Some(endpoint_param) = endpoint.parameters.iter()
                .find(|p| &p.name == param_name && p.source == ParameterSource::Path) {
                parameters.push(ParameterInfo {
                    name: param_name.clone(),
                    location: "path".to_string(),
                    param_type: endpoint_param.param_type.clone(),
                    description: endpoint_param.documentation.clone(),
                    required: true, // Path parameters are always required
                    example: None,
                });
            } else {
                // Default path parameter if not found in endpoint metadata
                parameters.push(ParameterInfo {
                    name: param_name.clone(),
                    location: "path".to_string(),
                    param_type: "string".to_string(),
                    description: None,
                    required: true,
                    example: None,
                });
            }
        }

        // Add query parameters
        for endpoint_param in &endpoint.parameters {
            if endpoint_param.source == ParameterSource::Query {
                parameters.push(ParameterInfo {
                    name: endpoint_param.name.clone(),
                    location: "query".to_string(),
                    param_type: endpoint_param.param_type.clone(),
                    description: endpoint_param.documentation.clone(),
                    required: !endpoint_param.optional,
                    example: None,
                });
            }
        }

        // Add header parameters
        for endpoint_param in &endpoint.parameters {
            if endpoint_param.source == ParameterSource::Header {
                parameters.push(ParameterInfo {
                    name: endpoint_param.name.clone(),
                    location: "header".to_string(),
                    param_type: endpoint_param.param_type.clone(),
                    description: endpoint_param.documentation.clone(),
                    required: !endpoint_param.optional,
                    example: None,
                });
            }
        }

        // Determine request schema
        let request_schema = endpoint.parameters.iter()
            .find(|p| p.source == ParameterSource::Body)
            .map(|p| p.param_type.clone());

        // Build response schemas
        let mut response_schemas = HashMap::new();
        if let Some(return_type) = &endpoint.return_type {
            if return_type != "()" && return_type != "ElifResponse" {
                response_schemas.insert("200".to_string(), return_type.clone());
            }
        }

        // Extract attributes
        let summary = endpoint.attributes.get("summary")
            .or_else(|| endpoint.attributes.get("description"))
            .cloned();
        
        let description = endpoint.documentation.clone()
            .or_else(|| endpoint.attributes.get("description").cloned());

        let operation_id = Some(format!("{}{}", 
            controller.name.to_lowercase(), 
            capitalize(&endpoint.method)
        ));

        let tags = vec![controller.name.clone()];

        // Determine security requirements
        let security = if endpoint.attributes.contains_key("requires_auth") {
            vec!["bearerAuth".to_string()]
        } else {
            Vec::new()
        };

        let deprecated = endpoint.attributes.get("deprecated")
            .map(|v| v == "true")
            .unwrap_or(false);

        Ok(RouteMetadata {
            method: endpoint.verb.clone(),
            path: endpoint.path.clone(),
            summary,
            description,
            operation_id,
            tags,
            request_schema,
            response_schemas,
            parameters,
            security,
            deprecated,
        })
    }

    /// Extract path parameters from a path pattern
    fn extract_path_parameters(&self, path: &str) -> OpenApiResult<Vec<String>> {
        let mut parameters = Vec::new();

        for caps in self.path_param_regex.captures_iter(path) {
            if let Some(param) = caps.get(1) {
                parameters.push(param.as_str().to_string());
            }
        }

        Ok(parameters)
    }

    /// Extract endpoint metadata from source code (simplified implementation)
    pub fn extract_from_source(&self, source_code: &str) -> OpenApiResult<Vec<EndpointMetadata>> {
        // This is a simplified implementation
        // In a real implementation, you would parse the Rust AST
        let mut endpoints = Vec::new();
        
        // Look for route attribute patterns
        let route_regex = Regex::new(r#"#\[route\((\w+),\s*"([^"]+)"\)\]"#).map_err(|e| {
            OpenApiError::route_discovery_error(format!("Failed to compile route regex: {}", e))
        })?;

        // Look for function definitions
        let fn_regex = Regex::new(r"pub\s+async\s+fn\s+(\w+)").map_err(|e| {
            OpenApiError::route_discovery_error(format!("Failed to compile function regex: {}", e))
        })?;

        for route_match in route_regex.captures_iter(source_code) {
            if let (Some(verb), Some(path)) = (route_match.get(1), route_match.get(2)) {
                // Find the next function after this route
                let route_end = route_match.get(0).unwrap().end();
                let remaining_code = &source_code[route_end..];
                
                if let Some(fn_match) = fn_regex.find(remaining_code) {
                    let fn_name = fn_regex.captures(&remaining_code[fn_match.start()..])
                        .and_then(|caps| caps.get(1))
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    endpoints.push(EndpointMetadata {
                        controller: "Unknown".to_string(),
                        method: fn_name,
                        verb: verb.as_str().to_uppercase(),
                        path: path.as_str().to_string(),
                        documentation: None,
                        parameters: Vec::new(),
                        return_type: Some("ElifResponse".to_string()),
                        attributes: HashMap::new(),
                    });
                }
            }
        }

        Ok(endpoints)
    }
}

/// Controller information for endpoint discovery
#[derive(Debug, Clone)]
pub struct ControllerInfo {
    /// Controller name
    pub name: String,
    /// Base path prefix
    pub base_path: Option<String>,
    /// Endpoints in this controller
    pub endpoints: Vec<EndpointMetadata>,
    /// Controller attributes
    pub attributes: HashMap<String, String>,
}

impl ControllerInfo {
    /// Create new controller info
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            base_path: None,
            endpoints: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Add endpoint to controller
    pub fn add_endpoint(mut self, endpoint: EndpointMetadata) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    /// Set base path
    pub fn with_base_path(mut self, base_path: &str) -> Self {
        self.base_path = Some(base_path.to_string());
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
}

impl EndpointMetadata {
    /// Create new endpoint metadata
    pub fn new(method: &str, verb: &str, path: &str) -> Self {
        Self {
            controller: "Unknown".to_string(),
            method: method.to_string(),
            verb: verb.to_string(),
            path: path.to_string(),
            documentation: None,
            parameters: Vec::new(),
            return_type: None,
            attributes: HashMap::new(),
        }
    }

    /// Add parameter
    pub fn with_parameter(mut self, parameter: EndpointParameter) -> Self {
        self.parameters.push(parameter);
        self
    }

    /// Set return type
    pub fn with_return_type(mut self, return_type: &str) -> Self {
        self.return_type = Some(return_type.to_string());
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    /// Set documentation
    pub fn with_documentation(mut self, doc: &str) -> Self {
        self.documentation = Some(doc.to_string());
        self
    }
}

impl EndpointParameter {
    /// Create new endpoint parameter
    pub fn new(name: &str, param_type: &str, source: ParameterSource) -> Self {
        Self {
            name: name.to_string(),
            param_type: param_type.to_string(),
            source,
            optional: false,
            documentation: None,
        }
    }

    /// Make parameter optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Add documentation
    pub fn with_documentation(mut self, doc: &str) -> Self {
        self.documentation = Some(doc.to_string());
        self
    }
}

/// Helper function to capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_discovery_creation() {
        let discovery = EndpointDiscovery::new().unwrap();
        assert!(discovery.path_param_regex.is_match("{id}"));
    }

    #[test]
    fn test_path_parameter_extraction() {
        let discovery = EndpointDiscovery::new().unwrap();
        
        let params = discovery.extract_path_parameters("/users/{id}/posts/{post_id}").unwrap();
        assert_eq!(params, vec!["id", "post_id"]);
        
        let no_params = discovery.extract_path_parameters("/users").unwrap();
        assert!(no_params.is_empty());
    }

    #[test]
    fn test_endpoint_metadata_creation() {
        let endpoint = EndpointMetadata::new("index", "GET", "/users")
            .with_return_type("Vec<User>")
            .with_attribute("summary", "List all users")
            .with_parameter(EndpointParameter::new("limit", "Option<i32>", ParameterSource::Query).optional());

        assert_eq!(endpoint.method, "index");
        assert_eq!(endpoint.verb, "GET");
        assert_eq!(endpoint.path, "/users");
        assert_eq!(endpoint.return_type, Some("Vec<User>".to_string()));
        assert_eq!(endpoint.parameters.len(), 1);
        assert_eq!(endpoint.attributes.get("summary"), Some(&"List all users".to_string()));
    }

    #[test]
    fn test_controller_info_creation() {
        let controller = ControllerInfo::new("Users")
            .with_base_path("/api/v1")
            .add_endpoint(EndpointMetadata::new("index", "GET", "/users"))
            .add_endpoint(EndpointMetadata::new("show", "GET", "/users/{id}"));

        assert_eq!(controller.name, "Users");
        assert_eq!(controller.base_path, Some("/api/v1".to_string()));
        assert_eq!(controller.endpoints.len(), 2);
    }

    #[test]
    fn test_route_metadata_conversion() {
        let discovery = EndpointDiscovery::new().unwrap();
        
        let controller = ControllerInfo::new("Users");
        let endpoint = EndpointMetadata::new("show", "GET", "/users/{id}")
            .with_return_type("User")
            .with_parameter(EndpointParameter::new("id", "i32", ParameterSource::Path))
            .with_attribute("summary", "Get user by ID");

        let route = discovery.convert_endpoint_to_route(&controller, &endpoint).unwrap();
        
        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/users/{id}");
        assert_eq!(route.summary, Some("Get user by ID".to_string()));
        assert_eq!(route.tags, vec!["Users".to_string()]);
        assert_eq!(route.parameters.len(), 1);
        assert_eq!(route.parameters[0].name, "id");
        assert_eq!(route.parameters[0].location, "path");
        assert!(route.parameters[0].required);
    }
}