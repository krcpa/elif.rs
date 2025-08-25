//! Basic usage example for elif-storage

use elif_storage::{LocalBackend, LocalStorageConfig, Storage, UploadOptions};
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗂️  elif-storage Basic Usage Example");

    // Create a temporary directory for this example
    let temp_dir = tempdir()?;
    let storage_path = temp_dir.path().to_path_buf();
    println!(
        "📁 Using temporary storage directory: {}",
        storage_path.display()
    );

    // Configure local storage
    let config = LocalStorageConfig::new().with_root_path(storage_path);

    // Create storage instance
    let storage = Storage::new(LocalBackend::new(config));

    // Example 1: Basic file operations
    println!("\n📝 Example 1: Basic file operations");

    let file_data = b"Hello, World! This is a test file.";
    let file_path = "documents/hello.txt";

    // Store a file
    println!("  Storing file: {}", file_path);
    let metadata = storage.put(file_path, file_data, None).await?;
    println!("  File stored successfully: {} bytes", metadata.size);

    // Check if file exists
    let exists = storage.exists(file_path).await?;
    println!("  File exists: {}", exists);

    // Get file metadata
    let file_metadata = storage.metadata(file_path).await?;
    if let Some(meta) = file_metadata {
        println!("  File metadata:");
        println!("    Size: {} bytes", meta.size);
        println!("    Content-Type: {}", meta.content_type);
        println!("    Created: {}", meta.created_at);
        println!("    ETag: {:?}", meta.etag);
    }

    // Retrieve the file
    let retrieved = storage.get(file_path).await?;
    if let Some(data) = retrieved {
        println!("  Retrieved {} bytes", data.len());
        println!("  Content: {}", String::from_utf8_lossy(&data));
    }

    // Example 2: File operations with options
    println!("\n🎨 Example 2: File operations with options");

    let image_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde";

    let options = UploadOptions::new()
        .content_type("image/png".to_string())
        .metadata("author".to_string(), "elif-storage".to_string())
        .metadata("version".to_string(), "1.0".to_string())
        .cache_control("public, max-age=3600".to_string());

    let image_path = "images/sample.png";
    println!("  Storing image with metadata: {}", image_path);
    let image_metadata = storage.put(image_path, image_data, Some(options)).await?;

    println!("  Image stored:");
    println!("    Size: {} bytes", image_metadata.size);
    println!("    Content-Type: {}", image_metadata.content_type);
    println!("    Custom metadata: {:?}", image_metadata.metadata);

    // Example 3: File listing
    println!("\n📋 Example 3: File listing");

    // Create a few more test files
    storage
        .put("documents/file1.txt", b"File 1 content", None)
        .await?;
    storage
        .put("documents/file2.txt", b"File 2 content", None)
        .await?;
    storage
        .put("documents/subdir/file3.txt", b"File 3 content", None)
        .await?;

    // List all files
    let all_files = storage.list(None, None).await?;
    println!("  All files ({} total):", all_files.len());
    for file in &all_files {
        println!("    {} ({} bytes)", file.path, file.size);
    }

    // List files with prefix
    let doc_files = storage.list(Some("documents/"), None).await?;
    println!("  Document files ({} total):", doc_files.len());
    for file in &doc_files {
        println!("    {} ({} bytes)", file.path, file.size);
    }

    // Example 4: File copy and move operations
    println!("\n🔄 Example 4: File copy and move operations");

    // Copy a file
    let copy_result = storage
        .copy("documents/hello.txt", "backup/hello_backup.txt", None)
        .await?;
    println!(
        "  Copied file to: {} ({} bytes)",
        copy_result.path, copy_result.size
    );

    // Move a file
    let move_result = storage
        .move_file("documents/file1.txt", "archive/file1.txt", None)
        .await?;
    println!(
        "  Moved file to: {} ({} bytes)",
        move_result.path, move_result.size
    );

    // Verify original file is gone
    let original_exists = storage.exists("documents/file1.txt").await?;
    println!("  Original file exists: {}", original_exists);

    // Example 5: File deletion
    println!("\n🗑️  Example 5: File deletion");

    // Delete a single file
    let deleted = storage.delete("documents/file2.txt").await?;
    println!("  File deleted: {}", deleted);

    // Delete multiple files
    let to_delete = vec!["images/sample.png", "backup/hello_backup.txt"];
    let deleted_files = storage.delete_many(&to_delete).await?;
    println!(
        "  Deleted {} files: {:?}",
        deleted_files.len(),
        deleted_files
    );

    // Final file count
    let remaining_files = storage.list(None, None).await?;
    println!("  Remaining files: {}", remaining_files.len());
    for file in &remaining_files {
        println!("    {}", file.path);
    }

    println!("\n✅ Example completed successfully!");

    Ok(())
}
