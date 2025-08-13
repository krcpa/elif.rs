use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub database: DatabaseConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url_env: String,
    pub migrations_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub openapi_file: String,
    pub map_file: String,
}

impl ProjectConfig {
    pub fn load(path: &PathBuf) -> Result<Self, crate::ElifError> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn manifest_path() -> PathBuf {
        PathBuf::from(".elif/manifest.yaml")
    }
    
    pub fn errors_path() -> PathBuf {
        PathBuf::from(".elif/errors.yaml")
    }
    
    pub fn policies_path() -> PathBuf {
        PathBuf::from(".elif/policies.yaml")
    }
}