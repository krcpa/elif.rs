//! Response helpers for maximum simplicity
//!
//! These functions provide one-liner response creation,
//! making elif the easiest HTTP framework in Rust.
//!
//! # Examples
//!
//! ```rust
//! use elif_http::response::{json, json_status, redirect, html};
//!
//! // Simple one-liners
//! async fn get_users() -> HttpResult<ElifResponse> {
//!     let users = vec!["Alice", "Bob"];
//!     Ok(json(&users))  // Simple JSON response
//! }
//!
//! async fn create_user() -> HttpResult<ElifResponse> {
//!     let user = User::new("Alice");
//!     Ok(json_status(&user, ElifStatusCode::CREATED))
//! }
//! ```

use crate::response::{ElifResponse, ElifStatusCode};
use serde::Serialize;
use std::collections::HashMap;

/// Create a JSON response with 200 OK status
pub fn json<T: Serialize>(data: &T) -> ElifResponse {
    ElifResponse::ok()
        .json(data)
        .unwrap_or_else(|_| ElifResponse::internal_server_error())
}

/// Create a JSON response with custom status code
pub fn json_status<T: Serialize>(data: &T, status: ElifStatusCode) -> ElifResponse {
    ElifResponse::with_status(status)
        .json(data)
        .unwrap_or_else(|_| ElifResponse::internal_server_error())
}

/// Create a JSON response with headers
pub fn json_with_headers<T: Serialize>(
    data: &T,
    headers: &[(&str, &str)],
) -> ElifResponse {
    let mut response = ElifResponse::ok()
        .json(data)
        .unwrap_or_else(|_| ElifResponse::internal_server_error());
    
    for (key, value) in headers {
        response = response.header(key, value)
            .unwrap_or_else(|_| ElifResponse::internal_server_error());
    }
    
    response
}

/// Create a temporary redirect response (302)
pub fn redirect(url: &str) -> ElifResponse {
    ElifResponse::redirect_temporary(url)
        .unwrap_or_else(|_| ElifResponse::internal_server_error())
}

/// Create a permanent redirect response (301)
pub fn redirect_permanent(url: &str) -> ElifResponse {
    ElifResponse::redirect_permanent(url)
        .unwrap_or_else(|_| ElifResponse::internal_server_error())
}

/// Create a plain text response
/// 
/// Simple equivalent: `return response($text)`
pub fn text<S: AsRef<str>>(content: S) -> ElifResponse {
    ElifResponse::ok()
        .text(content.as_ref())
}

/// Create an HTML response
/// 
/// Simple equivalent: `return response($html)->header('Content-Type', 'text/html')`
pub fn html<S: AsRef<str>>(content: S) -> ElifResponse {
    ElifResponse::ok()
        .text(content.as_ref())
        .with_header("content-type", "text/html; charset=utf-8")
}

/// Create a no content response (204)
/// 
/// Simple equivalent: `return response()->noContent()`
pub fn no_content() -> ElifResponse {
    ElifResponse::no_content()
}

/// Create a created response (201) with JSON data
/// 
/// Simple equivalent: `return response()->json($data, 201)`
pub fn created<T: Serialize>(data: &T) -> ElifResponse {
    json_status(data, ElifStatusCode::CREATED)
}

/// Create an accepted response (202) with optional data
/// 
/// Simple equivalent: `return response()->json($data, 202)`
pub fn accepted<T: Serialize>(data: &T) -> ElifResponse {
    json_status(data, ElifStatusCode::ACCEPTED)
}

/// Create a bad request response (400) with error message
/// 
/// Simple equivalent: `return response()->json(['error' => $message], 400)`
pub fn bad_request<S: AsRef<str>>(message: S) -> ElifResponse {
    let error = HashMap::from([
        ("error", message.as_ref()),
    ]);
    json_status(&error, ElifStatusCode::BAD_REQUEST)
}

/// Create an unauthorized response (401) with error message
/// 
/// Simple equivalent: `return response()->json(['error' => 'Unauthorized'], 401)`
pub fn unauthorized<S: AsRef<str>>(message: S) -> ElifResponse {
    let error = HashMap::from([
        ("error", message.as_ref()),
    ]);
    json_status(&error, ElifStatusCode::UNAUTHORIZED)
}

/// Create a forbidden response (403) with error message
/// 
/// Simple equivalent: `return response()->json(['error' => 'Forbidden'], 403)`
pub fn forbidden<S: AsRef<str>>(message: S) -> ElifResponse {
    let error = HashMap::from([
        ("error", message.as_ref()),
    ]);
    json_status(&error, ElifStatusCode::FORBIDDEN)
}

/// Create a not found response (404) with error message
/// 
/// Simple equivalent: `return response()->json(['error' => 'Not Found'], 404)`
pub fn not_found<S: AsRef<str>>(message: S) -> ElifResponse {
    let error = HashMap::from([
        ("error", message.as_ref()),
    ]);
    json_status(&error, ElifStatusCode::NOT_FOUND)
}

/// Create an internal server error response (500) with error message
/// 
/// Simple equivalent: `return response()->json(['error' => 'Internal Server Error'], 500)`
pub fn server_error<S: AsRef<str>>(message: S) -> ElifResponse {
    let error = HashMap::from([
        ("error", message.as_ref()),
    ]);
    json_status(&error, ElifStatusCode::INTERNAL_SERVER_ERROR)
}

/// Create a validation error response (422) with field errors
/// 
/// Simple equivalent: `return response()->json(['errors' => $errors], 422)`
pub fn validation_error<T: Serialize>(errors: &T) -> ElifResponse {
    let response_body = HashMap::from([
        ("errors", errors),
    ]);
    json_status(&response_body, ElifStatusCode::UNPROCESSABLE_ENTITY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_response() {
        let data = vec!["Alice", "Bob"];
        let response = json(&data);
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[test]
    fn test_json_status_response() {
        let data = vec!["Alice"];
        let response = json_status(&data, ElifStatusCode::CREATED);
        assert_eq!(response.status_code(), ElifStatusCode::CREATED);
    }

    #[test]
    fn test_redirect_response() {
        let response = redirect("https://example.com");
        assert_eq!(response.status_code(), ElifStatusCode::FOUND);
    }

    #[test]
    fn test_text_response() {
        let response = text("Hello, World!");
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[test]
    fn test_html_response() {
        let response = html("<h1>Hello, World!</h1>");
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[test]
    fn test_no_content_response() {
        let response = no_content();
        assert_eq!(response.status_code(), ElifStatusCode::NO_CONTENT);
    }

    #[test]
    fn test_created_response() {
        let data = vec!["Alice"];
        let response = created(&data);
        assert_eq!(response.status_code(), ElifStatusCode::CREATED);
    }

    #[test]
    fn test_error_responses() {
        let bad_req = bad_request("Invalid input");
        assert_eq!(bad_req.status_code(), ElifStatusCode::BAD_REQUEST);

        let unauthorized_resp = unauthorized("Please login");
        assert_eq!(unauthorized_resp.status_code(), ElifStatusCode::UNAUTHORIZED);

        let forbidden_resp = forbidden("Access denied");
        assert_eq!(forbidden_resp.status_code(), ElifStatusCode::FORBIDDEN);

        let not_found_resp = not_found("User not found");
        assert_eq!(not_found_resp.status_code(), ElifStatusCode::NOT_FOUND);

        let server_err = server_error("Database connection failed");
        assert_eq!(server_err.status_code(), ElifStatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_validation_error() {
        let errors = json!({
            "name": ["The name field is required"],
            "email": ["The email must be a valid email address"]
        });
        
        let response = validation_error(&errors);
        assert_eq!(response.status_code(), ElifStatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_json_with_headers() {
        let data = vec!["Alice", "Bob"];
        let headers = [("X-Total-Count", "2"), ("X-Custom", "value")];
        let response = json_with_headers(&data, &headers);
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }
}