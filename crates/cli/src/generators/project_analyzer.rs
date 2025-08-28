use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use elif_core::ElifError;
use serde_json::Value;
use serde::{Deserialize, Serialize};
use regex::Regex;

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
        let mut dependencies = HashSet::new();

        // Extract use statements from crate modules
        let crate_imports = self.extract_crate_imports(&content)?;
        dependencies.extend(crate_imports);

        // Extract extern crate statements
        let extern_crates = self.extract_extern_crates(&content)?;
        dependencies.extend(extern_crates);

        Ok(dependencies.into_iter().collect())
    }

    /// Extract all imports from crate modules using robust regex parsing
    /// Handles: use crate::services::UserService;
    ///         use crate::services::{UserService, AdminService};  
    ///         use crate::services::UserService as US;
    ///         use crate::modules::auth::{AuthService, TokenService};
    ///         Multi-line grouped imports with proper brace matching
    fn extract_crate_imports(&self, content: &str) -> Result<Vec<String>, ElifError> {
        let mut imports = Vec::new();
        
        // First, normalize the content by removing comments and extra whitespace
        let normalized = self.normalize_content(content);
        
        // Regex to match use crate:: statements, including multi-line ones
        // This handles cases where braces span multiple lines
        let use_regex = Regex::new(r"use\s+crate::([^;]+);")
            .map_err(|e| ElifError::Validation { message: format!("Invalid regex: {}", e) })?;
            
        for capture in use_regex.captures_iter(&normalized) {
            if let Some(import_part) = capture.get(1) {
                let import_str = import_part.as_str().trim();
                let parsed_imports = self.parse_import_statement(import_str)?;
                imports.extend(parsed_imports);
            }
        }
        
        Ok(imports)
    }
    
    /// Normalize content by removing comments and collapsing multi-line statements
    fn normalize_content(&self, content: &str) -> String {
        let mut normalized = String::new();
        let mut in_multiline_comment = false;
        let mut current_statement = String::new();
        let mut brace_depth = 0;
        
        for line in content.lines() {
            let mut line = line.to_string();
            
            // Handle multi-line comments
            if let Some(start) = line.find("/*") {
                if let Some(end) = line.find("*/") {
                    // Single-line /* */ comment
                    line = format!("{}{}", &line[..start], &line[end + 2..]);
                } else {
                    // Start of multi-line comment
                    line = line[..start].to_string();
                    in_multiline_comment = true;
                }
            }
            
            if in_multiline_comment {
                if let Some(end) = line.find("*/") {
                    line = line[end + 2..].to_string();
                    in_multiline_comment = false;
                } else {
                    continue;
                }
            }
            
            // Remove single-line comments
            if let Some(comment_pos) = line.find("//") {
                line = line[..comment_pos].to_string();
            }
            
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            // Handle multi-line statements (particularly use statements with braces)
            if trimmed.starts_with("use ") || !current_statement.is_empty() {
                current_statement.push(' ');
                current_statement.push_str(trimmed);
                
                // Count braces to detect statement completion
                for ch in trimmed.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => brace_depth -= 1,
                        ';' if brace_depth == 0 => {
                            // Statement complete
                            normalized.push_str(&current_statement);
                            normalized.push('\n');
                            current_statement.clear();
                            break;
                        }
                        _ => {}
                    }
                }
            } else {
                // Regular single-line statement
                normalized.push_str(trimmed);
                normalized.push('\n');
            }
        }
        
        // Add any remaining statement
        if !current_statement.is_empty() {
            normalized.push_str(&current_statement);
            normalized.push('\n');
        }
        
        normalized
    }
    
    /// Parse a single import statement to extract all imported items
    /// Handles: services::UserService
    ///         services::{UserService, AdminService}
    ///         services::UserService as US
    ///         modules::auth::{AuthService, TokenService}
    fn parse_import_statement(&self, import_str: &str) -> Result<Vec<String>, ElifError> {
        let mut imports = Vec::new();
        
        // Check if this is a grouped import (contains braces)
        if import_str.contains('{') && import_str.contains('}') {
            imports.extend(self.parse_grouped_import(import_str)?);
        } else {
            // Single import
            if let Some(single_import) = self.parse_single_import(import_str) {
                imports.push(single_import);
            }
        }
        
        Ok(imports)
    }
    
    /// Parse grouped imports like: services::{UserService, AdminService, super::BaseService}
    fn parse_grouped_import(&self, import_str: &str) -> Result<Vec<String>, ElifError> {
        let mut imports = Vec::new();
        
        // Regex to extract the grouped part
        let group_regex = Regex::new(r"([^{]+)\{([^}]+)\}")
            .map_err(|e| ElifError::Validation { message: format!("Invalid group regex: {}", e) })?;
            
        if let Some(captures) = group_regex.captures(import_str) {
            let _path_part = captures.get(1).map(|m| m.as_str().trim());
            let items_part = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            
            // Split by comma and clean each item
            for item in items_part.split(',') {
                let cleaned_item = item.trim();
                
                // Skip empty items and relative paths in grouped imports
                if cleaned_item.is_empty() || 
                   cleaned_item.starts_with("self::") || 
                   cleaned_item.starts_with("super::") ||
                   cleaned_item.starts_with("crate::") ||
                   cleaned_item == "self" ||
                   cleaned_item == "super" ||
                   cleaned_item == "crate" {
                    continue;
                }
                
                // Handle aliased imports (Item as Alias)
                let import_name = if let Some(as_pos) = cleaned_item.find(" as ") {
                    &cleaned_item[..as_pos]
                } else {
                    cleaned_item
                };
                
                // Extract just the final identifier
                if let Some(final_name) = import_name.split("::").last() {
                    if self.is_valid_rust_identifier(final_name) {
                        imports.push(final_name.to_string());
                    }
                }
            }
        }
        
        Ok(imports)
    }
    
    /// Parse single import like: services::UserService or services::UserService as US
    fn parse_single_import(&self, import_str: &str) -> Option<String> {
        // Handle aliased imports (remove " as Alias" part)
        let clean_import = if let Some(as_pos) = import_str.find(" as ") {
            &import_str[..as_pos]
        } else {
            import_str
        };
        
        // Split by :: and get the last part (the actual import name)
        let parts: Vec<&str> = clean_import.split("::").collect();
        if let Some(last_part) = parts.last() {
            let trimmed = last_part.trim();
            
            // Skip wildcard imports and bare relative keywords
            if trimmed == "*" || 
               trimmed == "self" || 
               trimmed == "super" ||
               trimmed == "crate" {
                return None;
            }
            
            // Validate it's a proper Rust identifier
            if self.is_valid_rust_identifier(trimmed) {
                return Some(trimmed.to_string());
            }
        }
        
        None
    }
    
    /// Extract extern crate statements
    fn extract_extern_crates(&self, content: &str) -> Result<Vec<String>, ElifError> {
        let mut crates = Vec::new();
        
        let extern_regex = Regex::new(r"extern\s+crate\s+([a-zA-Z_][a-zA-Z0-9_]*)")
            .map_err(|e| ElifError::Validation { message: format!("Invalid extern regex: {}", e) })?;
            
        for capture in extern_regex.captures_iter(content) {
            if let Some(crate_name) = capture.get(1) {
                crates.push(crate_name.as_str().to_string());
            }
        }
        
        Ok(crates)
    }
    
    /// Validate that a string is a valid Rust identifier
    fn is_valid_rust_identifier(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        
        // Must start with letter or underscore
        let mut chars = s.chars();
        if let Some(first) = chars.next() {
            if !first.is_ascii_alphabetic() && first != '_' {
                return false;
            }
        }
        
        // Rest must be alphanumeric or underscore
        for c in chars {
            if !c.is_ascii_alphanumeric() && c != '_' {
                return false;
            }
        }
        
        true
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
    use tempfile::TempDir;

    #[test]
    fn test_project_analyzer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        assert_eq!(analyzer.project_root, temp_dir.path());
    }

    #[test]
    fn test_extract_crate_imports() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        // Test simple import
        let content1 = "use crate::services::UserService;";
        let imports1 = analyzer.extract_crate_imports(content1).unwrap();
        assert_eq!(imports1, vec!["UserService"]);
        
        // Test grouped import  
        let content2 = "use crate::services::{UserService, AdminService};";
        let imports2 = analyzer.extract_crate_imports(content2).unwrap();
        let mut sorted_imports2 = imports2.clone();
        sorted_imports2.sort();
        assert_eq!(sorted_imports2, vec!["AdminService", "UserService"]);
        
        // Test aliased import
        let content3 = "use crate::services::UserService as US;";
        let imports3 = analyzer.extract_crate_imports(content3).unwrap();
        assert_eq!(imports3, vec!["UserService"]);
        
        // Test complex grouped import with aliases
        let content4 = "use crate::modules::auth::{AuthService, TokenService as TS, super::BaseService};";
        let imports4 = analyzer.extract_crate_imports(content4).unwrap();
        let mut sorted_imports4 = imports4.clone();
        sorted_imports4.sort();
        assert_eq!(sorted_imports4, vec!["AuthService", "TokenService"]);
        
        // Test multiple use statements
        let content5 = r#"
            use crate::services::UserService;
            use crate::controllers::UserController;
            use crate::models::{User, Role, Permission};
            use std::collections::HashMap;  // Should be ignored
        "#;
        let imports5 = analyzer.extract_crate_imports(content5).unwrap();
        let mut sorted_imports5 = imports5.clone();
        sorted_imports5.sort();
        assert_eq!(sorted_imports5, vec!["Permission", "Role", "User", "UserController", "UserService"]);
    }
    
    #[test]
    fn test_parse_grouped_import() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        // Test basic grouped import
        let imports1 = analyzer.parse_grouped_import("services::{UserService, AdminService}").unwrap();
        let mut sorted_imports1 = imports1.clone();
        sorted_imports1.sort();
        assert_eq!(sorted_imports1, vec!["AdminService", "UserService"]);
        
        // Test grouped import with aliases
        let imports2 = analyzer.parse_grouped_import("services::{UserService as US, AdminService}").unwrap();
        let mut sorted_imports2 = imports2.clone();
        sorted_imports2.sort();
        assert_eq!(sorted_imports2, vec!["AdminService", "UserService"]);
        
        // Test grouped import with relative paths (should be filtered out)
        let imports3 = analyzer.parse_grouped_import("services::{UserService, self::LocalService, super::ParentService}").unwrap();
        assert_eq!(imports3, vec!["UserService"]);
        
        // Test empty or invalid groups
        let imports4 = analyzer.parse_grouped_import("services::{}").unwrap();
        assert!(imports4.is_empty());
    }
    
    #[test]
    fn test_parse_single_import() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        // Test basic single import
        assert_eq!(
            analyzer.parse_single_import("services::UserService"),
            Some("UserService".to_string())
        );
        
        // Test aliased single import
        assert_eq!(
            analyzer.parse_single_import("services::UserService as US"),
            Some("UserService".to_string())
        );
        
        // Test deep path import
        assert_eq!(
            analyzer.parse_single_import("modules::auth::services::AuthService"),
            Some("AuthService".to_string())
        );
        
        // Test wildcard import (should be filtered out)
        assert_eq!(analyzer.parse_single_import("services::*"), None);
        
        // Test relative imports - `self::LocalService` extracts `LocalService`
        assert_eq!(
            analyzer.parse_single_import("self::LocalService"), 
            Some("LocalService".to_string())
        );
        
        // But bare relative keywords should be filtered out
        assert_eq!(analyzer.parse_single_import("self"), None);
        assert_eq!(analyzer.parse_single_import("super"), None);
    }
    
    #[test]
    fn test_is_valid_rust_identifier() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        // Valid identifiers
        assert!(analyzer.is_valid_rust_identifier("UserService"));
        assert!(analyzer.is_valid_rust_identifier("_private"));
        assert!(analyzer.is_valid_rust_identifier("user_service"));
        assert!(analyzer.is_valid_rust_identifier("User123"));
        
        // Invalid identifiers
        assert!(!analyzer.is_valid_rust_identifier("123User"));
        assert!(!analyzer.is_valid_rust_identifier("user-service"));
        assert!(!analyzer.is_valid_rust_identifier(""));
        assert!(!analyzer.is_valid_rust_identifier("user.service"));
        assert!(!analyzer.is_valid_rust_identifier("AdminService}"));
    }
    
    #[test]
    fn test_complex_real_world_imports() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = ProjectAnalyzer::new(temp_dir.path().to_path_buf());
        
        // Realistic Rust file content
        let content = r#"
            use std::collections::HashMap;
            use std::sync::Arc;
            
            use crate::services::{UserService, AdminService, EmailService as ES};
            use crate::models::{User, Role};
            use crate::controllers::auth::{AuthController, TokenController};
            use crate::middleware::auth::{JwtMiddleware, SessionMiddleware};
            use crate::middleware::cors::CorsMiddleware;
            use crate::utils::*;  // Wildcard should be ignored
            
            extern crate serde;
            extern crate tokio;
        "#;
        
        let crate_imports = analyzer.extract_crate_imports(content).unwrap();
        let extern_crates = analyzer.extract_extern_crates(content).unwrap();
        
        // Verify crate imports
        let mut sorted_crate = crate_imports.clone();
        sorted_crate.sort();
        assert_eq!(
            sorted_crate, 
            vec![
                "AdminService", "AuthController", "CorsMiddleware", 
                "EmailService", "JwtMiddleware", "Role", "SessionMiddleware", 
                "TokenController", "User", "UserService"
            ]
        );
        
        // Verify extern crates
        let mut sorted_extern = extern_crates.clone();
        sorted_extern.sort();
        assert_eq!(sorted_extern, vec!["serde", "tokio"]);
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