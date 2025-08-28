use super::{to_snake_case, pluralize_word, TemplateEngine};
use super::resource_generator::{GeneratedFile, GeneratedFileType};
use elif_core::ElifError;
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::json;
use serde::Serialize;

pub struct ApiGenerator {
    project_root: PathBuf,
    template_engine: TemplateEngine,
}

#[derive(Debug, Clone)]
pub struct ApiOptions {
    pub version: String,
    pub prefix: String,
    pub with_openapi: bool,
    pub with_versioning: bool,
    pub with_auth: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
}

impl Default for ApiOptions {
    fn default() -> Self {
        Self {
            version: "v1".to_string(),
            prefix: "api".to_string(),
            with_openapi: true,
            with_versioning: false,
            with_auth: false,
            title: None,
            description: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResource {
    pub name: String,
    pub endpoints: Vec<ApiEndpoint>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub description: Option<String>,
    pub parameters: Vec<ApiParameter>,
    pub responses: Vec<ApiResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct ApiParameter {
    pub name: String,
    pub param_type: String, // "path", "query", "body"
    pub data_type: String,
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct ApiResponse {
    pub status_code: u16,
    pub description: String,
    pub schema: Option<String>,
}

impl ApiGenerator {
    pub fn new(project_root: PathBuf) -> Result<Self, ElifError> {
        let template_engine = TemplateEngine::new()?;
        Ok(Self { 
            project_root,
            template_engine,
        })
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
        let context = self.create_template_context(resources, options);
        let content = self.template_engine.render("api_routes", &context)
            .map_err(|e| ElifError::system_error(format!("Failed to render api_routes template: {}", e)))?;

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

    fn create_template_context(&self, resources: &[ApiResource], options: &ApiOptions) -> HashMap<String, serde_json::Value> {
        let mut context = HashMap::new();
        
        context.insert("resources".to_string(), json!(resources));
        context.insert("version".to_string(), json!(options.version));
        context.insert("prefix".to_string(), json!(options.prefix));
        context.insert("with_versioning".to_string(), json!(options.with_versioning));
        context.insert("with_auth".to_string(), json!(options.with_auth));
        context.insert("title".to_string(), json!(format!("{} API", options.title.clone().unwrap_or_else(|| "Application".to_string()))));
        context.insert("description".to_string(), json!(options.description.clone().unwrap_or_else(|| "Generated API documentation".to_string())));
        context.insert("base_url".to_string(), json!(options.base_url.clone().unwrap_or_else(|| "http://localhost:3000".to_string())));
        context.insert("timestamp".to_string(), json!(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()));
        
        context
    }

    fn generate_openapi_spec(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<GeneratedFile, ElifError> {
        let context = self.create_template_context(resources, options);
        let content = self.template_engine.render("openapi_spec", &context)
            .map_err(|e| ElifError::system_error(format!("Failed to render openapi_spec template: {}", e)))?;

        Ok(GeneratedFile {
            path: self.project_root.join("openapi").join(format!("api_{}.yml", options.version)),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }



    fn generate_api_docs(
        &self,
        resources: &[ApiResource],
        options: &ApiOptions,
    ) -> Result<GeneratedFile, ElifError> {
        let context = self.create_template_context(resources, options);
        let content = self.template_engine.render("api_docs", &context)
            .map_err(|e| ElifError::system_error(format!("Failed to render api_docs template: {}", e)))?;

        Ok(GeneratedFile {
            path: self.project_root.join("docs").join(format!("api_{}.md", options.version)),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

}