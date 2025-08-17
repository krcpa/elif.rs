use crate::{
    config::TemplateConfig,
    error::EmailError,
    templates::{EmailTemplate, RenderedEmail, TemplateContext, TemplateDebugInfo, helpers},
};
use handlebars::{Handlebars, no_escape};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::RwLock,
    time::SystemTime,
};
use tracing::{debug, warn};

/// Email template engine using Handlebars
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    config: TemplateConfig,
    templates: RwLock<HashMap<String, EmailTemplate>>,
    layouts: RwLock<HashMap<String, String>>,
    partials: RwLock<HashMap<String, String>>,
    // Template file cache with modification times
    template_file_cache: RwLock<HashMap<String, (SystemTime, String)>>,
    layout_file_cache: RwLock<HashMap<String, (SystemTime, String)>>,
    partial_file_cache: RwLock<HashMap<String, (SystemTime, String)>>,
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
            template_file_cache: RwLock::new(HashMap::new()),
            layout_file_cache: RwLock::new(HashMap::new()),
            partial_file_cache: RwLock::new(HashMap::new()),
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

        let mut cache = if self.config.enable_cache {
            Some(self.layout_file_cache.write().map_err(|_| {
                EmailError::template("Failed to acquire write lock on layout file cache")
            })?)
        } else {
            None
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "hbs") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid layout filename: {:?}", path)))?;
                
                let path_str = path.to_string_lossy().to_string();
                let should_load = if self.config.enable_cache {
                    if let (Some(cache), Ok(metadata)) = (cache.as_ref(), fs::metadata(&path)) {
                        if let (Ok(modified), Some((cached_time, _))) = (metadata.modified(), cache.get(&path_str)) {
                            // Only load if file has been modified since last cache
                            modified > *cached_time
                        } else {
                            // File not in cache or metadata error, load it
                            true
                        }
                    } else {
                        true
                    }
                } else {
                    // No caching, always load
                    true
                };

                if should_load {
                    let content = fs::read_to_string(&path)?;
                    layouts.insert(name.to_string(), content.clone());
                    
                    // Update cache if enabled
                    if let (Some(cache), Ok(metadata)) = (cache.as_mut(), fs::metadata(&path)) {
                        if let Ok(modified) = metadata.modified() {
                            cache.insert(path_str, (modified, content));
                        }
                    }
                    
                    debug!("Loaded layout: {}", name);
                } else {
                    // Use cached content
                    if let Some(cache) = cache.as_ref() {
                        if let Some((_, cached_content)) = cache.get(&path_str) {
                            layouts.insert(name.to_string(), cached_content.clone());
                            debug!("Using cached layout: {}", name);
                        }
                    }
                }
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
            // Check if template uses {% extends %} syntax
            let processed_template = self.process_template_inheritance(html_template, &extended_context)?;
            let rendered_html = self.handlebars.render_template(&processed_template, &extended_context)?;
            
            // Apply layout if specified (fallback for old layout system)
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

    /// Process template inheritance using {% extends %} syntax
    fn process_template_inheritance(&self, template: &str, _context: &TemplateContext) -> Result<String, EmailError> {
        use regex::Regex;
        
        // Check if template has {% extends %} directive
        let extends_regex = Regex::new(r#"^\s*\{%\s*extends\s+["']([^"']+)["']\s*%\}\s*"#)
            .map_err(|e| EmailError::template(format!("Failed to compile regex: {}", e)))?;
        
        if let Some(captures) = extends_regex.captures(template) {
            let parent_name = captures.get(1)
                .ok_or_else(|| EmailError::template("Invalid extends syntax"))?
                .as_str();
            
            // Remove the extends directive from the template
            let child_template = extends_regex.replace(template, "").to_string();
            
            // Get parent layout
            let parent_layout = self.get_layout(parent_name)?;
            
            // Process template blocks
            self.process_template_blocks(&parent_layout, &child_template)
        } else {
            // No inheritance, return template as-is
            Ok(template.to_string())
        }
    }

    /// Process template blocks for inheritance
    fn process_template_blocks(&self, parent: &str, child: &str) -> Result<String, EmailError> {
        use regex::Regex;
        use std::collections::HashMap;
        
        // Extract blocks from child template
        let block_regex = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}(.*?)\{%\s*endblock\s*%\}"#)
            .map_err(|e| EmailError::template(format!("Failed to compile block regex: {}", e)))?;
        
        let mut child_blocks = HashMap::new();
        for captures in block_regex.captures_iter(child) {
            let block_name = captures.get(1)
                .ok_or_else(|| EmailError::template("Invalid block syntax"))?
                .as_str();
            let block_content = captures.get(2)
                .ok_or_else(|| EmailError::template("Invalid block content"))?
                .as_str();
            
            child_blocks.insert(block_name.to_string(), block_content.trim().to_string());
        }
        
        // Replace blocks in parent template
        let mut result = parent.to_string();
        for (block_name, block_content) in child_blocks {
            let block_pattern = format!(r#"\{{\{{\s*block\s+{}\s*\}}\}}(.*?)\{{\{{\s*endblock\s*\}}\}}"#, block_name);
            let block_regex = Regex::new(&block_pattern)
                .map_err(|e| EmailError::template(format!("Failed to compile replacement regex: {}", e)))?;
            
            // Replace with child content
            result = block_regex.replace_all(&result, &block_content).to_string();
        }
        
        // Handle default block syntax for handlebars
        let default_block_regex = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}(.*?)\{%\s*endblock\s*%\}"#)
            .map_err(|e| EmailError::template(format!("Failed to compile default block regex: {}", e)))?;
        
        result = default_block_regex.replace_all(&result, r#"{{$2}}"#).to_string();
        
        Ok(result)
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

        // Clear file caches if caching is enabled
        if self.config.enable_cache {
            {
                let mut cache = self.template_file_cache.write().map_err(|_| {
                    EmailError::template("Failed to acquire write lock on template file cache")
                })?;
                cache.clear();
            }
            
            {
                let mut cache = self.layout_file_cache.write().map_err(|_| {
                    EmailError::template("Failed to acquire write lock on layout file cache")
                })?;
                cache.clear();
            }

            {
                let mut cache = self.partial_file_cache.write().map_err(|_| {
                    EmailError::template("Failed to acquire write lock on partial file cache")
                })?;
                cache.clear();
            }
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

    /// Check if any template files need to be reloaded
    pub fn check_for_changes(&mut self) -> Result<bool, EmailError> {
        if !self.config.enable_cache {
            // If caching is disabled, always return true to force reload
            return Ok(true);
        }

        let mut has_changes = false;

        // Check template files
        if let Ok(entries) = fs::read_dir(&self.config.templates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        let path_str = path.to_string_lossy().to_string();
                        if let Ok(cache) = self.template_file_cache.read() {
                            if let Some((cached_time, _)) = cache.get(&path_str) {
                                if modified > *cached_time {
                                    has_changes = true;
                                    break;
                                }
                            } else {
                                // New file not in cache
                                has_changes = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Check layout files
        if !has_changes {
            if let Ok(entries) = fs::read_dir(&self.config.layouts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            let path_str = path.to_string_lossy().to_string();
                            if let Ok(cache) = self.layout_file_cache.read() {
                                if let Some((cached_time, _)) = cache.get(&path_str) {
                                    if modified > *cached_time {
                                        has_changes = true;
                                        break;
                                    }
                                } else {
                                    // New file not in cache
                                    has_changes = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check partial files
        if !has_changes {
            if let Ok(entries) = fs::read_dir(&self.config.partials_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            let path_str = path.to_string_lossy().to_string();
                            if let Ok(cache) = self.partial_file_cache.read() {
                                if let Some((cached_time, _)) = cache.get(&path_str) {
                                    if modified > *cached_time {
                                        has_changes = true;
                                        break;
                                    }
                                } else {
                                    // New file not in cache
                                    has_changes = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(has_changes)
    }

    /// Hot-reload templates if there are changes
    pub fn hot_reload(&mut self) -> Result<bool, EmailError> {
        if self.check_for_changes()? {
            debug!("Template changes detected, reloading...");
            self.reload()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Force reload all templates regardless of cache
    pub fn force_reload(&mut self) -> Result<(), EmailError> {
        debug!("Force reloading all templates...");
        self.reload()
    }

    /// Validate a template string for syntax errors
    pub fn validate_template(&self, template: &str, template_name: Option<&str>) -> Result<(), EmailError> {
        // Try to compile the template to check for syntax errors
        match self.handlebars.render_template(template, &HashMap::<String, serde_json::Value>::new()) {
            Ok(_) => Ok(()),
            Err(e) => {
                let context = if let Some(name) = template_name {
                    format!("template '{}': {}", name, e)
                } else {
                    format!("template: {}", e)
                };
                Err(EmailError::template(format!("Invalid {}", context)))
            }
        }
    }

    /// Validate all loaded templates
    pub fn validate_all_templates(&self) -> Result<Vec<String>, EmailError> {
        let mut errors = Vec::new();
        
        // Validate templates
        let templates = self.templates.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on templates")
        })?;

        for (name, template) in templates.iter() {
            // Validate HTML template
            if let Some(html) = &template.html_template {
                if let Err(e) = self.validate_template(html, Some(&format!("{} (HTML)", name))) {
                    errors.push(e.to_string());
                }
            }

            // Validate text template
            if let Some(text) = &template.text_template {
                if let Err(e) = self.validate_template(text, Some(&format!("{} (Text)", name))) {
                    errors.push(e.to_string());
                }
            }

            // Validate subject template
            if let Some(subject) = &template.subject_template {
                if let Err(e) = self.validate_template(subject, Some(&format!("{} (Subject)", name))) {
                    errors.push(e.to_string());
                }
            }
        }

        // Validate layouts
        let layouts = self.layouts.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on layouts")
        })?;

        for (name, layout) in layouts.iter() {
            if let Err(e) = self.validate_template(layout, Some(&format!("layout '{}'", name))) {
                errors.push(e.to_string());
            }
        }

        // Validate partials
        let partials = self.partials.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on partials")
        })?;

        for (name, partial) in partials.iter() {
            if let Err(e) = self.validate_template(partial, Some(&format!("partial '{}'", name))) {
                errors.push(e.to_string());
            }
        }

        Ok(errors)
    }

    /// Get detailed template information for debugging
    pub fn get_template_info(&self, template_name: &str) -> Result<TemplateDebugInfo, EmailError> {
        let templates = self.templates.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on templates")
        })?;

        let template = templates.get(template_name)
            .ok_or_else(|| EmailError::template(format!("Template '{}' not found", template_name)))?;

        Ok(TemplateDebugInfo {
            name: template.name.clone(),
            has_html: template.html_template.is_some(),
            has_text: template.text_template.is_some(),
            has_subject: template.subject_template.is_some(),
            layout: template.layout.clone(),
            metadata: template.metadata.clone(),
            html_content: template.html_template.clone(),
            text_content: template.text_template.clone(),
            subject_content: template.subject_template.clone(),
        })
    }

    /// List all validation errors in a human-readable format
    pub fn get_validation_report(&self) -> Result<String, EmailError> {
        let errors = self.validate_all_templates()?;
        
        if errors.is_empty() {
            Ok("✅ All templates are valid".to_string())
        } else {
            let mut report = format!("❌ Found {} validation error(s):\n\n", errors.len());
            for (i, error) in errors.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, error));
            }
            Ok(report)
        }
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