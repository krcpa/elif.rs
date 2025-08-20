//! Response abstraction for building HTTP responses
//! 
//! Provides fluent response building with status codes, headers, and JSON serialization.

use axum::{
    response::{Response, IntoResponse},
    body::{Body, Bytes},
};
use serde::Serialize;
use crate::errors::{HttpError, HttpResult};
use super::{ElifStatusCode, ElifHeaderMap, ElifHeaderName, ElifHeaderValue};

/// Response builder for creating HTTP responses with fluent API
#[derive(Debug)]
pub struct ElifResponse {
    status: ElifStatusCode,
    headers: ElifHeaderMap,
    body: ResponseBody,
}

/// Response body types
#[derive(Debug)]
pub enum ResponseBody {
    Empty,
    Text(String),
    Bytes(Bytes),
    Json(serde_json::Value),
}

impl ElifResponse {
    /// Create new response with OK status
    pub fn new() -> Self {
        Self {
            status: ElifStatusCode::OK,
            headers: ElifHeaderMap::new(),
            body: ResponseBody::Empty,
        }
    }

    /// Create response with specific status code
    pub fn with_status(status: ElifStatusCode) -> Self {
        Self {
            status,
            headers: ElifHeaderMap::new(),
            body: ResponseBody::Empty,
        }
    }

    /// Set response status code (consuming)
    pub fn status(mut self, status: ElifStatusCode) -> Self {
        self.status = status;
        self
    }

    /// Set response status code (borrowing - for middleware use)
    pub fn set_status(&mut self, status: ElifStatusCode) {
        self.status = status;
    }

    /// Get response status code
    pub fn status_code(&self) -> ElifStatusCode {
        self.status
    }

    /// Get response headers
    pub fn headers(&self) -> &ElifHeaderMap {
        &self.headers
    }

    /// Get mutable reference to response headers
    pub fn headers_mut(&mut self) -> &mut ElifHeaderMap {
        &mut self.headers
    }

    /// Check if response has a specific header
    pub fn has_header<K: AsRef<str>>(&self, key: K) -> bool {
        self.headers.contains_key_str(key.as_ref())
    }

    /// Get header value by name
    pub fn get_header<K: AsRef<str>>(&self, key: K) -> Option<&ElifHeaderValue> {
        self.headers.get_str(key.as_ref())
    }

    // Simple panic-safe convenience methods for development ease

    /// Add header to response (Simple - never panics)
    /// 
    /// Simple equivalent: `$response->header('X-Custom', 'value')`
    /// Returns 500 error response on invalid header names/values
    pub fn with_header<K, V>(self, key: K, value: V) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.header(key, value)
            .unwrap_or_else(|err| {
                tracing::error!("Header creation failed in with_header: {}", err);
                ElifResponse::internal_server_error()
            })
    }

    /// Set JSON body (Simple - never panics)
    /// 
    /// Simple equivalent: `$response->json($data)`
    /// Returns 500 error response on serialization failure
    pub fn with_json<T: Serialize>(self, data: &T) -> Self {
        self.json(data)
            .unwrap_or_else(|err| {
                tracing::error!("JSON serialization failed in with_json: {}", err);
                ElifResponse::internal_server_error()
            })
    }

    /// Set text body (Simple - never fails)
    /// 
    /// Simple equivalent: `response($text)`
    pub fn with_text<S: AsRef<str>>(mut self, content: S) -> Self {
        self.body = ResponseBody::Text(content.as_ref().to_string());
        self
    }

    /// Set HTML body with content-type (Simple - never fails)
    /// 
    /// Simple equivalent: `response($html)->header('Content-Type', 'text/html')`
    pub fn with_html<S: AsRef<str>>(self, content: S) -> Self {
        self.with_text(content)
            .with_header("content-type", "text/html; charset=utf-8")
    }

    /// Add header to response (consuming)
    pub fn header<K, V>(mut self, key: K, value: V) -> HttpResult<Self>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let header_name = ElifHeaderName::from_str(key.as_ref())
            .map_err(|e| HttpError::internal(format!("Invalid header name: {}", e)))?;
        let header_value = ElifHeaderValue::from_str(value.as_ref())
            .map_err(|e| HttpError::internal(format!("Invalid header value: {}", e)))?;
        
        self.headers.insert(header_name, header_value);
        Ok(self)
    }

    /// Add header to response (borrowing - for middleware use)
    pub fn add_header<K, V>(&mut self, key: K, value: V) -> HttpResult<()>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let header_name = ElifHeaderName::from_str(key.as_ref())
            .map_err(|e| HttpError::internal(format!("Invalid header name: {}", e)))?;
        let header_value = ElifHeaderValue::from_str(value.as_ref())
            .map_err(|e| HttpError::internal(format!("Invalid header value: {}", e)))?;
        
        self.headers.insert(header_name, header_value);
        Ok(())
    }

    /// Remove header from response
    pub fn remove_header<K: AsRef<str>>(&mut self, key: K) -> Option<ElifHeaderValue> {
        self.headers.remove_header(key.as_ref())
    }

    /// Set Content-Type header (consuming)
    pub fn content_type(self, content_type: &str) -> HttpResult<Self> {
        self.header("content-type", content_type)
    }

    /// Set Content-Type header (borrowing - for middleware use)
    pub fn set_content_type(&mut self, content_type: &str) -> HttpResult<()> {
        self.add_header("content-type", content_type)
    }

    /// Set response body as text (consuming)
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.body = ResponseBody::Text(text.into());
        self
    }

    /// Set response body as text (borrowing - for middleware use)
    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.body = ResponseBody::Text(text.into());
    }

    /// Set response body as bytes (consuming)
    pub fn bytes(mut self, bytes: Bytes) -> Self {
        self.body = ResponseBody::Bytes(bytes);
        self
    }

    /// Set response body as bytes (borrowing - for middleware use)
    pub fn set_bytes(&mut self, bytes: Bytes) {
        self.body = ResponseBody::Bytes(bytes);
    }

    /// Set response body as JSON (consuming)
    pub fn json<T: Serialize>(mut self, data: &T) -> HttpResult<Self> {
        let json_value = serde_json::to_value(data)
            .map_err(|e| HttpError::internal(format!("JSON serialization failed: {}", e)))?;
        self.body = ResponseBody::Json(json_value);
        Ok(self)
    }

    /// Set response body as JSON (borrowing - for middleware use)
    pub fn set_json<T: Serialize>(&mut self, data: &T) -> HttpResult<()> {
        let json_value = serde_json::to_value(data)
            .map_err(|e| HttpError::internal(format!("JSON serialization failed: {}", e)))?;
        self.body = ResponseBody::Json(json_value);
        Ok(())
    }

    /// Set response body as raw JSON value (consuming)
    pub fn json_value(mut self, value: serde_json::Value) -> Self {
        self.body = ResponseBody::Json(value);
        self
    }

    /// Set response body as raw JSON value (borrowing - for middleware use)
    pub fn set_json_value(&mut self, value: serde_json::Value) {
        self.body = ResponseBody::Json(value);
    }

    /// Build the response
    pub fn build(mut self) -> HttpResult<Response<Body>> {
        // Set default content type based on body type
        if !self.headers.contains_key_str("content-type") {
            match &self.body {
                ResponseBody::Json(_) => {
                    self = self.content_type("application/json")?;
                }
                ResponseBody::Text(_) => {
                    self = self.content_type("text/plain; charset=utf-8")?;
                }
                _ => {}
            }
        }

        let body = match self.body {
            ResponseBody::Empty => Body::empty(),
            ResponseBody::Text(text) => Body::from(text),
            ResponseBody::Bytes(bytes) => Body::from(bytes),
            ResponseBody::Json(value) => {
                let json_string = serde_json::to_string(&value)
                    .map_err(|e| HttpError::internal(format!("JSON serialization failed: {}", e)))?;
                Body::from(json_string)
            }
        };

        let mut response = Response::builder()
            .status(self.status.to_axum());
        
        // Add headers
        for (key, value) in self.headers.iter() {
            response = response.header(key.to_axum(), value.to_axum());
        }

        response.body(body)
            .map_err(|e| HttpError::internal(format!("Failed to build response: {}", e)))
    }

    /// Convert ElifResponse to Axum Response for backward compatibility
    pub(crate) fn into_axum_response(self) -> Response<Body> {
        IntoResponse::into_response(self)
    }

    /// Convert Axum Response to ElifResponse for backward compatibility
    pub(crate) async fn from_axum_response(response: Response<Body>) -> Self {
        let (parts, body) = response.into_parts();
        
        // Extract body bytes
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => Bytes::new(),
        };
        
        let mut elif_response = Self::with_status(ElifStatusCode::from_axum(parts.status));
        let headers = parts.headers.clone();
        elif_response.headers = ElifHeaderMap::from_axum(parts.headers);
        
        // Try to determine body type based on content-type header
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                if content_type_str.contains("application/json") {
                    // Try to parse as JSON
                    if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                        elif_response.body = ResponseBody::Json(json_value);
                    } else {
                        elif_response.body = ResponseBody::Bytes(body_bytes);
                    }
                } else if content_type_str.starts_with("text/") {
                    // Parse as text
                    match String::from_utf8(body_bytes.to_vec()) {
                        Ok(text) => elif_response.body = ResponseBody::Text(text),
                        Err(_) => elif_response.body = ResponseBody::Bytes(body_bytes),
                    }
                } else {
                    elif_response.body = ResponseBody::Bytes(body_bytes);
                }
            } else {
                elif_response.body = ResponseBody::Bytes(body_bytes);
            }
        } else if body_bytes.is_empty() {
            elif_response.body = ResponseBody::Empty;
        } else {
            elif_response.body = ResponseBody::Bytes(body_bytes);
        }
        
        elif_response
    }
}

impl Default for ElifResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience methods for common response types
impl ElifResponse {
    /// Create 200 OK response
    pub fn ok() -> Self {
        Self::with_status(ElifStatusCode::OK)
    }

    /// Create 201 Created response
    pub fn created() -> Self {
        Self::with_status(ElifStatusCode::CREATED)
    }

    /// Create 204 No Content response
    pub fn no_content() -> Self {
        Self::with_status(ElifStatusCode::NO_CONTENT)
    }

    /// Create 400 Bad Request response
    pub fn bad_request() -> Self {
        Self::with_status(ElifStatusCode::BAD_REQUEST)
    }

    /// Create 401 Unauthorized response
    pub fn unauthorized() -> Self {
        Self::with_status(ElifStatusCode::UNAUTHORIZED)
    }

    /// Create 403 Forbidden response
    pub fn forbidden() -> Self {
        Self::with_status(ElifStatusCode::FORBIDDEN)
    }

    /// Create 404 Not Found response
    pub fn not_found() -> Self {
        Self::with_status(ElifStatusCode::NOT_FOUND)
    }

    /// Create 422 Unprocessable Entity response
    pub fn unprocessable_entity() -> Self {
        Self::with_status(ElifStatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Create 500 Internal Server Error response
    pub fn internal_server_error() -> Self {
        Self::with_status(ElifStatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Create JSON response with data
    pub fn json_ok<T: Serialize>(data: &T) -> HttpResult<Response<Body>> {
        Self::ok().json(data)?.build()
    }

    /// Create JSON error response
    pub fn json_error(status: ElifStatusCode, message: &str) -> HttpResult<Response<Body>> {
        let error_data = serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message
            }
        });
        
        Self::with_status(status)
            .json_value(error_data)
            .build()
    }

    /// Create validation error response
    pub fn validation_error<T: Serialize>(errors: &T) -> HttpResult<Response<Body>> {
        let error_data = serde_json::json!({
            "error": {
                "code": 422,
                "message": "Validation failed",
                "details": errors
            }
        });
        
        Self::unprocessable_entity()
            .json_value(error_data)
            .build()
    }
}

/// Helper trait for converting types to ElifResponse
pub trait IntoElifResponse {
    fn into_response(self) -> ElifResponse;
}

impl IntoElifResponse for String {
    fn into_response(self) -> ElifResponse {
        ElifResponse::ok().text(self)
    }
}

impl IntoElifResponse for &str {
    fn into_response(self) -> ElifResponse {
        ElifResponse::ok().text(self)
    }
}

impl IntoElifResponse for ElifStatusCode {
    fn into_response(self) -> ElifResponse {
        ElifResponse::with_status(self)
    }
}

impl IntoElifResponse for ElifResponse {
    fn into_response(self) -> ElifResponse {
        self
    }
}

/// Convert ElifResponse to Axum Response
impl IntoResponse for ElifResponse {
    fn into_response(self) -> Response {
        match self.build() {
            Ok(response) => response,
            Err(e) => {
                // Fallback error response
                (ElifStatusCode::INTERNAL_SERVER_ERROR.to_axum(), format!("Response build failed: {}", e)).into_response()
            }
        }
    }
}

/// Redirect response builders
impl ElifResponse {
    /// Create 301 Moved Permanently redirect
    pub fn redirect_permanent(location: &str) -> HttpResult<Self> {
        Ok(Self::with_status(ElifStatusCode::MOVED_PERMANENTLY)
            .header("location", location)?)
    }

    /// Create 302 Found (temporary) redirect
    pub fn redirect_temporary(location: &str) -> HttpResult<Self> {
        Ok(Self::with_status(ElifStatusCode::FOUND)
            .header("location", location)?)
    }

    /// Create 303 See Other redirect
    pub fn redirect_see_other(location: &str) -> HttpResult<Self> {
        Ok(Self::with_status(ElifStatusCode::SEE_OTHER)
            .header("location", location)?)
    }
}

/// File download response builders
impl ElifResponse {
    /// Create file download response
    pub fn download(filename: &str, content: Bytes) -> HttpResult<Self> {
        let content_disposition = format!("attachment; filename=\"{}\"", filename);
        
        Ok(Self::ok()
            .header("content-disposition", content_disposition)?
            .header("content-type", "application/octet-stream")?
            .bytes(content))
    }

    /// Create inline file response (display in browser)
    pub fn file_inline(filename: &str, content_type: &str, content: Bytes) -> HttpResult<Self> {
        let content_disposition = format!("inline; filename=\"{}\"", filename);
        
        Ok(Self::ok()
            .header("content-disposition", content_disposition)?
            .header("content-type", content_type)?
            .bytes(content))
    }

    /// Create file response from filesystem path
    pub fn file<P: AsRef<std::path::Path>>(path: P) -> HttpResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read(path)
            .map_err(|e| HttpError::internal(format!("Failed to read file: {}", e)))?;
        
        let mime_type = Self::guess_mime_type(path);
        
        Ok(Self::ok()
            .header("content-type", mime_type)?
            .bytes(Bytes::from(content)))
    }
    
    /// Guess MIME type from file extension
    fn guess_mime_type(path: &std::path::Path) -> &'static str {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("html") | Some("htm") => "text/html; charset=utf-8",
            Some("css") => "text/css",
            Some("js") => "application/javascript",
            Some("json") => "application/json",
            Some("xml") => "application/xml",
            Some("pdf") => "application/pdf",
            Some("txt") => "text/plain; charset=utf-8",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("mp3") => "audio/mpeg",
            Some("wav") => "audio/wav",
            Some("zip") => "application/zip",
            Some("tar") => "application/x-tar",
            Some("gz") => "application/gzip",
            _ => "application/octet-stream",
        }
    }
}

/// Enhanced response helper methods for common patterns
impl ElifResponse {
    /// Create JSON response with data and optional status
    pub fn json_with_status<T: Serialize>(status: ElifStatusCode, data: &T) -> HttpResult<Self> {
        Self::with_status(status).json(data)
    }

    /// Create JSON response from serde_json::Value
    pub fn json_raw(value: serde_json::Value) -> Self {
        Self::ok().json_value(value)
    }

    /// Create JSON response from serde_json::Value with status
    pub fn json_raw_with_status(status: ElifStatusCode, value: serde_json::Value) -> Self {
        Self::with_status(status).json_value(value)
    }

    /// Create text response with custom content type
    pub fn text_with_type(content: &str, content_type: &str) -> HttpResult<Self> {
        Self::ok()
            .text(content)
            .header("content-type", content_type)
    }

    /// Create XML response
    pub fn xml<S: AsRef<str>>(content: S) -> HttpResult<Self> {
        Self::text_with_type(content.as_ref(), "application/xml; charset=utf-8")
    }

    /// Create CSV response
    pub fn csv<S: AsRef<str>>(content: S) -> HttpResult<Self> {
        Self::text_with_type(content.as_ref(), "text/csv; charset=utf-8")
    }

    /// Create JavaScript response
    pub fn javascript<S: AsRef<str>>(content: S) -> HttpResult<Self> {
        Self::text_with_type(content.as_ref(), "application/javascript; charset=utf-8")
    }

    /// Create CSS response
    pub fn css<S: AsRef<str>>(content: S) -> HttpResult<Self> {
        Self::text_with_type(content.as_ref(), "text/css; charset=utf-8")
    }

    /// Create streaming response with chunked transfer encoding
    pub fn stream() -> HttpResult<Self> {
        Self::ok()
            .header("transfer-encoding", "chunked")
    }

    /// Create Server-Sent Events (SSE) response
    pub fn sse() -> HttpResult<Self> {
        Self::ok()
            .header("content-type", "text/event-stream")?
            .header("cache-control", "no-cache")?
            .header("connection", "keep-alive")
    }

    /// Create JSONP response with callback
    pub fn jsonp<T: Serialize>(callback: &str, data: &T) -> HttpResult<Self> {
        let json_data = serde_json::to_string(data)
            .map_err(|e| HttpError::internal(format!("JSON serialization failed: {}", e)))?;
        
        let jsonp_content = format!("{}({});", callback, json_data);
        
        Self::ok()
            .text(jsonp_content)
            .header("content-type", "application/javascript; charset=utf-8")
    }

    /// Create image response from bytes with format detection
    pub fn image(content: Bytes, format: ImageFormat) -> HttpResult<Self> {
        let content_type = match format {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
        };

        Ok(Self::ok()
            .header("content-type", content_type)?
            .bytes(content))
    }

    /// Create binary response with custom MIME type
    pub fn binary(content: Bytes, mime_type: &str) -> HttpResult<Self> {
        Ok(Self::ok()
            .header("content-type", mime_type)?
            .bytes(content))
    }

    /// Create CORS preflight response
    pub fn cors_preflight(
        allowed_origins: &[&str],
        allowed_methods: &[&str], 
        allowed_headers: &[&str],
        max_age: Option<u32>
    ) -> HttpResult<Self> {
        let mut response = Self::no_content()
            .header("access-control-allow-origin", allowed_origins.join(","))?
            .header("access-control-allow-methods", allowed_methods.join(","))?
            .header("access-control-allow-headers", allowed_headers.join(","))?;

        if let Some(max_age) = max_age {
            response = response.header("access-control-max-age", max_age.to_string())?;
        }

        Ok(response)
    }

    /// Add CORS headers to existing response
    pub fn with_cors(
        mut self,
        origin: &str,
        credentials: bool,
        exposed_headers: Option<&[&str]>
    ) -> HttpResult<Self> {
        self = self.header("access-control-allow-origin", origin)?;
        
        if credentials {
            self = self.header("access-control-allow-credentials", "true")?;
        }
        
        if let Some(headers) = exposed_headers {
            self = self.header("access-control-expose-headers", headers.join(","))?;
        }
        
        Ok(self)
    }

    /// Create response with caching headers
    pub fn with_cache(mut self, max_age: u32, public: bool) -> HttpResult<Self> {
        let cache_control = if public {
            format!("public, max-age={}", max_age)
        } else {
            format!("private, max-age={}", max_age)
        };
        
        self = self.header("cache-control", cache_control)?;
        
        // Add ETag for cache validation (simple content-based)
        let etag = format!("\"{}\"", self.generate_etag());
        self = self.header("etag", etag)?;
        
        Ok(self)
    }

    /// Generate content-based ETag including response body
    fn generate_etag(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.status.as_u16().hash(&mut hasher);
        
        // Hash header keys and values for ETag generation
        for (key, value) in self.headers.iter() {
            key.as_str().hash(&mut hasher);
            if let Ok(value_str) = value.to_str() {
                value_str.hash(&mut hasher);
            }
        }
        
        // Hash the response body for a correct content-based ETag
        match &self.body {
            ResponseBody::Empty => "empty".hash(&mut hasher),
            ResponseBody::Text(text) => text.hash(&mut hasher),
            ResponseBody::Bytes(bytes) => bytes.hash(&mut hasher),
            ResponseBody::Json(value) => {
                // Use canonical JSON string representation for consistent hashing
                if let Ok(json_string) = serde_json::to_string(value) {
                    json_string.hash(&mut hasher);
                } else {
                    // Fallback if serialization fails
                    "invalid_json".hash(&mut hasher);
                }
            }
        }
        
        format!("{:x}", hasher.finish())
    }

    /// Create conditional response based on If-None-Match header
    pub fn conditional(self, request_etag: Option<&str>) -> Self {
        if let Some(request_etag) = request_etag {
            let response_etag = self.generate_etag();
            let response_etag_quoted = format!("\"{}\"", response_etag);
            
            if request_etag == response_etag_quoted || request_etag == "*" {
                return ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED);
            }
        }
        self
    }

    /// Add security headers to response
    pub fn with_security_headers(mut self) -> HttpResult<Self> {
        self = self.header("x-content-type-options", "nosniff")?;
        self = self.header("x-frame-options", "DENY")?;
        self = self.header("x-xss-protection", "1; mode=block")?;
        self = self.header("referrer-policy", "strict-origin-when-cross-origin")?;
        self = self.header("content-security-policy", "default-src 'self'")?;
        Ok(self)
    }

    /// Add performance headers
    pub fn with_performance_headers(mut self) -> HttpResult<Self> {
        self = self.header("x-dns-prefetch-control", "on")?;
        self = self.header("x-powered-by", "elif.rs")?;
        Ok(self)
    }
}

/// Image format enumeration for typed image responses
#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    WebP,
    Svg,
}

/// Response transformation helpers
impl ElifResponse {
    /// Transform response body with a closure
    pub fn transform_body<F>(mut self, transform: F) -> Self 
    where
        F: FnOnce(ResponseBody) -> ResponseBody,
    {
        self.body = transform(self.body);
        self
    }

    /// Add multiple headers at once
    pub fn with_headers<I, K, V>(mut self, headers: I) -> HttpResult<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        for (key, value) in headers {
            self = self.header(key, value)?;
        }
        Ok(self)
    }

    /// Create response and immediately build to Axum Response
    pub fn build_axum(self) -> Response<axum::body::Body> {
        match self.build() {
            Ok(response) => response,
            Err(e) => {
                // Fallback error response
                (ElifStatusCode::INTERNAL_SERVER_ERROR.to_axum(), 
                 format!("Response build failed: {}", e)).into_response()
            }
        }
    }

    /// Check if response is an error (4xx or 5xx status)
    pub fn is_error(&self) -> bool {
        self.status.as_u16() >= 400
    }

    /// Check if response is successful (2xx status)
    pub fn is_success(&self) -> bool {
        let status_code = self.status.as_u16();
        status_code >= 200 && status_code < 300
    }

    /// Check if response is a redirect (3xx status)
    pub fn is_redirect(&self) -> bool {
        let status_code = self.status.as_u16();
        status_code >= 300 && status_code < 400
    }

    /// Get response body size estimate
    pub fn body_size_estimate(&self) -> usize {
        match &self.body {
            ResponseBody::Empty => 0,
            ResponseBody::Text(text) => text.len(),
            ResponseBody::Bytes(bytes) => bytes.len(),
            ResponseBody::Json(value) => {
                // Estimate JSON serialization size
                serde_json::to_string(value)
                    .map(|s| s.len())
                    .unwrap_or(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_response_building() {
        let response = ElifResponse::ok()
            .text("Hello, World!");

        assert_eq!(response.status, ElifStatusCode::OK);
        match response.body {
            ResponseBody::Text(text) => assert_eq!(text, "Hello, World!"),
            _ => panic!("Expected text body"),
        }
    }

    #[test]
    fn test_json_response() {
        let data = json!({
            "name": "John Doe",
            "age": 30
        });

        let response = ElifResponse::ok()
            .json_value(data.clone());

        match response.body {
            ResponseBody::Json(value) => assert_eq!(value, data),
            _ => panic!("Expected JSON body"),
        }
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(ElifResponse::created().status, ElifStatusCode::CREATED);
        assert_eq!(ElifResponse::not_found().status, ElifStatusCode::NOT_FOUND);
        assert_eq!(ElifResponse::internal_server_error().status, ElifStatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_headers() {
        let response = ElifResponse::ok()
            .header("x-custom-header", "test-value")
            .unwrap();

        let custom_header = crate::response::headers::ElifHeaderName::from_str("x-custom-header").unwrap();
        assert!(response.headers.contains_key(&custom_header));
        assert_eq!(
            response.headers.get(&custom_header).unwrap(),
            &crate::response::headers::ElifHeaderValue::from_static("test-value")
        );
    }

    #[test]
    fn test_redirect_responses() {
        let redirect = ElifResponse::redirect_permanent("/new-location").unwrap();
        assert_eq!(redirect.status, ElifStatusCode::MOVED_PERMANENTLY);
        assert!(redirect.headers.contains_key(&crate::response::headers::ElifHeaderName::from_str("location").unwrap()));
    }

    #[test]
    fn test_status_code_getter() {
        let response = ElifResponse::created();
        assert_eq!(response.status_code(), ElifStatusCode::CREATED);
    }

    #[test]
    fn test_borrowing_api_headers() {
        let mut response = ElifResponse::ok();
        
        // Test borrowing header methods
        response.add_header("x-custom-header", "test-value").unwrap();
        response.set_content_type("application/json").unwrap();
        
        let built_response = response.build().unwrap();
        let headers = built_response.headers();
        
        assert!(headers.contains_key("x-custom-header"));
        assert_eq!(headers.get("x-custom-header").unwrap(), "test-value");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_borrowing_api_body() {
        let mut response = ElifResponse::ok();
        
        // Test borrowing text body method
        response.set_text("Hello World");
        assert_eq!(response.status_code(), ElifStatusCode::OK);
        match &response.body {
            ResponseBody::Text(text) => assert_eq!(text, "Hello World"),
            _ => panic!("Expected text body after calling set_text"),
        }
        
        // Test borrowing bytes body method
        let bytes_data = Bytes::from("binary data");
        response.set_bytes(bytes_data.clone());
        match &response.body {
            ResponseBody::Bytes(bytes) => assert_eq!(bytes, &bytes_data),
            _ => panic!("Expected bytes body after calling set_bytes"),
        }
        
        // Test borrowing JSON body method
        let data = json!({"message": "Hello"});
        response.set_json_value(data.clone());
        
        // Verify the body was set correctly
        match &response.body {
            ResponseBody::Json(value) => assert_eq!(*value, data),
            _ => panic!("Expected JSON body after calling set_json_value"),
        }
    }

    #[test]
    fn test_borrowing_api_status() {
        let mut response = ElifResponse::ok();
        
        // Test borrowing status method
        response.set_status(ElifStatusCode::CREATED);
        assert_eq!(response.status_code(), ElifStatusCode::CREATED);
        
        // Test multiple modifications
        response.set_status(ElifStatusCode::ACCEPTED);
        response.set_text("Updated");
        
        assert_eq!(response.status_code(), ElifStatusCode::ACCEPTED);
    }

    #[test] 
    fn test_borrowing_api_middleware_pattern() {
        // Test the pattern that caused issues in middleware v2
        let mut response = ElifResponse::ok().text("Original");
        
        // Simulate middleware adding headers iteratively
        let headers = vec![
            ("x-middleware-1", "executed"),
            ("x-middleware-2", "processed"), 
            ("x-custom", "value"),
        ];
        
        for (name, value) in headers {
            // This should work without ownership issues
            response.add_header(name, value).unwrap();
        }
        
        let built = response.build().unwrap();
        let response_headers = built.headers();
        
        assert!(response_headers.contains_key("x-middleware-1"));
        assert!(response_headers.contains_key("x-middleware-2")); 
        assert!(response_headers.contains_key("x-custom"));
    }

    #[test]
    fn test_etag_generation_includes_body_content() {
        // Test that ETag generation properly includes response body content
        
        // Same status and headers, different text bodies
        let response1 = ElifResponse::ok().with_text("Hello World");
        let response2 = ElifResponse::ok().with_text("Different Content");
        
        let etag1 = response1.generate_etag();
        let etag2 = response2.generate_etag();
        
        assert_ne!(etag1, etag2, "ETags should be different for different text content");
        
        // Same status and headers, different JSON bodies
        let json1 = serde_json::json!({"name": "Alice", "age": 30});
        let json2 = serde_json::json!({"name": "Bob", "age": 25});
        
        let response3 = ElifResponse::ok().with_json(&json1);
        let response4 = ElifResponse::ok().with_json(&json2);
        
        let etag3 = response3.generate_etag();
        let etag4 = response4.generate_etag();
        
        assert_ne!(etag3, etag4, "ETags should be different for different JSON content");
        
        // Same content should produce same ETag
        let response5 = ElifResponse::ok().with_text("Hello World");
        let etag5 = response5.generate_etag();
        
        assert_eq!(etag1, etag5, "ETags should be identical for identical content");
        
        // Different response types with same logical content should be different
        let response6 = ElifResponse::ok().with_json(&serde_json::json!("Hello World"));
        let etag6 = response6.generate_etag();
        
        assert_ne!(etag1, etag6, "ETags should be different for different body types even with same content");
    }

    #[test]
    fn test_etag_generation_different_body_types() {
        // Test ETag generation for all body types
        
        let empty_response = ElifResponse::ok();
        let text_response = ElifResponse::ok().with_text("test content");
        let bytes_response = ElifResponse::ok().bytes(Bytes::from("test content"));
        let json_response = ElifResponse::ok().with_json(&serde_json::json!({"key": "value"}));
        
        let empty_etag = empty_response.generate_etag();
        let text_etag = text_response.generate_etag();
        let bytes_etag = bytes_response.generate_etag();
        let json_etag = json_response.generate_etag();
        
        // All ETags should be different
        let etags = vec![&empty_etag, &text_etag, &bytes_etag, &json_etag];
        for i in 0..etags.len() {
            for j in (i + 1)..etags.len() {
                assert_ne!(etags[i], etags[j], 
                    "ETags should be unique for different body types: {} vs {}", etags[i], etags[j]);
            }
        }
        
        // ETags should be consistent for same content
        let text_response2 = ElifResponse::ok().with_text("test content");
        let text_etag2 = text_response2.generate_etag();
        assert_eq!(text_etag, text_etag2, "ETags should be consistent for identical text content");
    }

    #[test]
    fn test_etag_generation_with_status_and_headers() {
        // Verify that status codes and headers still affect ETag generation
        
        let base_content = "same content";
        
        // Different status codes should produce different ETags
        let response_200 = ElifResponse::ok().with_text(base_content);
        let response_201 = ElifResponse::created().with_text(base_content);
        
        let etag_200 = response_200.generate_etag();
        let etag_201 = response_201.generate_etag();
        
        assert_ne!(etag_200, etag_201, "ETags should be different for different status codes");
        
        // Different headers should produce different ETags
        let response_no_header = ElifResponse::ok().with_text(base_content);
        let response_with_header = ElifResponse::ok()
            .with_text(base_content)
            .with_header("x-custom", "value");
        
        let etag_no_header = response_no_header.generate_etag();
        let etag_with_header = response_with_header.generate_etag();
        
        assert_ne!(etag_no_header, etag_with_header, "ETags should be different when headers differ");
    }

    #[test]
    fn test_etag_conditional_response() {
        let response = ElifResponse::ok().with_text("Test content");
        let etag = response.generate_etag();
        let etag_quoted = format!("\"{}\"", etag);
        
        // Test If-None-Match with matching ETag (should return 304)
        let conditional_response = response.conditional(Some(&etag_quoted));
        assert_eq!(conditional_response.status_code(), ElifStatusCode::NOT_MODIFIED);
        
        // Test If-None-Match with non-matching ETag (should return original response)
        let response2 = ElifResponse::ok().with_text("Test content");
        let different_etag = "\"different_etag_value\"";
        let conditional_response2 = response2.conditional(Some(different_etag));
        assert_eq!(conditional_response2.status_code(), ElifStatusCode::OK);
        
        // Test wildcard match
        let wildcard_response = ElifResponse::ok().with_text("Test content").conditional(Some("*"));
        assert_eq!(wildcard_response.status_code(), ElifStatusCode::NOT_MODIFIED);
    }
}