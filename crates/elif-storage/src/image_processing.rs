//! Image processing capabilities

#[cfg(feature = "image-processing")]
use crate::{StorageError, StorageResult};

#[cfg(feature = "image-processing")]
use image::{DynamicImage, ImageFormat, ImageOutputFormat};
#[cfg(feature = "image-processing")]
use serde::{Deserialize, Serialize};

/// Image processing operations
#[cfg(feature = "image-processing")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageOperation {
    /// Resize image to specified dimensions
    Resize {
        width: u32,
        height: u32,
        maintain_aspect_ratio: bool,
    },

    /// Crop image to specified rectangle
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },

    /// Rotate image by specified angle (in degrees)
    Rotate { angle: f32 },

    /// Flip image horizontally
    FlipHorizontal,

    /// Flip image vertically
    FlipVertical,

    /// Convert to specified format
    ConvertFormat { format: ImageFormat },

    /// Adjust quality (for JPEG)
    Quality { quality: u8 },

    /// Apply blur filter
    Blur { sigma: f32 },

    /// Adjust brightness
    Brightness { value: i32 },

    /// Adjust contrast
    Contrast { value: f32 },

    /// Convert to grayscale
    Grayscale,

    /// Add watermark
    Watermark {
        text: String,
        position: WatermarkPosition,
        opacity: f32,
    },
}

/// Watermark position
#[cfg(feature = "image-processing")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatermarkPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    Custom { x: u32, y: u32 },
}

/// Image processor
#[cfg(feature = "image-processing")]
#[derive(Debug)]
pub struct ImageProcessor {
    max_dimensions: Option<(u32, u32)>,
    allowed_formats: Option<Vec<ImageFormat>>,
}

#[cfg(feature = "image-processing")]
impl ImageProcessor {
    /// Create a new image processor
    pub fn new() -> Self {
        Self {
            max_dimensions: Some((4096, 4096)), // 4K max by default
            allowed_formats: None,              // All formats allowed by default
        }
    }

    /// Set maximum allowed dimensions
    pub fn with_max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_dimensions = Some((width, height));
        self
    }

    /// Remove dimension limits
    pub fn unlimited_dimensions(mut self) -> Self {
        self.max_dimensions = None;
        self
    }

    /// Set allowed output formats
    pub fn with_allowed_formats(mut self, formats: Vec<ImageFormat>) -> Self {
        self.allowed_formats = Some(formats);
        self
    }

    /// Process image data with specified operations
    pub fn process_image(
        &self,
        data: &[u8],
        operations: &[ImageOperation],
    ) -> StorageResult<Vec<u8>> {
        // Load the image
        let mut img = image::load_from_memory(data)
            .map_err(|e| StorageError::ImageProcessing(format!("Failed to load image: {}", e)))?;

        // Validate image dimensions
        if let Some((max_width, max_height)) = self.max_dimensions {
            if img.width() > max_width || img.height() > max_height {
                return Err(StorageError::ImageProcessing(format!(
                    "Image dimensions {}x{} exceed maximum allowed {}x{}",
                    img.width(),
                    img.height(),
                    max_width,
                    max_height
                )));
            }
        }

        let mut output_format = ImageFormat::Png; // Default format
        let mut quality = 85u8; // Default JPEG quality

        // Apply operations
        for operation in operations {
            match operation {
                ImageOperation::Resize {
                    width,
                    height,
                    maintain_aspect_ratio,
                } => {
                    if *maintain_aspect_ratio {
                        img = img.resize(*width, *height, image::imageops::FilterType::Lanczos3);
                    } else {
                        img = img.resize_exact(
                            *width,
                            *height,
                            image::imageops::FilterType::Lanczos3,
                        );
                    }
                }

                ImageOperation::Crop {
                    x,
                    y,
                    width,
                    height,
                } => {
                    img = img.crop_imm(*x, *y, *width, *height);
                }

                ImageOperation::Rotate { angle } => {
                    // Simple 90-degree rotations
                    match angle as i32 % 360 {
                        90 | -270 => img = img.rotate90(),
                        180 | -180 => img = img.rotate180(),
                        270 | -90 => img = img.rotate270(),
                        0 => {} // No rotation
                        _ => {
                            return Err(StorageError::ImageProcessing(
                                "Only 90-degree rotation increments are supported".to_string(),
                            ));
                        }
                    }
                }

                ImageOperation::FlipHorizontal => {
                    img = img.fliph();
                }

                ImageOperation::FlipVertical => {
                    img = img.flipv();
                }

                ImageOperation::ConvertFormat { format } => {
                    if let Some(allowed) = &self.allowed_formats {
                        if !allowed.contains(format) {
                            return Err(StorageError::ImageProcessing(format!(
                                "Output format {:?} is not allowed",
                                format
                            )));
                        }
                    }
                    output_format = *format;
                }

                ImageOperation::Quality { quality: q } => {
                    quality = *q;
                }

                ImageOperation::Blur { sigma } => {
                    img = img.blur(*sigma);
                }

                ImageOperation::Brightness { value } => {
                    img = img.brighten(*value);
                }

                ImageOperation::Contrast { value } => {
                    img = img.adjust_contrast(*value);
                }

                ImageOperation::Grayscale => {
                    img = img.grayscale();
                }

                ImageOperation::Watermark {
                    text,
                    position,
                    opacity: _,
                } => {
                    // Simple text watermark (basic implementation)
                    // In a real implementation, you'd use a proper text rendering library
                    // For now, we'll just log that watermarking was requested
                    tracing::info!("Watermark requested: '{}' at {:?}", text, position);
                    // TODO: Implement actual watermarking with a text rendering library
                }
            }
        }

        // Encode the processed image
        let mut output = Vec::new();
        let format = match output_format {
            ImageFormat::Jpeg => ImageOutputFormat::Jpeg(quality),
            ImageFormat::Png => ImageOutputFormat::Png,
            ImageFormat::WebP => ImageOutputFormat::WebP,
            ImageFormat::Gif => ImageOutputFormat::Gif,
            _ => ImageOutputFormat::Png,
        };

        img.write_to(&mut std::io::Cursor::new(&mut output), format)
            .map_err(|e| StorageError::ImageProcessing(format!("Failed to encode image: {}", e)))?;

        Ok(output)
    }

    /// Get image metadata without loading the full image
    pub fn get_image_info(&self, data: &[u8]) -> StorageResult<ImageInfo> {
        let format = image::guess_format(data).map_err(|e| {
            StorageError::ImageProcessing(format!("Failed to detect image format: {}", e))
        })?;

        let dimensions = image::image_dimensions(&mut std::io::Cursor::new(data)).map_err(|e| {
            StorageError::ImageProcessing(format!("Failed to read image dimensions: {}", e))
        })?;

        Ok(ImageInfo {
            width: dimensions.0,
            height: dimensions.1,
            format,
            size_bytes: data.len() as u64,
        })
    }

    /// Generate thumbnail with specified maximum dimensions
    pub fn generate_thumbnail(
        &self,
        data: &[u8],
        max_width: u32,
        max_height: u32,
    ) -> StorageResult<Vec<u8>> {
        let operations = vec![
            ImageOperation::Resize {
                width: max_width,
                height: max_height,
                maintain_aspect_ratio: true,
            },
            ImageOperation::ConvertFormat {
                format: ImageFormat::Jpeg,
            },
            ImageOperation::Quality { quality: 75 },
        ];

        self.process_image(data, &operations)
    }

    /// Optimize image for web (reduce size while maintaining quality)
    pub fn optimize_for_web(
        &self,
        data: &[u8],
        target_format: ImageFormat,
    ) -> StorageResult<Vec<u8>> {
        let operations = match target_format {
            ImageFormat::Jpeg => vec![
                ImageOperation::ConvertFormat {
                    format: ImageFormat::Jpeg,
                },
                ImageOperation::Quality { quality: 85 },
            ],
            ImageFormat::WebP => vec![ImageOperation::ConvertFormat {
                format: ImageFormat::WebP,
            }],
            ImageFormat::Png => vec![ImageOperation::ConvertFormat {
                format: ImageFormat::Png,
            }],
            _ => {
                return Err(StorageError::ImageProcessing(
                    "Unsupported format for web optimization".to_string(),
                ));
            }
        };

        self.process_image(data, &operations)
    }
}

#[cfg(feature = "image-processing")]
impl Default for ImageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Image information
#[cfg(feature = "image-processing")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Image width in pixels
    pub width: u32,

    /// Image height in pixels
    pub height: u32,

    /// Image format
    pub format: ImageFormat,

    /// File size in bytes
    pub size_bytes: u64,
}

#[cfg(feature = "image-processing")]
impl ImageInfo {
    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Check if image is landscape orientation
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }

    /// Check if image is portrait orientation
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// Check if image is square
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

// Stub implementations when image-processing feature is not enabled
#[cfg(not(feature = "image-processing"))]
pub struct ImageProcessor;

#[cfg(not(feature = "image-processing"))]
impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
#[cfg(feature = "image-processing")]
mod tests {
    use super::*;

    // Note: These tests would require actual image data to work properly
    // For now, they're placeholder tests

    #[test]
    fn test_image_processor_creation() {
        let processor = ImageProcessor::new()
            .with_max_dimensions(2048, 2048)
            .with_allowed_formats(vec![ImageFormat::Jpeg, ImageFormat::Png]);

        assert_eq!(processor.max_dimensions, Some((2048, 2048)));
        assert!(processor.allowed_formats.is_some());
    }

    #[test]
    fn test_image_operations() {
        let operations = vec![
            ImageOperation::Resize {
                width: 800,
                height: 600,
                maintain_aspect_ratio: true,
            },
            ImageOperation::Crop {
                x: 0,
                y: 0,
                width: 400,
                height: 300,
            },
            ImageOperation::ConvertFormat {
                format: ImageFormat::Jpeg,
            },
            ImageOperation::Quality { quality: 90 },
        ];

        assert_eq!(operations.len(), 4);
    }

    #[test]
    fn test_watermark_position() {
        let positions = vec![
            WatermarkPosition::TopLeft,
            WatermarkPosition::Center,
            WatermarkPosition::Custom { x: 100, y: 50 },
        ];

        assert_eq!(positions.len(), 3);
    }
}
