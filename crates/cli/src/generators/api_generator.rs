use super::{to_snake_case, to_pascal_case, pluralize_word};
use super::resource_generator::{GeneratedFile, GeneratedFileType};
use elif_core::ElifError;
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::{json, Value};

pub struct ApiGenerator {
    project_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ApiOptions {
    pub version: String,
    pub prefix: String,
    pub with_openapi: bool,
    pub with_versioning: bool,
}

impl Default for ApiOptions {
    fn default() -> Self {
        Self {
            version: "v1".to_string(),
            prefix: "api".to_string(),
            with_openapi: true,
            with_versioning: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiResource {
    pub name: String,
    pub endpoints: Vec<ApiEndpoint>,
}

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub description: Option<String>,
    pub parameters: Vec<ApiParameter>,
    pub responses: Vec<ApiResponse>,
}

#[derive(Debug, Clone)]
pub struct ApiParameter {
    pub name: String,
    pub param_type: String, // "path", "query", "body"
    pub data_type: String,
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub status_code: u16,
    pub description: String,
    pub schema: Option<String>,
}

impl ApiGenerator {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn generate_api(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<Vec<GeneratedFile>, ElifError> {
        let mut generated_files = Vec::new();

        // Generate API routes file
        let routes_file = self.generate_api_routes(resources, options)?;
        generated_files.push(routes_file);

        // Generate version-specific module if versioning is enabled
        if options.with_versioning {
            let version_module = self.generate_version_module(resources, options)?;
            generated_files.push(version_module);
        }

        // Generate OpenAPI specification if enabled
        if options.with_openapi {
            let openapi_file = self.generate_openapi_spec(resources, options)?;
            generated_files.push(openapi_file);
        }

        // Generate API documentation
        let docs_file = self.generate_api_docs(resources, options)?;
        generated_files.push(docs_file);

        Ok(generated_files)
    }

    fn generate_api_routes(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<GeneratedFile, ElifError> {
        let mut content = String::new();

        content.push_str(&format!(
            r#"use elif_http::prelude::*;
use elif_core::ServiceContainer;
use std::sync::Arc;

// Import controllers
{}

pub fn setup_api_routes(container: Arc<ServiceContainer>) -> Router {{
    let mut router = Router::new();
    
    // API {} routes
    let api_group = router.group("/{}"){}{{
        
{}

        api_group
    }};
    
    router.mount("", api_group);
    
    router
}}
"#,
            self.generate_controller_imports(resources),
            options.version,
            options.prefix,
            if options.with_versioning {
                &format!(".group(\"{}\")", options.version)
            } else { "" },
            self.generate_route_definitions(resources, options)
        ));

        let filename = if options.with_versioning {
            format!("{}_routes.rs", options.version)
        } else {
            "routes.rs".to_string()
        };

        Ok(GeneratedFile {
            path: self.project_root.join("src").join("routes").join(filename),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

    fn generate_controller_imports(&self, resources: &[ApiResource]) -> String {
        resources
            .iter()
            .map(|resource| {
                let snake_name = to_snake_case(&resource.name);
                format!("use crate::controllers::{}_controller::{}Controller;", 
                    snake_name, to_pascal_case(&resource.name))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn generate_route_definitions(&self, resources: &[ApiResource], _options: &ApiOptions) -> String {
        let mut routes = String::new();

        for resource in resources {
            let snake_name = to_snake_case(&resource.name);
            let controller_name = format!("{}Controller", to_pascal_case(&resource.name));
            let plural_path = pluralize_word(&snake_name);

            routes.push_str(&format!(
                r#"        // {} routes
        let {}_controller = Arc::new({}::new(container.clone()));
        
"#,
                resource.name, snake_name, controller_name
            ));

            // Add standard CRUD routes
            routes.push_str(&format!(
                r#"        api_group.get("/{}", {{
            let controller = {}_controller.clone();
            move |req| controller.index(req)
        }});
        
        api_group.post("/{}", {{
            let controller = {}_controller.clone();
            move |req| controller.store(req)
        }});
        
        api_group.get("/{}/:id", {{
            let controller = {}_controller.clone();
            move |req| controller.show(req)
        }});
        
        api_group.patch("/{}/:id", {{
            let controller = {}_controller.clone();
            move |req| controller.update(req)
        }});
        
        api_group.delete("/{}/:id", {{
            let controller = {}_controller.clone();
            move |req| controller.destroy(req)
        }});
        
"#,
                plural_path, snake_name,
                plural_path, snake_name,
                plural_path, snake_name,
                plural_path, snake_name,
                plural_path, snake_name,
            ));

            // Add custom endpoints
            for endpoint in &resource.endpoints {
                if !Self::is_standard_crud_endpoint(&endpoint.method, &endpoint.path) {
                    routes.push_str(&format!(
                        r#"        api_group.{}("{}", {{
            let controller = {}_controller.clone();
            move |req| controller.{}(req)
        }});
        
"#,
                        endpoint.method.to_lowercase(),
                        endpoint.path,
                        snake_name,
                        to_snake_case(&endpoint.handler)
                    ));
                }
            }
        }

        routes
    }

    fn is_standard_crud_endpoint(method: &str, path: &str) -> bool {
        matches!(
            (method.to_uppercase().as_str(), path),
            ("GET", "/") | 
            ("POST", "/") | 
            ("GET", "/:id") | 
            ("PATCH", "/:id") | 
            ("PUT", "/:id") | 
            ("DELETE", "/:id")
        )
    }

    fn generate_version_module(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<GeneratedFile, ElifError> {
        let content = format!(
            r#"//! API {} Module
//! 
//! This module contains all {} API routes and handlers.

pub mod routes;

// Re-export main route setup function
pub use routes::setup_api_routes;

// Version information
pub const VERSION: &str = "{}";
pub const API_PREFIX: &str = "/{}";

// API metadata
pub fn api_info() -> serde_json::Value {{
    serde_json::json!({{
        "version": VERSION,
        "prefix": API_PREFIX,
        "resources": [
            {}
        ]
    }})
}}
"#,
            options.version.to_uppercase(),
            options.version,
            options.version,
            options.prefix,
            resources
                .iter()
                .map(|r| format!("\"{}\"", pluralize_word(&to_snake_case(&r.name))))
                .collect::<Vec<_>>()
                .join(",\n            ")
        );

        Ok(GeneratedFile {
            path: self.project_root.join("src").join("api").join(&options.version).join("mod.rs"),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

    fn generate_openapi_spec(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<GeneratedFile, ElifError> {
        let mut spec = json!({
            "openapi": "3.0.3",
            "info": {
                "title": "API Documentation",
                "description": "Generated API documentation",
                "version": options.version,
                "contact": {
                    "name": "API Support"
                }
            },
            "servers": [
                {
                    "url": format!("/{}/{}", options.prefix, options.version),
                    "description": format!("API {} Server", options.version.to_uppercase())
                }
            ],
            "paths": {},
            "components": {
                "schemas": {},
                "responses": {
                    "NotFound": {
                        "description": "Resource not found",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    },
                    "ValidationError": {
                        "description": "Validation error",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ValidationErrorResponse"
                                }
                            }
                        }
                    }
                },
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer",
                        "bearerFormat": "JWT"
                    }
                }
            },
            "security": [
                {
                    "bearerAuth": []
                }
            ]
        });

        // Add paths for each resource
        let paths = spec["paths"].as_object_mut().unwrap();
        for resource in resources {
            let resource_paths = self.generate_openapi_paths_for_resource(resource);
            for (path, operations) in resource_paths {
                paths.insert(path, operations);
            }
        }

        // Add common schemas
        let schemas = spec["components"]["schemas"].as_object_mut().unwrap();
        self.add_common_schemas(schemas);

        // Add resource schemas
        for resource in resources {
            self.add_resource_schemas(resource, schemas);
        }

        let content = serde_yaml::to_string(&spec)
            .map_err(|e| ElifError::Validation { message: format!("Failed to serialize OpenAPI spec: {}", e) })?;

        Ok(GeneratedFile {
            path: self.project_root.join("openapi").join(format!("api_{}.yml", options.version)),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

    fn generate_openapi_paths_for_resource(&self, resource: &ApiResource) -> HashMap<String, Value> {
        let mut paths = HashMap::new();
        let snake_name = to_snake_case(&resource.name);
        let pascal_name = to_pascal_case(&resource.name);
        let plural_path = format!("/{}", pluralize_word(&snake_name));
        let singular_path = format!("/{}/:id", pluralize_word(&snake_name));

        // List endpoint
        paths.insert(plural_path.clone(), json!({
            "get": {
                "summary": format!("List all {}", pluralize_word(&resource.name)),
                "description": format!("Retrieve a paginated list of all {}", pluralize_word(&resource.name.to_lowercase())),
                "tags": [pascal_name],
                "parameters": [
                    {
                        "name": "page",
                        "in": "query",
                        "description": "Page number for pagination",
                        "required": false,
                        "schema": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 1
                        }
                    },
                    {
                        "name": "per_page",
                        "in": "query", 
                        "description": "Number of items per page",
                        "required": false,
                        "schema": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 20
                        }
                    }
                ],
                "responses": {
                    "200": {
                        "description": format!("List of {}", pluralize_word(&resource.name.to_lowercase())),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{}Collection", pascal_name)
                                }
                            }
                        }
                    }
                }
            },
            "post": {
                "summary": format!("Create a new {}", resource.name.to_lowercase()),
                "description": format!("Create a new {} with the provided data", resource.name.to_lowercase()),
                "tags": [pascal_name],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": format!("#/components/schemas/Create{}Request", pascal_name)
                            }
                        }
                    }
                },
                "responses": {
                    "201": {
                        "description": format!("{} created successfully", pascal_name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{}Resource", pascal_name)
                                }
                            }
                        }
                    },
                    "422": {
                        "$ref": "#/components/responses/ValidationError"
                    }
                }
            }
        }));

        // Individual resource endpoints
        let id_path = singular_path.replace(":id", "{id}");
        paths.insert(id_path, json!({
            "get": {
                "summary": format!("Get {} by ID", resource.name.to_lowercase()),
                "description": format!("Retrieve a specific {} by its ID", resource.name.to_lowercase()),
                "tags": [pascal_name],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "description": format!("{} ID", pascal_name),
                        "required": true,
                        "schema": {
                            "type": "string",
                            "format": "uuid"
                        }
                    }
                ],
                "responses": {
                    "200": {
                        "description": format!("{} details", pascal_name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{}Resource", pascal_name)
                                }
                            }
                        }
                    },
                    "404": {
                        "$ref": "#/components/responses/NotFound"
                    }
                }
            },
            "patch": {
                "summary": format!("Update {}", resource.name.to_lowercase()),
                "description": format!("Update an existing {} with the provided data", resource.name.to_lowercase()),
                "tags": [pascal_name],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "description": format!("{} ID", pascal_name),
                        "required": true,
                        "schema": {
                            "type": "string",
                            "format": "uuid"
                        }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": format!("#/components/schemas/Update{}Request", pascal_name)
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": format!("{} updated successfully", pascal_name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{}Resource", pascal_name)
                                }
                            }
                        }
                    },
                    "404": {
                        "$ref": "#/components/responses/NotFound"
                    },
                    "422": {
                        "$ref": "#/components/responses/ValidationError"
                    }
                }
            },
            "delete": {
                "summary": format!("Delete {}", resource.name.to_lowercase()),
                "description": format!("Delete an existing {}", resource.name.to_lowercase()),
                "tags": [pascal_name],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "description": format!("{} ID", pascal_name),
                        "required": true,
                        "schema": {
                            "type": "string",
                            "format": "uuid"
                        }
                    }
                ],
                "responses": {
                    "204": {
                        "description": format!("{} deleted successfully", pascal_name)
                    },
                    "404": {
                        "$ref": "#/components/responses/NotFound"
                    }
                }
            }
        }));

        paths
    }

    fn add_common_schemas(&self, schemas: &mut serde_json::Map<String, Value>) {
        schemas.insert("ErrorResponse".to_string(), json!({
            "type": "object",
            "properties": {
                "error": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string"
                        },
                        "message": {
                            "type": "string"
                        },
                        "hint": {
                            "type": "string"
                        }
                    },
                    "required": ["code", "message"]
                }
            },
            "required": ["error"]
        }));

        schemas.insert("ValidationErrorResponse".to_string(), json!({
            "type": "object",
            "properties": {
                "error": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "example": "VALIDATION_FAILED"
                        },
                        "message": {
                            "type": "string"
                        },
                        "details": {
                            "type": "object",
                            "additionalProperties": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                }
                            }
                        }
                    },
                    "required": ["code", "message"]
                }
            },
            "required": ["error"]
        }));
    }

    fn add_resource_schemas(&self, resource: &ApiResource, schemas: &mut serde_json::Map<String, Value>) {
        let pascal_name = to_pascal_case(&resource.name);

        // Basic resource schema (simplified)
        schemas.insert(format!("{}Resource", pascal_name), json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "format": "uuid"
                },
                "created_at": {
                    "type": "string",
                    "format": "date-time"
                },
                "updated_at": {
                    "type": "string",
                    "format": "date-time"
                }
            },
            "required": ["id", "created_at", "updated_at"]
        }));

        // Collection schema
        schemas.insert(format!("{}Collection", pascal_name), json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "array",
                    "items": {
                        "$ref": format!("#/components/schemas/{}Resource", pascal_name)
                    }
                },
                "meta": {
                    "type": "object",
                    "properties": {
                        "total": {
                            "type": "integer"
                        }
                    },
                    "required": ["total"]
                }
            },
            "required": ["data", "meta"]
        }));

        // Create request schema (simplified)
        schemas.insert(format!("Create{}Request", pascal_name), json!({
            "type": "object",
            "properties": {},
            "required": []
        }));

        // Update request schema (simplified)
        schemas.insert(format!("Update{}Request", pascal_name), json!({
            "type": "object", 
            "properties": {},
            "required": []
        }));
    }

    fn generate_api_docs(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<GeneratedFile, ElifError> {
        let content = format!(
            r#"# API {} Documentation

## Overview

This API provides access to the following resources:

{}

## Base URL

```
/{}/{}
```

## Authentication

This API uses Bearer token authentication. Include your token in the Authorization header:

```
Authorization: Bearer <your_token>
```

## Common Response Format

### Success Response
```json
{{
  "data": {{ ... }}
}}
```

### Error Response
```json
{{
  "error": {{
    "code": "ERROR_CODE",
    "message": "Human readable error message",
    "hint": "Optional hint for resolving the error"
  }}
}}
```

## Resources

{}

## Status Codes

- `200` - OK
- `201` - Created 
- `204` - No Content
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `422` - Unprocessable Entity
- `500` - Internal Server Error

## Rate Limiting

API requests are rate limited. Check the following headers in the response:

- `X-RateLimit-Limit` - The number of requests per time window
- `X-RateLimit-Remaining` - The number of requests remaining in the current window
- `X-RateLimit-Reset` - The time when the current window resets

## Pagination

List endpoints support pagination with the following parameters:

- `page` - Page number (default: 1)
- `per_page` - Items per page (default: 20, max: 100)

Response includes pagination metadata:

```json
{{
  "data": [...],
  "meta": {{
    "total": 100,
    "page": 1,
    "per_page": 20,
    "pages": 5
  }}
}}
```
"#,
            options.version.to_uppercase(),
            resources
                .iter()
                .map(|r| format!("- {}", to_pascal_case(&r.name)))
                .collect::<Vec<_>>()
                .join("\n"),
            options.prefix,
            options.version,
            self.generate_resource_docs(resources)
        );

        Ok(GeneratedFile {
            path: self.project_root.join("docs").join(format!("api_{}.md", options.version)),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

    fn generate_resource_docs(&self, resources: &[ApiResource]) -> String {
        resources
            .iter()
            .map(|resource| {
                let pascal_name = to_pascal_case(&resource.name);
                let snake_name = to_snake_case(&resource.name);
                let plural_path = pluralize_word(&snake_name);

                format!(
                    r#"### {}

#### List {}
`GET /{}`

Retrieve a paginated list of all {}.

#### Create {}
`POST /{}`

Create a new {} with the provided data.

#### Get {} by ID
`GET /{}/:id`

Retrieve a specific {} by its ID.

#### Update {}
`PATCH /{}/:id`

Update an existing {} with the provided data.

#### Delete {}
`DELETE /{}/:id`

Delete an existing {}.

"#,
                    pascal_name,
                    pluralize_word(&pascal_name), plural_path, pluralize_word(&resource.name.to_lowercase()),
                    pascal_name, plural_path, resource.name.to_lowercase(),
                    pascal_name, plural_path, resource.name.to_lowercase(),
                    pascal_name, plural_path, resource.name.to_lowercase(),
                    pascal_name, plural_path, resource.name.to_lowercase(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}