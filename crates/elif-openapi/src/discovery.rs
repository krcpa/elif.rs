/*!
Project discovery and introspection for OpenAPI generation.

This module provides functionality to discover API routes, controllers, and models
from an elif.rs project structure.
*/

use crate::{
    error::{OpenApiError, OpenApiResult},
    endpoints::{ControllerInfo, EndpointMetadata, EndpointParameter, ParameterSource},
};
use std::fs;
use std::path::{Path, PathBuf};
use toml;

/// Project discovery service for analyzing elif.rs projects
pub struct ProjectDiscovery {
    /// Project root directory
    project_root: PathBuf,
}

/// Discovered project structure
#[derive(Debug, Clone)]
pub struct ProjectStructure {
    /// Controllers found in the project
    pub controllers: Vec<ControllerInfo>,
    /// Models/schemas found in the project
    pub models: Vec<ModelInfo>,
    /// Project metadata
    pub metadata: ProjectMetadata,
}

/// Model/schema information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Fields in the model
    pub fields: Vec<ModelField>,
    /// Model documentation
    pub documentation: Option<String>,
    /// Model attributes/derives
    pub derives: Vec<String>,
}

/// Model field information
#[derive(Debug, Clone)]
pub struct ModelField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: String,
    /// Field documentation
    pub documentation: Option<String>,
    /// Whether field is optional
    pub optional: bool,
}

/// Project metadata
#[derive(Debug, Clone)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Project description
    pub description: Option<String>,
    /// Authors
    pub authors: Vec<String>,
}

impl ProjectDiscovery {
    /// Create new project discovery service
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    /// Discover project structure
    pub fn discover(&self) -> OpenApiResult<ProjectStructure> {
        let metadata = self.discover_project_metadata()?;
        let controllers = self.discover_controllers()?;
        let models = self.discover_models()?;

        Ok(ProjectStructure {
            controllers,
            models,
            metadata,
        })
    }

    /// Discover project metadata from Cargo.toml using proper TOML parsing
    fn discover_project_metadata(&self) -> OpenApiResult<ProjectMetadata> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");
        
        if !cargo_toml_path.exists() {
            return Ok(ProjectMetadata {
                name: "Unknown".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                authors: Vec::new(),
            });
        }

        let cargo_content = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read Cargo.toml: {}", e)
            ))?;

        // Parse TOML properly using toml crate
        let toml_value: toml::Value = cargo_content.parse()
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to parse Cargo.toml: {}", e)
            ))?;

        // Extract package information from [package] table
        let package = toml_value.get("package")
            .ok_or_else(|| OpenApiError::route_discovery_error(
                "No [package] section found in Cargo.toml".to_string()
            ))?;

        let name = package.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
            
        let version = package.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();
            
        let description = package.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract authors array
        let authors = package.get("authors")
            .and_then(|v| v.as_array())
            .map(|authors_array| {
                authors_array.iter()
                    .filter_map(|author| author.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        Ok(ProjectMetadata {
            name,
            version,
            description,
            authors,
        })
    }

    /// Discover controllers from src/controllers directory
    fn discover_controllers(&self) -> OpenApiResult<Vec<ControllerInfo>> {
        let controllers_dir = self.project_root.join("src").join("controllers");
        
        if !controllers_dir.exists() {
            return Ok(Vec::new());
        }

        let mut controllers = Vec::new();

        let entries = fs::read_dir(&controllers_dir)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read controllers directory: {}", e)
            ))?;

        for entry in entries {
            let entry = entry.map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read controller entry: {}", e)
            ))?;

            let path = entry.path();
            if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                if let Some(controller) = self.analyze_controller_file(&path)? {
                    controllers.push(controller);
                }
            }
        }

        Ok(controllers)
    }

    /// Analyze a controller file
    fn analyze_controller_file(&self, path: &Path) -> OpenApiResult<Option<ControllerInfo>> {
        let content = fs::read_to_string(path)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read controller file {}: {}", path.display(), e)
            ))?;

        let controller_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown")
            .replace("_controller", "")
            .replace("_", " ")
            .split_whitespace()
            .map(|word| capitalize(word))
            .collect::<String>();

        let endpoints = self.extract_endpoints_from_content(&content)?;

        if endpoints.is_empty() {
            return Ok(None);
        }

        let mut controller = ControllerInfo::new(&controller_name);
        for endpoint in endpoints {
            controller = controller.add_endpoint(endpoint);
        }

        Ok(Some(controller))
    }

    /// Extract endpoints from controller file content using AST parsing
    fn extract_endpoints_from_content(&self, content: &str) -> OpenApiResult<Vec<EndpointMetadata>> {
        // Parse the Rust source code into an AST
        let ast = syn::parse_file(content)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to parse Rust file: {}", e)
            ))?;

        let mut endpoints = Vec::new();

        // Walk the AST to find functions with route attributes
        for item in &ast.items {
            if let syn::Item::Fn(func) = item {
                if let Some(endpoint) = self.extract_endpoint_from_function(func)? {
                    endpoints.push(endpoint);
                }
            }
        }

        Ok(endpoints)
    }

    /// Extract endpoint from a function using AST analysis
    fn extract_endpoint_from_function(&self, func: &syn::ItemFn) -> OpenApiResult<Option<EndpointMetadata>> {
        // Look for route attributes
        let mut route_info = None;
        
        for attr in &func.attrs {
            if let Some((verb, path)) = self.parse_route_attribute_ast(attr)? {
                route_info = Some((verb, path));
                break;
            }
        }

        let Some((verb, path)) = route_info else {
            return Ok(None);
        };

        // Get function name
        let function_name = func.sig.ident.to_string();

        // Create endpoint metadata
        let mut endpoint = EndpointMetadata::new(&function_name, &verb, &path);

        // Extract parameters from function signature
        let params = self.extract_function_parameters_ast(&func.sig)?;
        for param in params {
            endpoint = endpoint.with_parameter(param);
        }

        // Extract documentation from function attributes
        let doc = self.extract_documentation_ast(&func.attrs);
        if let Some(doc) = doc {
            endpoint = endpoint.with_documentation(&doc);
        }

        // Extract return type information
        if let syn::ReturnType::Type(_, ty) = &func.sig.output {
            endpoint.return_type = Some(self.type_to_string(ty));
        }

        Ok(Some(endpoint))
    }

    /// Parse route attribute using AST
    fn parse_route_attribute_ast(&self, attr: &syn::Attribute) -> OpenApiResult<Option<(String, String)>> {
        // Check if this is a route attribute
        let path_segments: Vec<String> = attr.path().segments.iter()
            .map(|seg| seg.ident.to_string())
            .collect();

        // Look for route-like attributes (route, get, post, put, delete, etc.)
        if path_segments.len() != 1 {
            return Ok(None);
        }

        let attr_name = &path_segments[0];
        let (verb, path) = match attr_name.as_str() {
            "route" => {
                // Parse #[route(GET, "/path")] or #[route(method = "GET", path = "/path")]
                self.parse_route_macro(attr)?
            }
            "get" => ("GET".to_string(), self.parse_simple_route_macro(attr)?),
            "post" => ("POST".to_string(), self.parse_simple_route_macro(attr)?),
            "put" => ("PUT".to_string(), self.parse_simple_route_macro(attr)?),
            "delete" => ("DELETE".to_string(), self.parse_simple_route_macro(attr)?),
            "patch" => ("PATCH".to_string(), self.parse_simple_route_macro(attr)?),
            "head" => ("HEAD".to_string(), self.parse_simple_route_macro(attr)?),
            "options" => ("OPTIONS".to_string(), self.parse_simple_route_macro(attr)?),
            _ => return Ok(None),
        };

        Ok(Some((verb, path)))
    }

    /// Parse route macro like #[route(GET, "/path")]
    fn parse_route_macro(&self, attr: &syn::Attribute) -> OpenApiResult<(String, String)> {
        match &attr.meta {
            syn::Meta::List(meta_list) => {
                let tokens = &meta_list.tokens;
                let token_str = tokens.to_string();
                
                // Simple parsing for now - can be enhanced
                let parts: Vec<&str> = token_str.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let verb = parts[0].trim_matches('"').to_uppercase();
                    let path = parts[1].trim_matches('"').to_string();
                    Ok((verb, path))
                } else {
                    Err(OpenApiError::route_discovery_error("Invalid route attribute format".to_string()))
                }
            }
            _ => Err(OpenApiError::route_discovery_error("Expected route attribute with arguments".to_string())),
        }
    }

    /// Parse simple route macro like #[get("/path")]
    fn parse_simple_route_macro(&self, attr: &syn::Attribute) -> OpenApiResult<String> {
        match &attr.meta {
            syn::Meta::List(meta_list) => {
                let tokens = &meta_list.tokens;
                let path = tokens.to_string().trim_matches('"').to_string();
                Ok(path)
            }
            _ => Err(OpenApiError::route_discovery_error("Expected route attribute with path".to_string())),
        }
    }

    /// Extract function parameters using AST analysis
    fn extract_function_parameters_ast(&self, sig: &syn::Signature) -> OpenApiResult<Vec<EndpointParameter>> {
        let mut parameters = Vec::new();

        for input in &sig.inputs {
            match input {
                syn::FnArg::Typed(pat_type) => {
                    let param_name = match &*pat_type.pat {
                        syn::Pat::Ident(ident) => ident.ident.to_string(),
                        _ => continue, // Skip complex patterns
                    };

                    let type_str = self.type_to_string(&pat_type.ty);
                    let (source, optional) = self.determine_parameter_source(&type_str);

                    parameters.push(EndpointParameter {
                        name: param_name,
                        param_type: type_str,
                        source,
                        optional,
                        documentation: None,
                    });
                }
                syn::FnArg::Receiver(_) => continue, // Skip self parameters
            }
        }

        Ok(parameters)
    }

    /// Determine parameter source from type information
    fn determine_parameter_source(&self, type_str: &str) -> (ParameterSource, bool) {
        if type_str.contains("Path<") || type_str.contains("PathParams") {
            (ParameterSource::Path, false)
        } else if type_str.contains("Query<") || type_str.contains("QueryParams") {
            (ParameterSource::Query, type_str.contains("Option<"))
        } else if type_str.contains("Header<") || type_str.contains("HeaderMap") {
            (ParameterSource::Header, type_str.contains("Option<"))
        } else if type_str.contains("Json<") || type_str.contains("Form<") || type_str.contains("Request") {
            (ParameterSource::Body, false)
        } else {
            // Default to query parameter
            (ParameterSource::Query, type_str.contains("Option<"))
        }
    }

    /// Convert syn::Type to string representation
    fn type_to_string(&self, ty: &syn::Type) -> String {
        quote::quote!(#ty).to_string()
    }

    /// Extract documentation from function attributes
    fn extract_documentation_ast(&self, attrs: &[syn::Attribute]) -> Option<String> {
        let mut doc_lines = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &meta.value {
                        doc_lines.push(lit_str.value().trim().to_string());
                    }
                }
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }


    /// Discover models from src/models directory
    fn discover_models(&self) -> OpenApiResult<Vec<ModelInfo>> {
        let models_dir = self.project_root.join("src").join("models");
        
        if !models_dir.exists() {
            return Ok(Vec::new());
        }

        let mut models = Vec::new();

        let entries = fs::read_dir(&models_dir)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read models directory: {}", e)
            ))?;

        for entry in entries {
            let entry = entry.map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read model entry: {}", e)
            ))?;

            let path = entry.path();
            if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                if let Some(model) = self.analyze_model_file(&path)? {
                    models.push(model);
                }
            }
        }

        Ok(models)
    }

    /// Analyze a model file using AST parsing
    fn analyze_model_file(&self, path: &Path) -> OpenApiResult<Option<ModelInfo>> {
        let content = fs::read_to_string(path)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to read model file {}: {}", path.display(), e)
            ))?;

        let model_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Parse the Rust source code into an AST
        let ast = syn::parse_file(&content)
            .map_err(|e| OpenApiError::route_discovery_error(
                format!("Failed to parse model file {}: {}", path.display(), e)
            ))?;

        // Extract struct definition using AST
        if let Some(model) = self.extract_struct_from_ast(&ast, &model_name)? {
            return Ok(Some(model));
        }

        Ok(None)
    }

    /// Extract struct definition from AST
    fn extract_struct_from_ast(&self, ast: &syn::File, model_name: &str) -> OpenApiResult<Option<ModelInfo>> {
        // Walk the AST to find struct definitions
        for item in &ast.items {
            if let syn::Item::Struct(item_struct) = item {
                let struct_name = item_struct.ident.to_string();
                
                // Check if this is the struct we're looking for (case-insensitive)
                if struct_name.to_lowercase() == model_name.to_lowercase() {
                    // Extract derive attributes
                    let derives = self.extract_derives_from_attrs(&item_struct.attrs);
                    
                    // Extract documentation
                    let doc = self.extract_documentation_ast(&item_struct.attrs);
                    
                    // Extract fields
                    let fields = self.extract_struct_fields_from_ast(&item_struct.fields)?;

                    return Ok(Some(ModelInfo {
                        name: struct_name,
                        fields,
                        documentation: doc,
                        derives,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Extract derive attributes from struct attributes using AST
    fn extract_derives_from_attrs(&self, attrs: &[syn::Attribute]) -> Vec<String> {
        let mut derives = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("derive") {
                if let syn::Meta::List(meta_list) = &attr.meta {
                    let derive_tokens = meta_list.tokens.to_string();
                    // Parse comma-separated derive tokens
                    derives.extend(
                        derive_tokens
                            .split(',')
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                    );
                }
            }
        }

        derives
    }

    /// Extract struct fields from AST Fields
    fn extract_struct_fields_from_ast(&self, fields: &syn::Fields) -> OpenApiResult<Vec<ModelField>> {
        let mut model_fields = Vec::new();

        match fields {
            syn::Fields::Named(fields_named) => {
                for field in &fields_named.named {
                    if let Some(field_name) = &field.ident {
                        let field_name = field_name.to_string();
                        let field_type = self.type_to_string(&field.ty);
                        let optional = field_type.starts_with("Option<") || field_type.contains("Option <");
                        
                        // Extract field documentation
                        let documentation = self.extract_documentation_ast(&field.attrs);

                        model_fields.push(ModelField {
                            name: field_name,
                            field_type,
                            documentation,
                            optional,
                        });
                    }
                }
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                // Handle tuple structs
                for (index, field) in fields_unnamed.unnamed.iter().enumerate() {
                    let field_name = format!("field_{}", index);
                    let field_type = self.type_to_string(&field.ty);
                    let optional = field_type.starts_with("Option<") || field_type.contains("Option <");
                    
                    // Extract field documentation  
                    let documentation = self.extract_documentation_ast(&field.attrs);

                    model_fields.push(ModelField {
                        name: field_name,
                        field_type,
                        documentation,
                        optional,
                    });
                }
            }
            syn::Fields::Unit => {
                // Unit structs have no fields
            }
        }

        Ok(model_fields)
    }



    /// Bridge function for old line-based documentation extraction (used by model parsing)
    /// TODO: Replace with AST-based model parsing
    fn extract_documentation_from_lines(&self, lines: &[&str], route_index: usize) -> Option<String> {
        let mut doc_lines = Vec::new();

        // Look backwards for documentation comments
        for i in (0..route_index).rev() {
            let line = lines[i].trim();
            if line.starts_with("///") {
                doc_lines.insert(0, line.trim_start_matches("///").trim());
            } else if line.starts_with("//!") {
                doc_lines.insert(0, line.trim_start_matches("//!").trim());
            } else if !line.is_empty() && !line.starts_with("//") {
                break;
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join(" "))
        }
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_project_discovery_creation() {
        let temp_dir = TempDir::new().unwrap();
        let discovery = ProjectDiscovery::new(temp_dir.path());
        assert_eq!(discovery.project_root, temp_dir.path());
    }

    #[test]
    fn test_ast_based_struct_parsing() {
        let discovery = ProjectDiscovery::new(".");
        
        let test_code = r#"
            #[derive(Debug, Clone, Serialize)]
            /// A test user model
            pub struct User {
                /// User ID
                pub id: i32,
                /// User email address
                pub email: Option<String>,
                pub name: String,
            }
        "#;

        let ast = syn::parse_file(test_code).unwrap();
        let model = discovery.extract_struct_from_ast(&ast, "user").unwrap().unwrap();
        
        assert_eq!(model.name, "User");
        assert_eq!(model.fields.len(), 3);
        assert!(model.derives.contains(&"Debug".to_string()));
        assert!(model.derives.contains(&"Clone".to_string()));
        assert!(model.derives.contains(&"Serialize".to_string()));
        assert!(model.documentation.is_some());
        
        // Check fields
        let id_field = model.fields.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(id_field.field_type, "i32");
        assert!(!id_field.optional);
        
        let email_field = model.fields.iter().find(|f| f.name == "email").unwrap();
        assert_eq!(email_field.field_type, "Option < String >");
        assert!(email_field.optional);
    }

    #[test]
    fn test_robust_toml_parsing() {
        // Test with realistic Cargo.toml content including comments, tables, and various TOML features
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        let complex_toml_content = r#"
# This is a comment
[package]
name = "test-project"  # Inline comment
version = "1.2.3"
description = "A test project with complex TOML structure"
authors = ["John Doe <john@example.com>", "Jane Smith <jane@example.com>"]

# Some other sections that should not interfere
[dependencies]
serde = "1.0"

[dev-dependencies]
tokio-test = "0.4"

# Another comment
[features]
default = []
        "#;

        fs::write(&cargo_toml_path, complex_toml_content).unwrap();

        let discovery = ProjectDiscovery::new(temp_dir.path());
        let metadata = discovery.discover_project_metadata().unwrap();

        assert_eq!(metadata.name, "test-project");
        assert_eq!(metadata.version, "1.2.3");
        assert_eq!(metadata.description, Some("A test project with complex TOML structure".to_string()));
        assert_eq!(metadata.authors.len(), 2);
        assert!(metadata.authors.contains(&"John Doe <john@example.com>".to_string()));
        assert!(metadata.authors.contains(&"Jane Smith <jane@example.com>".to_string()));
    }

    #[test]
    fn test_toml_parsing_with_missing_package_section() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        let invalid_toml_content = r#"
# No package section
[dependencies]
serde = "1.0"
        "#;

        fs::write(&cargo_toml_path, invalid_toml_content).unwrap();

        let discovery = ProjectDiscovery::new(temp_dir.path());
        let result = discovery.discover_project_metadata();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No [package] section found"));
    }

    #[test]
    fn test_toml_parsing_with_minimal_package() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        let minimal_toml_content = r#"
[package]
name = "minimal-project"
version = "0.1.0"
        "#;

        fs::write(&cargo_toml_path, minimal_toml_content).unwrap();

        let discovery = ProjectDiscovery::new(temp_dir.path());
        let metadata = discovery.discover_project_metadata().unwrap();

        assert_eq!(metadata.name, "minimal-project");
        assert_eq!(metadata.version, "0.1.0");
        assert_eq!(metadata.description, None);
        assert!(metadata.authors.is_empty());
    }

    #[test]
    fn test_toml_parsing_with_different_key_ordering() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        // Test with different key ordering than typical
        let reordered_toml_content = r#"
[package]
authors = ["Author One"]
description = "Description first"
name = "reordered-project"
version = "2.0.0"
        "#;

        fs::write(&cargo_toml_path, reordered_toml_content).unwrap();

        let discovery = ProjectDiscovery::new(temp_dir.path());
        let metadata = discovery.discover_project_metadata().unwrap();

        assert_eq!(metadata.name, "reordered-project");
        assert_eq!(metadata.version, "2.0.0");
        assert_eq!(metadata.description, Some("Description first".to_string()));
        assert_eq!(metadata.authors, vec!["Author One".to_string()]);
    }
}