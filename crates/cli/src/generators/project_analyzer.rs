use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use elif_core::ElifError;
use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ProjectAnalyzer {
    project_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub controllers: Vec<String>,
    pub services: Vec<String>,
    pub providers: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub modules: HashMap<String, ModuleInfo>,
    pub models: Vec<String>,
    pub services: Vec<String>,
    pub controllers: Vec<String>,
    pub middleware: Vec<String>,
    pub migrations: Vec<String>,
    pub dependencies: HashMap<String, Vec<String>>,
}

impl ProjectAnalyzer {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn analyze_project_structure(&self) -> Result<ProjectStructure, ElifError> {
        let mut structure = ProjectStructure {
            modules: HashMap::new(),
            models: Vec::new(),
            services: Vec::new(),
            controllers: Vec::new(),
            middleware: Vec::new(),
            migrations: Vec::new(),
            dependencies: HashMap::new(),
        };

        // Analyze modules
        self.analyze_modules(&mut structure)?;
        
        // Analyze models
        self.analyze_models(&mut structure)?;
        
        // Analyze services (global)
        self.analyze_global_services(&mut structure)?;
        
        // Analyze controllers (global)
        self.analyze_global_controllers(&mut structure)?;
        
        // Analyze middleware
        self.analyze_middleware(&mut structure)?;
        
        // Analyze migrations
        self.analyze_migrations(&mut structure)?;
        
        // Build dependency graph
        self.build_dependency_graph(&mut structure)?;

        Ok(structure)
    }

    fn analyze_modules(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        let modules_dir = self.project_root.join("src/modules");
        
        if !modules_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&modules_dir).map_err(|e| ElifError::Io(e))? {
            let entry = entry.map_err(|e| ElifError::Io(e))?;
            let path = entry.path();
            
            if path.is_dir() {
                let module_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| ElifError::Validation {
                        message: "Invalid module directory name".to_string()
                    })?
                    .to_string();
                
                let module_info = self.analyze_module(&path, &module_name)?;
                structure.modules.insert(module_name, module_info);
            }
        }

        Ok(())
    }

    fn analyze_module(&self, module_path: &Path, module_name: &str) -> Result<ModuleInfo, ElifError> {
        let mut module_info = ModuleInfo {
            name: module_name.to_string(),
            path: module_path.to_path_buf(),
            controllers: Vec::new(),
            services: Vec::new(),
            providers: Vec::new(),
            dependencies: Vec::new(),
        };

        // Analyze controllers
        let controllers_dir = module_path.join("controllers");
        if controllers_dir.exists() {
            module_info.controllers = self.find_rust_files(&controllers_dir)?;
        }

        // Analyze services
        let services_dir = module_path.join("services");
        if services_dir.exists() {
            module_info.services = self.find_rust_files(&services_dir)?;
        }

        // Analyze providers
        let providers_dir = module_path.join("providers");
        if providers_dir.exists() {
            module_info.providers = self.find_rust_files(&providers_dir)?;
        }

        // Extract dependencies from mod.rs
        let mod_file = module_path.join("mod.rs");
        if mod_file.exists() {
            module_info.dependencies = self.extract_dependencies_from_file(&mod_file)?;
        }

        Ok(module_info)
    }

    fn analyze_models(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        let models_dir = self.project_root.join("src/models");
        if models_dir.exists() {
            structure.models = self.find_rust_files(&models_dir)?;
        }
        Ok(())
    }

    fn analyze_global_services(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        let services_dir = self.project_root.join("src/services");
        if services_dir.exists() {
            structure.services = self.find_rust_files(&services_dir)?;
        }
        Ok(())
    }

    fn analyze_global_controllers(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        let controllers_dir = self.project_root.join("src/controllers");
        if controllers_dir.exists() {
            structure.controllers = self.find_rust_files(&controllers_dir)?;
        }
        Ok(())
    }

    fn analyze_middleware(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        let middleware_dir = self.project_root.join("src/middleware");
        if middleware_dir.exists() {
            structure.middleware = self.find_rust_files(&middleware_dir)?;
        }
        Ok(())
    }

    fn analyze_migrations(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        let migrations_dir = self.project_root.join("src/database/migrations");
        if migrations_dir.exists() {
            let mut migrations = Vec::new();
            for entry in fs::read_dir(&migrations_dir).map_err(|e| ElifError::Io(e))? {
                let entry = entry.map_err(|e| ElifError::Io(e))?;
                let path = entry.path();
                
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sql") {
                    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                        migrations.push(filename.to_string());
                    }
                }
            }
            structure.migrations = migrations;
        }
        Ok(())
    }

    fn find_rust_files(&self, dir: &Path) -> Result<Vec<String>, ElifError> {
        let mut files = Vec::new();
        
        if !dir.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(dir).map_err(|e| ElifError::Io(e))? {
            let entry = entry.map_err(|e| ElifError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if filename != "mod" { // Skip mod.rs files
                        files.push(filename.to_string());
                    }
                }
            }
        }

        Ok(files)
    }

    fn extract_dependencies_from_file(&self, file_path: &Path) -> Result<Vec<String>, ElifError> {
        let content = fs::read_to_string(file_path).map_err(|e| ElifError::Io(e))?;
        let mut dependencies = Vec::new();

        // Extract use statements and module declarations
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Look for use statements from crate modules
            if trimmed.starts_with("use crate::") {
                if let Some(import) = self.extract_crate_import(trimmed) {
                    dependencies.push(import);
                }
            }
            
            // Look for extern crate statements
            if trimmed.starts_with("extern crate ") {
                if let Some(crate_name) = trimmed.strip_prefix("extern crate ") {
                    let crate_name = crate_name.trim_end_matches(';');
                    dependencies.push(crate_name.to_string());
                }
            }
        }

        Ok(dependencies)
    }

    fn extract_crate_import(&self, use_statement: &str) -> Option<String> {
        // Extract module/service names from use statements like:
        // use crate::services::UserService;
        // use crate::modules::auth::AuthService;
        
        if let Some(import_part) = use_statement.strip_prefix("use crate::") {
            let import_part = import_part.trim_end_matches(';');
            let parts: Vec<&str> = import_part.split("::").collect();
            
            if parts.len() >= 2 {
                return Some(parts[parts.len() - 1].to_string());
            }
        }
        
        None
    }

    fn build_dependency_graph(&self, structure: &mut ProjectStructure) -> Result<(), ElifError> {
        // Build a comprehensive dependency graph
        for (module_name, module_info) in &structure.modules {
            let mut module_dependencies = HashSet::new();
            
            // Add dependencies from the module itself
            for dep in &module_info.dependencies {
                module_dependencies.insert(dep.clone());
            }
            
            // Add dependencies from services, controllers, etc.
            for service in &module_info.services {
                let service_file = module_info.path.join("services").join(format!("{}.rs", service));
                if service_file.exists() {
                    if let Ok(deps) = self.extract_dependencies_from_file(&service_file) {
                        for dep in deps {
                            module_dependencies.insert(dep);
                        }
                    }
                }
            }
            
            structure.dependencies.insert(
                module_name.clone(), 
                module_dependencies.into_iter().collect()
            );
        }

        Ok(())
    }

    pub fn suggest_module_for_resource(&self, resource_name: &str) -> Result<Option<String>, ElifError> {
        let structure = self.analyze_project_structure()?;
        
        // Simple heuristic: look for modules that might be related
        let resource_lower = resource_name.to_lowercase();
        
        for (module_name, _) in &structure.modules {
            let module_lower = module_name.to_lowercase();
            
            // Check for plural/singular matches
            if module_lower.contains(&resource_lower) || 
               resource_lower.contains(&module_lower) ||
               self.are_related_words(&resource_lower, &module_lower) {
                return Ok(Some(module_name.clone()));
            }
        }
        
        Ok(None)
    }

    fn are_related_words(&self, word1: &str, word2: &str) -> bool {
        // Simple word relationship detection
        // This could be enhanced with more sophisticated algorithms
        
        // Check if one is plural of the other
        let word1_singular = word1.trim_end_matches('s');
        let word2_singular = word2.trim_end_matches('s');
        
        word1_singular == word2_singular || 
        word1 == word2_singular ||
        word2 == word1_singular
    }

    pub fn generate_project_context(&self) -> Result<HashMap<String, Value>, ElifError> {
        let structure = self.analyze_project_structure()?;
        let mut context = HashMap::new();
        
        // Convert structure to JSON for template context
        context.insert("modules".to_string(), serde_json::to_value(&structure.modules)?);
        context.insert("models".to_string(), serde_json::to_value(&structure.models)?);
        context.insert("services".to_string(), serde_json::to_value(&structure.services)?);
        context.insert("controllers".to_string(), serde_json::to_value(&structure.controllers)?);
        context.insert("middleware".to_string(), serde_json::to_value(&structure.middleware)?);
        context.insert("migrations".to_string(), serde_json::to_value(&structure.migrations)?);
        context.insert("dependencies".to_string(), serde_json::to_value(&structure.dependencies)?);
        
        // Add project metadata
        context.insert("project_root".to_string(), serde_json::to_value(&self.project_root)?);
        context.insert("has_modules".to_string(), serde_json::to_value(!structure.modules.is_empty())?);
        context.insert("has_models".to_string(), serde_json::to_value(!structure.models.is_empty())?);
        
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_project_analyzer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        assert_eq!(analyzer.project_root, temp_dir.path());
    }

    #[test]
    fn test_extract_crate_import() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        assert_eq!(
            analyzer.extract_crate_import("use crate::services::UserService;"),
            Some("UserService".to_string())
        );
        
        assert_eq!(
            analyzer.extract_crate_import("use crate::modules::auth::AuthService;"),
            Some("AuthService".to_string())
        );
    }

    #[test]
    fn test_are_related_words() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        assert!(analyzer.are_related_words("user", "users"));
        assert!(analyzer.are_related_words("users", "user"));
        assert!(analyzer.are_related_words("blog", "blogs"));
        assert!(!analyzer.are_related_words("user", "blog"));
    }
}