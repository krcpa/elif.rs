//! Response abstraction for building HTTP responses
//! 
//! Provides fluent response building with status codes, headers, and JSON serialization.

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{Response, IntoResponse},
    body::{Body, Bytes},
};
use serde::Serialize;
use crate::errors::{HttpError, HttpResult};

/// Framework-native status codes - use instead of axum::http::StatusCode
pub use axum::http::StatusCode as ElifStatusCode;

/// Framework-native header map - use instead of axum::http::HeaderMap  
pub use axum::http::HeaderMap as ElifHeaderMap;

/// Response builder for creating HTTP responses with fluent API
#[derive(Debug)]
pub struct ElifResponse {
    status: StatusCode,
    headers: HeaderMap,
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
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: ResponseBody::Empty,
        }
    }

    /// Create response with specific status code
    pub fn with_status(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: ResponseBody::Empty,
        }
    }

    /// Set response status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Get response status code
    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    /// Add header to response
    pub fn header<K, V>(mut self, key: K, value: V) -> HttpResult<Self>
    where
        K: TryInto<HeaderName>,
        K::Error: std::fmt::Display,
        V: TryInto<HeaderValue>,
        V::Error: std::fmt::Display,
    {
        let header_name = key.try_into()
            .map_err(|e| HttpError::internal(format!("Invalid header name: {}", e)))?;
        let header_value = value.try_into()
            .map_err(|e| HttpError::internal(format!("Invalid header value: {}", e)))?;
        
        self.headers.insert(header_name, header_value);
        Ok(self)
    }

    /// Set Content-Type header
    pub fn content_type(self, content_type: &str) -> HttpResult<Self> {
        self.header("content-type", content_type)
    }

    /// Set response body as text
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.body = ResponseBody::Text(text.into());
        self
    }

    /// Set response body as bytes
    pub fn bytes(mut self, bytes: Bytes) -> Self {
        self.body = ResponseBody::Bytes(bytes);
        self
    }

    /// Set response body as JSON
    pub fn json<T: Serialize>(mut self, data: &T) -> HttpResult<Self> {
        let json_value = serde_json::to_value(data)
            .map_err(|e| HttpError::internal(format!("JSON serialization failed: {}", e)))?;
        self.body = ResponseBody::Json(json_value);
        Ok(self)
    }

    /// Set response body as raw JSON value
    pub fn json_value(mut self, value: serde_json::Value) -> Self {
        self.body = ResponseBody::Json(value);
        self
    }

    /// Build the response
    pub fn build(mut self) -> HttpResult<Response<Body>> {
        // Set default content type based on body type
        if !self.headers.contains_key("content-type") {
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
            .status(self.status);
        
        // Add headers
        for (key, value) in self.headers.iter() {
            response = response.header(key, value);
        }

        response.body(body)
            .map_err(|e| HttpError::internal(format!("Failed to build response: {}", e)))
    }

    /// Convert ElifResponse to Axum Response for backward compatibility
    pub fn into_axum_response(self) -> Response<Body> {
        IntoResponse::into_response(self)
    }

    /// Convert Axum Response to ElifResponse for backward compatibility
    pub async fn from_axum_response(response: Response<Body>) -> Self {
        let (parts, body) = response.into_parts();
        
        // Extract body bytes
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => Bytes::new(),
        };
        
        let mut elif_response = Self::with_status(parts.status);
        let headers = parts.headers.clone();
        elif_response.headers = parts.headers;
        
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
        Self::with_status(StatusCode::OK)
    }

    /// Create 201 Created response
    pub fn created() -> Self {
        Self::with_status(StatusCode::CREATED)
    }

    /// Create 204 No Content response
    pub fn no_content() -> Self {
        Self::with_status(StatusCode::NO_CONTENT)
    }

    /// Create 400 Bad Request response
    pub fn bad_request() -> Self {
        Self::with_status(StatusCode::BAD_REQUEST)
    }

    /// Create 401 Unauthorized response
    pub fn unauthorized() -> Self {
        Self::with_status(StatusCode::UNAUTHORIZED)
    }

    /// Create 403 Forbidden response
    pub fn forbidden() -> Self {
        Self::with_status(StatusCode::FORBIDDEN)
    }

    /// Create 404 Not Found response
    pub fn not_found() -> Self {
        Self::with_status(StatusCode::NOT_FOUND)
    }

    /// Create 422 Unprocessable Entity response
    pub fn unprocessable_entity() -> Self {
        Self::with_status(StatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Create 500 Internal Server Error response
    pub fn internal_server_error() -> Self {
        Self::with_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Create JSON response with data
    pub fn json_ok<T: Serialize>(data: &T) -> HttpResult<Response<Body>> {
        Self::ok().json(data)?.build()
    }

    /// Create JSON error response
    pub fn json_error(status: StatusCode, message: &str) -> HttpResult<Response<Body>> {
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

impl IntoElifResponse for StatusCode {
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
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Response build failed: {}", e)).into_response()
            }
        }
    }
}

/// Redirect response builders
impl ElifResponse {
    /// Create 301 Moved Permanently redirect
    pub fn redirect_permanent(location: &str) -> HttpResult<Self> {
        Ok(Self::with_status(StatusCode::MOVED_PERMANENTLY)
            .header("location", location)?)
    }

    /// Create 302 Found (temporary) redirect
    pub fn redirect_temporary(location: &str) -> HttpResult<Self> {
        Ok(Self::with_status(StatusCode::FOUND)
            .header("location", location)?)
    }

    /// Create 303 See Other redirect
    pub fn redirect_see_other(location: &str) -> HttpResult<Self> {
        Ok(Self::with_status(StatusCode::SEE_OTHER)
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

        assert_eq!(response.status, StatusCode::OK);
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
        assert_eq!(ElifResponse::created().status, StatusCode::CREATED);
        assert_eq!(ElifResponse::not_found().status, StatusCode::NOT_FOUND);
        assert_eq!(ElifResponse::internal_server_error().status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_headers() {
        let response = ElifResponse::ok()
            .header("x-custom-header", "test-value")
            .unwrap();

        assert!(response.headers.contains_key("x-custom-header"));
        assert_eq!(
            response.headers.get("x-custom-header").unwrap(),
            &HeaderValue::from_static("test-value")
        );
    }

    #[test]
    fn test_redirect_responses() {
        let redirect = ElifResponse::redirect_permanent("/new-location").unwrap();
        assert_eq!(redirect.status, StatusCode::MOVED_PERMANENTLY);
        assert!(redirect.headers.contains_key("location"));
    }

    #[test]
    fn test_status_code_getter() {
        let response = ElifResponse::created();
        assert_eq!(response.status_code(), StatusCode::CREATED);
    }
}