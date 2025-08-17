//! Storage configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum file size in bytes (None = unlimited)
    pub max_file_size: Option<u64>,
    
    /// Allowed file types (MIME type prefixes, None = all allowed)
    pub allowed_types: Option<Vec<String>>,
    
    /// Default cache control header
    pub default_cache_control: Option<String>,
    
    /// Enable file validation
    pub validate_files: bool,
    
    /// Enable access control
    #[cfg(feature = "access-control")]
    pub enable_access_control: bool,
    
    /// Temporary file cleanup interval (in seconds)
    pub cleanup_interval: u64,
    
    /// Temporary file max age (in seconds)
    pub temp_file_max_age: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_file_size: Some(100 * 1024 * 1024), // 100MB default
            allowed_types: None, // All types allowed by default
            default_cache_control: Some("public, max-age=3600".to_string()),
            validate_files: true,
            #[cfg(feature = "access-control")]
            enable_access_control: false,
            cleanup_interval: 3600, // 1 hour
            temp_file_max_age: 86400, // 24 hours
        }
    }
}

impl StorageConfig {
    /// Create a new storage configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set maximum file size
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = Some(size);
        self
    }
    
    /// Remove file size limit
    pub fn unlimited_file_size(mut self) -> Self {
        self.max_file_size = None;
        self
    }
    
    /// Set allowed file types
    pub fn with_allowed_types(mut self, types: Vec<String>) -> Self {
        self.allowed_types = Some(types);
        self
    }
    
    /// Allow all file types
    pub fn allow_all_types(mut self) -> Self {
        self.allowed_types = None;
        self
    }
    
    /// Add allowed file type
    pub fn allow_type(mut self, mime_type: String) -> Self {
        match &mut self.allowed_types {
            Some(types) => types.push(mime_type),
            None => self.allowed_types = Some(vec![mime_type]),
        }
        self
    }
    
    /// Set default cache control
    pub fn with_cache_control(mut self, cache_control: String) -> Self {
        self.default_cache_control = Some(cache_control);
        self
    }
    
    /// Disable cache control
    pub fn no_cache_control(mut self) -> Self {
        self.default_cache_control = None;
        self
    }
    
    /// Enable file validation
    pub fn with_validation(mut self) -> Self {
        self.validate_files = true;
        self
    }
    
    /// Disable file validation
    pub fn without_validation(mut self) -> Self {
        self.validate_files = false;
        self
    }
    
    /// Set cleanup interval
    pub fn with_cleanup_interval(mut self, interval: u64) -> Self {
        self.cleanup_interval = interval;
        self
    }
    
    /// Set temporary file max age
    pub fn with_temp_file_max_age(mut self, max_age: u64) -> Self {
        self.temp_file_max_age = max_age;
        self
    }
}

/// Local storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    /// Root directory for file storage
    pub root_path: PathBuf,
    
    /// Create directories if they don't exist
    pub create_directories: bool,
    
    /// File permissions (Unix only)
    #[cfg(unix)]
    pub file_permissions: Option<u32>,
    
    /// Directory permissions (Unix only)
    #[cfg(unix)]
    pub directory_permissions: Option<u32>,
}

impl Default for LocalStorageConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("./storage"),
            create_directories: true,
            #[cfg(unix)]
            file_permissions: Some(0o644),
            #[cfg(unix)]
            directory_permissions: Some(0o755),
        }
    }
}

impl LocalStorageConfig {
    /// Create new local storage configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set root path
    pub fn with_root_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.root_path = path.into();
        self
    }
    
    /// Enable automatic directory creation
    pub fn create_directories(mut self) -> Self {
        self.create_directories = true;
        self
    }
    
    /// Disable automatic directory creation
    pub fn no_create_directories(mut self) -> Self {
        self.create_directories = false;
        self
    }
    
    /// Set file permissions (Unix only)
    #[cfg(unix)]
    pub fn with_file_permissions(mut self, permissions: u32) -> Self {
        self.file_permissions = Some(permissions);
        self
    }
    
    /// Set directory permissions (Unix only)
    #[cfg(unix)]
    pub fn with_directory_permissions(mut self, permissions: u32) -> Self {
        self.directory_permissions = Some(permissions);
        self
    }
}

/// AWS S3 configuration
#[cfg(feature = "aws-s3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    
    /// AWS region
    pub region: String,
    
    /// Custom endpoint (for S3-compatible services)
    pub endpoint: Option<String>,
    
    /// Access key ID (if not using IAM roles)
    pub access_key_id: Option<String>,
    
    /// Secret access key (if not using IAM roles)
    pub secret_access_key: Option<String>,
    
    /// Session token (for temporary credentials)
    pub session_token: Option<String>,
    
    /// Path prefix for all files
    pub prefix: Option<String>,
    
    /// Default ACL for uploaded files
    pub default_acl: Option<String>,
    
    /// Enable server-side encryption
    pub server_side_encryption: bool,
    
    /// KMS key ID for encryption
    pub kms_key_id: Option<String>,
    
    /// Use path-style URLs
    pub path_style: bool,
    
    /// CDN domain for public URLs
    pub cdn_domain: Option<String>,
}

#[cfg(feature = "aws-s3")]
impl S3Config {
    /// Create new S3 configuration
    pub fn new(bucket: String) -> Self {
        Self {
            bucket,
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            prefix: None,
            default_acl: None,
            server_side_encryption: false,
            kms_key_id: None,
            path_style: false,
            cdn_domain: None,
        }
    }
    
    /// Set AWS region
    pub fn with_region(mut self, region: String) -> Self {
        self.region = region;
        self
    }
    
    /// Set custom endpoint (for S3-compatible services)
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }
    
    /// Set AWS credentials
    pub fn with_credentials(mut self, access_key_id: String, secret_access_key: String) -> Self {
        self.access_key_id = Some(access_key_id);
        self.secret_access_key = Some(secret_access_key);
        self
    }
    
    /// Set session token for temporary credentials
    pub fn with_session_token(mut self, session_token: String) -> Self {
        self.session_token = Some(session_token);
        self
    }
    
    /// Set path prefix
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }
    
    /// Set default ACL
    pub fn with_acl(mut self, acl: String) -> Self {
        self.default_acl = Some(acl);
        self
    }
    
    /// Enable server-side encryption
    pub fn with_encryption(mut self) -> Self {
        self.server_side_encryption = true;
        self
    }
    
    /// Set KMS key for encryption
    pub fn with_kms_key(mut self, key_id: String) -> Self {
        self.kms_key_id = Some(key_id);
        self.server_side_encryption = true;
        self
    }
    
    /// Use path-style URLs
    pub fn path_style(mut self) -> Self {
        self.path_style = true;
        self
    }
    
    /// Set CDN domain
    pub fn with_cdn(mut self, domain: String) -> Self {
        self.cdn_domain = Some(domain);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert_eq!(config.max_file_size, Some(100 * 1024 * 1024));
        assert_eq!(config.allowed_types, None);
        assert!(config.validate_files);
        assert_eq!(config.cleanup_interval, 3600);
        assert_eq!(config.temp_file_max_age, 86400);
    }
    
    #[test]
    fn test_storage_config_builder() {
        let config = StorageConfig::new()
            .with_max_file_size(50 * 1024 * 1024)
            .allow_type("image/".to_string())
            .allow_type("text/plain".to_string())
            .with_cache_control("no-cache".to_string())
            .without_validation();
            
        assert_eq!(config.max_file_size, Some(50 * 1024 * 1024));
        assert_eq!(config.allowed_types, Some(vec!["image/".to_string(), "text/plain".to_string()]));
        assert_eq!(config.default_cache_control, Some("no-cache".to_string()));
        assert!(!config.validate_files);
    }
    
    #[test]
    fn test_local_config_defaults() {
        let config = LocalStorageConfig::default();
        assert_eq!(config.root_path, PathBuf::from("./storage"));
        assert!(config.create_directories);
        
        #[cfg(unix)]
        {
            assert_eq!(config.file_permissions, Some(0o644));
            assert_eq!(config.directory_permissions, Some(0o755));
        }
    }
    
    #[test]
    fn test_local_config_builder() {
        let config = LocalStorageConfig::new()
            .with_root_path("/tmp/storage")
            .no_create_directories();
            
        assert_eq!(config.root_path, PathBuf::from("/tmp/storage"));
        assert!(!config.create_directories);
    }
    
    #[cfg(feature = "aws-s3")]
    #[test]
    fn test_s3_config() {
        let config = S3Config::new("my-bucket".to_string())
            .with_region("us-west-2".to_string())
            .with_prefix("uploads/".to_string())
            .with_encryption()
            .with_cdn("cdn.example.com".to_string());
            
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.prefix, Some("uploads/".to_string()));
        assert!(config.server_side_encryption);
        assert_eq!(config.cdn_domain, Some("cdn.example.com".to_string()));
    }
}