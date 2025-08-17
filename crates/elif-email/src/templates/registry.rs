use crate::{
    config::TemplateConfig,
    error::EmailError,
    templates::{EmailTemplate, TemplateEngine},
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
};
use tracing::{debug, info};

/// Template registry for managing multiple template engines
pub struct TemplateRegistry {
    engines: RwLock<HashMap<String, Arc<TemplateEngine>>>,
    default_engine: RwLock<Option<Arc<TemplateEngine>>>,
}

impl TemplateRegistry {
    /// Create new template registry
    pub fn new() -> Self {
        Self {
            engines: RwLock::new(HashMap::new()),
            default_engine: RwLock::new(None),
        }
    }

    /// Register a template engine
    pub fn register_engine(
        &self,
        name: impl Into<String>,
        engine: Arc<TemplateEngine>,
    ) -> Result<(), EmailError> {
        let name = name.into();
        
        let mut engines = self.engines.write().map_err(|_| {
            EmailError::template("Failed to acquire write lock on engines")
        })?;

        engines.insert(name.clone(), engine);
        
        // Set as default if it's the first engine
        if engines.len() == 1 {
            let mut default_engine = self.default_engine.write().map_err(|_| {
                EmailError::template("Failed to acquire write lock on default engine")
            })?;
            *default_engine = engines.get(&name).cloned();
        }

        debug!("Registered template engine: {}", name);
        Ok(())
    }

    /// Create and register a new template engine
    pub fn create_engine(
        &self,
        name: impl Into<String>,
        config: TemplateConfig,
    ) -> Result<Arc<TemplateEngine>, EmailError> {
        let engine = Arc::new(TemplateEngine::new(config)?);
        let name = name.into();
        
        self.register_engine(&name, engine.clone())?;
        
        info!("Created and registered template engine: {}", name);
        Ok(engine)
    }

    /// Get template engine by name
    pub fn get_engine(&self, name: &str) -> Result<Arc<TemplateEngine>, EmailError> {
        let engines = self.engines.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on engines")
        })?;

        engines.get(name)
            .cloned()
            .ok_or_else(|| EmailError::template(format!("Template engine '{}' not found", name)))
    }

    /// Get default template engine
    pub fn get_default_engine(&self) -> Result<Arc<TemplateEngine>, EmailError> {
        let default_engine = self.default_engine.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on default engine")
        })?;

        default_engine.clone()
            .ok_or_else(|| EmailError::template("No default template engine set"))
    }

    /// Set default template engine
    pub fn set_default_engine(&self, name: &str) -> Result<(), EmailError> {
        let engine = self.get_engine(name)?;
        
        let mut default_engine = self.default_engine.write().map_err(|_| {
            EmailError::template("Failed to acquire write lock on default engine")
        })?;
        
        *default_engine = Some(engine);
        
        debug!("Set default template engine: {}", name);
        Ok(())
    }

    /// List available template engines
    pub fn list_engines(&self) -> Result<Vec<String>, EmailError> {
        let engines = self.engines.read().map_err(|_| {
            EmailError::template("Failed to acquire read lock on engines")
        })?;

        Ok(engines.keys().cloned().collect())
    }

    /// Register template in default engine
    pub fn register_template(&self, template: EmailTemplate) -> Result<(), EmailError> {
        let engine = self.get_default_engine()?;
        engine.register_template(template)
    }

    /// Register template in specific engine
    pub fn register_template_in_engine(
        &self,
        engine_name: &str,
        template: EmailTemplate,
    ) -> Result<(), EmailError> {
        let engine = self.get_engine(engine_name)?;
        engine.register_template(template)
    }

    /// Get template from default engine
    pub fn get_template(&self, name: &str) -> Result<EmailTemplate, EmailError> {
        let engine = self.get_default_engine()?;
        engine.get_template(name)
    }

    /// Get template from specific engine
    pub fn get_template_from_engine(
        &self,
        engine_name: &str,
        template_name: &str,
    ) -> Result<EmailTemplate, EmailError> {
        let engine = self.get_engine(engine_name)?;
        engine.get_template(template_name)
    }

    /// Reload all engines (simplified - requires mutable access)
    pub fn reload_all(&self) -> Result<(), EmailError> {
        // TODO: Implement proper reloading with interior mutability
        info!("Template engine reloading not implemented yet");
        Ok(())
    }

    /// Reload specific engine (simplified - requires mutable access)
    pub fn reload_engine(&self, _name: &str) -> Result<(), EmailError> {
        // TODO: Implement proper reloading with interior mutability
        debug!("Template engine reloading not implemented yet");
        Ok(())
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global template registry instance
static GLOBAL_REGISTRY: std::sync::OnceLock<TemplateRegistry> = std::sync::OnceLock::new();

/// Get global template registry
pub fn global_registry() -> &'static TemplateRegistry {
    GLOBAL_REGISTRY.get_or_init(TemplateRegistry::new)
}

/// Initialize global template registry with config
pub fn init_global_registry(config: TemplateConfig) -> Result<(), EmailError> {
    let registry = global_registry();
    registry.create_engine("default", config)?;
    Ok(())
}

/// Template discovery utilities
pub mod discovery {
    use super::*;
    use std::fs;

    /// Discover templates in a directory structure
    pub fn discover_templates(
        base_dir: impl AsRef<Path>,
    ) -> Result<Vec<(String, EmailTemplate)>, EmailError> {
        let base_path = base_dir.as_ref();
        let mut templates = Vec::new();

        if !base_path.exists() {
            return Ok(templates);
        }

        let entries = fs::read_dir(base_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let template_name = path.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid template directory name: {:?}", path)))?;

                let template = discover_template_from_directory(template_name, &path)?;
                templates.push((template_name.to_string(), template));
            }
        }

        Ok(templates)
    }

    /// Discover template from directory structure
    fn discover_template_from_directory(
        name: &str,
        dir: &Path,
    ) -> Result<EmailTemplate, EmailError> {
        let mut builder = EmailTemplate::builder(name);

        // Look for different file variants
        let files = fs::read_dir(dir)?;
        
        for file in files {
            let file = file?;
            let file_path = file.path();
            
            if file_path.is_file() {
                let file_name = file_path.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| EmailError::template(format!("Invalid file name: {:?}", file_path)))?;

                match file_name {
                    "html.hbs" | "html.handlebars" => {
                        let content = fs::read_to_string(&file_path)?;
                        builder = builder.html_template(content);
                    }
                    "text.hbs" | "text.handlebars" => {
                        let content = fs::read_to_string(&file_path)?;
                        builder = builder.text_template(content);
                    }
                    "subject.hbs" | "subject.handlebars" => {
                        let content = fs::read_to_string(&file_path)?;
                        builder = builder.subject_template(content);
                    }
                    "meta.json" => {
                        let content = fs::read_to_string(&file_path)?;
                        let metadata: HashMap<String, String> = serde_json::from_str(&content)?;
                        for (key, value) in metadata {
                            builder = builder.metadata(key, value);
                        }
                    }
                    _ => {
                        // Ignore other files
                    }
                }
            }
        }

        Ok(builder.build())
    }

    /// Auto-discover and register templates in registry
    pub fn auto_discover_and_register(
        registry: &TemplateRegistry,
        engine_name: &str,
        base_dir: impl AsRef<Path>,
    ) -> Result<usize, EmailError> {
        let templates = discover_templates(base_dir)?;
        let mut count = 0;

        for (name, template) in templates {
            registry.register_template_in_engine(engine_name, template)?;
            debug!("Auto-discovered and registered template: {}", name);
            count += 1;
        }

        info!("Auto-discovered {} templates in engine '{}'", count, engine_name);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_template_registry() {
        let registry = TemplateRegistry::new();
        
        let temp_dir = TempDir::new().unwrap();
        let config = TemplateConfig {
            templates_dir: temp_dir.path().to_string_lossy().to_string(),
            layouts_dir: temp_dir.path().to_string_lossy().to_string(),
            partials_dir: temp_dir.path().to_string_lossy().to_string(),
            enable_cache: false,
            template_extension: ".hbs".to_string(),
        };

        let engine = registry.create_engine("test", config).unwrap();
        assert!(engine.list_templates().is_ok());

        let retrieved = registry.get_engine("test").unwrap();
        assert!(Arc::ptr_eq(&engine, &retrieved));
    }

    #[test]
    fn test_default_engine() {
        let registry = TemplateRegistry::new();
        
        let temp_dir = TempDir::new().unwrap();
        let config = TemplateConfig {
            templates_dir: temp_dir.path().to_string_lossy().to_string(),
            layouts_dir: temp_dir.path().to_string_lossy().to_string(),
            partials_dir: temp_dir.path().to_string_lossy().to_string(),
            enable_cache: false,
            template_extension: ".hbs".to_string(),
        };

        let engine = registry.create_engine("default", config).unwrap();
        let default_engine = registry.get_default_engine().unwrap();
        
        assert!(Arc::ptr_eq(&engine, &default_engine));
    }
}