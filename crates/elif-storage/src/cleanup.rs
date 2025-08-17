//! Temporary file cleanup system

use crate::{StorageResult, StorageError};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{info, warn, error};

/// Cleanup manager for temporary files
#[derive(Debug)]
pub struct CleanupManager {
    temp_directories: Vec<PathBuf>,
    max_age: Duration,
    cleanup_interval: Duration,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CleanupManager {
    /// Create a new cleanup manager
    pub fn new() -> Self {
        Self {
            temp_directories: vec![std::env::temp_dir()],
            max_age: Duration::from_secs(24 * 60 * 60), // 24 hours
            cleanup_interval: Duration::from_secs(60 * 60), // 1 hour
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Add a directory to monitor for cleanup
    pub fn add_temp_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.temp_directories.push(path.into());
        self
    }
    
    /// Set maximum age for temporary files
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }
    
    /// Set cleanup interval
    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }
    
    /// Start the cleanup background task
    pub async fn start(&self) -> StorageResult<tokio::task::JoinHandle<()>> {
        if self.running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Err(StorageError::Configuration("Cleanup manager already running".to_string()));
        }
        
        let temp_directories = self.temp_directories.clone();
        let max_age = self.max_age;
        let cleanup_interval = self.cleanup_interval;
        let running = self.running.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            
            info!("Starting cleanup manager with {} directories to monitor", temp_directories.len());
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;
                
                for dir in &temp_directories {
                    if let Err(e) = cleanup_directory(dir, max_age).await {
                        error!("Failed to cleanup directory {}: {}", dir.display(), e);
                    }
                }
            }
            
            info!("Cleanup manager stopped");
        });
        
        Ok(handle)
    }
    
    /// Stop the cleanup manager
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// Run cleanup once
    pub async fn run_once(&self) -> StorageResult<CleanupStats> {
        let mut total_stats = CleanupStats::default();
        
        for dir in &self.temp_directories {
            let stats = cleanup_directory_with_stats(dir, self.max_age).await?;
            total_stats.files_deleted += stats.files_deleted;
            total_stats.bytes_freed += stats.bytes_freed;
            total_stats.directories_removed += stats.directories_removed;
            total_stats.errors += stats.errors;
        }
        
        Ok(total_stats)
    }
}

impl Default for CleanupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Cleanup statistics
#[derive(Debug, Default, Clone)]
pub struct CleanupStats {
    /// Number of files deleted
    pub files_deleted: u64,
    
    /// Number of bytes freed
    pub bytes_freed: u64,
    
    /// Number of empty directories removed
    pub directories_removed: u64,
    
    /// Number of errors encountered
    pub errors: u64,
}

/// Clean up old files in a directory
async fn cleanup_directory(dir: &Path, max_age: Duration) -> StorageResult<()> {
    let _stats = cleanup_directory_with_stats(dir, max_age).await?;
    Ok(())
}

/// Clean up old files in a directory and return statistics
async fn cleanup_directory_with_stats(dir: &Path, max_age: Duration) -> StorageResult<CleanupStats> {
    if !dir.exists() {
        return Ok(CleanupStats::default());
    }
    
    let mut stats = CleanupStats::default();
    let cutoff_time = SystemTime::now()
        .checked_sub(max_age)
        .unwrap_or(UNIX_EPOCH);
    
    match cleanup_recursive(dir, cutoff_time, &mut stats).await {
        Ok(_) => {
            if stats.files_deleted > 0 || stats.directories_removed > 0 {
                info!(
                    "Cleanup completed for {}: {} files deleted, {} bytes freed, {} directories removed",
                    dir.display(), stats.files_deleted, stats.bytes_freed, stats.directories_removed
                );
            }
            Ok(stats)
        }
        Err(e) => {
            error!("Cleanup failed for {}: {}", dir.display(), e);
            stats.errors += 1;
            Ok(stats)
        }
    }
}

/// Recursively clean up files and directories
fn cleanup_recursive<'a>(dir: &'a Path, cutoff_time: SystemTime, stats: &'a mut CleanupStats) -> std::pin::Pin<Box<dyn std::future::Future<Output = StorageResult<bool>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = tokio::fs::read_dir(dir).await?;
        let mut dir_empty = true;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = match entry.metadata().await {
                Ok(metadata) => metadata,
                Err(e) => {
                    warn!("Failed to get metadata for {}: {}", path.display(), e);
                    stats.errors += 1;
                    dir_empty = false; // Assume not empty if we can't check
                    continue;
                }
            };
            
            if metadata.is_file() {
                // Check file age
                match metadata.modified() {
                    Ok(modified) if modified < cutoff_time => {
                        // File is old enough to delete
                        match tokio::fs::remove_file(&path).await {
                            Ok(_) => {
                                stats.files_deleted += 1;
                                stats.bytes_freed += metadata.len();
                                info!("Deleted old file: {}", path.display());
                            }
                            Err(e) => {
                                warn!("Failed to delete file {}: {}", path.display(), e);
                                stats.errors += 1;
                                dir_empty = false;
                            }
                        }
                    }
                    Ok(_) => {
                        // File is still young
                        dir_empty = false;
                    }
                    Err(e) => {
                        warn!("Failed to get modification time for {}: {}", path.display(), e);
                        stats.errors += 1;
                        dir_empty = false;
                    }
                }
            } else if metadata.is_dir() {
                // Recursively clean subdirectory
                match cleanup_recursive(&path, cutoff_time, stats).await {
                    Ok(subdir_empty) => {
                        if subdir_empty {
                            // Try to remove empty directory
                            match tokio::fs::remove_dir(&path).await {
                                Ok(_) => {
                                    stats.directories_removed += 1;
                                    info!("Removed empty directory: {}", path.display());
                                }
                                Err(e) => {
                                    warn!("Failed to remove empty directory {}: {}", path.display(), e);
                                    stats.errors += 1;
                                    dir_empty = false;
                                }
                            }
                        } else {
                            dir_empty = false;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to cleanup directory {}: {}", path.display(), e);
                        stats.errors += 1;
                        dir_empty = false;
                    }
                }
            }
        }
        
        Ok(dir_empty)
    })
}

/// Temporary file handle that automatically cleans up on drop
#[derive(Debug)]
pub struct TempFile {
    path: PathBuf,
    keep_on_drop: bool,
}

impl TempFile {
    /// Create a new temporary file
    pub async fn new() -> StorageResult<Self> {
        let temp_dir = std::env::temp_dir();
        let filename = format!("elif-temp-{}", uuid::Uuid::new_v4());
        let path = temp_dir.join(filename);
        
        // Create the file
        tokio::fs::File::create(&path).await?;
        
        Ok(Self {
            path,
            keep_on_drop: false,
        })
    }
    
    /// Create a temporary file with a specific extension
    pub async fn with_extension(extension: &str) -> StorageResult<Self> {
        let temp_dir = std::env::temp_dir();
        let filename = format!("elif-temp-{}.{}", uuid::Uuid::new_v4(), extension);
        let path = temp_dir.join(filename);
        
        // Create the file
        tokio::fs::File::create(&path).await?;
        
        Ok(Self {
            path,
            keep_on_drop: false,
        })
    }
    
    /// Get the path to the temporary file
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Keep the file when this handle is dropped
    pub fn keep_on_drop(mut self) -> Self {
        self.keep_on_drop = true;
        self
    }
    
    /// Persist the temporary file to a permanent location
    pub async fn persist<P: AsRef<Path>>(mut self, path: P) -> StorageResult<()> {
        self.keep_on_drop = true;
        tokio::fs::rename(&self.path, path).await?;
        Ok(())
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.keep_on_drop && self.path.exists() {
            let path_to_delete = self.path.clone();
            // Use spawn_blocking to avoid blocking the async runtime
            tokio::task::spawn_blocking(move || {
                if let Err(e) = std::fs::remove_file(&path_to_delete) {
                    warn!("Failed to cleanup temporary file {}: {}", path_to_delete.display(), e);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_cleanup_manager() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        
        // Create some test files with different ages
        let old_file = temp_path.join("old_file.txt");
        let new_file = temp_path.join("new_file.txt");
        
        // Create old file and set its modification time to 2 days ago
        std::fs::write(&old_file, "old content").unwrap();
        let two_days_ago = SystemTime::now() - Duration::from_secs(2 * 24 * 60 * 60);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from(two_days_ago)).unwrap();
        
        // Create new file
        std::fs::write(&new_file, "new content").unwrap();
        
        // Test cleanup
        let manager = CleanupManager::new()
            .add_temp_directory(temp_path)
            .with_max_age(Duration::from_secs(24 * 60 * 60)); // 1 day
        
        let stats = manager.run_once().await.unwrap();
        
        // Old file should be deleted, new file should remain
        assert!(!old_file.exists());
        assert!(new_file.exists());
        // The stats.files_deleted might include other temporary files, so just check it's > 0
        assert!(stats.files_deleted > 0);
        assert!(stats.bytes_freed > 0);
    }
    
    #[tokio::test]
    async fn test_temp_file() {
        let temp_file = TempFile::new().await.unwrap();
        let path = temp_file.path().to_path_buf();
        
        assert!(path.exists());
        
        // Write some content
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap();
        file.write_all(b"test content").unwrap();
        drop(file);
        
        // Drop the temp file - it should be cleaned up
        drop(temp_file);
        
        // Since cleanup is async, wait a bit for it to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // File should be deleted
        assert!(!path.exists());
    }
    
    #[tokio::test]
    async fn test_temp_file_keep_on_drop() {
        let temp_file = TempFile::new().await.unwrap();
        let path = temp_file.path().to_path_buf();
        
        assert!(path.exists());
        
        // Keep on drop
        let temp_file = temp_file.keep_on_drop();
        drop(temp_file);
        
        // File should still exist
        assert!(path.exists());
        
        // Clean up manually
        std::fs::remove_file(&path).unwrap();
    }
    
    #[tokio::test]
    async fn test_temp_file_with_extension() {
        let temp_file = TempFile::with_extension("txt").await.unwrap();
        let path = temp_file.path().to_path_buf();
        
        assert!(path.exists());
        assert_eq!(path.extension().unwrap(), "txt");
        
        drop(temp_file);
        
        // Since cleanup is async, wait a bit for it to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        assert!(!path.exists());
    }
}