# elif-storage

A comprehensive multi-backend file storage system for the elif.rs framework.

## Features

- **Multi-backend support**: Local filesystem, AWS S3 support
- **File upload validation**: Size, type, and content validation with security checks
- **Image processing**: Resize, crop, optimize, and format conversion (optional feature)
- **CDN integration**: Signed URLs and public URL generation
- **Access control**: File permissions and access control system (optional feature)
- **Temporary file management**: Automatic cleanup of temporary files
- **Streaming support**: Handle large files efficiently with async streaming
- **Async-first**: Built for modern async Rust applications

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
elif-storage = { version = "0.1.0", path = "path/to/elif/crates/elif-storage" }
```

### Basic Usage

```rust
use elif_storage::{Storage, LocalBackend, LocalStorageConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure local storage
    let config = LocalStorageConfig::new()
        .with_root_path("./storage");
    
    // Create storage instance
    let storage = Storage::new(LocalBackend::new(config));
    
    // Store a file
    let file_data = b"Hello, World!";
    let metadata = storage.put("documents/hello.txt", file_data, None).await?;
    println!("Stored file: {} bytes", metadata.size);
    
    // Retrieve a file
    let retrieved = storage.get("documents/hello.txt").await?;
    if let Some(data) = retrieved {
        println!("Retrieved: {}", String::from_utf8_lossy(&data));
    }
    
    Ok(())
}
```

### AWS S3 Backend

Enable the AWS S3 feature:

```toml
[dependencies]
elif-storage = { version = "0.1.0", path = "path/to/elif", features = ["aws-s3"] }
```

```rust
use elif_storage::{Storage, S3Backend, S3Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure S3 storage
    let config = S3Config::new("my-bucket".to_string())
        .with_region("us-east-1".to_string())
        .with_credentials("access-key".to_string(), "secret-key".to_string());
    
    // Create S3 storage instance
    let storage = Storage::new(S3Backend::new(config).await?);
    
    // Use the same API as local storage
    let file_data = b"Hello from S3!";
    let metadata = storage.put("documents/s3-file.txt", file_data, None).await?;
    
    // Generate signed URL
    let signed_url = storage.signed_url("documents/s3-file.txt", Duration::from_secs(3600)).await?;
    println!("Signed URL: {}", signed_url);
    
    Ok(())
}
```

## Features

### File Validation

The storage system includes comprehensive file validation:

```rust
use elif_storage::{Storage, LocalBackend, LocalStorageConfig, ValidationConfig, FileValidator};

// Create validator with custom rules
let validation_config = ValidationConfig::new()
    .max_size(10 * 1024 * 1024)  // 10MB max
    .allow_mime_types(vec!["image/".to_string(), "text/".to_string()])
    .block_extensions(vec!["exe".to_string(), "bat".to_string()])
    .validate_content();  // Check file content matches declared type

let validator = FileValidator::new(validation_config);

// Validate a file before storage
let file_data = b"some file content";
validator.validate_file("document.txt", file_data, "text/plain")?;
```

### Upload Options

Customize file uploads with metadata and options:

```rust
use elif_storage::UploadOptions;

let options = UploadOptions::new()
    .content_type("image/jpeg".to_string())
    .metadata("author".to_string(), "John Doe".to_string())
    .metadata("project".to_string(), "MyApp".to_string())
    .cache_control("public, max-age=3600".to_string())
    .overwrite();  // Allow overwriting existing files

let metadata = storage.put("uploads/image.jpg", image_data, Some(options)).await?;
```

### Image Processing

Enable image processing features:

```toml
[dependencies]
elif-storage = { version = "0.1.0", features = ["image-processing"] }
```

```rust
use elif_storage::{ImageProcessor, ImageOperation};

let processor = ImageProcessor::new()
    .with_max_dimensions(2048, 2048)
    .with_allowed_formats(vec![ImageFormat::Jpeg, ImageFormat::Png]);

let operations = vec![
    ImageOperation::Resize { width: 800, height: 600, maintain_aspect_ratio: true },
    ImageOperation::ConvertFormat { format: ImageFormat::Jpeg },
    ImageOperation::Quality { quality: 85 },
];

let processed_data = processor.process_image(&original_data, &operations)?;
```

### Temporary File Management

Automatic cleanup of temporary files:

```rust
use elif_storage::{TempFile, CleanupManager};
use std::time::Duration;

// Create temporary file that auto-deletes on drop
let temp_file = TempFile::new().await?;
// Use temp_file.path() to get the path
// File is automatically deleted when temp_file is dropped

// Or manage cleanup manually
let cleanup_manager = CleanupManager::new()
    .add_temp_directory("/tmp/my-app")
    .with_max_age(Duration::from_secs(24 * 60 * 60))  // 24 hours
    .with_cleanup_interval(Duration::from_secs(60 * 60));  // Check hourly

let handle = cleanup_manager.start().await?;
```

## Storage Backend API

All storage backends implement the `StorageBackend` trait:

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put(&self, path: &str, data: &[u8], options: Option<UploadOptions>) -> StorageResult<FileMetadata>;
    async fn put_stream<S>(&self, path: &str, stream: S, options: Option<UploadOptions>) -> StorageResult<FileMetadata>;
    async fn get(&self, path: &str) -> StorageResult<Option<Bytes>>;
    async fn get_stream(&self, path: &str) -> StorageResult<Option<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>>>;
    async fn exists(&self, path: &str) -> StorageResult<bool>;
    async fn metadata(&self, path: &str) -> StorageResult<Option<FileMetadata>>;
    async fn delete(&self, path: &str) -> StorageResult<bool>;
    async fn list(&self, prefix: Option<&str>, limit: Option<u32>) -> StorageResult<Vec<FileMetadata>>;
    async fn copy(&self, from: &str, to: &str, options: Option<UploadOptions>) -> StorageResult<FileMetadata>;
    async fn move_file(&self, from: &str, to: &str, options: Option<UploadOptions>) -> StorageResult<FileMetadata>;
    async fn signed_url(&self, path: &str, expires_in: Duration) -> StorageResult<String>;
    async fn public_url(&self, path: &str) -> StorageResult<String>;
    // ... more methods
}
```

## Available Features

- `default`: Local filesystem backend only
- `aws-s3`: Enable AWS S3 backend support
- `image-processing`: Enable image manipulation capabilities
- `access-control`: Enable file permissions and access control
- `all`: Enable all features

## Examples

Run the included examples:

```bash
# Basic usage example
cargo run --example basic_usage

# Image processing example (requires image-processing feature)
cargo run --example image_processing --features image-processing

# S3 example (requires aws-s3 feature)
cargo run --example s3_usage --features aws-s3
```

## Safety and Security

- **Path sanitization**: Prevents directory traversal attacks
- **Content validation**: Verifies file content matches declared MIME type
- **Malware detection**: Basic detection of executable files and dangerous content
- **Size limits**: Configurable file size restrictions
- **Type restrictions**: MIME type allowlists and blocklists

## License

This project is licensed under the MIT OR Apache-2.0 license.