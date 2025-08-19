//! # Compression Middleware
//!
//! Provides response compression using gzip and brotli algorithms.

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::ElifRequest;
use crate::response::ElifResponse;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::io::Write;

/// Compression algorithm type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionAlgorithm {
    /// Gzip compression (RFC 1952)
    Gzip,
    /// Brotli compression (RFC 7932) 
    Brotli,
    /// No compression
    Identity,
}

impl CompressionAlgorithm {
    /// Parse compression algorithm from Accept-Encoding header
    pub fn from_accept_encoding(accept_encoding: &str) -> Vec<Self> {
        let mut algorithms = Vec::new();
        
        // Parse Accept-Encoding header (e.g., "gzip, deflate, br")
        for encoding in accept_encoding.split(',') {
            let encoding = encoding.trim().to_lowercase();
            match encoding.as_str() {
                "gzip" => algorithms.push(Self::Gzip),
                "br" => algorithms.push(Self::Brotli),
                "identity" => algorithms.push(Self::Identity),
                _ => continue,
            }
        }
        
        // Default to identity if no supported encodings found
        if algorithms.is_empty() {
            algorithms.push(Self::Identity);
        }
        
        algorithms
    }
    
    /// Get the encoding name for response headers
    pub fn encoding_name(&self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Brotli => "br",
            Self::Identity => "identity",
        }
    }
}

/// Configuration for compression middleware
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Minimum response size to compress (in bytes)
    pub min_size: usize,
    /// Maximum response size to compress (in bytes)
    pub max_size: usize,
    /// Compression level (1-9 for gzip, 1-11 for brotli)
    pub level: u32,
    /// Content types to compress
    pub content_types: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 1024,  // 1KB minimum
            max_size: 10 * 1024 * 1024,  // 10MB maximum
            level: 6,  // Balanced compression level
            content_types: vec![
                "text/html".to_string(),
                "text/css".to_string(),
                "text/javascript".to_string(),
                "text/plain".to_string(),
                "application/json".to_string(),
                "application/javascript".to_string(),
                "application/xml".to_string(),
                "text/xml".to_string(),
                "image/svg+xml".to_string(),
            ],
        }
    }
}

/// Middleware for compressing HTTP responses
#[derive(Debug)]
pub struct CompressionMiddleware {
    config: CompressionConfig,
}

impl CompressionMiddleware {
    /// Create new compression middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: CompressionConfig::default(),
        }
    }
    
    /// Create compression middleware with custom configuration
    pub fn with_config(config: CompressionConfig) -> Self {
        Self { config }
    }
    
    /// Set minimum size for compression
    pub fn min_size(mut self, min_size: usize) -> Self {
        self.config.min_size = min_size;
        self
    }
    
    /// Set maximum size for compression
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.config.max_size = max_size;
        self
    }
    
    /// Set compression level
    pub fn level(mut self, level: u32) -> Self {
        self.config.level = level.clamp(1, 11); // Clamp to valid range
        self
    }
    
    /// Add content type to compress
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.config.content_types.push(content_type.into());
        self
    }
    
    /// Determine if response should be compressed
    fn should_compress(&self, headers: &HeaderMap, body_size: usize) -> bool {
        // Check size limits
        if body_size < self.config.min_size || body_size > self.config.max_size {
            return false;
        }
        
        // Check if already compressed
        if headers.contains_key("content-encoding") {
            return false;
        }
        
        // Check content type
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                let content_type_lower = content_type_str.to_lowercase();
                return self.config.content_types.iter().any(|ct| {
                    content_type_lower.starts_with(&ct.to_lowercase())
                });
            }
        }
        
        false
    }
    
    /// Choose best compression algorithm based on client support
    fn choose_algorithm(&self, accept_encoding: Option<&HeaderValue>) -> CompressionAlgorithm {
        if let Some(accept_encoding) = accept_encoding {
            if let Ok(accept_str) = accept_encoding.to_str() {
                let supported = CompressionAlgorithm::from_accept_encoding(accept_str);
                
                // Prefer brotli if supported (better compression)
                if supported.contains(&CompressionAlgorithm::Brotli) {
                    return CompressionAlgorithm::Brotli;
                }
                
                // Fall back to gzip
                if supported.contains(&CompressionAlgorithm::Gzip) {
                    return CompressionAlgorithm::Gzip;
                }
            }
        }
        
        CompressionAlgorithm::Identity
    }
    
    /// Compress data using gzip
    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.config.level));
        encoder.write_all(data)?;
        encoder.finish()
    }
    
    /// Compress data using brotli
    fn compress_brotli(&self, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        use brotli::enc::BrotliEncoderParams;
        use std::io::Cursor;
        
        let params = BrotliEncoderParams {
            quality: self.config.level.min(11) as i32,
            ..Default::default()
        };
        
        let mut output = Vec::new();
        let mut cursor = Cursor::new(data);
        
        brotli::BrotliCompress(&mut cursor, &mut output, &params)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        Ok(output)
    }
    
    /// Compress response body
    async fn compress_response(&self, mut response: ElifResponse, algorithm: CompressionAlgorithm) -> ElifResponse {
        if algorithm == CompressionAlgorithm::Identity {
            return response;
        }
        
        // Get response body - this is a limitation mentioned in CLAUDE.md
        // For now, we'll work with what's available
        let axum_response = response.into_axum_response();
        let (parts, body) = axum_response.into_parts();
        
        // Collect body bytes
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => {
                // If we can't read the body, return as-is
                let response = axum::response::Response::from_parts(parts, axum::body::Body::empty());
                return ElifResponse::from_axum_response(response).await;
            }
        };
        
        // Check if we should compress
        if !self.should_compress(&parts.headers, body_bytes.len()) {
            let response = axum::response::Response::from_parts(parts, axum::body::Body::from(body_bytes));
            return ElifResponse::from_axum_response(response).await;
        }
        
        // Compress the data
        let compressed_data = match algorithm {
            CompressionAlgorithm::Gzip => {
                match self.compress_gzip(&body_bytes) {
                    Ok(data) => data,
                    Err(_) => {
                        // Compression failed, return original
                        let response = axum::response::Response::from_parts(parts, axum::body::Body::from(body_bytes));
                        return ElifResponse::from_axum_response(response).await;
                    }
                }
            }
            CompressionAlgorithm::Brotli => {
                match self.compress_brotli(&body_bytes) {
                    Ok(data) => data,
                    Err(_) => {
                        // Compression failed, return original
                        let response = axum::response::Response::from_parts(parts, axum::body::Body::from(body_bytes));
                        return ElifResponse::from_axum_response(response).await;
                    }
                }
            }
            CompressionAlgorithm::Identity => body_bytes.to_vec(),
        };
        
        // Build new response with compressed body and headers
        let mut new_parts = parts;
        
        // Update headers
        new_parts.headers.insert(
            HeaderName::from_static("content-encoding"),
            HeaderValue::from_static(algorithm.encoding_name()),
        );
        
        // Update content length
        new_parts.headers.insert(
            HeaderName::from_static("content-length"),
            HeaderValue::try_from(compressed_data.len().to_string()).unwrap(),
        );
        
        // Add vary header
        new_parts.headers.insert(
            HeaderName::from_static("vary"),
            HeaderValue::from_static("Accept-Encoding"),
        );
        
        let response = axum::response::Response::from_parts(
            new_parts,
            axum::body::Body::from(compressed_data),
        );
        
        ElifResponse::from_axum_response(response).await
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CompressionMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        // Get Accept-Encoding header
        let accept_encoding = request.header("accept-encoding").cloned();
        let algorithm = self.choose_algorithm(accept_encoding.as_ref());
        
        let middleware = CompressionMiddleware {
            config: self.config.clone(),
        };
        
        Box::pin(async move {
            // Execute next middleware/handler
            let response = next.run(request).await;
            
            // Compress response if needed
            middleware.compress_response(response, algorithm).await
        })
    }
    
    fn name(&self) -> &'static str {
        "CompressionMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifResponse;
    use axum::http::{HeaderMap, Method, StatusCode};
    use crate::request::ElifRequest;
    
    #[test]
    fn test_compression_algorithm_parsing() {
        let algorithms = CompressionAlgorithm::from_accept_encoding("gzip, deflate, br");
        assert!(algorithms.contains(&CompressionAlgorithm::Gzip));
        assert!(algorithms.contains(&CompressionAlgorithm::Brotli));
        
        let algorithms = CompressionAlgorithm::from_accept_encoding("br;q=1.0, gzip;q=0.8");
        assert!(algorithms.contains(&CompressionAlgorithm::Brotli));
        assert!(algorithms.contains(&CompressionAlgorithm::Gzip));
        
        let algorithms = CompressionAlgorithm::from_accept_encoding("identity");
        assert_eq!(algorithms, vec![CompressionAlgorithm::Identity]);
    }
    
    #[test]
    fn test_compression_config() {
        let config = CompressionConfig::default();
        assert_eq!(config.min_size, 1024);
        assert_eq!(config.max_size, 10 * 1024 * 1024);
        assert_eq!(config.level, 6);
        assert!(config.content_types.contains(&"application/json".to_string()));
    }
    
    #[test]
    fn test_should_compress() {
        let middleware = CompressionMiddleware::new();
        
        // Test with JSON content type and appropriate size
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        assert!(middleware.should_compress(&headers, 2048)); // 2KB
        
        // Test with too small size
        assert!(!middleware.should_compress(&headers, 512)); // 512B
        
        // Test with unsupported content type
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "image/jpeg".parse().unwrap());
        assert!(!middleware.should_compress(&headers, 2048));
        
        // Test with existing compression
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("content-encoding", "gzip".parse().unwrap());
        assert!(!middleware.should_compress(&headers, 2048));
    }
    
    #[test]
    fn test_algorithm_selection() {
        let middleware = CompressionMiddleware::new();
        
        // Test brotli preference
        let header = HeaderValue::from_static("gzip, br");
        assert_eq!(
            middleware.choose_algorithm(Some(&header)),
            CompressionAlgorithm::Brotli
        );
        
        // Test gzip fallback
        let header = HeaderValue::from_static("gzip");
        assert_eq!(
            middleware.choose_algorithm(Some(&header)),
            CompressionAlgorithm::Gzip
        );
        
        // Test no compression
        let header = HeaderValue::from_static("identity");
        assert_eq!(
            middleware.choose_algorithm(Some(&header)),
            CompressionAlgorithm::Identity
        );
    }
    
    #[tokio::test]
    async fn test_compression_middleware() {
        let middleware = CompressionMiddleware::new();
        
        // Create request with accept-encoding
        let mut headers = HeaderMap::new();
        headers.insert("accept-encoding", "gzip".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        // Create next handler that returns JSON response
        let next = Next::new(|_req| {
            Box::pin(async move {
                let json_data = serde_json::json!({
                    "message": "Hello, World!".repeat(100), // Make it large enough to compress
                    "data": (0..100).collect::<Vec<i32>>()
                });
                ElifResponse::ok().json_value(json_data)
            })
        });
        
        // Execute middleware
        let response = middleware.handle(request, next).await;
        
        // Response should be successful
        assert_eq!(response.status_code(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_compression_builder_pattern() {
        let middleware = CompressionMiddleware::new()
            .min_size(2048)
            .max_size(5 * 1024 * 1024)
            .level(9)
            .content_type("application/xml");
        
        assert_eq!(middleware.config.min_size, 2048);
        assert_eq!(middleware.config.max_size, 5 * 1024 * 1024);
        assert_eq!(middleware.config.level, 9);
        assert!(middleware.config.content_types.contains(&"application/xml".to_string()));
    }
    
    #[test] 
    fn test_gzip_compression() {
        let middleware = CompressionMiddleware::new();
        let test_data = b"Hello, World! This is a test string for compression.".repeat(10);
        
        let compressed = middleware.compress_gzip(&test_data).unwrap();
        assert!(compressed.len() < test_data.len());
        assert!(!compressed.is_empty());
    }
    
    #[test]
    fn test_brotli_compression() {
        let middleware = CompressionMiddleware::new();
        let test_data = b"Hello, World! This is a test string for compression.".repeat(10);
        
        let compressed = middleware.compress_brotli(&test_data).unwrap();
        assert!(compressed.len() < test_data.len());
        assert!(!compressed.is_empty());
    }
}