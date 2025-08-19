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
}