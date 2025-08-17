use crate::{
    config::TemplateConfig,
    error::EmailError,
    templates::{EmailTemplate, RenderedEmail, TemplateContext, helpers},
};
use handlebars::{Handlebars, no_escape};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::RwLock,
};
use tracing::debug;

/// Email template engine using Handlebars
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    config: TemplateConfig,
    templates: RwLock<HashMap<String, EmailTemplate>>,
    layouts: RwLock<HashMap<String, String>>,
    partials: RwLock<HashMap<String, String>>,
}

impl TemplateEngine {
    /// Create new template engine
    pub fn new(config: TemplateConfig) -> Result<Self, EmailError> {
        let mut handlebars = Handlebars::new();
        
        // Register built-in helpers
        helpers::register_email_helpers(&mut handlebars);
        
        // Disable HTML escaping for email content
        handlebars.register_escape_fn(no_escape);
        
        let engine = Self {
            handlebars,
            config,
            templates: RwLock::new(HashMap::new()),
            layouts: RwLock::new(HashMap::new()),
            partials: RwLock::new(HashMap::new()),
        };

        // Load templates, layouts, and partials from filesystem if directories exist
        let mut engine_mut = engine;
        engine_mut.load_from_filesystem()?;

        Ok(engine_mut)
    }

    /// Load templates from filesystem
    pub fn load_from_filesystem(&mut self) -> Result<(), EmailError> {
        // Load layouts
        if Path::new(&self.config.layouts_dir).exists() {
            self.load_layouts()?;
        } else {
            debug!("Layouts directory does not exist: {}", self.config.layouts_dir);
        }

        // Load partials
        if Path::new(&self.config.partials_dir).exists() {
            self.load_partials()?;
        } else {
            debug!("Partials directory does not exist: {}", self.config.partials_dir);
        }

        // Load templates
        if Path::new(&self.config.templates_dir).exists() {
            self.load_templates_from_directory()?;
        } else {
            debug!("Templates directory does not exist: {}", self.config.templates_dir);
        }

        Ok(())
    }

    /// Load layouts from directory
    fn load_layouts(&self) -> Result<(), EmailError> {
        let layouts_path = Path::new(&self.config.layouts_dir);
        let entries = fs::read_dir(layouts_path)?;

        let mut layouts = self.layouts.write().map_err(|_| {
            EmailError::template("Failed to acquire write lock on layouts")
        })?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "hbs") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid layout filename: {:?}", path)))?;
                
                let content = fs::read_to_string(&path)?;
                layouts.insert(name.to_string(), content);
                
                debug!("Loaded layout: {}", name);
            }
        }

        Ok(())
    }

    /// Load partials from directory
    fn load_partials(&mut self) -> Result<(), EmailError> {
        let partials_path = Path::new(&self.config.partials_dir);
        let entries = fs::read_dir(partials_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "hbs") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid partial filename: {:?}", path)))?;
                
                let content = fs::read_to_string(&path)?;
                
                self.handlebars.register_partial(name, content)?;
                
                let mut partials = self.partials.write().map_err(|_| {
                    EmailError::template("Failed to acquire write lock on partials")
                })?;
                partials.insert(name.to_string(), fs::read_to_string(&path)?);
                
                debug!("Loaded partial: {}", name);
            }
        }

        Ok(())
    }

    /// Load templates from directory
    fn load_templates_from_directory(&self) -> Result<(), EmailError> {
        let templates_path = Path::new(&self.config.templates_dir);
        let entries = fs::read_dir(templates_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Each subdirectory represents a template with HTML/text variants
                let template_name = path.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid template directory name: {:?}", path)))?;
                
                self.load_template_from_directory(template_name, &path)?;
            } else if path.is_file() && path.extension().map_or(false, |ext| ext == "hbs") {
                // Single file template (assume HTML)
                let template_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid template filename: {:?}", path)))?;
                
                let content = fs::read_to_string(&path)?;
                let template = EmailTemplate::builder(template_name)
                    .html_template(content)
                    .build();
                
                self.register_template(template)?;
            }
        }

        Ok(())
    }

    /// Load a specific template from its directory
    fn load_template_from_directory(&self, name: &str, dir: &Path) -> Result<(), EmailError> {
        let mut template_builder = EmailTemplate::builder(name);
        
        // Look for HTML template
        let html_path = dir.join(format!("html{}", self.config.template_extension));
        if html_path.exists() {
            let html_content = fs::read_to_string(&html_path)?;
            template_builder = template_builder.html_template(html_content);
        }

        // Look for text template
        let text_path = dir.join(format!("text{}", self.config.template_extension));
        if text_path.exists() {
            let text_content = fs::read_to_string(&text_path)?;
            template_builder = template_builder.text_template(text_content);
        }

        // Look for subject template
        let subject_path = dir.join(format!("subject{}", self.config.template_extension));
        if subject_path.exists() {
            let subject_content = fs::read_to_string(&subject_path)?;
            template_builder = template_builder.subject_template(subject_content);
        }

        // Look for metadata file
        let metadata_path = dir.join("meta.json");
        if metadata_path.exists() {
            let metadata_content = fs::read_to_string(&metadata_path)?;
            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_content)?;
            for (key, value) in metadata {
                template_builder = template_builder.metadata(key, value);
            }
        }

        let template = template_builder.build();
        self.register_template(template)?;
        
        debug!("Loaded template: {}", name);
        Ok(())
    }

    /// Register a template
    pub fn register_template(&self, template: EmailTemplate) -> Result<(), EmailError> {
        let mut templates = self.templates.write().map_err(|_| {
            EmailError::template("Failed to acquire write lock on templates")
        })?;

        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Get template by name
    pub fn get_template(&self, name: &str) -> Result<EmailTemplate, EmailError> {
        let templates = self.templates.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on templates")
        })?;

        templates.get(name)
            .cloned()
            .ok_or_else(|| EmailError::template(format!("Template '{}' not found", name)))
    }

    /// Render template with context
    pub fn render_template(
        &self,
        template: &EmailTemplate,
        context: &TemplateContext,
    ) -> Result<RenderedEmail, EmailError> {
        // Add template metadata to context
        let mut extended_context = context.clone();
        for (key, value) in &template.metadata {
            extended_context.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        // Render subject
        let subject = if let Some(subject_template) = &template.subject_template {
            self.handlebars.render_template(subject_template, &extended_context)?
        } else {
            // Fallback to a default subject
            extended_context.get("subject")
                .and_then(|v| v.as_str())
                .unwrap_or("No Subject")
                .to_string()
        };

        // Render HTML content
        let html_content = if let Some(html_template) = &template.html_template {
            let rendered_html = self.handlebars.render_template(html_template, &extended_context)?;
            
            // Apply layout if specified
            if let Some(layout_name) = &template.layout {
                let layout = self.get_layout(layout_name)?;
                let mut layout_context = extended_context.clone();
                layout_context.insert("content".to_string(), serde_json::Value::String(rendered_html));
                layout_context.insert("subject".to_string(), serde_json::Value::String(subject.clone()));
                
                Some(self.handlebars.render_template(&layout, &layout_context)?)
            } else {
                Some(rendered_html)
            }
        } else {
            None
        };

        // Render text content
        let text_content = if let Some(text_template) = &template.text_template {
            Some(self.handlebars.render_template(text_template, &extended_context)?)
        } else {
            None
        };

        Ok(RenderedEmail {
            html_content,
            text_content,
            subject,
        })
    }

    /// Get layout by name
    fn get_layout(&self, name: &str) -> Result<String, EmailError> {
        let layouts = self.layouts.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on layouts")
        })?;

        layouts.get(name)
            .cloned()
            .ok_or_else(|| EmailError::template(format!("Layout '{}' not found", name)))
    }

    /// List available templates
    pub fn list_templates(&self) -> Result<Vec<String>, EmailError> {
        let templates = self.templates.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on templates")
        })?;

        Ok(templates.keys().cloned().collect())
    }

    /// Reload templates from filesystem
    pub fn reload(&mut self) -> Result<(), EmailError> {
        // Clear existing templates, layouts, and partials
        {
            let mut templates = self.templates.write().map_err(|_| {
                EmailError::template("Failed to acquire write lock on templates")
            })?;
            templates.clear();
        }
        
        {
            let mut layouts = self.layouts.write().map_err(|_| {
                EmailError::template("Failed to acquire write lock on layouts")
            })?;
            layouts.clear();
        }

        {
            let mut partials = self.partials.write().map_err(|_| {
                EmailError::template("Failed to acquire write lock on partials")
            })?;
            partials.clear();
        }

        // Reload from filesystem
        self.load_from_filesystem()?;
        
        debug!("Templates reloaded successfully");
        Ok(())
    }

    /// Register custom helper
    pub fn register_helper<F>(&mut self, name: &str, helper: F) -> Result<(), EmailError>
    where
        F: handlebars::HelperDef + Send + Sync + 'static,
    {
        self.handlebars.register_helper(name, Box::new(helper));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_config(temp_dir: &TempDir) -> TemplateConfig {
        TemplateConfig {
            templates_dir: temp_dir.path().join("templates").to_string_lossy().to_string(),
            layouts_dir: temp_dir.path().join("layouts").to_string_lossy().to_string(),
            partials_dir: temp_dir.path().join("partials").to_string_lossy().to_string(),
            enable_cache: false,
            template_extension: ".hbs".to_string(),
        }
    }

    #[test]
    fn test_template_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_template_registration_and_rendering() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let engine = TemplateEngine::new(config).unwrap();

        let template = EmailTemplate::builder("test")
            .html_template("<h1>Hello {{name}}</h1>")
            .text_template("Hello {{name}}")
            .subject_template("Welcome {{name}}")
            .build();

        engine.register_template(template).unwrap();

        let mut context = TemplateContext::new();
        context.insert("name".to_string(), serde_json::Value::String("World".to_string()));

        let template = engine.get_template("test").unwrap();
        let rendered = engine.render_template(&template, &context).unwrap();

        assert_eq!(rendered.subject, "Welcome World");
        assert_eq!(rendered.html_content, Some("<h1>Hello World</h1>".to_string()));
        assert_eq!(rendered.text_content, Some("Hello World".to_string()));
    }
}