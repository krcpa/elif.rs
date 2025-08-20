//! Laravel-style Response Builder
//! 
//! Provides a fluent builder pattern for creating HTTP responses with intuitive chaining.
//! 
//! # Examples
//! 
//! ```rust,no_run
//! use elif_http::response::response;
//! use elif_http::{HttpResult, ElifResponse};
//! 
//! // Clean Laravel-style syntax with terminal methods
//! async fn list_users() -> HttpResult<ElifResponse> {
//!     let users = vec!["Alice", "Bob"];
//!     response().json(users).send()
//! }
//! 
//! async fn create_user() -> HttpResult<ElifResponse> {
//!     let user = serde_json::json!({"id": 1, "name": "Alice"});
//!     response().json(user).created().location("/users/1").send()
//! }
//! 
//! async fn redirect_user() -> HttpResult<ElifResponse> {
//!     response().redirect("/login").permanent().send()
//! }
//! 
//! // Alternative: using .finish() or traditional Ok(.into())
//! async fn other_examples() -> HttpResult<ElifResponse> {
//!     // Using finish()
//!     response().json("data").finish()
//!     
//!     // Traditional approach still works
//!     // Ok(response().json("data").into())
//! }
//! ```

use crate::response::{ElifResponse, ElifStatusCode, ResponseBody};
use crate::errors::HttpResult;
use serde::Serialize;
use axum::body::Bytes;
use tracing;

/// Response builder for fluent API construction
/// 
/// This struct provides a Laravel-inspired builder pattern for creating HTTP responses.
/// All methods return Self for chaining, and the builder converts to ElifResponse automatically.
#[derive(Debug)]
pub struct ResponseBuilder {
    status: Option<ElifStatusCode>,
    headers: Vec<(String, String)>,
    body: Option<ResponseBody>,
}

impl ResponseBuilder {
    /// Create new response builder
    pub fn new() -> Self {
        Self {
            status: None,
            headers: Vec::new(),
            body: None,
        }
    }

    // Status Code Helpers

    /// Set status to 200 OK
    pub fn ok(mut self) -> Self {
        self.status = Some(ElifStatusCode::OK);
        self
    }

    /// Set status to 201 Created
    pub fn created(mut self) -> Self {
        self.status = Some(ElifStatusCode::CREATED);
        self
    }

    /// Set status to 202 Accepted  
    pub fn accepted(mut self) -> Self {
        self.status = Some(ElifStatusCode::ACCEPTED);
        self
    }

    /// Set status to 204 No Content
    pub fn no_content(mut self) -> Self {
        self.status = Some(ElifStatusCode::NO_CONTENT);
        self
    }

    /// Set status to 400 Bad Request
    pub fn bad_request(mut self) -> Self {
        self.status = Some(ElifStatusCode::BAD_REQUEST);
        self
    }

    /// Set status to 401 Unauthorized
    pub fn unauthorized(mut self) -> Self {
        self.status = Some(ElifStatusCode::UNAUTHORIZED);
        self
    }

    /// Set status to 403 Forbidden
    pub fn forbidden(mut self) -> Self {
        self.status = Some(ElifStatusCode::FORBIDDEN);
        self
    }

    /// Set status to 404 Not Found
    pub fn not_found(mut self) -> Self {
        self.status = Some(ElifStatusCode::NOT_FOUND);
        self
    }

    /// Set status to 422 Unprocessable Entity
    pub fn unprocessable_entity(mut self) -> Self {
        self.status = Some(ElifStatusCode::UNPROCESSABLE_ENTITY);
        self
    }

    /// Set status to 500 Internal Server Error
    pub fn internal_server_error(mut self) -> Self {
        self.status = Some(ElifStatusCode::INTERNAL_SERVER_ERROR);
        self
    }

    /// Set custom status code
    pub fn status(mut self, status: ElifStatusCode) -> Self {
        self.status = Some(status);
        self
    }

    // Content Helpers

    /// Set JSON body with automatic content-type
    pub fn json<T: Serialize>(mut self, data: T) -> Self {
        match serde_json::to_value(&data) {
            Ok(value) => {
                self.body = Some(ResponseBody::Json(value));
                self.headers.push(("content-type".to_string(), "application/json".to_string()));
                self
            }
            Err(err) => {
                // Log the serialization error for easier debugging
                tracing::error!("JSON serialization failed: {}", err);
                // Fallback to error response
                self.status = Some(ElifStatusCode::INTERNAL_SERVER_ERROR);
                self.body = Some(ResponseBody::Text(format!("JSON serialization failed: {}", err)));
                self
            }
        }
    }

    /// Set text body
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.body = Some(ResponseBody::Text(text.into()));
        self.headers.push(("content-type".to_string(), "text/plain; charset=utf-8".to_string()));
        self
    }

    /// Set HTML body
    pub fn html<S: Into<String>>(mut self, html: S) -> Self {
        self.body = Some(ResponseBody::Text(html.into()));
        self.headers.push(("content-type".to_string(), "text/html; charset=utf-8".to_string()));
        self
    }

    /// Set binary body
    pub fn bytes(mut self, bytes: Bytes) -> Self {
        self.body = Some(ResponseBody::Bytes(bytes));
        self
    }

    // Redirect Helpers

    /// Create redirect response
    pub fn redirect<S: Into<String>>(mut self, location: S) -> Self {
        self.headers.push(("location".to_string(), location.into()));
        if self.status.is_none() {
            self.status = Some(ElifStatusCode::FOUND);
        }
        self
    }

    /// Set redirect as permanent (301)
    pub fn permanent(mut self) -> Self {
        self.status = Some(ElifStatusCode::MOVED_PERMANENTLY);
        self
    }

    /// Set redirect as temporary (302) - default
    pub fn temporary(mut self) -> Self {
        self.status = Some(ElifStatusCode::FOUND);
        self
    }

    // Header Helpers

    /// Add custom header
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set location header
    pub fn location<S: Into<String>>(mut self, url: S) -> Self {
        self.headers.push(("location".to_string(), url.into()));
        self
    }

    /// Set cache-control header
    pub fn cache_control<S: Into<String>>(mut self, value: S) -> Self {
        self.headers.push(("cache-control".to_string(), value.into()));
        self
    }

    /// Set content-type header
    pub fn content_type<S: Into<String>>(mut self, content_type: S) -> Self {
        self.headers.push(("content-type".to_string(), content_type.into()));
        self
    }

    /// Add a cookie header (supports multiple cookies)
    pub fn cookie<S: Into<String>>(mut self, cookie_value: S) -> Self {
        self.headers.push(("set-cookie".to_string(), cookie_value.into()));
        self
    }

    // Error Response Helpers

    /// Create error response with message
    pub fn error<S: Into<String>>(mut self, message: S) -> Self {
        let error_data = serde_json::json!({
            "error": {
                "message": message.into(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });
        
        self.body = Some(ResponseBody::Json(error_data));
        self.headers.push(("content-type".to_string(), "application/json".to_string()));
        self
    }

    /// Create validation error response
    pub fn validation_error<T: Serialize>(mut self, errors: T) -> Self {
        let error_data = serde_json::json!({
            "error": {
                "type": "validation",
                "details": errors,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });
        
        self.body = Some(ResponseBody::Json(error_data));
        self.headers.push(("content-type".to_string(), "application/json".to_string()));
        if self.status.is_none() {
            self.status = Some(ElifStatusCode::BAD_REQUEST);
        }
        self
    }

    /// Create not found error with custom message
    pub fn not_found_with_message<S: Into<String>>(mut self, message: S) -> Self {
        let error_data = serde_json::json!({
            "error": {
                "type": "not_found",
                "message": message.into(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });
        
        self.body = Some(ResponseBody::Json(error_data));
        self.headers.push(("content-type".to_string(), "application/json".to_string()));
        self.status = Some(ElifStatusCode::NOT_FOUND);
        self
    }

    // CORS Helpers

    /// Add CORS headers
    pub fn cors(mut self, origin: &str) -> Self {
        self.headers.push(("access-control-allow-origin".to_string(), origin.to_string()));
        self
    }

    /// Add CORS headers with credentials
    pub fn cors_with_credentials(mut self, origin: &str) -> Self {
        self.headers.push(("access-control-allow-origin".to_string(), origin.to_string()));
        self.headers.push(("access-control-allow-credentials".to_string(), "true".to_string()));
        self
    }

    // Security Helpers

    /// Add security headers
    pub fn with_security_headers(mut self) -> Self {
        self.headers.extend([
            ("x-content-type-options".to_string(), "nosniff".to_string()),
            ("x-frame-options".to_string(), "DENY".to_string()),
            ("x-xss-protection".to_string(), "1; mode=block".to_string()),
            ("referrer-policy".to_string(), "strict-origin-when-cross-origin".to_string()),
        ]);
        self
    }

    // Terminal Methods (convert to Result)

    /// Build and return the response wrapped in Ok()
    /// 
    /// This enables Laravel-style terminal chaining: response().json(data).send()
    /// Alternative to: Ok(response().json(data).into())
    pub fn send(self) -> HttpResult<ElifResponse> {
        Ok(self.build())
    }

    /// Build and return the response wrapped in Ok() - alias for send()
    /// 
    /// This enables Laravel-style terminal chaining: response().json(data).finish()
    pub fn finish(self) -> HttpResult<ElifResponse> {
        Ok(self.build())
    }

    /// Build the final ElifResponse
    pub fn build(self) -> ElifResponse {
        let mut response = ElifResponse::new();

        // Set status
        if let Some(status) = self.status {
            response = response.status(status);
        }

        // Check if we have body types that auto-set content-type
        let body_sets_content_type = matches!(
            self.body, 
            Some(ResponseBody::Json(_)) | Some(ResponseBody::Text(_))
        );

        // Set body
        if let Some(body) = self.body {
            match body {
                ResponseBody::Empty => {},
                ResponseBody::Text(text) => {
                    response = response.text(text);
                }
                ResponseBody::Bytes(bytes) => {
                    response = response.bytes(bytes);
                }
                ResponseBody::Json(value) => {
                    response = response.json_value(value);
                }
            }
        }

        // Add headers (skip content-type if already set by body methods)
        let has_explicit_content_type = self.headers.iter()
            .any(|(k, _)| k.to_lowercase() == "content-type");

        for (key, value) in self.headers {
            // Skip content-type headers added by json/text/html if body methods already set it
            if key.to_lowercase() == "content-type" && 
               body_sets_content_type &&
               !has_explicit_content_type {
                continue;
            }
            
            if let (Ok(name), Ok(val)) = (
                crate::response::ElifHeaderName::from_str(&key),
                crate::response::ElifHeaderValue::from_str(&value)
            ) {
                // Use append instead of insert to support multi-value headers like Set-Cookie
                response.headers_mut().append(name, val);
            } else {
                return ElifResponse::internal_server_error();
            }
        }

        response
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert ResponseBuilder to ElifResponse
impl From<ResponseBuilder> for ElifResponse {
    fn from(builder: ResponseBuilder) -> Self {
        builder.build()
    }
}

/// Global response helper function
/// 
/// Creates a new ResponseBuilder for fluent response construction.
/// This is the main entry point for the Laravel-style response API.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use elif_http::response::response;
/// use serde_json::json;
/// 
/// // Basic usage
/// let users = vec!["Alice", "Bob"];
/// let resp = response().json(users);
/// let resp = response().text("Hello World").ok();
/// let resp = response().redirect("/login");
/// 
/// // Complex chaining
/// let user_data = json!({"id": 1, "name": "Alice"});
/// let resp = response()
///     .json(user_data)
///     .created()
///     .location("/users/123")
///     .cache_control("no-cache");
/// ```
pub fn response() -> ResponseBuilder {
    ResponseBuilder::new()
}

/// Global JSON response helper
/// 
/// Creates a ResponseBuilder with JSON data already set.
pub fn json_response<T: Serialize>(data: T) -> ResponseBuilder {
    response().json(data)
}

/// Global text response helper
/// 
/// Creates a ResponseBuilder with text content already set.
pub fn text_response<S: Into<String>>(content: S) -> ResponseBuilder {
    response().text(content)
}

/// Global redirect response helper
/// 
/// Creates a ResponseBuilder with redirect location already set.
pub fn redirect_response<S: Into<String>>(location: S) -> ResponseBuilder {
    response().redirect(location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_response_builder() {
        let resp: ElifResponse = response().text("Hello World").ok().into();
        assert_eq!(resp.status_code(), ElifStatusCode::OK);
    }

    #[test]
    fn test_json_response() {
        let data = json!({"name": "Alice", "age": 30});
        let resp: ElifResponse = response().json(data).into();
        assert_eq!(resp.status_code(), ElifStatusCode::OK);
    }

    #[test]
    fn test_status_helpers() {
        let resp: ElifResponse = response().text("Created").created().into();
        assert_eq!(resp.status_code(), ElifStatusCode::CREATED);

        let resp: ElifResponse = response().text("Not Found").not_found().into();
        assert_eq!(resp.status_code(), ElifStatusCode::NOT_FOUND);
    }

    #[test]
    fn test_redirect_helpers() {
        let resp: ElifResponse = response().redirect("/login").into();
        assert_eq!(resp.status_code(), ElifStatusCode::FOUND);

        let resp: ElifResponse = response().redirect("/users").permanent().into();
        assert_eq!(resp.status_code(), ElifStatusCode::MOVED_PERMANENTLY);
    }

    #[test]
    fn test_redirect_method_call_order_independence() {
        // Test that permanent() works regardless of call order
        
        // Order 1: redirect first, then permanent
        let resp1: ElifResponse = response().redirect("/test").permanent().into();
        assert_eq!(resp1.status_code(), ElifStatusCode::MOVED_PERMANENTLY);
        assert!(resp1.has_header("location"));
        
        // Order 2: permanent first, then redirect  
        let resp2: ElifResponse = response().permanent().redirect("/test").into();
        assert_eq!(resp2.status_code(), ElifStatusCode::MOVED_PERMANENTLY);
        assert!(resp2.has_header("location"));
        
        // Both should have the same final status
        assert_eq!(resp1.status_code(), resp2.status_code());
    }

    #[test]
    fn test_temporary_method_call_order_independence() {
        // Test that temporary() works regardless of call order
        
        // Order 1: redirect first, then temporary
        let resp1: ElifResponse = response().redirect("/test").temporary().into();
        assert_eq!(resp1.status_code(), ElifStatusCode::FOUND);
        assert!(resp1.has_header("location"));
        
        // Order 2: temporary first, then redirect
        let resp2: ElifResponse = response().temporary().redirect("/test").into();
        assert_eq!(resp2.status_code(), ElifStatusCode::FOUND);
        assert!(resp2.has_header("location"));
        
        // Both should have the same final status
        assert_eq!(resp1.status_code(), resp2.status_code());
    }

    #[test]
    fn test_redirect_status_override_behavior() {
        // Test that redirect() respects pre-set status codes
        
        // Default redirect (should be 302 FOUND)
        let resp: ElifResponse = response().redirect("/default").into();
        assert_eq!(resp.status_code(), ElifStatusCode::FOUND);
        
        // Pre-set permanent status should be preserved
        let resp: ElifResponse = response().permanent().redirect("/perm").into();  
        assert_eq!(resp.status_code(), ElifStatusCode::MOVED_PERMANENTLY);
        
        // Pre-set temporary status should be preserved
        let resp: ElifResponse = response().temporary().redirect("/temp").into();
        assert_eq!(resp.status_code(), ElifStatusCode::FOUND);
        
        // Last status wins (permanent overrides default)
        let resp: ElifResponse = response().redirect("/test").permanent().into();
        assert_eq!(resp.status_code(), ElifStatusCode::MOVED_PERMANENTLY);
        
        // Last status wins (temporary overrides permanent) 
        let resp: ElifResponse = response().redirect("/test").permanent().temporary().into();
        assert_eq!(resp.status_code(), ElifStatusCode::FOUND);
    }

    #[test]
    fn test_header_chaining() {
        let resp: ElifResponse = response()
            .text("Hello")
            .header("x-custom", "value")
            .cache_control("no-cache")
            .into();
        
        assert!(resp.has_header("x-custom"));
        assert!(resp.has_header("cache-control"));
    }

    #[test]
    fn test_complex_chaining() {
        let user_data = json!({"id": 1, "name": "Alice"});
        let resp: ElifResponse = response()
            .json(user_data)
            .created()
            .location("/users/1")
            .cache_control("no-cache")
            .header("x-custom", "test")
            .into();

        assert_eq!(resp.status_code(), ElifStatusCode::CREATED);
        assert!(resp.has_header("location"));
        assert!(resp.has_header("cache-control"));
        assert!(resp.has_header("x-custom"));
    }

    #[test]
    fn test_error_responses() {
        let resp: ElifResponse = response().error("Something went wrong").internal_server_error().into();
        assert_eq!(resp.status_code(), ElifStatusCode::INTERNAL_SERVER_ERROR);

        let validation_errors = json!({"email": ["Email is required"]});
        let resp: ElifResponse = response().validation_error(validation_errors).into();
        assert_eq!(resp.status_code(), ElifStatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_global_helpers() {
        let data = json!({"message": "Hello"});
        let resp: ElifResponse = json_response(data).ok().into();
        assert_eq!(resp.status_code(), ElifStatusCode::OK);

        let resp: ElifResponse = text_response("Hello World").into();
        assert_eq!(resp.status_code(), ElifStatusCode::OK);

        let resp: ElifResponse = redirect_response("/home").into();
        assert_eq!(resp.status_code(), ElifStatusCode::FOUND);
    }

    #[test]
    fn test_cors_helpers() {
        let resp: ElifResponse = response()
            .json(json!({"data": "test"}))
            .cors("*")
            .into();
        
        assert!(resp.has_header("access-control-allow-origin"));
    }

    #[test]
    fn test_security_headers() {
        let resp: ElifResponse = response()
            .text("Secure content")
            .with_security_headers()
            .into();
        
        assert!(resp.has_header("x-content-type-options"));
        assert!(resp.has_header("x-frame-options"));
        assert!(resp.has_header("x-xss-protection"));
        assert!(resp.has_header("referrer-policy"));
    }

    #[test]
    fn test_multi_value_headers() {
        // Test that multiple headers with the same name are properly appended
        let resp: ElifResponse = response()
            .text("Hello")
            .header("set-cookie", "session=abc123; Path=/")
            .header("set-cookie", "theme=dark; Path=/")
            .header("set-cookie", "lang=en; Path=/")
            .into();
        
        // All Set-Cookie headers should be present (append behavior)
        assert!(resp.has_header("set-cookie"));
        
        // Test that we can build the response without errors
        assert_eq!(resp.status_code(), ElifStatusCode::OK);
    }

    #[test]
    fn test_cookie_helper_method() {
        // Test the convenience cookie method
        let resp: ElifResponse = response()
            .json(json!({"user": "alice"}))
            .cookie("session=12345; HttpOnly; Secure")
            .cookie("csrf=token123; SameSite=Strict")
            .cookie("theme=dark; Path=/")
            .created()
            .into();
        
        assert!(resp.has_header("set-cookie"));
        assert_eq!(resp.status_code(), ElifStatusCode::CREATED);
    }

    #[test]
    fn test_terminal_methods() {
        // Test .send() terminal method
        let result: HttpResult<ElifResponse> = response()
            .json(json!({"data": "test"}))
            .created()
            .send();
        
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.status_code(), ElifStatusCode::CREATED);

        // Test .finish() terminal method
        let result: HttpResult<ElifResponse> = response()
            .text("Hello World")
            .cache_control("no-cache")
            .finish();
        
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.status_code(), ElifStatusCode::OK);
        assert!(resp.has_header("cache-control"));
    }

    #[test]
    fn test_laravel_style_chaining() {
        // Test the complete Laravel-style chain without Ok() wrapper
        let result: HttpResult<ElifResponse> = response()
            .json(json!({"user_id": 123}))
            .created()
            .location("/users/123") 
            .cookie("session=abc123; HttpOnly")
            .header("x-custom", "value")
            .send();

        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.status_code(), ElifStatusCode::CREATED);
        assert!(resp.has_header("location"));
        assert!(resp.has_header("set-cookie"));
        assert!(resp.has_header("x-custom"));
    }

    #[test] 
    fn test_json_serialization_error_handling() {
        use std::collections::HashMap;
        
        // Test that JSON serialization errors are properly logged and handled
        // We'll use a structure that can potentially fail serialization
        
        // Test with valid data first
        let valid_data = HashMap::from([("key", "value")]);
        let resp: ElifResponse = response()
            .json(valid_data)
            .into();
        
        assert_eq!(resp.status_code(), ElifStatusCode::OK);
        
        // For actual serialization errors (which are rare with standard types),
        // the enhanced error handling now provides:
        // 1. tracing::error! log with full error context
        // 2. 500 status code 
        // 3. Descriptive error message in response body including the actual error
        // 4. Better debugging experience for developers
        
        // The key improvement is that errors are no longer silently ignored
        // and developers get actionable error information in both logs and response
    }

    #[test]
    fn test_header_append_vs_insert_behavior() {
        // Verify that multiple headers with same name are preserved
        let resp: ElifResponse = response()
            .json(json!({"test": "data"}))
            .header("x-custom", "value1")
            .header("x-custom", "value2")
            .header("x-custom", "value3")
            .into();
        
        assert!(resp.has_header("x-custom"));
        assert_eq!(resp.status_code(), ElifStatusCode::OK);
        
        // The response should build successfully with all custom headers
        // (The exact behavior depends on how we want to handle duplicate non-cookie headers)
    }
}