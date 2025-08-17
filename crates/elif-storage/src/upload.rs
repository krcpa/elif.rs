//! File upload utilities

use crate::{StorageResult, StorageError};
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// File upload stream wrapper
pub struct UploadStream<S> {
    inner: S,
    bytes_read: u64,
    max_size: Option<u64>,
}

impl<S> UploadStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>>,
{
    /// Create a new upload stream with optional size limit
    pub fn new(stream: S, max_size: Option<u64>) -> Self {
        Self {
            inner: stream,
            bytes_read: 0,
            max_size,
        }
    }
    
    /// Get the total bytes read so far
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
}

impl<S> Stream for UploadStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.bytes_read += chunk.len() as u64;
                
                // Check size limit
                if let Some(max_size) = self.max_size {
                    if self.bytes_read > max_size {
                        return Poll::Ready(Some(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Upload size {} exceeds maximum {}", self.bytes_read, max_size),
                        ))));
                    }
                }
                
                Poll::Ready(Some(Ok(chunk)))
            }
            other => other,
        }
    }
}

/// Multipart upload manager for large files
pub struct MultipartUpload {
    upload_id: String,
    parts: Vec<UploadPart>,
    current_part: u32,
}

/// Upload part information
#[derive(Debug, Clone)]
pub struct UploadPart {
    pub part_number: u32,
    pub etag: String,
    pub size: u64,
}

impl MultipartUpload {
    /// Create a new multipart upload
    pub fn new(upload_id: String) -> Self {
        Self {
            upload_id,
            parts: Vec::new(),
            current_part: 1,
        }
    }
    
    /// Get the upload ID
    pub fn upload_id(&self) -> &str {
        &self.upload_id
    }
    
    /// Add a completed part
    pub fn add_part(&mut self, etag: String, size: u64) {
        self.parts.push(UploadPart {
            part_number: self.current_part,
            etag,
            size,
        });
        self.current_part += 1;
    }
    
    /// Get all parts
    pub fn parts(&self) -> &[UploadPart] {
        &self.parts
    }
    
    /// Get total size of uploaded parts
    pub fn total_size(&self) -> u64 {
        self.parts.iter().map(|p| p.size).sum()
    }
    
    /// Check if upload is complete (has at least one part)
    pub fn is_complete(&self) -> bool {
        !self.parts.is_empty()
    }
}

/// Upload progress callback
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Upload configuration
pub struct UploadConfig {
    /// Chunk size for streaming uploads (default: 8MB)
    pub chunk_size: usize,
    
    /// Enable progress reporting
    pub report_progress: bool,
    
    /// Progress callback
    pub progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    
    /// Maximum concurrent uploads for multipart
    pub max_concurrent_parts: usize,
    
    /// Minimum part size for multipart uploads (default: 5MB)
    pub min_part_size: u64,
}

impl std::fmt::Debug for UploadConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadConfig")
            .field("chunk_size", &self.chunk_size)
            .field("report_progress", &self.report_progress)
            .field("progress_callback", &self.progress_callback.as_ref().map(|_| "Some(callback)"))
            .field("max_concurrent_parts", &self.max_concurrent_parts)
            .field("min_part_size", &self.min_part_size)
            .finish()
    }
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            chunk_size: 8 * 1024 * 1024, // 8MB
            report_progress: false,
            progress_callback: None,
            max_concurrent_parts: 4,
            min_part_size: 5 * 1024 * 1024, // 5MB
        }
    }
}

impl UploadConfig {
    /// Create new upload configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set chunk size
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
    
    /// Enable progress reporting
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64, u64) + Send + Sync + 'static,
    {
        self.report_progress = true;
        self.progress_callback = Some(Box::new(callback));
        self
    }
    
    /// Set maximum concurrent parts for multipart uploads
    pub fn with_max_concurrent_parts(mut self, max: usize) -> Self {
        self.max_concurrent_parts = max;
        self
    }
    
    /// Set minimum part size for multipart uploads
    pub fn with_min_part_size(mut self, size: u64) -> Self {
        self.min_part_size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{stream, StreamExt};
    use std::io;
    
    #[tokio::test]
    async fn test_upload_stream() {
        let data = vec![
            Ok(Bytes::from("hello")),
            Ok(Bytes::from(" ")),
            Ok(Bytes::from("world")),
        ];
        
        let stream = stream::iter(data);
        let mut upload_stream = UploadStream::new(stream, Some(20));
        
        let mut result = Vec::new();
        while let Some(chunk) = upload_stream.next().await {
            result.push(chunk.unwrap());
        }
        
        let combined: Bytes = result.into_iter().collect::<Vec<_>>().concat().into();
        assert_eq!(combined, Bytes::from("hello world"));
        assert_eq!(upload_stream.bytes_read(), 11);
    }
    
    #[tokio::test]
    async fn test_upload_stream_size_limit() {
        let data = vec![
            Ok(Bytes::from("hello")),
            Ok(Bytes::from(" world")),
        ];
        
        let stream = stream::iter(data);
        let mut upload_stream = UploadStream::new(stream, Some(5)); // Limit to 5 bytes
        
        // First chunk should succeed
        let chunk1 = upload_stream.next().await.unwrap().unwrap();
        assert_eq!(chunk1, Bytes::from("hello"));
        
        // Second chunk should fail due to size limit
        let chunk2 = upload_stream.next().await.unwrap();
        assert!(chunk2.is_err());
        assert_eq!(chunk2.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }
    
    #[test]
    fn test_multipart_upload() {
        let mut upload = MultipartUpload::new("test-upload-123".to_string());
        
        assert_eq!(upload.upload_id(), "test-upload-123");
        assert!(!upload.is_complete());
        assert_eq!(upload.total_size(), 0);
        
        upload.add_part("etag1".to_string(), 1000);
        upload.add_part("etag2".to_string(), 2000);
        
        assert!(upload.is_complete());
        assert_eq!(upload.total_size(), 3000);
        assert_eq!(upload.parts().len(), 2);
        
        let part1 = &upload.parts()[0];
        assert_eq!(part1.part_number, 1);
        assert_eq!(part1.etag, "etag1");
        assert_eq!(part1.size, 1000);
    }
    
    #[test]
    fn test_upload_config() {
        let config = UploadConfig::new()
            .with_chunk_size(16 * 1024 * 1024)
            .with_max_concurrent_parts(8)
            .with_min_part_size(10 * 1024 * 1024);
        
        assert_eq!(config.chunk_size, 16 * 1024 * 1024);
        assert_eq!(config.max_concurrent_parts, 8);
        assert_eq!(config.min_part_size, 10 * 1024 * 1024);
        assert!(!config.report_progress);
    }
}