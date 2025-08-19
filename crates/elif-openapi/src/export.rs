/*!
Export functionality for OpenAPI specifications.

This module provides functionality to export OpenAPI specifications to various
formats including Postman collections and Insomnia workspaces.
*/

use crate::{
    error::OpenApiResult,
    specification::OpenApiSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Export service for OpenAPI specifications
pub struct OpenApiExporter;

impl OpenApiExporter {
    /// Export to Postman collection format
    pub fn export_postman(spec: &OpenApiSpec) -> OpenApiResult<PostmanCollection> {
        let mut collection = PostmanCollection::new(&spec.info.title, &spec.info.description);

        // Set collection info
        collection.info.version = spec.info.version.clone();
        
        // Process paths and create requests
        for (path, path_item) in &spec.paths {
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
                    let request = Self::create_postman_request(spec, method, path, op)?;
                    collection.add_item(PostmanItem::Request(request));
                }
            }
        }

        Ok(collection)
    }

    /// Create Postman request from OpenAPI operation
    fn create_postman_request(
        spec: &OpenApiSpec,
        method: &str,
        path: &str,
        operation: &crate::specification::Operation,
    ) -> OpenApiResult<PostmanRequest> {
        let mut request = PostmanRequest {
            name: operation.summary.clone().unwrap_or_else(|| {
                format!("{} {}", method, path)
            }),
            method: method.to_string(),
            header: Vec::new(),
            url: PostmanUrl::new(path),
            body: None,
        };

        // Add base URL from servers
        if let Some(server) = spec.servers.first() {
            request.url.host = Some(vec![server.url.clone()]);
        }

        // Process parameters
        for param in &operation.parameters {
            match param.location.as_str() {
                "header" => {
                    request.header.push(PostmanHeader {
                        key: param.name.clone(),
                        value: param.example
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("{{value}}")
                            .to_string(),
                        description: param.description.clone(),
                    });
                }
                "query" => {
                    request.url.query.push(PostmanQuery {
                        key: param.name.clone(),
                        value: param.example
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("{{value}}")
                            .to_string(),
                        description: param.description.clone(),
                    });
                }
                "path" => {
                    request.url.variable.push(PostmanVariable {
                        key: param.name.clone(),
                        value: param.example
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("{{value}}")
                            .to_string(),
                        description: param.description.clone(),
                    });
                }
                _ => {}
            }
        }

        // Process request body
        if let Some(request_body) = &operation.request_body {
            if let Some(json_content) = request_body.content.get("application/json") {
                if let Some(schema) = &json_content.schema {
                    let example = Self::generate_request_body_example(schema)?;
                    request.body = Some(PostmanBody {
                        mode: "raw".to_string(),
                        raw: serde_json::to_string_pretty(&example)?,
                        options: Some(PostmanBodyOptions {
                            raw: PostmanRawOptions {
                                language: "json".to_string(),
                            },
                        }),
                    });
                }
            }
        }

        Ok(request)
    }

    /// Generate example request body from schema
    fn generate_request_body_example(schema: &crate::specification::Schema) -> OpenApiResult<Value> {
        crate::utils::OpenApiUtils::generate_example_from_schema(schema)
    }

    /// Export to Insomnia workspace format
    pub fn export_insomnia(spec: &OpenApiSpec) -> OpenApiResult<InsomniaWorkspace> {
        let mut workspace = InsomniaWorkspace::new(&spec.info.title);
        let workspace_id = workspace.workspace.id.clone();

        // Create base environment
        let base_url = spec.servers
            .first()
            .map(|s| s.url.clone())
            .unwrap_or_else(|| "http://localhost:3000".to_string());

        workspace.add_environment("Base Environment", &base_url);

        // Process paths and create requests
        for (path, path_item) in &spec.paths {
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
                    let request = Self::create_insomnia_request(method, path, op, &workspace_id)?;
                    workspace.add_resource(InsomniaResource::Request(request));
                }
            }
        }

        Ok(workspace)
    }

    /// Create Insomnia request from OpenAPI operation
    fn create_insomnia_request(
        method: &str,
        path: &str,
        operation: &crate::specification::Operation,
        parent_id: &str,
    ) -> OpenApiResult<InsomniaRequest> {
        let mut request = InsomniaRequest::new(
            &operation.summary.clone().unwrap_or_else(|| format!("{} {}", method, path)),
            method,
            "{{ _.base_url }}", // Use environment variable
            parent_id,
        );

        // Build full URL with path
        request.url = format!("{{{{ _.base_url }}}}{}", path);

        // Process parameters
        for param in &operation.parameters {
            match param.location.as_str() {
                "header" => {
                    request.headers.push(InsomniaHeader {
                        name: param.name.clone(),
                        value: param.example
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
                "query" => {
                    request.parameters.push(InsomniaParameter {
                        name: param.name.clone(),
                        value: param.example
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
                _ => {}
            }
        }

        // Process request body
        if let Some(request_body) = &operation.request_body {
            if let Some(json_content) = request_body.content.get("application/json") {
                if let Some(schema) = &json_content.schema {
                    let example = Self::generate_request_body_example(schema)?;
                    request.body = InsomniaBody {
                        mime_type: "application/json".to_string(),
                        text: serde_json::to_string_pretty(&example)?,
                    };
                }
            }
        }

        Ok(request)
    }
}

// Postman collection format structures

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanCollection {
    pub info: PostmanInfo,
    pub item: Vec<PostmanItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanInfo {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub schema: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PostmanItem {
    Request(PostmanRequest),
    Folder(PostmanFolder),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanFolder {
    pub name: String,
    pub item: Vec<PostmanItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanRequest {
    pub name: String,
    pub method: String,
    pub header: Vec<PostmanHeader>,
    pub url: PostmanUrl,
    pub body: Option<PostmanBody>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanHeader {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanUrl {
    pub raw: Option<String>,
    pub host: Option<Vec<String>>,
    pub path: Vec<String>,
    pub query: Vec<PostmanQuery>,
    pub variable: Vec<PostmanVariable>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanQuery {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanVariable {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanBody {
    pub mode: String,
    pub raw: String,
    pub options: Option<PostmanBodyOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanBodyOptions {
    pub raw: PostmanRawOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostmanRawOptions {
    pub language: String,
}

impl PostmanCollection {
    pub fn new(name: &str, description: &Option<String>) -> Self {
        Self {
            info: PostmanInfo {
                name: name.to_string(),
                description: description.clone(),
                version: "1.0.0".to_string(),
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json".to_string(),
            },
            item: Vec::new(),
        }
    }

    pub fn add_item(&mut self, item: PostmanItem) {
        self.item.push(item);
    }
}

impl PostmanUrl {
    pub fn new(path: &str) -> Self {
        let path_parts: Vec<String> = path
            .split('/')
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect();

        Self {
            raw: None,
            host: None,
            path: path_parts,
            query: Vec::new(),
            variable: Vec::new(),
        }
    }
}

// Insomnia workspace format structures

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaWorkspace {
    #[serde(rename = "_type")]
    pub workspace_type: String,
    pub workspace: InsomniaWorkspaceInfo,
    pub resources: Vec<InsomniaResource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaWorkspaceInfo {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum InsomniaResource {
    #[serde(rename = "request")]
    Request(InsomniaRequest),
    #[serde(rename = "environment")]
    Environment(InsomniaEnvironment),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaRequest {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<InsomniaHeader>,
    pub parameters: Vec<InsomniaParameter>,
    pub body: InsomniaBody,
    #[serde(rename = "parentId")]
    pub parent_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaParameter {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaBody {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsomniaEnvironment {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub data: HashMap<String, String>,
    #[serde(rename = "parentId")]
    pub parent_id: String,
}

impl InsomniaWorkspace {
    pub fn new(name: &str) -> Self {
        let workspace_id = format!("wrk_{}", uuid::Uuid::new_v4().simple());
        
        Self {
            workspace_type: "export".to_string(),
            workspace: InsomniaWorkspaceInfo {
                id: workspace_id,
                name: name.to_string(),
                description: "Exported from OpenAPI specification".to_string(),
            },
            resources: Vec::new(),
        }
    }

    pub fn add_resource(&mut self, resource: InsomniaResource) {
        self.resources.push(resource);
    }

    pub fn add_environment(&mut self, name: &str, base_url: &str) {
        let mut data = HashMap::new();
        data.insert("base_url".to_string(), base_url.to_string());

        let environment = InsomniaEnvironment {
            id: format!("env_{}", uuid::Uuid::new_v4().simple()),
            name: name.to_string(),
            data,
            parent_id: self.workspace.id.clone(),
        };

        self.resources.push(InsomniaResource::Environment(environment));
    }
}

impl InsomniaRequest {
    pub fn new(name: &str, method: &str, url: &str, parent_id: &str) -> Self {
        Self {
            id: format!("req_{}", uuid::Uuid::new_v4().simple()),
            name: name.to_string(),
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            parameters: Vec::new(),
            body: InsomniaBody {
                mime_type: "application/json".to_string(),
                text: "".to_string(),
            },
            parent_id: parent_id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specification::{ApiInfo, OpenApiSpec, Operation};
    use std::collections::HashMap;

    #[test]
    fn test_postman_collection_creation() {
        let collection = PostmanCollection::new("Test API", &Some("Test description".to_string()));
        assert_eq!(collection.info.name, "Test API");
        assert_eq!(collection.info.description, Some("Test description".to_string()));
        assert!(collection.item.is_empty());
    }

    #[test]
    fn test_insomnia_workspace_creation() {
        let workspace = InsomniaWorkspace::new("Test API");
        assert_eq!(workspace.workspace.name, "Test API");
        assert!(workspace.resources.is_empty());
    }

    #[test]
    fn test_postman_export() {
        let spec = OpenApiSpec::new("Test API", "1.0.0");
        let collection = OpenApiExporter::export_postman(&spec).unwrap();
        assert_eq!(collection.info.name, "Test API");
        // Empty spec should have no items
        assert_eq!(collection.item.len(), 0);
    }
}