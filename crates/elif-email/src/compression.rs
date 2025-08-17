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
            
            // Update content type to indicate compression
            if !attachment.content_type.contains("gzip") {
                attachment.content_type = format!("{}; compression=gzip", attachment.content_type);
            }
        }
        
        Ok(())
    }
    
    /// Get compression ratio estimate for an attachment
    pub fn estimate_compression_ratio(attachment: &Attachment) -> f64 {
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
}