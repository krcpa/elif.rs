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

    /// Discover project metadata from Cargo.toml
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

        // Simple TOML parsing (in a real implementation, use a TOML parser)
        let name = self.extract_toml_value(&cargo_content, "name")
            .unwrap_or_else(|| "Unknown".to_string());
        let version = self.extract_toml_value(&cargo_content, "version")
            .unwrap_or_else(|| "1.0.0".to_string());
        let description = self.extract_toml_value(&cargo_content, "description");
        
        // Extract authors (simplified)
        let authors = if let Some(authors_str) = self.extract_toml_value(&cargo_content, "authors") {
            vec![authors_str]
        } else {
            Vec::new()
        };

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

    /// Analyze a model file
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

        // Extract struct definition
        if let Some(model) = self.extract_struct_from_content(&content, &model_name)? {
            return Ok(Some(model));
        }

        Ok(None)
    }

    /// Extract struct definition from content
    fn extract_struct_from_content(&self, content: &str, model_name: &str) -> OpenApiResult<Option<ModelInfo>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut model_info: Option<ModelInfo> = None;

        for i in 0..lines.len() {
            let line = lines[i].trim();
            
            // Look for struct definition
            if line.starts_with("pub struct") || line.starts_with("struct") {
                let struct_name = self.extract_struct_name(line);
                if struct_name.to_lowercase() == model_name.to_lowercase() {
                    let derives = self.extract_derives(&lines, i);
                    let doc = self.extract_documentation_from_lines(&lines, i);
                    let fields = self.extract_struct_fields(&lines, i)?;

                    model_info = Some(ModelInfo {
                        name: struct_name,
                        fields,
                        documentation: doc,
                        derives,
                    });
                    break;
                }
            }
        }

        Ok(model_info)
    }

    /// Extract struct name from definition line
    fn extract_struct_name(&self, line: &str) -> String {
        line.split_whitespace()
            .find(|word| !word.starts_with('#') && *word != "pub" && *word != "struct")
            .unwrap_or("Unknown")
            .trim_end_matches('{')
            .to_string()
    }

    /// Extract derive attributes
    fn extract_derives(&self, lines: &[&str], struct_index: usize) -> Vec<String> {
        let mut derives = Vec::new();

        // Look backwards for derive attributes
        for i in (0..struct_index).rev() {
            let line = lines[i].trim();
            if line.starts_with("#[derive(") {
                let derive_content = line
                    .trim_start_matches("#[derive(")
                    .trim_end_matches(")]");
                derives.extend(
                    derive_content
                        .split(',')
                        .map(|d| d.trim().to_string())
                );
            } else if !line.is_empty() && !line.starts_with("#") && !line.starts_with("//") {
                break;
            }
        }

        derives
    }

    /// Extract struct fields
    fn extract_struct_fields(&self, lines: &[&str], struct_index: usize) -> OpenApiResult<Vec<ModelField>> {
        let mut fields = Vec::new();
        let mut in_struct = false;

        for i in struct_index..lines.len() {
            let line = lines[i].trim();
            
            if line.contains('{') {
                in_struct = true;
                continue;
            }
            
            if line == "}" {
                break;
            }
            
            if in_struct && !line.is_empty() && !line.starts_with("//") {
                if let Some(field) = self.parse_struct_field(line) {
                    fields.push(field);
                }
            }
        }

        Ok(fields)
    }

    /// Parse a struct field line
    fn parse_struct_field(&self, line: &str) -> Option<ModelField> {
        let line = line.trim().trim_end_matches(',');
        
        // Skip attributes and visibility modifiers
        if line.starts_with('#') || line.starts_with("pub") {
            return None;
        }

        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let name = parts[0].trim().to_string();
        let field_type = parts[1].trim().to_string();
        let optional = field_type.starts_with("Option<");

        Some(ModelField {
            name,
            field_type,
            documentation: None,
            optional,
        })
    }

    /// Extract value from simple TOML content
    fn extract_toml_value(&self, content: &str, key: &str) -> Option<String> {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with(&format!("{} =", key)) {
                return line
                    .split('=')
                    .nth(1)
                    .map(|v| v.trim().trim_matches('"').to_string());
            }
        }
        None
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
    fn test_struct_name_extraction() {
        let discovery = ProjectDiscovery::new(".");
        
        assert_eq!(discovery.extract_struct_name("pub struct User {"), "User");
        assert_eq!(discovery.extract_struct_name("struct Post"), "Post");
        assert_eq!(discovery.extract_struct_name("pub struct Comment<'a> {"), "Comment<'a>");
    }

    #[test]
    fn test_struct_field_parsing() {
        let discovery = ProjectDiscovery::new(".");
        
        let field1 = discovery.parse_struct_field("id: i32,").unwrap();
        assert_eq!(field1.name, "id");
        assert_eq!(field1.field_type, "i32");
        assert!(!field1.optional);

        let field2 = discovery.parse_struct_field("email: Option<String>,").unwrap();
        assert_eq!(field2.name, "email");
        assert_eq!(field2.field_type, "Option<String>");
        assert!(field2.optional);
    }

    #[test]
    fn test_toml_value_extraction() {
        let discovery = ProjectDiscovery::new(".");
        let content = r#"
name = "test-project"
version = "1.0.0"
description = "A test project"
        "#;

        assert_eq!(discovery.extract_toml_value(content, "name"), Some("test-project".to_string()));
        assert_eq!(discovery.extract_toml_value(content, "version"), Some("1.0.0".to_string()));
        assert_eq!(discovery.extract_toml_value(content, "description"), Some("A test project".to_string()));
        assert_eq!(discovery.extract_toml_value(content, "missing"), None);
    }
}