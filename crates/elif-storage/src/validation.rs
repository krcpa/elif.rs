//! File validation utilities

use crate::{StorageResult, StorageError};
use std::collections::HashSet;

/// File validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum file size in bytes
    pub max_file_size: Option<u64>,
    
    /// Minimum file size in bytes
    pub min_file_size: Option<u64>,
    
    /// Allowed MIME types (prefixes)
    pub allowed_mime_types: Option<HashSet<String>>,
    
    /// Blocked MIME types (prefixes)
    pub blocked_mime_types: Option<HashSet<String>>,
    
    /// Allowed file extensions
    pub allowed_extensions: Option<HashSet<String>>,
    
    /// Blocked file extensions
    pub blocked_extensions: Option<HashSet<String>>,
    
    /// Enable content validation (magic number checking)
    pub validate_content: bool,
    
    /// Maximum filename length
    pub max_filename_length: Option<usize>,
    
    /// Allow unicode characters in filenames
    pub allow_unicode_filenames: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_file_size: Some(100 * 1024 * 1024), // 100MB
            min_file_size: Some(1), // At least 1 byte
            allowed_mime_types: None,
            blocked_mime_types: Some({
                let mut blocked = HashSet::new();
                // Block potentially dangerous file types
                blocked.insert("application/x-executable".to_string());
                blocked.insert("application/x-msdownload".to_string());
                blocked.insert("application/x-dosexec".to_string());
                blocked
            }),
            allowed_extensions: None,
            blocked_extensions: Some({
                let mut blocked = HashSet::new();
                // Block dangerous extensions
                blocked.insert("exe".to_string());
                blocked.insert("bat".to_string());
                blocked.insert("cmd".to_string());
                blocked.insert("com".to_string());
                blocked.insert("scr".to_string());
                blocked.insert("pif".to_string());
                blocked
            }),
            validate_content: true,
            max_filename_length: Some(255),
            allow_unicode_filenames: true,
        }
    }
}

impl ValidationConfig {
    /// Create a new validation configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set maximum file size
    pub fn max_size(mut self, size: u64) -> Self {
        self.max_file_size = Some(size);
        self
    }
    
    /// Remove maximum file size limit
    pub fn unlimited_size(mut self) -> Self {
        self.max_file_size = None;
        self
    }
    
    /// Set minimum file size
    pub fn min_size(mut self, size: u64) -> Self {
        self.min_file_size = Some(size);
        self
    }
    
    /// Allow specific MIME types only
    pub fn allow_mime_types<I>(mut self, types: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.allowed_mime_types = Some(types.into_iter().collect());
        self
    }
    
    /// Block specific MIME types
    pub fn block_mime_types<I>(mut self, types: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.blocked_mime_types = Some(types.into_iter().collect());
        self
    }
    
    /// Allow specific file extensions only
    pub fn allow_extensions<I>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.allowed_extensions = Some(extensions.into_iter().collect());
        self
    }
    
    /// Block specific file extensions
    pub fn block_extensions<I>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.blocked_extensions = Some(extensions.into_iter().collect());
        self
    }
    
    /// Enable content validation
    pub fn validate_content(mut self) -> Self {
        self.validate_content = true;
        self
    }
    
    /// Disable content validation
    pub fn skip_content_validation(mut self) -> Self {
        self.validate_content = false;
        self
    }
    
    /// Set maximum filename length
    pub fn max_filename_length(mut self, length: usize) -> Self {
        self.max_filename_length = Some(length);
        self
    }
    
    /// Allow unicode characters in filenames
    pub fn allow_unicode_filenames(mut self) -> Self {
        self.allow_unicode_filenames = true;
        self
    }
    
    /// Disallow unicode characters in filenames
    pub fn ascii_filenames_only(mut self) -> Self {
        self.allow_unicode_filenames = false;
        self
    }
}

/// File validator
#[derive(Debug)]
pub struct FileValidator {
    config: ValidationConfig,
}

impl FileValidator {
    /// Create a new file validator
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }
    
    /// Validate file size
    pub fn validate_size(&self, size: u64) -> StorageResult<()> {
        if let Some(max_size) = self.config.max_file_size {
            if size > max_size {
                return Err(StorageError::FileTooLarge(size, max_size));
            }
        }
        
        if let Some(min_size) = self.config.min_file_size {
            if size < min_size {
                return Err(StorageError::Validation(format!(
                    "File too small: {} bytes, minimum required: {} bytes",
                    size, min_size
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate MIME type
    pub fn validate_mime_type(&self, mime_type: &str) -> StorageResult<()> {
        // Check blocked types first
        if let Some(blocked) = &self.config.blocked_mime_types {
            for blocked_type in blocked {
                if mime_type.starts_with(blocked_type) {
                    return Err(StorageError::UnsupportedFileType(format!(
                        "File type '{}' is blocked", mime_type
                    )));
                }
            }
        }
        
        // Check allowed types
        if let Some(allowed) = &self.config.allowed_mime_types {
            let is_allowed = allowed.iter().any(|allowed_type| mime_type.starts_with(allowed_type));
            if !is_allowed {
                return Err(StorageError::UnsupportedFileType(format!(
                    "File type '{}' is not allowed", mime_type
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate file extension
    pub fn validate_extension(&self, filename: &str) -> StorageResult<()> {
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        // Check blocked extensions first
        if let Some(blocked) = &self.config.blocked_extensions {
            if blocked.contains(&extension) {
                return Err(StorageError::UnsupportedFileType(format!(
                    "File extension '{}' is blocked", extension
                )));
            }
        }
        
        // Check allowed extensions
        if let Some(allowed) = &self.config.allowed_extensions {
            if !allowed.contains(&extension) {
                return Err(StorageError::UnsupportedFileType(format!(
                    "File extension '{}' is not allowed", extension
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate filename
    pub fn validate_filename(&self, filename: &str) -> StorageResult<()> {
        // Check filename length
        if let Some(max_length) = self.config.max_filename_length {
            if filename.len() > max_length {
                return Err(StorageError::Validation(format!(
                    "Filename too long: {} characters, maximum allowed: {}",
                    filename.len(), max_length
                )));
            }
        }
        
        // Check for empty filename
        if filename.trim().is_empty() {
            return Err(StorageError::Validation("Filename cannot be empty".to_string()));
        }
        
        // Check for dangerous characters
        let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*', '\0'];
        if filename.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(StorageError::Validation(format!(
                "Filename contains dangerous characters: '{}'", filename
            )));
        }
        
        // Check for unicode if not allowed
        if !self.config.allow_unicode_filenames && !filename.is_ascii() {
            return Err(StorageError::Validation(
                "Unicode characters not allowed in filename".to_string()
            ));
        }
        
        // Check for reserved names (Windows)
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL",
            "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
            "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        
        let name_part = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
            .to_uppercase();
        
        if reserved_names.contains(&name_part.as_str()) {
            return Err(StorageError::Validation(format!(
                "Filename '{}' is reserved", filename
            )));
        }
        
        Ok(())
    }
    
    /// Validate file content (magic number checking)
    pub fn validate_content(&self, filename: &str, content: &[u8], declared_mime: &str) -> StorageResult<()> {
        if !self.config.validate_content {
            return Ok(());
        }
        
        let detected_mime = detect_mime_from_content(content);
        
        // If we can detect the MIME type, ensure it matches the declared type
        if let Some(detected) = detected_mime {
            // Allow some flexibility - just check the major type matches
            let declared_major = declared_mime.split('/').next().unwrap_or(declared_mime);
            let detected_major = detected.split('/').next().unwrap_or(&detected);
            
            if declared_major != "application" && declared_major != detected_major {
                return Err(StorageError::Validation(format!(
                    "File content does not match declared type. Declared: '{}', Detected: '{}'",
                    declared_mime, detected
                )));
            }
        }
        
        // Check for dangerous content patterns
        self.scan_for_dangerous_content(filename, content)?;
        
        Ok(())
    }
    
    /// Scan content for dangerous patterns
    fn scan_for_dangerous_content(&self, filename: &str, content: &[u8]) -> StorageResult<()> {
        // Check for executable signatures
        if content.len() >= 2 {
            match &content[0..2] {
                [0x4D, 0x5A] => { // PE executable (Windows)
                    return Err(StorageError::Validation(
                        "File appears to be a Windows executable".to_string()
                    ));
                }
                [0x7F, 0x45] if content.len() >= 4 && &content[2..4] == [0x4C, 0x46] => { // ELF
                    return Err(StorageError::Validation(
                        "File appears to be a Linux executable".to_string()
                    ));
                }
                _ => {}
            }
        }
        
        // Check for Mach-O executables (macOS)
        if content.len() >= 4 {
            match &content[0..4] {
                [0xFE, 0xED, 0xFA, 0xCE] | 
                [0xFE, 0xED, 0xFA, 0xCF] | 
                [0xCE, 0xFA, 0xED, 0xFE] | 
                [0xCF, 0xFA, 0xED, 0xFE] => {
                    return Err(StorageError::Validation(
                        "File appears to be a macOS executable".to_string()
                    ));
                }
                _ => {}
            }
        }
        
        // Check for script files that might be dangerous
        if let Ok(text) = std::str::from_utf8(content) {
            let text_lower = text.to_lowercase();
            
            // Check for script shebangs
            if text.starts_with("#!") {
                let extension = std::path::Path::new(filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                // Allow some safe script types
                let safe_scripts = ["sh", "bash", "py", "rb", "js", "pl"];
                if !safe_scripts.contains(&extension.as_str()) {
                    return Err(StorageError::Validation(
                        "Executable script files are not allowed".to_string()
                    ));
                }
            }
            
            // Check for dangerous PowerShell or batch commands
            let dangerous_patterns = [
                "invoke-expression", "iex", "invoke-webrequest", "iwr",
                "start-process", "downloadstring", "downloadfile",
                "@echo off", "cmd.exe", "powershell.exe",
            ];
            
            for pattern in &dangerous_patterns {
                if text_lower.contains(pattern) {
                    return Err(StorageError::Validation(format!(
                        "File contains potentially dangerous content: '{}'", pattern
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate complete file
    pub fn validate_file(&self, filename: &str, content: &[u8], mime_type: &str) -> StorageResult<()> {
        self.validate_filename(filename)?;
        self.validate_size(content.len() as u64)?;
        self.validate_extension(filename)?;
        self.validate_mime_type(mime_type)?;
        self.validate_content(filename, content, mime_type)?;
        
        Ok(())
    }
}

/// Detect MIME type from file content using magic numbers
fn detect_mime_from_content(content: &[u8]) -> Option<String> {
    if content.len() < 4 {
        return None;
    }
    
    match &content[0..4] {
        [0xFF, 0xD8, 0xFF, _] => Some("image/jpeg".to_string()),
        [0x89, 0x50, 0x4E, 0x47] => Some("image/png".to_string()),
        [0x47, 0x49, 0x46, 0x38] => Some("image/gif".to_string()),
        [0x25, 0x50, 0x44, 0x46] => Some("application/pdf".to_string()),
        [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08] => {
            Some("application/zip".to_string())
        }
        _ => {
            // Check for text content
            if content.iter().take(1024).all(|&b| b.is_ascii() && (b >= 32 || b == 9 || b == 10 || b == 13)) {
                Some("text/plain".to_string())
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validation_config() {
        let config = ValidationConfig::new()
            .max_size(50 * 1024 * 1024)
            .allow_mime_types(vec!["image/".to_string(), "text/".to_string()])
            .block_extensions(vec!["exe".to_string(), "bat".to_string()])
            .ascii_filenames_only();
        
        assert_eq!(config.max_file_size, Some(50 * 1024 * 1024));
        assert!(config.allowed_mime_types.as_ref().unwrap().contains("image/"));
        assert!(config.blocked_extensions.as_ref().unwrap().contains("exe"));
        assert!(!config.allow_unicode_filenames);
    }
    
    #[test]
    fn test_file_validator_size() {
        let config = ValidationConfig::new().max_size(1000).min_size(10);
        let validator = FileValidator::new(config);
        
        assert!(validator.validate_size(500).is_ok());
        assert!(validator.validate_size(1000).is_ok());
        assert!(validator.validate_size(1001).is_err());
        assert!(validator.validate_size(5).is_err());
    }
    
    #[test]
    fn test_file_validator_mime_type() {
        let config = ValidationConfig::new()
            .allow_mime_types(vec!["image/".to_string(), "text/plain".to_string()]);
        let validator = FileValidator::new(config);
        
        assert!(validator.validate_mime_type("image/jpeg").is_ok());
        assert!(validator.validate_mime_type("image/png").is_ok());
        assert!(validator.validate_mime_type("text/plain").is_ok());
        assert!(validator.validate_mime_type("application/pdf").is_err());
        assert!(validator.validate_mime_type("text/html").is_err());
    }
    
    #[test]
    fn test_file_validator_extension() {
        let config = ValidationConfig::new()
            .block_extensions(vec!["exe".to_string(), "bat".to_string()]);
        let validator = FileValidator::new(config);
        
        assert!(validator.validate_extension("document.pdf").is_ok());
        assert!(validator.validate_extension("image.jpg").is_ok());
        assert!(validator.validate_extension("script.exe").is_err());
        assert!(validator.validate_extension("script.bat").is_err());
        assert!(validator.validate_extension("Script.EXE").is_err()); // Case insensitive
    }
    
    #[test]
    fn test_file_validator_filename() {
        let config = ValidationConfig::new().max_filename_length(20).ascii_filenames_only();
        let validator = FileValidator::new(config);
        
        assert!(validator.validate_filename("document.pdf").is_ok());
        assert!(validator.validate_filename("very_long_filename_that_exceeds_limit.txt").is_err());
        assert!(validator.validate_filename("").is_err());
        assert!(validator.validate_filename("file<script>.txt").is_err()); // Dangerous char
        assert!(validator.validate_filename("测试.txt").is_err()); // Unicode not allowed
        assert!(validator.validate_filename("CON.txt").is_err()); // Reserved name
    }
    
    #[test]
    fn test_detect_mime_from_content() {
        // JPEG
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_mime_from_content(&jpeg_data), Some("image/jpeg".to_string()));
        
        // PNG
        let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_mime_from_content(&png_data), Some("image/png".to_string()));
        
        // Text
        let text_data = b"Hello, World!";
        assert_eq!(detect_mime_from_content(text_data), Some("text/plain".to_string()));
        
        // Unknown binary
        let binary_data = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_mime_from_content(&binary_data), None);
    }
    
    #[test]
    fn test_dangerous_content_detection() {
        let config = ValidationConfig::new();
        let validator = FileValidator::new(config);
        
        // PE executable
        let pe_data = [0x4D, 0x5A, 0x90, 0x00]; // MZ header
        assert!(validator.scan_for_dangerous_content("test.txt", &pe_data).is_err());
        
        // ELF executable
        let elf_data = [0x7F, 0x45, 0x4C, 0x46]; // ELF header
        assert!(validator.scan_for_dangerous_content("test.txt", &elf_data).is_err());
        
        // Safe text
        let safe_text = b"This is just normal text content.";
        assert!(validator.scan_for_dangerous_content("test.txt", safe_text).is_ok());
        
        // Dangerous script content
        let dangerous_script = b"powershell.exe -Command Invoke-Expression";
        assert!(validator.scan_for_dangerous_content("test.txt", dangerous_script).is_err());
    }
}