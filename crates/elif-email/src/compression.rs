use crate::{Attachment, EmailError, config::AttachmentConfig};

/// Compression utilities for email attachments
pub struct AttachmentCompressor;

impl AttachmentCompressor {
    /// Compress an attachment if beneficial
    pub fn compress_if_beneficial(attachment: &mut Attachment, config: &AttachmentConfig) -> Result<(), EmailError> {
        if !config.auto_compress || attachment.compressed {
            return Ok(());
        }
        
        if !attachment.can_compress() {
            return Ok(());
        }
        
        // For now, only implement basic text compression using gzip
        // In a full implementation, you might want image compression, etc.
        match attachment.content_type.as_str() {
            "text/plain" | "text/html" | "text/css" | "text/javascript" 
            | "application/json" | "application/xml" => {
                Self::gzip_compress(attachment)?;
            }
            // Image compression would go here with libraries like image, mozjpeg, etc.
            _ => {
                // No compression for this type
            }
        }
        
        Ok(())
    }
    
    /// Compress text content using gzip
    fn gzip_compress(attachment: &mut Attachment) -> Result<(), EmailError> {
        use std::io::Write;
        use flate2::{Compression, write::GzEncoder};
        
        // Only compress if it will save significant space
        if attachment.content.len() < 1024 {
            return Ok(()); // Too small to benefit from compression
        }
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&attachment.content)
            .map_err(|e| EmailError::configuration(format!("Compression failed: {}", e)))?;
        
        let compressed = encoder.finish()
            .map_err(|e| EmailError::configuration(format!("Compression failed: {}", e)))?;
        
        // Only use compressed version if it's actually smaller
        if compressed.len() < attachment.content.len() {
            attachment.content = compressed;
            attachment.size = attachment.content.len();
            attachment.compressed = true;
            
            // Update filename and content type to reflect compression in a standard way
            attachment.filename = format!("{}.gz", attachment.filename);
            attachment.content_type = "application/gzip".to_string();
        }
        
        Ok(())
    }
    
    /// Get compression ratio estimate for an attachment
    pub fn estimate_compression_ratio(attachment: &Attachment) -> f64 {
        // If already compressed (gzip), no further compression
        if attachment.compressed || attachment.content_type == "application/gzip" {
            return 1.0;
        }
        
        if !attachment.can_compress() {
            return 1.0;
        }
        
        match attachment.content_type.as_str() {
            "text/plain" => 0.3,  // Text compresses very well
            "text/html" => 0.4,
            "text/css" => 0.5,
            "text/javascript" => 0.6,
            "application/json" => 0.4,
            "application/xml" => 0.5,
            "image/png" => 0.9,   // Already compressed
            "image/jpeg" => 0.95, // Already compressed
            "image/gif" => 0.9,   // Already compressed
            _ => 1.0,
        }
    }
}

/// Validate a collection of attachments
pub fn validate_attachments(attachments: &[Attachment], config: &AttachmentConfig) -> Result<(), EmailError> {
    // Check count
    if attachments.len() > config.max_count {
        return Err(EmailError::validation(
            "attachment_count",
            format!("Too many attachments: {} (max: {})", attachments.len(), config.max_count)
        ));
    }
    
    // Check total size
    let total_size: usize = attachments.iter().map(|a| a.size).sum();
    if total_size > config.max_total_size {
        return Err(EmailError::validation(
            "attachments_total_size", 
            format!("Total attachments size too large: {} bytes (max: {} bytes)",
                total_size, config.max_total_size)
        ));
    }
    
    // Validate each attachment
    for attachment in attachments {
        attachment.validate(config)?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression_ratio_estimates() {
        let text_attachment = Attachment::new("test.txt", b"Hello World".to_vec());
        assert_eq!(AttachmentCompressor::estimate_compression_ratio(&text_attachment), 0.3);
        
        let image_attachment = Attachment::new("test.jpg", b"JPEG data".to_vec());
        assert_eq!(AttachmentCompressor::estimate_compression_ratio(&image_attachment), 0.95);
    }
    
    #[test]
    fn test_attachment_validation() {
        let config = AttachmentConfig::default();
        let attachments = vec![
            Attachment::new("test.txt", b"Hello".to_vec()),
        ];
        
        let result = validate_attachments(&attachments, &config);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_attachment_validation_too_many() {
        let mut config = AttachmentConfig::default();
        config.max_count = 1;
        
        let attachments = vec![
            Attachment::new("test1.txt", b"Hello".to_vec()),
            Attachment::new("test2.txt", b"World".to_vec()),
        ];
        
        let result = validate_attachments(&attachments, &config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_compression_updates_filename_and_content_type() {
        let mut attachment = Attachment::new("test.txt", b"Hello World! This is a test document that should compress well because it has repeating text. Hello World! This is a test document that should compress well because it has repeating text.".to_vec());
        let config = AttachmentConfig {
            auto_compress: true,
            ..AttachmentConfig::default()
        };
        
        let original_filename = attachment.filename.clone();
        let original_content_type = attachment.content_type.clone();
        
        let result = AttachmentCompressor::compress_if_beneficial(&mut attachment, &config);
        assert!(result.is_ok());
        
        // If compression occurred, filename and content type should be updated
        if attachment.compressed {
            assert_eq!(attachment.filename, format!("{}.gz", original_filename));
            assert_eq!(attachment.content_type, "application/gzip");
        } else {
            // If no compression, should remain unchanged
            assert_eq!(attachment.filename, original_filename);
            assert_eq!(attachment.content_type, original_content_type);
        }
    }
    
    #[test]
    fn test_compression_ratio_for_gzip_files() {
        let mut attachment = Attachment::new("test.txt.gz", b"compressed data".to_vec());
        attachment.content_type = "application/gzip".to_string();
        attachment.compressed = true;
        
        // Should not attempt further compression
        assert_eq!(AttachmentCompressor::estimate_compression_ratio(&attachment), 1.0);
        
        let text_attachment = Attachment::new("test.txt", b"plain text".to_vec());
        assert_eq!(AttachmentCompressor::estimate_compression_ratio(&text_attachment), 0.3);
    }
}