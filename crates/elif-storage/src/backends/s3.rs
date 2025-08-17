//! AWS S3 storage backend

#[cfg(feature = "aws-s3")]
use crate::{StorageBackend, StorageResult, StorageError, FileMetadata, UploadOptions, StorageStats};
#[cfg(feature = "aws-s3")]
use crate::config::S3Config;
#[cfg(feature = "aws-s3")]
use async_trait::async_trait;
#[cfg(feature = "aws-s3")]
use bytes::Bytes;
#[cfg(feature = "aws-s3")]
use futures::Stream;
#[cfg(feature = "aws-s3")]
use std::time::Duration;
#[cfg(feature = "aws-s3")]
use aws_sdk_s3::{Client, Config, Region};
#[cfg(feature = "aws-s3")]
use aws_sdk_s3::types::{ObjectCannedAcl, ServerSideEncryption};
#[cfg(feature = "aws-s3")]
use aws_sdk_s3::primitives::ByteStream;
#[cfg(feature = "aws-s3")]
use chrono::Utc;
#[cfg(feature = "aws-s3")]
use std::collections::HashMap;

/// AWS S3 storage backend
#[cfg(feature = "aws-s3")]
#[derive(Debug, Clone)]
pub struct S3Backend {
    client: Client,
    config: S3Config,
}

#[cfg(feature = "aws-s3")]
impl S3Backend {
    /// Create a new S3 storage backend
    pub async fn new(config: S3Config) -> StorageResult<Self> {
        let aws_config = if let (Some(key_id), Some(secret_key)) = (&config.access_key_id, &config.secret_access_key) {
            // Use explicit credentials
            let credentials = aws_sdk_s3::config::Credentials::new(
                key_id,
                secret_key,
                config.session_token.clone(),
                None,
                "elif-storage",
            );
            
            let mut builder = aws_config::from_env()
                .credentials_provider(credentials)
                .region(Region::new(config.region.clone()));
                
            if let Some(endpoint) = &config.endpoint {
                builder = builder.endpoint_url(endpoint);
            }
            
            builder.load().await
        } else {
            // Use default credential chain (IAM roles, environment, etc.)
            let mut builder = aws_config::from_env()
                .region(Region::new(config.region.clone()));
                
            if let Some(endpoint) = &config.endpoint {
                builder = builder.endpoint_url(endpoint);
            }
            
            builder.load().await
        };
        
        let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
            .force_path_style(config.path_style)
            .build();
            
        let client = Client::from_conf(s3_config);
        
        // Test connection by checking if bucket exists
        if let Err(e) = client.head_bucket().bucket(&config.bucket).send().await {
            return Err(StorageError::Configuration(format!(
                "Cannot access S3 bucket '{}': {}", config.bucket, e
            )));
        }
        
        Ok(Self { client, config })
    }
    
    /// Get the full S3 key for a storage path
    fn get_s3_key(&self, path: &str) -> String {
        if let Some(prefix) = &self.config.prefix {
            format!("{}/{}", prefix.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.trim_start_matches('/').to_string()
        }
    }
    
    /// Get the public URL for a file
    fn get_public_url(&self, s3_key: &str) -> String {
        if let Some(cdn_domain) = &self.config.cdn_domain {
            format!("https://{}/{}", cdn_domain, s3_key)
        } else if self.config.path_style {
            format!("https://s3.{}.amazonaws.com/{}/{}", self.config.region, self.config.bucket, s3_key)
        } else {
            format!("https://{}.s3.{}.amazonaws.com/{}", self.config.bucket, self.config.region, s3_key)
        }
    }
    
    /// Convert S3 error to StorageError
    fn convert_s3_error(&self, error: aws_sdk_s3::Error) -> StorageError {
        match error {
            aws_sdk_s3::Error::NoSuchKey(_) => StorageError::FileNotFound("File not found in S3".to_string()),
            aws_sdk_s3::Error::NoSuchBucket(_) => StorageError::Configuration("S3 bucket not found".to_string()),
            aws_sdk_s3::Error::AccessDenied(_) => StorageError::PermissionDenied("Access denied to S3 resource".to_string()),
            _ => StorageError::Backend(format!("S3 error: {}", error)),
        }
    }
}

#[cfg(feature = "aws-s3")]
#[async_trait]
impl StorageBackend for S3Backend {
    async fn put(&self, path: &str, data: &[u8], options: Option<UploadOptions>) -> StorageResult<FileMetadata> {
        let s3_key = self.get_s3_key(path);
        
        let mut put_request = self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .body(ByteStream::from(Bytes::copy_from_slice(data)));
        
        // Set content type
        if let Some(opts) = &options {
            if let Some(content_type) = &opts.content_type {
                put_request = put_request.content_type(content_type);
            }
            
            // Set cache control
            if let Some(cache_control) = &opts.cache_control {
                put_request = put_request.cache_control(cache_control);
            }
            
            // Set content disposition
            if let Some(content_disposition) = &opts.content_disposition {
                put_request = put_request.content_disposition(content_disposition);
            }
            
            // Set metadata
            for (key, value) in &opts.metadata {
                put_request = put_request.metadata(key, value);
            }
        }
        
        // Set ACL
        if let Some(acl) = &self.config.default_acl {
            if let Ok(canned_acl) = acl.parse::<ObjectCannedAcl>() {
                put_request = put_request.acl(canned_acl);
            }
        }
        
        // Set encryption
        if self.config.server_side_encryption {
            put_request = put_request.server_side_encryption(ServerSideEncryption::Aes256);
            
            if let Some(kms_key) = &self.config.kms_key_id {
                put_request = put_request
                    .server_side_encryption(ServerSideEncryption::AwsKms)
                    .ssekms_key_id(kms_key);
            }
        }
        
        let result = put_request.send().await
            .map_err(|e| self.convert_s3_error(e.into()))?;
        
        let now = Utc::now();
        let content_type = options
            .as_ref()
            .and_then(|o| o.content_type.clone())
            .unwrap_or_else(|| crate::detect_content_type(path, data));
        
        let metadata = FileMetadata {
            path: path.to_string(),
            size: data.len() as u64,
            content_type,
            created_at: now,
            modified_at: now,
            etag: result.e_tag().map(|s| s.to_string()),
            metadata: options
                .as_ref()
                .map(|o| o.metadata.clone())
                .unwrap_or_default(),
            #[cfg(feature = "access-control")]
            permissions: options.as_ref().and_then(|o| o.permissions.clone()),
        };
        
        Ok(metadata)
    }
    
    async fn put_stream<S>(&self, path: &str, stream: S, options: Option<UploadOptions>) -> StorageResult<FileMetadata>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin,
    {
        let s3_key = self.get_s3_key(path);
        
        // Convert the stream to ByteStream
        let byte_stream = ByteStream::from_futures_stream(stream);
        
        let mut put_request = self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .body(byte_stream);
        
        // Set options (same as put method)
        if let Some(opts) = &options {
            if let Some(content_type) = &opts.content_type {
                put_request = put_request.content_type(content_type);
            }
            
            if let Some(cache_control) = &opts.cache_control {
                put_request = put_request.cache_control(cache_control);
            }
            
            if let Some(content_disposition) = &opts.content_disposition {
                put_request = put_request.content_disposition(content_disposition);
            }
            
            for (key, value) in &opts.metadata {
                put_request = put_request.metadata(key, value);
            }
        }
        
        if let Some(acl) = &self.config.default_acl {
            if let Ok(canned_acl) = acl.parse::<ObjectCannedAcl>() {
                put_request = put_request.acl(canned_acl);
            }
        }
        
        if self.config.server_side_encryption {
            put_request = put_request.server_side_encryption(ServerSideEncryption::Aes256);
            
            if let Some(kms_key) = &self.config.kms_key_id {
                put_request = put_request
                    .server_side_encryption(ServerSideEncryption::AwsKms)
                    .ssekms_key_id(kms_key);
            }
        }
        
        let result = put_request.send().await
            .map_err(|e| self.convert_s3_error(e.into()))?;
        
        // Get the actual size from the response or default to 0
        let size = result.content_length().unwrap_or(0) as u64;
        
        let now = Utc::now();
        let content_type = options
            .as_ref()
            .and_then(|o| o.content_type.clone())
            .unwrap_or_else(|| crate::detect_content_type(path, &[]));
        
        let metadata = FileMetadata {
            path: path.to_string(),
            size,
            content_type,
            created_at: now,
            modified_at: now,
            etag: result.e_tag().map(|s| s.to_string()),
            metadata: options
                .as_ref()
                .map(|o| o.metadata.clone())
                .unwrap_or_default(),
            #[cfg(feature = "access-control")]
            permissions: options.as_ref().and_then(|o| o.permissions.clone()),
        };
        
        Ok(metadata)
    }
    
    async fn get(&self, path: &str) -> StorageResult<Option<Bytes>> {
        let s3_key = self.get_s3_key(path);
        
        let result = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .send()
            .await;
        
        match result {
            Ok(response) => {
                let bytes = response.body.collect().await
                    .map_err(|e| StorageError::Backend(format!("Failed to read S3 object body: {}", e)))?
                    .into_bytes();
                Ok(Some(bytes))
            }
            Err(e) => {
                match e.into_service_error() {
                    aws_sdk_s3::operation::get_object::GetObjectError::NoSuchKey(_) => Ok(None),
                    other => Err(self.convert_s3_error(other.into())),
                }
            }
        }
    }
    
    async fn get_stream(&self, path: &str) -> StorageResult<Option<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>>> {
        let s3_key = self.get_s3_key(path);
        
        let result = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .send()
            .await;
        
        match result {
            Ok(response) => {
                let stream = response.body.map(|result| {
                    result.map_err(|e| std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("S3 stream error: {}", e)
                    ))
                });
                Ok(Some(Box::new(stream)))
            }
            Err(e) => {
                match e.into_service_error() {
                    aws_sdk_s3::operation::get_object::GetObjectError::NoSuchKey(_) => Ok(None),
                    other => Err(self.convert_s3_error(other.into())),
                }
            }
        }
    }
    
    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let s3_key = self.get_s3_key(path);
        
        let result = self.client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .send()
            .await;
        
        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                match e.into_service_error() {
                    aws_sdk_s3::operation::head_object::HeadObjectError::NotFound(_) => Ok(false),
                    other => Err(self.convert_s3_error(other.into())),
                }
            }
        }
    }
    
    async fn metadata(&self, path: &str) -> StorageResult<Option<FileMetadata>> {
        let s3_key = self.get_s3_key(path);
        
        let result = self.client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .send()
            .await;
        
        match result {
            Ok(response) => {
                let size = response.content_length().unwrap_or(0) as u64;
                let content_type = response.content_type().unwrap_or("application/octet-stream").to_string();
                let last_modified = response.last_modified()
                    .map(|dt| dt.as_secs_f64())
                    .map(|secs| chrono::DateTime::from_timestamp(secs as i64, ((secs.fract() * 1_000_000_000.0) as u32)))
                    .flatten()
                    .unwrap_or_else(|| Utc::now());
                
                // Extract custom metadata
                let custom_metadata = response.metadata()
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                
                let file_metadata = FileMetadata {
                    path: path.to_string(),
                    size,
                    content_type,
                    created_at: last_modified, // S3 doesn't track creation time separately
                    modified_at: last_modified,
                    etag: response.e_tag().map(|s| s.to_string()),
                    metadata: custom_metadata,
                    #[cfg(feature = "access-control")]
                    permissions: None, // Would need to be stored in metadata
                };
                
                Ok(Some(file_metadata))
            }
            Err(e) => {
                match e.into_service_error() {
                    aws_sdk_s3::operation::head_object::HeadObjectError::NotFound(_) => Ok(None),
                    other => Err(self.convert_s3_error(other.into())),
                }
            }
        }
    }
    
    async fn delete(&self, path: &str) -> StorageResult<bool> {
        let s3_key = self.get_s3_key(path);
        
        // Check if object exists first
        let exists = self.exists(path).await?;
        if !exists {
            return Ok(false);
        }
        
        let _result = self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| self.convert_s3_error(e.into()))?;
        
        Ok(true)
    }
    
    async fn list(&self, prefix: Option<&str>, limit: Option<u32>) -> StorageResult<Vec<FileMetadata>> {
        let s3_prefix = if let Some(prefix) = prefix {
            Some(self.get_s3_key(prefix))
        } else {
            self.config.prefix.clone()
        };
        
        let mut list_request = self.client
            .list_objects_v2()
            .bucket(&self.config.bucket);
        
        if let Some(prefix) = &s3_prefix {
            list_request = list_request.prefix(prefix);
        }
        
        if let Some(limit) = limit {
            list_request = list_request.max_keys(limit as i32);
        }
        
        let result = list_request.send().await
            .map_err(|e| self.convert_s3_error(e.into()))?;
        
        let mut files = Vec::new();
        
        if let Some(objects) = result.contents() {
            for object in objects {
                if let (Some(key), Some(size)) = (object.key(), object.size()) {
                    // Convert S3 key back to storage path
                    let storage_path = if let Some(prefix) = &self.config.prefix {
                        key.strip_prefix(&format!("{}/", prefix.trim_end_matches('/')))
                            .unwrap_or(key)
                            .to_string()
                    } else {
                        key.to_string()
                    };
                    
                    let last_modified = object.last_modified()
                        .map(|dt| dt.as_secs_f64())
                        .map(|secs| chrono::DateTime::from_timestamp(secs as i64, ((secs.fract() * 1_000_000_000.0) as u32)))
                        .flatten()
                        .unwrap_or_else(|| Utc::now());
                    
                    let file_metadata = FileMetadata {
                        path: storage_path,
                        size: size as u64,
                        content_type: crate::detect_content_type(key, &[]),
                        created_at: last_modified,
                        modified_at: last_modified,
                        etag: object.e_tag().map(|s| s.to_string()),
                        metadata: HashMap::new(),
                        #[cfg(feature = "access-control")]
                        permissions: None,
                    };
                    
                    files.push(file_metadata);
                }
            }
        }
        
        Ok(files)
    }
    
    async fn copy(&self, from: &str, to: &str, _options: Option<UploadOptions>) -> StorageResult<FileMetadata> {
        let from_key = self.get_s3_key(from);
        let to_key = self.get_s3_key(to);
        
        let copy_source = format!("{}/{}", self.config.bucket, from_key);
        
        let _result = self.client
            .copy_object()
            .copy_source(&copy_source)
            .bucket(&self.config.bucket)
            .key(&to_key)
            .send()
            .await
            .map_err(|e| self.convert_s3_error(e.into()))?;
        
        // Return metadata for the copied file
        self.metadata(to).await?
            .ok_or_else(|| StorageError::Backend("Failed to get metadata for copied file".to_string()))
    }
    
    async fn move_file(&self, from: &str, to: &str, options: Option<UploadOptions>) -> StorageResult<FileMetadata> {
        // S3 doesn't have a native move operation, so we copy then delete
        let metadata = self.copy(from, to, options).await?;
        self.delete(from).await?;
        Ok(metadata)
    }
    
    async fn signed_url(&self, path: &str, expires_in: Duration) -> StorageResult<String> {
        let s3_key = self.get_s3_key(path);
        
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::Backend(format!("Failed to create presigning config: {}", e)))?;
        
        let presigned_request = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to generate presigned URL: {}", e)))?;
        
        Ok(presigned_request.uri().to_string())
    }
    
    async fn public_url(&self, path: &str) -> StorageResult<String> {
        let s3_key = self.get_s3_key(path);
        Ok(self.get_public_url(&s3_key))
    }
    
    async fn stats(&self) -> StorageResult<StorageStats> {
        // For S3, we'd need to list all objects to get accurate stats
        // This is expensive, so we return basic stats
        Ok(StorageStats {
            total_files: 0, // Would require full bucket listing
            total_size: 0,  // Would require full bucket listing
            available_space: None, // S3 doesn't have space limits
            used_space: None,
        })
    }
    
    async fn delete_many(&self, paths: &[&str]) -> StorageResult<Vec<String>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        
        // S3 supports batch delete operations
        let delete_objects: Vec<_> = paths.iter()
            .map(|path| {
                aws_sdk_s3::types::ObjectIdentifier::builder()
                    .key(self.get_s3_key(path))
                    .build()
                    .unwrap()
            })
            .collect();
        
        let delete_request = aws_sdk_s3::types::Delete::builder()
            .set_objects(Some(delete_objects))
            .build()
            .unwrap();
        
        let result = self.client
            .delete_objects()
            .bucket(&self.config.bucket)
            .delete(delete_request)
            .send()
            .await
            .map_err(|e| self.convert_s3_error(e.into()))?;
        
        let mut deleted = Vec::new();
        
        if let Some(deleted_objects) = result.deleted() {
            for obj in deleted_objects {
                if let Some(key) = obj.key() {
                    // Convert S3 key back to storage path
                    let storage_path = if let Some(prefix) = &self.config.prefix {
                        key.strip_prefix(&format!("{}/", prefix.trim_end_matches('/')))
                            .unwrap_or(key)
                            .to_string()
                    } else {
                        key.to_string()
                    };
                    deleted.push(storage_path);
                }
            }
        }
        
        Ok(deleted)
    }
}

// Stub implementation when aws-s3 feature is not enabled
#[cfg(not(feature = "aws-s3"))]
pub struct S3Backend;

#[cfg(not(feature = "aws-s3"))]
impl S3Backend {
    pub fn new(_config: crate::config::S3Config) -> Result<Self, crate::StorageError> {
        Err(crate::StorageError::Configuration(
            "S3 backend requires the 'aws-s3' feature to be enabled".to_string()
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "aws-s3")]
mod tests {
    use super::*;
    
    // Note: These tests would require AWS credentials and a test bucket
    // They are placeholder tests for the structure
    
    #[test]
    fn test_s3_key_generation() {
        let config = S3Config::new("test-bucket".to_string());
        let backend = S3Backend { 
            client: Client::new(&aws_config::Config::builder().build()),
            config,
        };
        
        assert_eq!(backend.get_s3_key("test.txt"), "test.txt");
        assert_eq!(backend.get_s3_key("/test.txt"), "test.txt");
    }
    
    #[test]
    fn test_s3_key_with_prefix() {
        let config = S3Config::new("test-bucket".to_string())
            .with_prefix("uploads/");
        let backend = S3Backend { 
            client: Client::new(&aws_config::Config::builder().build()),
            config,
        };
        
        assert_eq!(backend.get_s3_key("test.txt"), "uploads/test.txt");
        assert_eq!(backend.get_s3_key("/test.txt"), "uploads/test.txt");
    }
    
    #[test]
    fn test_public_url_generation() {
        let config = S3Config::new("test-bucket".to_string())
            .with_region("us-west-2".to_string());
        let backend = S3Backend { 
            client: Client::new(&aws_config::Config::builder().build()),
            config,
        };
        
        let url = backend.get_public_url("test.txt");
        assert!(url.contains("test-bucket"));
        assert!(url.contains("us-west-2"));
        assert!(url.contains("test.txt"));
    }
    
    #[test]
    fn test_public_url_with_cdn() {
        let config = S3Config::new("test-bucket".to_string())
            .with_cdn("cdn.example.com".to_string());
        let backend = S3Backend { 
            client: Client::new(&aws_config::Config::builder().build()),
            config,
        };
        
        let url = backend.get_public_url("test.txt");
        assert_eq!(url, "https://cdn.example.com/test.txt");
    }
}