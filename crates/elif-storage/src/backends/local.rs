//! Local filesystem storage backend

use crate::config::LocalStorageConfig;
use crate::{
    FileMetadata, StorageBackend, StorageError, StorageResult, StorageStats, UploadOptions,
};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Local filesystem storage backend
#[derive(Debug, Clone)]
pub struct LocalBackend {
    config: LocalStorageConfig,
}

impl LocalBackend {
    /// Create a new local storage backend
    pub fn new(config: LocalStorageConfig) -> Self {
        Self { config }
    }

    /// Get the full filesystem path for a storage path
    fn full_path(&self, path: &str) -> PathBuf {
        // Sanitize the path to prevent directory traversal
        let sanitized = self.sanitize_path(path);
        self.config.root_path.join(sanitized)
    }

    /// Sanitize a path to prevent directory traversal attacks
    fn sanitize_path(&self, path: &str) -> PathBuf {
        let path = path.trim_start_matches('/');
        let components: Vec<&str> = path
            .split('/')
            .filter(|component| !component.is_empty() && *component != "." && *component != "..")
            .collect();

        components.iter().collect()
    }

    /// Ensure the parent directory exists
    async fn ensure_parent_dir(&self, file_path: &Path) -> StorageResult<()> {
        if !self.config.create_directories {
            return Ok(());
        }

        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    StorageError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("Failed to create directory {}: {}", parent.display(), e),
                    ))
                })?;

                #[cfg(unix)]
                if let Some(permissions) = self.config.directory_permissions {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(permissions);
                    std::fs::set_permissions(parent, perms).map_err(|e| {
                        StorageError::Io(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            format!("Failed to set directory permissions: {}", e),
                        ))
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Set file permissions after creation
    #[cfg(unix)]
    async fn set_file_permissions(&self, file_path: &Path) -> StorageResult<()> {
        if let Some(permissions) = self.config.file_permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions);
            fs::set_permissions(file_path, perms).await.map_err(|e| {
                StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("Failed to set file permissions: {}", e),
                ))
            })?;
        }
        Ok(())
    }

    #[cfg(not(unix))]
    async fn set_file_permissions(&self, _file_path: &Path) -> StorageResult<()> {
        Ok(())
    }

    /// Generate ETag for file (using file size and modification time)
    async fn generate_etag(&self, file_path: &Path) -> StorageResult<String> {
        let metadata = fs::metadata(file_path).await?;
        let size = metadata.len();
        let modified = metadata
            .modified()
            .map_err(StorageError::Io)?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| StorageError::Backend(format!("Time error: {}", e)))?
            .as_secs();

        Ok(format!("{}-{}", size, modified))
    }
}

#[async_trait]
impl StorageBackend for LocalBackend {
    async fn put(
        &self,
        path: &str,
        data: &[u8],
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata> {
        let file_path = self.full_path(path);

        // Check if file exists and overwrite is not allowed
        if let Some(opts) = &options {
            if !opts.overwrite && file_path.exists() {
                return Err(StorageError::Backend(format!(
                    "File already exists: {}",
                    path
                )));
            }
        }

        // Ensure parent directory exists
        self.ensure_parent_dir(&file_path).await?;

        // Write the file
        fs::write(&file_path, data).await?;

        // Set file permissions
        self.set_file_permissions(&file_path).await?;

        // Generate metadata
        let content_type = options
            .as_ref()
            .and_then(|o| o.content_type.clone())
            .unwrap_or_else(|| crate::detect_content_type(path, data));

        let etag = self.generate_etag(&file_path).await?;
        let now = Utc::now();

        let metadata = FileMetadata {
            path: path.to_string(),
            size: data.len() as u64,
            content_type,
            created_at: now,
            modified_at: now,
            etag: Some(etag),
            metadata: options
                .as_ref()
                .map(|o| o.metadata.clone())
                .unwrap_or_default(),
            #[cfg(feature = "access-control")]
            permissions: options.as_ref().and_then(|o| o.permissions.clone()),
        };

        // Store extended metadata in a sidecar file if we have custom metadata
        if !metadata.metadata.is_empty() {
            let metadata_path = format!("{}.metadata", file_path.display());
            let metadata_json = serde_json::to_string(&metadata.metadata).map_err(|e| {
                StorageError::Backend(format!("Failed to serialize metadata: {}", e))
            })?;
            fs::write(&metadata_path, metadata_json).await?;
        }

        Ok(metadata)
    }

    async fn put_stream<S>(
        &self,
        path: &str,
        mut stream: S,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin,
    {
        let file_path = self.full_path(path);

        // Check if file exists and overwrite is not allowed
        if let Some(opts) = &options {
            if !opts.overwrite && file_path.exists() {
                return Err(StorageError::Backend(format!(
                    "File already exists: {}",
                    path
                )));
            }
        }

        // Ensure parent directory exists
        self.ensure_parent_dir(&file_path).await?;

        // Create file and write stream to it
        let mut file = fs::File::create(&file_path).await?;
        let mut total_size = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            total_size += chunk.len() as u64;
        }

        file.flush().await?;
        drop(file);

        // Set file permissions
        self.set_file_permissions(&file_path).await?;

        // Generate metadata
        let content_type = options
            .as_ref()
            .and_then(|o| o.content_type.clone())
            .unwrap_or_else(|| {
                // For streams, we can't analyze content, so detect from extension
                crate::detect_content_type(path, &[])
            });

        let etag = self.generate_etag(&file_path).await?;
        let now = Utc::now();

        let metadata = FileMetadata {
            path: path.to_string(),
            size: total_size,
            content_type,
            created_at: now,
            modified_at: now,
            etag: Some(etag),
            metadata: options
                .as_ref()
                .map(|o| o.metadata.clone())
                .unwrap_or_default(),
            #[cfg(feature = "access-control")]
            permissions: options.as_ref().and_then(|o| o.permissions.clone()),
        };

        // Store extended metadata if needed
        if !metadata.metadata.is_empty() {
            let metadata_path = format!("{}.metadata", file_path.display());
            let metadata_json = serde_json::to_string(&metadata.metadata).map_err(|e| {
                StorageError::Backend(format!("Failed to serialize metadata: {}", e))
            })?;
            fs::write(&metadata_path, metadata_json).await?;
        }

        Ok(metadata)
    }

    async fn get(&self, path: &str) -> StorageResult<Option<Bytes>> {
        let file_path = self.full_path(path);

        if !file_path.exists() {
            return Ok(None);
        }

        let data = fs::read(&file_path).await?;
        Ok(Some(Bytes::from(data)))
    }

    async fn get_stream(
        &self,
        path: &str,
    ) -> StorageResult<Option<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>>>
    {
        let file_path = self.full_path(path);

        if !file_path.exists() {
            return Ok(None);
        }

        let file = fs::File::open(&file_path).await?;
        let stream = tokio_util::io::ReaderStream::new(file);
        let byte_stream = stream.map(|result| result);

        Ok(Some(Box::new(byte_stream)))
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let file_path = self.full_path(path);
        Ok(file_path.exists())
    }

    async fn metadata(&self, path: &str) -> StorageResult<Option<FileMetadata>> {
        let file_path = self.full_path(path);

        if !file_path.exists() {
            return Ok(None);
        }

        let fs_metadata = fs::metadata(&file_path).await?;
        let size = fs_metadata.len();

        let created_at = fs_metadata.created().map_err(StorageError::Io)?.into();

        let modified_at = fs_metadata.modified().map_err(StorageError::Io)?.into();

        let etag = self.generate_etag(&file_path).await?;

        // Try to load extended metadata
        let metadata_path = format!("{}.metadata", file_path.display());
        let custom_metadata = if Path::new(&metadata_path).exists() {
            let metadata_json = fs::read_to_string(&metadata_path).await?;
            serde_json::from_str::<HashMap<String, String>>(&metadata_json).unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Detect content type
        let content_type = if size <= 1024 * 1024 {
            // For small files, read a bit to detect content type
            let sample = fs::read(&file_path).await.unwrap_or_default();
            crate::detect_content_type(path, &sample)
        } else {
            // For large files, just use extension
            crate::detect_content_type(path, &[])
        };

        let file_metadata = FileMetadata {
            path: path.to_string(),
            size,
            content_type,
            created_at,
            modified_at,
            etag: Some(etag),
            metadata: custom_metadata,
            #[cfg(feature = "access-control")]
            permissions: None, // Would load from metadata if implemented
        };

        Ok(Some(file_metadata))
    }

    async fn delete(&self, path: &str) -> StorageResult<bool> {
        let file_path = self.full_path(path);

        if !file_path.exists() {
            return Ok(false);
        }

        fs::remove_file(&file_path).await?;

        // Also remove metadata file if it exists
        let metadata_path = format!("{}.metadata", file_path.display());
        if Path::new(&metadata_path).exists() {
            let _ = fs::remove_file(&metadata_path).await;
        }

        Ok(true)
    }

    async fn list(
        &self,
        prefix: Option<&str>,
        limit: Option<u32>,
    ) -> StorageResult<Vec<FileMetadata>> {
        let search_path = if let Some(prefix) = prefix {
            self.full_path(prefix)
        } else {
            self.config.root_path.clone()
        };

        let mut files = Vec::new();
        let limit = limit.unwrap_or(1000) as usize;

        fn collect_files<'a>(
            dir: &'a Path,
            root: &'a Path,
            files: &'a mut Vec<FileMetadata>,
            limit: usize,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = StorageResult<()>> + Send + 'a>>
        {
            Box::pin(async move {
                if files.len() >= limit {
                    return Ok(());
                }

                let mut entries = fs::read_dir(dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    if files.len() >= limit {
                        break;
                    }

                    let path = entry.path();

                    if path.is_file() {
                        // Skip metadata files
                        if path.extension().and_then(|e| e.to_str()) == Some("metadata") {
                            continue;
                        }

                        let relative_path = path
                            .strip_prefix(root)
                            .map_err(|e| StorageError::Backend(format!("Path error: {}", e)))?;

                        let relative_str = relative_path.to_string_lossy().replace('\\', "/");

                        // Get file metadata
                        if let Ok(Some(metadata)) =
                            LocalBackend::metadata_for_path(&path, &relative_str).await
                        {
                            files.push(metadata);
                        }
                    } else if path.is_dir() {
                        collect_files(&path, root, files, limit).await?;
                    }
                }

                Ok(())
            })
        }

        collect_files(&search_path, &self.config.root_path, &mut files, limit).await?;

        Ok(files)
    }

    async fn copy(
        &self,
        from: &str,
        to: &str,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata> {
        let from_path = self.full_path(from);
        let to_path = self.full_path(to);

        if !from_path.exists() {
            return Err(StorageError::FileNotFound(from.to_string()));
        }

        // Check if destination exists and overwrite is not allowed
        if let Some(opts) = &options {
            if !opts.overwrite && to_path.exists() {
                return Err(StorageError::Backend(format!(
                    "File already exists: {}",
                    to
                )));
            }
        }

        // Ensure parent directory exists
        self.ensure_parent_dir(&to_path).await?;

        // Copy the file
        fs::copy(&from_path, &to_path).await?;

        // Set file permissions
        self.set_file_permissions(&to_path).await?;

        // Copy metadata file if it exists
        let from_metadata_path = format!("{}.metadata", from_path.display());
        let to_metadata_path = format!("{}.metadata", to_path.display());

        if Path::new(&from_metadata_path).exists() {
            let _ = fs::copy(&from_metadata_path, &to_metadata_path).await;
        }

        // Generate new metadata for the copied file
        self.metadata(to).await?.ok_or_else(|| {
            StorageError::Backend("Failed to get metadata for copied file".to_string())
        })
    }

    async fn move_file(
        &self,
        from: &str,
        to: &str,
        options: Option<UploadOptions>,
    ) -> StorageResult<FileMetadata> {
        let from_path = self.full_path(from);
        let to_path = self.full_path(to);

        if !from_path.exists() {
            return Err(StorageError::FileNotFound(from.to_string()));
        }

        // Check if destination exists and overwrite is not allowed
        if let Some(opts) = &options {
            if !opts.overwrite && to_path.exists() {
                return Err(StorageError::Backend(format!(
                    "File already exists: {}",
                    to
                )));
            }
        }

        // Ensure parent directory exists
        self.ensure_parent_dir(&to_path).await?;

        // Move the file
        fs::rename(&from_path, &to_path).await?;

        // Move metadata file if it exists
        let from_metadata_path = format!("{}.metadata", from_path.display());
        let to_metadata_path = format!("{}.metadata", to_path.display());

        if Path::new(&from_metadata_path).exists() {
            let _ = fs::rename(&from_metadata_path, &to_metadata_path).await;
        }

        // Generate new metadata for the moved file
        self.metadata(to).await?.ok_or_else(|| {
            StorageError::Backend("Failed to get metadata for moved file".to_string())
        })
    }

    async fn signed_url(&self, path: &str, _expires_in: Duration) -> StorageResult<String> {
        // Local storage doesn't support signed URLs, return file:// URL
        let file_path = self.full_path(path);
        if !file_path.exists() {
            return Err(StorageError::FileNotFound(path.to_string()));
        }

        Ok(format!("file://{}", file_path.display()))
    }

    async fn public_url(&self, path: &str) -> StorageResult<String> {
        // Local storage doesn't have public URLs, return file:// URL
        let file_path = self.full_path(path);
        if !file_path.exists() {
            return Err(StorageError::FileNotFound(path.to_string()));
        }

        Ok(format!("file://{}", file_path.display()))
    }

    async fn stats(&self) -> StorageResult<StorageStats> {
        let _total_files = 0u64;
        let _total_size = 0u64;

        fn collect_stats<'a>(
            dir: &'a Path,
            stats: &'a mut (u64, u64),
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = StorageResult<()>> + Send + 'a>>
        {
            Box::pin(async move {
                let mut entries = fs::read_dir(dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();

                    if path.is_file() {
                        // Skip metadata files
                        if path.extension().and_then(|e| e.to_str()) == Some("metadata") {
                            continue;
                        }

                        let metadata = fs::metadata(&path).await?;
                        stats.0 += 1;
                        stats.1 += metadata.len();
                    } else if path.is_dir() {
                        collect_stats(&path, stats).await?;
                    }
                }

                Ok(())
            })
        }

        let mut stats = (0u64, 0u64);
        collect_stats(&self.config.root_path, &mut stats).await?;

        // Try to get filesystem info
        let (available_space, used_space) =
            if let Ok(_metadata) = fs::metadata(&self.config.root_path).await {
                // This is platform-specific and simplified
                (None, None)
            } else {
                (None, None)
            };

        Ok(StorageStats {
            total_files: stats.0,
            total_size: stats.1,
            available_space,
            used_space,
        })
    }
}

impl LocalBackend {
    /// Helper method to generate metadata for a file path
    async fn metadata_for_path(
        file_path: &Path,
        relative_path: &str,
    ) -> StorageResult<Option<FileMetadata>> {
        if !file_path.exists() {
            return Ok(None);
        }

        let fs_metadata = fs::metadata(file_path).await?;
        let size = fs_metadata.len();

        let created_at = fs_metadata.created().map_err(StorageError::Io)?.into();

        let modified_at = fs_metadata.modified().map_err(StorageError::Io)?.into();

        // Generate ETag
        let modified_timestamp = fs_metadata
            .modified()
            .map_err(StorageError::Io)?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| StorageError::Backend(format!("Time error: {}", e)))?
            .as_secs();
        let etag = format!("{}-{}", size, modified_timestamp);

        // Try to load extended metadata
        let metadata_path = format!("{}.metadata", file_path.display());
        let custom_metadata = if Path::new(&metadata_path).exists() {
            let metadata_json = fs::read_to_string(&metadata_path).await?;
            serde_json::from_str::<HashMap<String, String>>(&metadata_json).unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Detect content type
        let content_type = crate::detect_content_type(relative_path, &[]);

        let file_metadata = FileMetadata {
            path: relative_path.to_string(),
            size,
            content_type,
            created_at,
            modified_at,
            etag: Some(etag),
            metadata: custom_metadata,
            #[cfg(feature = "access-control")]
            permissions: None,
        };

        Ok(Some(file_metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_backend() -> (LocalBackend, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let config = LocalStorageConfig::new().with_root_path(temp_dir.path().to_path_buf());
        let backend = LocalBackend::new(config);
        (backend, temp_dir)
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let (backend, _temp_dir) = create_test_backend().await;

        let data = b"Hello, World!";
        let metadata = backend.put("test.txt", data, None).await.unwrap();

        assert_eq!(metadata.path, "test.txt");
        assert_eq!(metadata.size, data.len() as u64);
        assert_eq!(metadata.content_type, "text/plain");

        let retrieved = backend.get("test.txt").await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data);
    }

    #[tokio::test]
    async fn test_exists_and_delete() {
        let (backend, _temp_dir) = create_test_backend().await;

        let data = b"Test data";
        backend.put("test.txt", data, None).await.unwrap();

        assert!(backend.exists("test.txt").await.unwrap());
        assert!(!backend.exists("nonexistent.txt").await.unwrap());

        assert!(backend.delete("test.txt").await.unwrap());
        assert!(!backend.exists("test.txt").await.unwrap());
        assert!(!backend.delete("nonexistent.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_metadata() {
        let (backend, _temp_dir) = create_test_backend().await;

        let data = b"Test metadata";
        let options = UploadOptions::new()
            .content_type("text/custom".to_string())
            .metadata("key1".to_string(), "value1".to_string())
            .metadata("key2".to_string(), "value2".to_string());

        backend.put("test.txt", data, Some(options)).await.unwrap();

        let metadata = backend.metadata("test.txt").await.unwrap().unwrap();
        assert_eq!(metadata.path, "test.txt");
        assert_eq!(metadata.size, data.len() as u64);
        assert!(metadata.etag.is_some());
        assert_eq!(metadata.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(metadata.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[tokio::test]
    async fn test_copy_and_move() {
        let (backend, _temp_dir) = create_test_backend().await;

        let data = b"Test copy/move";
        backend.put("original.txt", data, None).await.unwrap();

        // Test copy
        backend
            .copy("original.txt", "copy.txt", None)
            .await
            .unwrap();
        assert!(backend.exists("original.txt").await.unwrap());
        assert!(backend.exists("copy.txt").await.unwrap());

        let copied_data = backend.get("copy.txt").await.unwrap().unwrap();
        assert_eq!(copied_data.as_ref(), data);

        // Test move
        backend
            .move_file("original.txt", "moved.txt", None)
            .await
            .unwrap();
        assert!(!backend.exists("original.txt").await.unwrap());
        assert!(backend.exists("moved.txt").await.unwrap());

        let moved_data = backend.get("moved.txt").await.unwrap().unwrap();
        assert_eq!(moved_data.as_ref(), data);
    }

    #[tokio::test]
    async fn test_list() {
        let (backend, _temp_dir) = create_test_backend().await;

        // Create some test files
        backend
            .put("dir1/file1.txt", b"content1", None)
            .await
            .unwrap();
        backend
            .put("dir1/file2.txt", b"content2", None)
            .await
            .unwrap();
        backend
            .put("dir2/file3.txt", b"content3", None)
            .await
            .unwrap();

        // List all files
        let files = backend.list(None, None).await.unwrap();
        assert_eq!(files.len(), 3);

        // Check file paths
        let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        assert!(paths.contains(&"dir1/file1.txt".to_string()));
        assert!(paths.contains(&"dir1/file2.txt".to_string()));
        assert!(paths.contains(&"dir2/file3.txt".to_string()));
    }

    #[tokio::test]
    async fn test_sanitize_path() {
        let (backend, _temp_dir) = create_test_backend().await;

        // Test directory traversal prevention
        assert_eq!(
            backend.sanitize_path("../../../etc/passwd"),
            PathBuf::from("etc/passwd")
        );
        assert_eq!(
            backend.sanitize_path("./test/../file.txt"),
            PathBuf::from("test/file.txt")
        );
        assert_eq!(
            backend.sanitize_path("normal/path/file.txt"),
            PathBuf::from("normal/path/file.txt")
        );
        assert_eq!(
            backend.sanitize_path("/absolute/path"),
            PathBuf::from("absolute/path")
        );
    }
}
