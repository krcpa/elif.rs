//! # Compression Middleware
//!
//! Provides response compression using tower-http's battle-tested CompressionLayer.
//! This is an adapter to make it work with the V2 middleware pattern.

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::ElifRequest;
use crate::response::ElifResponse;
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower::{Service, Layer};
use http_body_util::BodyExt;

/// Configuration for compression middleware
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression level
    pub level: CompressionLevel,
    /// Enable gzip compression
    pub enable_gzip: bool,
    /// Enable brotli compression  
    pub enable_brotli: bool,
    /// Enable deflate compression
    pub enable_deflate: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            level: CompressionLevel::default(),
            enable_gzip: true,
            enable_brotli: true,
            enable_deflate: false, // Less common, disabled by default
        }
    }
}

/// Middleware for compressing HTTP responses using tower-http
pub struct CompressionMiddleware {
    layer: CompressionLayer,
}

impl CompressionMiddleware {
    /// Create new compression middleware with default configuration
    pub fn new() -> Self {
        let config = CompressionConfig::default();
        Self::with_config(config)
    }
    
    /// Create compression middleware with custom configuration
    pub fn with_config(config: CompressionConfig) -> Self {
        let mut layer = CompressionLayer::new().quality(config.level);
        
        // Enable/disable compression algorithms based on config
        if !config.enable_gzip {
            layer = layer.no_gzip();
        }
        if !config.enable_brotli {
            layer = layer.no_br();
        }
        if !config.enable_deflate {
            layer = layer.no_deflate();
        }
        
        Self { layer }
    }
    
    /// Set compression level (consuming)
    pub fn level(self, level: CompressionLevel) -> Self {
        Self {
            layer: self.layer.quality(level),
        }
    }
    
    /// Set fast compression (level 1)
    pub fn fast(self) -> Self {
        self.level(CompressionLevel::Fastest)
    }
    
    /// Set best compression (level 9)
    pub fn best(self) -> Self {
        self.level(CompressionLevel::Best)
    }
    
    /// Disable gzip compression
    pub fn no_gzip(self) -> Self {
        Self {
            layer: self.layer.no_gzip(),
        }
    }
    
    /// Disable brotli compression
    pub fn no_brotli(self) -> Self {
        Self {
            layer: self.layer.no_br(),
        }
    }
    
    /// Disable deflate compression
    pub fn no_deflate(self) -> Self {
        Self {
            layer: self.layer.no_deflate(),
        }
    }
    
    /// Enable only gzip compression
    pub fn gzip_only(self) -> Self {
        Self {
            layer: self.layer.no_br().no_deflate(),
        }
    }
    
    /// Enable only brotli compression
    pub fn brotli_only(self) -> Self {
        Self {
            layer: self.layer.no_gzip().no_deflate(),
        }
    }
}

impl std::fmt::Debug for CompressionMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompressionMiddleware")
            .field("layer", &"<CompressionLayer>")
            .finish()
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CompressionMiddleware {
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
        }
    }
}

impl Middleware for CompressionMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let layer = self.layer.clone();
        
        Box::pin(async move {
            // Check if the client accepts compression from the original request
            let accept_encoding = request.header("accept-encoding")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_owned())
                .unwrap_or_default();
            
            let wants_compression = accept_encoding.contains("gzip") || 
                                   accept_encoding.contains("br") || 
                                   accept_encoding.contains("deflate");
            
            // First get the response from the next handler
            let response = next.run(request).await;
            
            if !wants_compression {
                // Client doesn't want compression, return as-is
                return response;
            }
            
            let axum_response = response.into_axum_response();
            let (parts, body) = axum_response.into_parts();
            
            // Read the response body to compress it
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes,
                Err(_) => {
                    // Can't read body, return as-is
                    let response = axum::response::Response::from_parts(parts, axum::body::Body::empty());
                    return ElifResponse::from_axum_response(response).await;
                }
            };
            
            // Store copies for fallback use
            let parts_clone = parts.clone();
            let body_bytes_clone = body_bytes.clone();
            
            // Create a mock request for the compression service
            let mock_request = axum::extract::Request::builder()
                .uri("/")
                .header("accept-encoding", &accept_encoding)
                .body(axum::body::Body::empty())
                .unwrap();
            
            // Create a service that returns our response body
            let service = tower::service_fn(move |_req: axum::extract::Request| {
                let response_parts = parts.clone();
                let response_body = body_bytes.clone();
                async move {
                    let response = axum::response::Response::from_parts(
                        response_parts,
                        axum::body::Body::from(response_body)
                    );
                    Ok::<axum::response::Response, std::convert::Infallible>(response)
                }
            });
            
            // Apply compression layer
            let mut compression_service = layer.layer(service);
            
            // Call the compression service
            match compression_service.call(mock_request).await {
                Ok(compressed_response) => {
                    // Extract the compressed response
                    let (compressed_parts, compressed_body) = compressed_response.into_parts();
                    
                    // Convert CompressionBody to bytes
                    match compressed_body.collect().await {
                        Ok(collected) => {
                            // Get the compressed bytes
                            let compressed_bytes = collected.to_bytes();
                            
                            // Create final response with compressed body
                            let final_response = axum::response::Response::from_parts(
                                compressed_parts,
                                axum::body::Body::from(compressed_bytes)
                            );
                            
                            // Convert back to ElifResponse
                            ElifResponse::from_axum_response(final_response).await
                        }
                        Err(_) => {
                            // Fallback: return original response if compression fails
                            let original_response = axum::response::Response::from_parts(
                                parts_clone, 
                                axum::body::Body::from(body_bytes_clone)
                            );
                            ElifResponse::from_axum_response(original_response).await
                        }
                    }
                }
                Err(_) => {
                    // Fallback: return original response if compression service fails
                    let original_response = axum::response::Response::from_parts(
                        parts_clone, 
                        axum::body::Body::from(body_bytes_clone)
                    );
                    ElifResponse::from_axum_response(original_response).await
                }
            }
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
    fn test_compression_config() {
        let config = CompressionConfig::default();
        assert!(config.enable_gzip);
        assert!(config.enable_brotli);
        assert!(!config.enable_deflate);
    }
    
    #[tokio::test]
    async fn test_compression_middleware() {
        let middleware = CompressionMiddleware::new();
        
        // Create request with accept-encoding
        let mut headers = HeaderMap::new();
        headers.insert("accept-encoding", "gzip, br".parse().unwrap());
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
            .best()           // Maximum compression
            .gzip_only();     // Only gzip
        
        // Test that it builds without errors
        assert_eq!(middleware.name(), "CompressionMiddleware");
    }
    
    #[test]
    fn test_compression_levels() {
        let fast = CompressionMiddleware::new().fast();
        let best = CompressionMiddleware::new().best();
        let custom = CompressionMiddleware::new().level(CompressionLevel::Precise(5));
        
        // All should build without errors
        assert_eq!(fast.name(), "CompressionMiddleware");
        assert_eq!(best.name(), "CompressionMiddleware");
        assert_eq!(custom.name(), "CompressionMiddleware");
    }
    
    #[test]
    fn test_algorithm_selection() {
        let gzip_only = CompressionMiddleware::new().gzip_only();
        let brotli_only = CompressionMiddleware::new().brotli_only();
        let no_brotli = CompressionMiddleware::new().no_brotli();
        
        // All should build without errors
        assert_eq!(gzip_only.name(), "CompressionMiddleware");
        assert_eq!(brotli_only.name(), "CompressionMiddleware");
        assert_eq!(no_brotli.name(), "CompressionMiddleware");
    }
    
    #[test]
    fn test_clone() {
        let middleware = CompressionMiddleware::new().best();
        let cloned = middleware.clone();
        
        assert_eq!(cloned.name(), "CompressionMiddleware");
    }
}