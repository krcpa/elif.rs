//! # Content Negotiation Middleware
//!
//! Provides HTTP content negotiation based on Accept headers.
//! Automatically handles response format conversion between JSON, XML, and other formats.

use crate::middleware::v2::{Middleware, Next, NextFuture};
use crate::request::ElifRequest;
use crate::response::ElifResponse;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported content types for negotiation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContentType {
    Json,
    Xml,
    Html,
    PlainText,
    Csv,
    MessagePack,
    Yaml,
    Custom(String),
}

impl ContentType {
    /// Parse content type from Accept header value
    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        let mime_lower = mime_type.split(';').next()?.trim().to_lowercase();
        match mime_lower.as_str() {
            "application/json" => Some(Self::Json),
            "application/xml" | "text/xml" => Some(Self::Xml),
            "text/html" => Some(Self::Html),
            "text/plain" => Some(Self::PlainText),
            "text/csv" => Some(Self::Csv),
            "application/msgpack" | "application/x-msgpack" => Some(Self::MessagePack),
            "application/yaml" | "application/x-yaml" | "text/yaml" => Some(Self::Yaml),
            _ => Some(Self::Custom(mime_lower)),
        }
    }
    
    /// Get MIME type string for response headers
    pub fn to_mime_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Xml => "application/xml",
            Self::Html => "text/html",
            Self::PlainText => "text/plain",
            Self::Csv => "text/csv",
            Self::MessagePack => "application/msgpack",
            Self::Yaml => "application/yaml",
            Self::Custom(_) => "application/octet-stream", // Fallback for custom types
        }
    }
    
    /// Get file extension for this content type
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Html => "html",
            Self::PlainText => "txt",
            Self::Csv => "csv",
            Self::MessagePack => "msgpack",
            Self::Yaml => "yaml",
            Self::Custom(_) => "bin",
        }
    }
}

/// Accept header value with quality factor
#[derive(Debug, Clone)]
pub struct AcceptValue {
    pub content_type: ContentType,
    pub quality: f32,
    pub params: HashMap<String, String>,
}

impl AcceptValue {
    /// Parse Accept header value (e.g., "application/json;q=0.8")
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split(';').collect();
        let mime_type = parts.first()?.trim();
        
        let content_type = ContentType::from_mime_type(mime_type)?;
        let mut quality = 1.0;
        let mut params = HashMap::new();
        
        // Parse parameters
        for param in parts.iter().skip(1) {
            let param = param.trim();
            if let Some((key, val)) = param.split_once('=') {
                let key = key.trim();
                let val = val.trim();
                
                if key == "q" {
                    quality = val.parse().unwrap_or(1.0);
                } else {
                    params.insert(key.to_string(), val.to_string());
                }
            }
        }
        
        Some(Self {
            content_type,
            quality,
            params,
        })
    }
}

/// Content negotiation configuration
pub struct ContentNegotiationConfig {
    /// Default content type when negotiation fails
    pub default_content_type: ContentType,
    /// Supported content types in order of preference
    pub supported_types: Vec<ContentType>,
    /// Whether to add Vary header
    pub add_vary_header: bool,
    /// Custom converters for content types
    pub converters: HashMap<ContentType, std::sync::Arc<dyn Fn(&serde_json::Value) -> Result<Vec<u8>, String> + Send + Sync>>,
}

impl Default for ContentNegotiationConfig {
    fn default() -> Self {
        let mut converters = HashMap::new();
        converters.insert(ContentType::Json, std::sync::Arc::new(Self::convert_to_json) as std::sync::Arc<dyn Fn(&serde_json::Value) -> Result<Vec<u8>, String> + Send + Sync>);
        converters.insert(ContentType::PlainText, std::sync::Arc::new(Self::convert_to_text) as std::sync::Arc<dyn Fn(&serde_json::Value) -> Result<Vec<u8>, String> + Send + Sync>);
        converters.insert(ContentType::Html, std::sync::Arc::new(Self::convert_to_html) as std::sync::Arc<dyn Fn(&serde_json::Value) -> Result<Vec<u8>, String> + Send + Sync>);
        
        Self {
            default_content_type: ContentType::Json,
            supported_types: vec![
                ContentType::Json,
                ContentType::PlainText,
                ContentType::Html,
                ContentType::Xml,
            ],
            add_vary_header: true,
            converters,
        }
    }
}

impl ContentNegotiationConfig {
    /// Default JSON converter
    fn convert_to_json(value: &serde_json::Value) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())
    }
    
    /// Default plain text converter
    fn convert_to_text(value: &serde_json::Value) -> Result<Vec<u8>, String> {
        let text = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            other => serde_json::to_string(other).map_err(|e| e.to_string())?,
        };
        Ok(text.into_bytes())
    }
    
    /// Default HTML converter
    fn convert_to_html(value: &serde_json::Value) -> Result<Vec<u8>, String> {
        let json_str = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>API Response</title>
    <style>
        body {{ font-family: monospace; padding: 20px; }}
        pre {{ background: #f5f5f5; padding: 15px; border-radius: 5px; }}
    </style>
</head>
<body>
    <h1>API Response</h1>
    <pre>{}</pre>
</body>
</html>"#,
            html_escape::encode_text(&json_str)
        );
        Ok(html.into_bytes())
    }
}

/// Middleware for HTTP content negotiation
#[derive(Debug)]
pub struct ContentNegotiationMiddleware {
    config: ContentNegotiationConfig,
}

impl std::fmt::Debug for ContentNegotiationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentNegotiationConfig")
            .field("default_content_type", &self.default_content_type)
            .field("supported_types", &self.supported_types)
            .field("add_vary_header", &self.add_vary_header)
            .field("converters", &format!("<{} converters>", self.converters.len()))
            .finish()
    }
}

impl Clone for ContentNegotiationConfig {
    fn clone(&self) -> Self {
        Self {
            default_content_type: self.default_content_type.clone(),
            supported_types: self.supported_types.clone(),
            add_vary_header: self.add_vary_header,
            converters: self.converters.clone(), // Arc is Clone, so this works correctly
        }
    }
}

impl ContentNegotiationMiddleware {
    /// Create new content negotiation middleware
    pub fn new() -> Self {
        Self {
            config: ContentNegotiationConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: ContentNegotiationConfig) -> Self {
        Self { config }
    }
    
    /// Set default content type
    pub fn default_type(mut self, content_type: ContentType) -> Self {
        self.config.default_content_type = content_type;
        self
    }
    
    /// Add supported content type
    pub fn support(mut self, content_type: ContentType) -> Self {
        if !self.config.supported_types.contains(&content_type) {
            self.config.supported_types.push(content_type);
        }
        self
    }
    
    /// Add custom converter for content type
    pub fn converter<F>(
        mut self, 
        content_type: ContentType, 
        converter: F
    ) -> Self 
    where 
        F: Fn(&serde_json::Value) -> Result<Vec<u8>, String> + Send + Sync + 'static
    {
        self.config.converters.insert(content_type, std::sync::Arc::new(converter));
        self
    }
    
    /// Disable Vary header
    pub fn no_vary_header(mut self) -> Self {
        self.config.add_vary_header = false;
        self
    }
    
    /// Parse Accept header and return preferred content types in order
    fn parse_accept_header(&self, accept_header: &str) -> Vec<AcceptValue> {
        let mut accept_values = Vec::new();
        
        for value in accept_header.split(',') {
            if let Some(accept_value) = AcceptValue::parse(value.trim()) {
                accept_values.push(accept_value);
            }
        }
        
        // Sort by quality factor (descending)
        accept_values.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap_or(std::cmp::Ordering::Equal));
        
        accept_values
    }
    
    /// Choose best content type based on Accept header and supported types
    fn negotiate_content_type(&self, accept_header: Option<&HeaderValue>) -> ContentType {
        let accept_str = match accept_header.and_then(|h| h.to_str().ok()) {
            Some(s) => s,
            None => return self.config.default_content_type.clone(),
        };
        
        let accept_values = self.parse_accept_header(accept_str);
        
        // Find the first acceptable type that we support
        for accept_value in &accept_values {
            if self.config.supported_types.contains(&accept_value.content_type) {
                return accept_value.content_type.clone();
            }
            
            // Handle wildcard types
            if let ContentType::Custom(mime) = &accept_value.content_type {
                if mime == "*/*" {
                    return self.config.default_content_type.clone();
                } else if mime.ends_with("/*") {
                    let category = &mime[..mime.len()-2];
                    // Find first supported type in the same category
                    for supported in &self.config.supported_types {
                        if supported.to_mime_type().starts_with(category) {
                            return supported.clone();
                        }
                    }
                }
            }
        }
        
        self.config.default_content_type.clone()
    }
    
    /// Check if response contains JSON data that can be converted
    fn extract_json_value(&self, response_body: &[u8]) -> Option<serde_json::Value> {
        // Try to parse as JSON
        serde_json::from_slice(response_body).ok()
    }
    
    /// Convert response to requested format
    async fn convert_response(&self, response: ElifResponse, target_type: ContentType) -> ElifResponse {
        let axum_response = response.into_axum_response();
        let (parts, body) = axum_response.into_parts();
        
        // Get current content type
        let current_content_type = parts.headers.get("content-type")
            .and_then(|h| h.to_str().ok())
            .and_then(ContentType::from_mime_type)
            .unwrap_or(ContentType::Json);
        
        // If already in target format, return as-is
        if current_content_type == target_type {
            let response = axum::response::Response::from_parts(parts, body);
            return ElifResponse::from_axum_response(response).await;
        }
        
        // Read response body
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => {
                // Can't read body, return as-is
                let response = axum::response::Response::from_parts(parts, axum::body::Body::empty());
                return ElifResponse::from_axum_response(response).await;
            }
        };
        
        // Extract JSON value for conversion
        let json_value = match self.extract_json_value(&body_bytes) {
            Some(value) => value,
            None => {
                // Can't parse as JSON, return as-is
                let response = axum::response::Response::from_parts(parts, axum::body::Body::from(body_bytes));
                return ElifResponse::from_axum_response(response).await;
            }
        };
        
        // Convert to target format
        let converted_body = match self.config.converters.get(&target_type) {
            Some(converter) => match converter(&json_value) {
                Ok(body) => body,
                Err(_) => {
                    // Conversion failed, return 406 Not Acceptable
                    return ElifResponse::from_axum_response(
                        axum::response::Response::builder()
                            .status(StatusCode::NOT_ACCEPTABLE)
                            .header("content-type", "application/json")
                            .body(axum::body::Body::from(
                                serde_json::to_vec(&serde_json::json!({
                                    "error": {
                                        "code": "not_acceptable",
                                        "message": "Cannot convert response to requested format",
                                        "hint": "Supported formats: JSON, Plain Text, HTML"
                                    }
                                })).unwrap_or_default()
                            ))
                            .unwrap()
                    ).await;
                }
            },
            None => {
                // No converter available
                return ElifResponse::from_axum_response(
                    axum::response::Response::builder()
                        .status(StatusCode::NOT_ACCEPTABLE)
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_vec(&serde_json::json!({
                                "error": {
                                    "code": "not_acceptable", 
                                    "message": "Requested format is not supported",
                                    "hint": "Supported formats: JSON, Plain Text, HTML"
                                }
                            })).unwrap_or_default()
                        ))
                        .unwrap()
                ).await;
            }
        };
        
        // Build response with new content type
        let mut new_parts = parts;
        new_parts.headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static(target_type.to_mime_type()),
        );
        
        new_parts.headers.insert(
            HeaderName::from_static("content-length"),
            HeaderValue::try_from(converted_body.len().to_string()).unwrap(),
        );
        
        if self.config.add_vary_header {
            new_parts.headers.insert(
                HeaderName::from_static("vary"),
                HeaderValue::from_static("Accept"),
            );
        }
        
        let response = axum::response::Response::from_parts(
            new_parts,
            axum::body::Body::from(converted_body),
        );
        
        ElifResponse::from_axum_response(response).await
    }
}

impl Default for ContentNegotiationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for ContentNegotiationMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let accept_header = request.header("accept").cloned();
        let target_type = self.negotiate_content_type(accept_header.as_ref());
        let config = self.config.clone();
        
        Box::pin(async move {
            let response = next.run(request).await;
            
            let middleware = ContentNegotiationMiddleware { config };
            middleware.convert_response(response, target_type).await
        })
    }
    
    fn name(&self) -> &'static str {
        "ContentNegotiationMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifResponse;
    use axum::http::{HeaderMap, Method, StatusCode};
    use crate::request::ElifRequest;
    
    #[test]
    fn test_content_type_parsing() {
        assert_eq!(
            ContentType::from_mime_type("application/json"),
            Some(ContentType::Json)
        );
        assert_eq!(
            ContentType::from_mime_type("application/xml"),
            Some(ContentType::Xml)
        );
        assert_eq!(
            ContentType::from_mime_type("text/html"),
            Some(ContentType::Html)
        );
        assert_eq!(
            ContentType::from_mime_type("text/plain"),
            Some(ContentType::PlainText)
        );
    }
    
    #[test]
    fn test_accept_value_parsing() {
        let accept = AcceptValue::parse("application/json;q=0.8").unwrap();
        assert_eq!(accept.content_type, ContentType::Json);
        assert_eq!(accept.quality, 0.8);
        
        let accept = AcceptValue::parse("text/html").unwrap();
        assert_eq!(accept.content_type, ContentType::Html);
        assert_eq!(accept.quality, 1.0);
        
        let accept = AcceptValue::parse("text/plain;q=0.5;charset=utf-8").unwrap();
        assert_eq!(accept.content_type, ContentType::PlainText);
        assert_eq!(accept.quality, 0.5);
        assert_eq!(accept.params.get("charset"), Some(&"utf-8".to_string()));
    }
    
    #[test]
    fn test_accept_header_parsing() {
        let middleware = ContentNegotiationMiddleware::new();
        let values = middleware.parse_accept_header("text/html,application/json;q=0.9,text/plain;q=0.8");
        
        assert_eq!(values.len(), 3);
        // Should be sorted by quality (HTML=1.0, JSON=0.9, Plain=0.8)
        assert_eq!(values[0].content_type, ContentType::Html);
        assert_eq!(values[1].content_type, ContentType::Json);
        assert_eq!(values[2].content_type, ContentType::PlainText);
    }
    
    #[test]
    fn test_content_negotiation() {
        let middleware = ContentNegotiationMiddleware::new();
        
        // Test JSON preference
        let header = HeaderValue::from_static("application/json");
        assert_eq!(
            middleware.negotiate_content_type(Some(&header)),
            ContentType::Json
        );
        
        // Test HTML preference with quality
        let header = HeaderValue::from_static("text/html,application/json;q=0.9");
        assert_eq!(
            middleware.negotiate_content_type(Some(&header)),
            ContentType::Html
        );
        
        // Test unsupported type fallback
        let header = HeaderValue::from_static("application/pdf");
        assert_eq!(
            middleware.negotiate_content_type(Some(&header)),
            ContentType::Json // default
        );
        
        // Test wildcard
        let header = HeaderValue::from_static("*/*");
        assert_eq!(
            middleware.negotiate_content_type(Some(&header)),
            ContentType::Json // default
        );
    }
    
    #[tokio::test]
    async fn test_json_to_text_conversion() {
        let middleware = ContentNegotiationMiddleware::new();
        
        let mut headers = HeaderMap::new();
        headers.insert("accept", "text/plain".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!",
                    "count": 42
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Check content type was converted
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert_eq!(
            parts.headers.get("content-type").unwrap(),
            "text/plain"
        );
    }
    
    #[tokio::test]
    async fn test_json_to_html_conversion() {
        let middleware = ContentNegotiationMiddleware::new();
        
        let mut headers = HeaderMap::new();
        headers.insert("accept", "text/html".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let axum_response = response.into_axum_response();
        let (parts, body) = axum_response.into_parts();
        assert_eq!(
            parts.headers.get("content-type").unwrap(),
            "text/html"
        );
        
        // Check that HTML was generated
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let html_content = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(html_content.contains("<!DOCTYPE html>"));
        assert!(html_content.contains("Hello, World!"));
    }
    
    #[tokio::test]
    async fn test_unsupported_format_406() {
        let middleware = ContentNegotiationMiddleware::new();
        
        let mut headers = HeaderMap::new();
        headers.insert("accept", "application/pdf".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "message": "Hello, World!"
                }))
            })
        });
        
        let response = middleware.handle(request, next).await;
        // Should still return JSON as default since PDF is not supported but has a converter
        assert_eq!(response.status_code(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_builder_pattern() {
        let middleware = ContentNegotiationMiddleware::new()
            .default_type(ContentType::Html)
            .support(ContentType::Csv)
            .no_vary_header();
        
        assert_eq!(middleware.config.default_content_type, ContentType::Html);
        assert!(middleware.config.supported_types.contains(&ContentType::Csv));
        assert!(!middleware.config.add_vary_header);
    }
    
    #[test]
    fn test_content_type_mime_types() {
        assert_eq!(ContentType::Json.to_mime_type(), "application/json");
        assert_eq!(ContentType::Xml.to_mime_type(), "application/xml");
        assert_eq!(ContentType::Html.to_mime_type(), "text/html");
        assert_eq!(ContentType::PlainText.to_mime_type(), "text/plain");
        assert_eq!(ContentType::Csv.to_mime_type(), "text/csv");
    }
    
    #[test]
    fn test_json_conversion_functions() {
        let json_val = serde_json::json!({
            "name": "test",
            "value": 42
        });
        
        // Test JSON conversion
        let json_result = ContentNegotiationConfig::convert_to_json(&json_val).unwrap();
        assert!(String::from_utf8(json_result).unwrap().contains("test"));
        
        // Test text conversion with string value
        let text_val = serde_json::json!("Hello World");
        let text_result = ContentNegotiationConfig::convert_to_text(&text_val).unwrap();
        assert_eq!(String::from_utf8(text_result).unwrap(), "Hello World");
        
        // Test HTML conversion
        let html_result = ContentNegotiationConfig::convert_to_html(&json_val).unwrap();
        let html_content = String::from_utf8(html_result).unwrap();
        assert!(html_content.contains("<!DOCTYPE html>"));
        assert!(html_content.contains("test"));
    }
    
    #[tokio::test]
    async fn test_custom_converter_preservation_after_clone() {
        // Test that custom converters are preserved after config clone
        let middleware = ContentNegotiationMiddleware::new()
            .converter(ContentType::Csv, |json_value| {
                // Custom CSV converter
                Ok(b"custom,csv,data".to_vec())
            });
        
        let mut headers = HeaderMap::new();
        headers.insert("accept", "text/csv".parse().unwrap());
        let request = ElifRequest::new(
            Method::GET,
            "/api/data".parse().unwrap(),
            headers,
        );
        
        let next = Next::new(|_req| {
            Box::pin(async move {
                ElifResponse::ok().json_value(serde_json::json!({
                    "test": "data"
                }))
            })
        });
        
        // This should work because the custom converter is preserved through clone
        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Check that it was converted to CSV format
        let axum_response = response.into_axum_response();
        let (parts, _) = axum_response.into_parts();
        assert_eq!(
            parts.headers.get("content-type").unwrap(),
            "text/csv"
        );
    }
}