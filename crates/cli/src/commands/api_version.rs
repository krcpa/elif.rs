use elif_core::ElifError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use std::collections::HashMap;

/// API version configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersionConfig {
    /// Current API versions
    pub versions: HashMap<String, ApiVersionInfo>,
    /// Default version
    pub default_version: Option<String>,
}

/// Information about a single API version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersionInfo {
    /// Version identifier
    pub version: String,
    /// Whether this version is deprecated
    pub deprecated: bool,
    /// Deprecation message
    pub deprecation_message: Option<String>,
    /// Sunset date (when version will be removed)
    pub sunset_date: Option<String>,
    /// Migration guide URL or path
    pub migration_guide: Option<String>,
    /// Breaking changes from previous version
    pub breaking_changes: Vec<String>,
}

/// Create a new API version
pub async fn create_version(version: &str, description: Option<String>) -> Result<(), ElifError> {
    println!("🔧 Creating API version: {}", version);
    
    let config_path = "api_versions.json";
    let mut config = load_version_config(config_path)?;
    
    // Check if version already exists
    if config.versions.contains_key(version) {
        return Err(ElifError::Validation { 
            message: format!("API version '{}' already exists", version) 
        });
    }
    
    // Add new version
    let version_info = ApiVersionInfo {
        version: version.to_string(),
        deprecated: false,
        deprecation_message: None,
        sunset_date: None,
        migration_guide: Some(format!("docs/api/migrations/v{}.md", version.replace("v", ""))),
        breaking_changes: Vec::new(),
    };
    
    config.versions.insert(version.to_string(), version_info);
    
    // Set as default if it's the first version
    if config.versions.len() == 1 {
        config.default_version = Some(version.to_string());
        println!("🎯 Set {} as the default API version", version);
    }
    
    // Save configuration
    save_version_config(config_path, &config)?;
    
    // Create migration guide
    create_migration_guide(version, description).await?;
    
    println!("✅ API version {} created successfully", version);
    println!("📖 Migration guide created at: docs/api/migrations/v{}.md", version.replace("v", ""));
    
    Ok(())
}

/// Deprecate an API version
pub async fn deprecate_version(version: &str, message: Option<String>, sunset_date: Option<String>) -> Result<(), ElifError> {
    println!("⚠️  Deprecating API version: {}", version);
    
    let config_path = "api_versions.json";
    let mut config = load_version_config(config_path)?;
    
    // Check if version exists
    let version_info = config.versions.get_mut(version)
        .ok_or_else(|| ElifError::Validation { 
            message: format!("API version '{}' not found", version) 
        })?;
    
    // Update deprecation info
    version_info.deprecated = true;
    version_info.deprecation_message = message.clone();
    version_info.sunset_date = sunset_date.clone();
    
    // Save configuration
    save_version_config(config_path, &config)?;
    
    println!("✅ API version {} marked as deprecated", version);
    if let Some(msg) = &message {
        println!("📝 Deprecation message: {}", msg);
    }
    if let Some(date) = &sunset_date {
        println!("🌅 Sunset date: {}", date);
    }
    
    Ok(())
}

/// List all API versions
pub async fn list_versions() -> Result<(), ElifError> {
    println!("📋 API Versions:");
    
    let config_path = "api_versions.json";
    let config = load_version_config(config_path)?;
    
    if config.versions.is_empty() {
        println!("   No API versions configured");
        return Ok(());
    }
    
    for (version, info) in &config.versions {
        let status = if info.deprecated { "⚠️  DEPRECATED" } else { "✅ ACTIVE" };
        let default = if config.default_version.as_ref() == Some(version) { " (DEFAULT)" } else { "" };
        
        println!("   {} {}{}", status, version, default);
        
        if let Some(msg) = &info.deprecation_message {
            println!("      📝 {}", msg);
        }
        
        if let Some(date) = &info.sunset_date {
            println!("      🌅 Sunset: {}", date);
        }
        
        if !info.breaking_changes.is_empty() {
            println!("      💥 Breaking changes: {}", info.breaking_changes.len());
        }
        
        if let Some(guide) = &info.migration_guide {
            println!("      📖 Migration guide: {}", guide);
        }
    }
    
    Ok(())
}

/// Generate migration guide between versions
pub async fn generate_migration_guide(from_version: &str, to_version: &str) -> Result<(), ElifError> {
    println!("📖 Generating migration guide from {} to {}", from_version, to_version);
    
    let config_path = "api_versions.json";
    let config = load_version_config(config_path)?;
    
    // Validate versions exist
    let from_info = config.versions.get(from_version)
        .ok_or_else(|| ElifError::Validation { 
            message: format!("Source version '{}' not found", from_version) 
        })?;
        
    let to_info = config.versions.get(to_version)
        .ok_or_else(|| ElifError::Validation { 
            message: format!("Target version '{}' not found", to_version) 
        })?;
    
    // Create migration guide content
    let guide_content = format!(
r#"# Migration Guide: {} to {}

## Overview
This guide helps you migrate from API version {} to version {}.

## Breaking Changes
{}

## New Features
- Enhanced error handling with version-aware responses
- Improved deprecation warnings
- Migration assistance headers

## Step-by-Step Migration

### 1. Update API Version
Update your API version header or URL path:

**Header-based versioning:**
```
Api-Version: {}
```

**URL-based versioning:**
```
/api/{}/endpoint
```

### 2. Handle Response Changes
Review response format changes and update your client code accordingly.

### 3. Test Your Integration
Run comprehensive tests to ensure compatibility with the new version.

## Compatibility Timeline
- **Deprecation Date**: {}
- **Sunset Date**: {}

## Support
If you need help with migration, check our documentation or contact support.
"#,
        from_version, to_version,
        from_version, to_version,
        if to_info.breaking_changes.is_empty() {
            "No breaking changes in this version.".to_string()
        } else {
            to_info.breaking_changes.iter()
                .map(|change| format!("- {}", change))
                .collect::<Vec<_>>()
                .join("\n")
        },
        to_version, to_version,
        from_info.deprecation_message.as_deref().unwrap_or("Not set"),
        from_info.sunset_date.as_deref().unwrap_or("Not set")
    );
    
    // Create migration guide file
    let guide_path = format!("docs/api/migrations/{}_to_{}.md", 
        from_version.replace("v", ""), to_version.replace("v", ""));
    
    if let Some(parent) = Path::new(&guide_path).parent() {
        fs::create_dir_all(parent).map_err(|e| ElifError::Io(e))?;
    }
    
    fs::write(&guide_path, guide_content).map_err(|e| ElifError::Io(e))?;
    
    println!("✅ Migration guide created at: {}", guide_path);
    Ok(())
}

/// Validate API version configuration
pub async fn validate_versions() -> Result<(), ElifError> {
    println!("🔍 Validating API version configuration...");
    
    let config_path = "api_versions.json";
    let config = load_version_config(config_path)?;
    
    let mut issues = Vec::new();
    
    // Check if default version exists
    if let Some(default) = &config.default_version {
        if !config.versions.contains_key(default) {
            issues.push(format!("Default version '{}' does not exist", default));
        }
    } else if !config.versions.is_empty() {
        issues.push("No default version set".to_string());
    }
    
    // Check for deprecated versions without sunset dates
    for (version, info) in &config.versions {
        if info.deprecated && info.sunset_date.is_none() {
            issues.push(format!("Deprecated version '{}' has no sunset date", version));
        }
        
        if let Some(guide_path) = &info.migration_guide {
            if !Path::new(guide_path).exists() {
                issues.push(format!("Migration guide for version '{}' not found: {}", version, guide_path));
            }
        }
    }
    
    if issues.is_empty() {
        println!("✅ API version configuration is valid");
    } else {
        println!("❌ Found {} issues:", issues.len());
        for issue in issues {
            println!("   - {}", issue);
        }
        return Err(ElifError::Validation { 
            message: "API version configuration has validation errors".to_string() 
        });
    }
    
    Ok(())
}

/// Load version configuration from file
fn load_version_config(path: &str) -> Result<ApiVersionConfig, ElifError> {
    if !Path::new(path).exists() {
        // Create default configuration
        let config = ApiVersionConfig {
            versions: HashMap::new(),
            default_version: None,
        };
        save_version_config(path, &config)?;
        return Ok(config);
    }
    
    let content = fs::read_to_string(path).map_err(|e| ElifError::Io(e))?;
    let config: ApiVersionConfig = serde_json::from_str(&content)
        .map_err(|e| ElifError::Json(e))?;
    
    Ok(config)
}

/// Save version configuration to file
fn save_version_config(path: &str, config: &ApiVersionConfig) -> Result<(), ElifError> {
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| ElifError::Json(e))?;
    
    fs::write(path, content).map_err(|e| ElifError::Io(e))?;
    Ok(())
}

/// Create a basic migration guide template
async fn create_migration_guide(version: &str, description: Option<String>) -> Result<(), ElifError> {
    let guide_content = format!(
r#"# API Version {} Migration Guide

## Overview
{}

## Changes in This Version
- List key changes here
- Breaking changes (if any)
- New features
- Deprecated features

## Migration Steps
1. Update your API version to `{}`
2. Review breaking changes below
3. Update your client code
4. Test your integration

## Breaking Changes
- None in this version

## New Features
- Enhanced error responses
- Improved validation messages
- Better deprecation warnings

## Examples

### Before (Previous Version)
```json
{{
  "error": "Not found"
}}
```

### After (Version {})
```json
{{
  "error": {{
    "code": "NOT_FOUND",
    "message": "Resource not found",
    "details": "The requested resource could not be found"
  }},
  "api_version": "{}",
  "migration_info": null
}}
```

## Support
For questions about this migration, consult the API documentation or contact support.
"#,
        version,
        description.unwrap_or_else(|| format!("Migration guide for API version {}", version)),
        version, version, version
    );
    
    let guide_path = format!("docs/api/migrations/v{}.md", version.replace("v", ""));
    
    if let Some(parent) = Path::new(&guide_path).parent() {
        fs::create_dir_all(parent).map_err(|e| ElifError::Io(e))?;
    }
    
    fs::write(guide_path, guide_content).map_err(|e| ElifError::Io(e))?;
    Ok(())
}