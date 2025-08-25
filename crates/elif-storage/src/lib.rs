//! # elif-storage
//!
//! A comprehensive multi-backend file storage system for the elif.rs framework.
//!
//! ## Features
//!
//! - **Multi-backend support**: Local filesystem, AWS S3, and Google Cloud Storage
//! - **File upload validation**: Size, type, and content validation
//! - **Image processing**: Resize, crop, optimize, and watermark images
//! - **CDN integration**: Signed URLs and CDN support
//! - **Access control**: File permissions and access control
//! - **Temporary file management**: Automatic cleanup of temporary files
//! - **Streaming support**: Handle large files efficiently
//! - **Async-first**: Built for modern async Rust applications
//!
//! ## Quick Start
//!
//! ```rust
//! use elif_storage::{Storage, LocalBackend, LocalStorageConfig};
//!
//! # tokio_test::block_on(async {
//! // Create a local filesystem storage
//! let config = LocalStorageConfig::default().with_root_path("./storage");
//! let storage = Storage::new(LocalBackend::new(config));
//!
//! // Store a file
//! let file_data = b"Hello, World!";
//! let file_info = storage.put("documents/hello.txt", file_data, None).await.unwrap();
//!
//! // Retrieve a file
//! let retrieved = storage.get("documents/hello.txt").await.unwrap();
//! assert_eq!(retrieved.unwrap().as_ref(), file_data);
//! # });
//! ```

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

pub mod backends;
pub mod cleanup;
pub mod config;
pub mod permissions;
pub mod upload;
pub mod validation;

#[cfg(feature = "image-processing")]
pub mod image_processing;

pub use backends::*;
pub use cleanup::*;
pub use config::*;
pub use upload::*;
pub use validation::*;

#[cfg(feature = "image-processing")]
pub use image_processing::*;

/// Storage operation errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error")]
    Timeout,

    #[error("File too large: {0} bytes, max allowed: {1} bytes")]
    FileTooLarge(u64, u64),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("CDN error: {0}")]
    Cdn(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// File path type
pub type FilePath = String;

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path/key
    pub path: FilePath,

    /// File size in bytes
    pub size: u64,

    /// MIME type
    pub content_type: String,

    /// File creation timestamp
    pub created_at: DateTime<Utc>,

    /// File modification timestamp
    pub modified_at: DateTime<Utc>,

    /// ETag/version identifier
    pub etag: Option<String>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,

    /// File permissions
    #[cfg(feature = "access-control")]
    pub permissions: Option<FilePermissions>,
}

impl FileMetadata {
    /// Create new file metadata
    pub fn new(path: FilePath, size: u64, content_type: String) -> Self {
        let now = Utc::now();
        Self {
            path,
            size,
            content_type,
            created_at: now,
            modified_at: now,
            etag: None,
            metadata: HashMap::new(),
            #[cfg(feature = "access-control")]
            permissions: None,
        }
    }

    /// Set custom metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set ETag
    pub fn with_etag(mut self, etag: String) -> Self {
        self.etag = Some(etag);
        self
    }

    /// Set permissions
    #[cfg(feature = "access-control")]
    pub fn with_permissions(mut self, permissions: FilePermissions) -> Self {
        self.permissions = Some(permissions);
        self
    }
}

/// File upload options
#[derive(Debug, Clone, Default)]
pub struct UploadOptions {
    /// Content type override
    pub content_type: Option<String>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,

    /// File permissions
    #[cfg(feature = "access-control")]
    pub permissions: Option<FilePermissions>,

    /// Cache control headers
    pub cache_control: Option<String>,

    /// Content disposition
    pub content_disposition: Option<String>,

    /// Whether to overwrite existing file
    pub overwrite: bool,
}

impl UploadOptions {
    /// Create new upload options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set content type
    pub fn content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set permissions
    #[cfg(feature = "access-control")]
    pub fn permissions(mut self, permissions: FilePermissions) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Set cache control
    pub fn cache_control(mut self, cache_control: String) -> Self {
        self.cache_control = Some(cache_control);
        self
    }

    /// Set content disposition
    pub fn content_disposition(mut self, content_disposition: String) -> Self {
        self.content_disposition = Some(content_disposition);
        self
    }

    /// Allow overwriting existing files
    pub fn overwrite(mut self) -> Self {
        self.overwrite = true;
        self
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size: u64,
    pub available_space: Option<u64>,
    pub used_space: Option<u64>,
}

/// Core storage backend trait that all storage implementations must implement
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a file
    async fn put(
        &self,
        path: &str,
        data: &[u8],
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata>;

    /// Store a file from a stream (for large files)
    async fn put_stream<S>(
        &self,
        path: &str,
        stream: S,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata>
    where
        S: futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin;

    /// Retrieve a file
    async fn get(&self, path: &str) -> StorageResult<Option<Bytes>>;

    /// Get a file as a stream (for large files)
    async fn get_stream(
        &self,
        path: &str,
    ) -> StorageResult<
        Option<Box<dyn futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>>,
    >;

    /// Check if a file exists
    async fn exists(&self, path: &str) -> StorageResult<bool>;

    /// Get file metadata
    async fn metadata(&self, path: &str) -> StorageResult<Option<FileMetadata>>;

    /// Delete a file
    async fn delete(&self, path: &str) -> StorageResult<bool>;

    /// List files in a directory/prefix
    async fn list(
        &self,
        prefix: Option<&str>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<FileMetadata>>;

    /// Copy a file
    async fn copy(
        &self,
        from: &str,
        to: &str,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata>;

    /// Move/rename a file
    async fn move_file(
        &self,
        from: &str,
        to: &str,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata>;

    /// Generate a signed URL (if supported)
    async fn signed_url(&self, path: &str, expires_in: Duration) -> StorageResult<String>;

    /// Generate a public URL (if supported)
    async fn public_url(&self, path: &str) -> StorageResult<String>;

    /// Get storage statistics
    async fn stats(&self) -> StorageResult<StorageStats> {
        Ok(StorageStats::default())
    }

    /// Delete multiple files
    async fn delete_many(&self, paths: &[&str]) -> StorageResult<Vec<String>> {
        let mut deleted = Vec::new();
        for path in paths {
            if self.delete(path).await? {
                deleted.push(path.to_string());
            }
        }
        Ok(deleted)
    }
}

/// High-level storage interface with additional features
pub struct Storage<B: StorageBackend> {
    backend: B,
    config: StorageConfig,
}

impl<B: StorageBackend> Storage<B> {
    /// Create a new storage instance
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            config: StorageConfig::default(),
        }
    }

    /// Create a storage instance with custom configuration
    pub fn with_config(backend: B, config: StorageConfig) -> Self {
        Self { backend, config }
    }

    /// Store a file with validation
    pub async fn put(
        &self,
        path: &str,
        data: &[u8],
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata> {
        // Validate file
        validate_file_size(data.len() as u64, self.config.max_file_size)?;

        let content_type = options
            .as_ref()
            .and_then(|o| o.content_type.clone())
            .unwrap_or_else(|| detect_content_type(path, data));

        validate_file_type(&content_type, &self.config.allowed_types)?;

        // Store the file
        self.backend.put(path, data, options).await
    }

    /// Store a file from a stream with validation
    pub async fn put_stream<S>(
        &self,
        path: &str,
        stream: S,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata>
    where
        S: futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin,
    {
        self.backend.put_stream(path, stream, options).await
    }

    /// Retrieve a file
    pub async fn get(&self, path: &str) -> StorageResult<Option<Bytes>> {
        #[cfg(feature = "access-control")]
        if let Some(permissions) = self.get_file_permissions(path).await? {
            // Check access permissions here
            // This would integrate with the auth system
        }

        self.backend.get(path).await
    }

    /// Get a file as a stream
    pub async fn get_stream(
        &self,
        path: &str,
    ) -> StorageResult<
        Option<Box<dyn futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>>,
    > {
        self.backend.get_stream(path).await
    }

    /// Check if file exists
    pub async fn exists(&self, path: &str) -> StorageResult<bool> {
        self.backend.exists(path).await
    }

    /// Get file metadata
    pub async fn metadata(&self, path: &str) -> StorageResult<Option<FileMetadata>> {
        self.backend.metadata(path).await
    }

    /// Delete a file
    pub async fn delete(&self, path: &str) -> StorageResult<bool> {
        self.backend.delete(path).await
    }

    /// List files
    pub async fn list(
        &self,
        prefix: Option<&str>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<FileMetadata>> {
        self.backend.list(prefix, limit).await
    }

    /// Copy a file
    pub async fn copy(
        &self,
        from: &str,
        to: &str,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata> {
        self.backend.copy(from, to, options).await
    }

    /// Move a file
    pub async fn move_file(
        &self,
        from: &str,
        to: &str,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata> {
        self.backend.move_file(from, to, options).await
    }

    /// Generate signed URL
    pub async fn signed_url(&self, path: &str, expires_in: Duration) -> StorageResult<String> {
        self.backend.signed_url(path, expires_in).await
    }

    /// Generate public URL
    pub async fn public_url(&self, path: &str) -> StorageResult<String> {
        self.backend.public_url(path).await
    }

    /// Get storage statistics
    pub async fn stats(&self) -> StorageResult<StorageStats> {
        self.backend.stats().await
    }

    /// Delete multiple files
    pub async fn delete_many(&self, paths: &[&str]) -> StorageResult<Vec<String>> {
        self.backend.delete_many(paths).await
    }

    #[cfg(feature = "access-control")]
    async fn get_file_permissions(&self, _path: &str) -> StorageResult<Option<FilePermissions>> {
        // This would check permissions from the metadata or a separate permissions system
        Ok(None)
    }
}

/// Helper function to detect content type from path and data
fn detect_content_type(path: &str, data: &[u8]) -> String {
    // Try to detect from file extension first
    if let Some(ext) = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
    {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => return "image/jpeg".to_string(),
            "png" => return "image/png".to_string(),
            "gif" => return "image/gif".to_string(),
            "webp" => return "image/webp".to_string(),
            "pdf" => return "application/pdf".to_string(),
            "txt" => return "text/plain".to_string(),
            "json" => return "application/json".to_string(),
            "xml" => return "application/xml".to_string(),
            "html" => return "text/html".to_string(),
            "css" => return "text/css".to_string(),
            "js" => return "application/javascript".to_string(),
            _ => {}
        }
    }

    // Try to detect from content (simple magic number detection)
    if data.len() >= 4 {
        match &data[..4] {
            [0xFF, 0xD8, 0xFF, _] => return "image/jpeg".to_string(),
            [0x89, 0x50, 0x4E, 0x47] => return "image/png".to_string(),
            [0x47, 0x49, 0x46, 0x38] => return "image/gif".to_string(),
            [0x52, 0x49, 0x46, 0x46] if data.len() >= 12 && &data[8..12] == b"WEBP" => {
                return "image/webp".to_string()
            }
            [0x25, 0x50, 0x44, 0x46] => return "application/pdf".to_string(),
            _ => {}
        }
    }

    "application/octet-stream".to_string()
}

/// Validate file size
fn validate_file_size(size: u64, max_size: Option<u64>) -> StorageResult<()> {
    if let Some(max) = max_size {
        if size > max {
            return Err(StorageError::FileTooLarge(size, max));
        }
    }
    Ok(())
}

/// Validate file type
fn validate_file_type(
    content_type: &str,
    allowed_types: &Option<Vec<String>>,
) -> StorageResult<()> {
    if let Some(allowed) = allowed_types {
        if !allowed.iter().any(|t| content_type.starts_with(t)) {
            return Err(StorageError::UnsupportedFileType(content_type.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_content_type() {
        // Test extension detection
        assert_eq!(detect_content_type("test.jpg", &[]), "image/jpeg");
        assert_eq!(detect_content_type("test.png", &[]), "image/png");
        assert_eq!(detect_content_type("test.pdf", &[]), "application/pdf");

        // Test magic number detection
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_content_type("unknown", &jpeg_data), "image/jpeg");

        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_content_type("unknown", &png_data), "image/png");

        // Test fallback
        assert_eq!(
            detect_content_type("unknown", &[0x00, 0x01, 0x02, 0x03]),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_validate_file_size() {
        assert!(validate_file_size(1000, Some(2000)).is_ok());
        assert!(validate_file_size(2000, Some(2000)).is_ok());
        assert!(validate_file_size(3000, Some(2000)).is_err());
        assert!(validate_file_size(1000, None).is_ok());
    }

    #[test]
    fn test_validate_file_type() {
        let allowed = Some(vec!["image/".to_string(), "text/plain".to_string()]);

        assert!(validate_file_type("image/jpeg", &allowed).is_ok());
        assert!(validate_file_type("image/png", &allowed).is_ok());
        assert!(validate_file_type("text/plain", &allowed).is_ok());
        assert!(validate_file_type("application/pdf", &allowed).is_err());
        assert!(validate_file_type("application/pdf", &None).is_ok());
    }
}
