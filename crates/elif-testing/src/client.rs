//! HTTP testing client and response utilities
//!
//! Provides a fluent API for making HTTP requests in tests and
//! comprehensive assertions for validating responses.

use std::collections::HashMap;
use serde_json::{Value as JsonValue, json};
use crate::{TestError, TestResult};

/// HTTP test client for making requests in tests
#[derive(Clone)]
pub struct TestClient {
    base_url: String,
    headers: HashMap<String, String>,
    auth_token: Option<String>,
}

impl TestClient {
    /// Create a new test client
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            headers: HashMap::new(),
            auth_token: None,
        }
    }
    
    /// Create a test client with a custom base URL
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            headers: HashMap::new(),
            auth_token: None,
        }
    }
    
    /// Set a header for all requests
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }
    
    /// Set multiple headers
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }
    
    /// Set authentication token (JWT)
    pub fn authenticated_with_token(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        self.auth_token = Some(token.clone());
        self.headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }
    
    /// Authenticate as a specific user (requires user to implement auth traits)
    pub fn authenticated_as<T>(self, _user: &T) -> Self 
    where 
        T: AuthenticatedUser,
    {
        // This would generate a JWT token for the user in tests
        let token = "test_jwt_token"; // Placeholder - would generate real token
        self.authenticated_with_token(token)
    }
    
    /// Make a GET request
    pub fn get(self, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self, "GET".to_string(), path.into())
    }
    
    /// Make a POST request
    pub fn post(self, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self, "POST".to_string(), path.into())
    }
    
    /// Make a PUT request
    pub fn put(self, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self, "PUT".to_string(), path.into())
    }
    
    /// Make a PATCH request
    pub fn patch(self, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self, "PATCH".to_string(), path.into())
    }
    
    /// Make a DELETE request
    pub fn delete(self, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self, "DELETE".to_string(), path.into())
    }
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for users that can be authenticated in tests
pub trait AuthenticatedUser {
    /// Get the user ID for authentication
    fn id(&self) -> String;
    
    /// Get user roles for RBAC testing
    fn roles(&self) -> Vec<String> {
        vec![]
    }
    
    /// Get user permissions for testing
    fn permissions(&self) -> Vec<String> {
        vec![]
    }
}

/// Request builder for fluent API
pub struct RequestBuilder {
    client: TestClient,
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    query_params: HashMap<String, String>,
}

impl RequestBuilder {
    fn new(client: TestClient, method: String, path: String) -> Self {
        Self {
            client,
            method,
            path,
            headers: HashMap::new(),
            body: None,
            query_params: HashMap::new(),
        }
    }
    
    /// Set a request header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }
    
    /// Set JSON body for the request
    pub fn json<T: serde::Serialize>(mut self, data: &T) -> Self {
        match serde_json::to_string(data) {
            Ok(json_str) => {
                self.body = Some(json_str);
                self.headers.insert("Content-Type".to_string(), "application/json".to_string());
            },
            Err(_) => {
                // This would be handled in send()
            }
        }
        self
    }
    
    /// Set form data body
    pub fn form(mut self, data: HashMap<String, String>) -> Self {
        let form_data = data.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        self.body = Some(form_data);
        self.headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
        self
    }
    
    /// Set plain text body
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
    
    /// Add query parameter
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }
    
    /// Add multiple query parameters
    pub fn queries(mut self, params: HashMap<String, String>) -> Self {
        self.query_params.extend(params);
        self
    }
    
    /// Send the request and return a test response
    pub async fn send(self) -> TestResult<TestResponse> {
        // Build the full URL
        let mut url = format!("{}{}", self.client.base_url, self.path);
        if !self.query_params.is_empty() {
            let query_string = self.query_params.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url.push_str(&format!("?{}", query_string));
        }
        
        // Make actual HTTP request using reqwest (pure HTTP client, no axum)
        let client = reqwest::Client::new();
        let mut request_builder = match self.method.as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => return Err(TestError::Setup(format!("Unsupported HTTP method: {}", self.method))),
        };
        
        // Add headers from client and request
        let mut all_headers = self.client.headers.clone();
        all_headers.extend(self.headers);
        
        for (name, value) in all_headers {
            request_builder = request_builder.header(&name, &value);
        }
        
        // Add body if present
        if let Some(body) = self.body {
            request_builder = request_builder.body(body);
        }
        
        // Execute request
        let response = request_builder.send().await
            .map_err(|e| TestError::Setup(format!("HTTP request failed: {}", e)))?;
            
        let status_code = response.status().as_u16();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(name, value)| {
                (name.to_string(), value.to_str().unwrap_or("").to_string())
            })
            .collect();
            
        let body = response.text().await
            .map_err(|e| TestError::Setup(format!("Failed to read response body: {}", e)))?;
        
        Ok(TestResponse {
            status_code,
            headers,
            body,
        })
    }
}

/// Test response wrapper with assertion methods
pub struct TestResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: String,
}

impl TestResponse {
    /// Get the response status code
    pub fn status(&self) -> u16 {
        self.status_code
    }
    
    /// Get response headers
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
    
    /// Get response body as string
    pub fn body(&self) -> &str {
        &self.body
    }
    
    /// Get response body as JSON
    pub fn json(&self) -> TestResult<JsonValue> {
        let json_value: JsonValue = serde_json::from_str(&self.body)?;
        Ok(json_value)
    }
    
    /// Assert the response status code
    pub fn assert_status(self, expected_status: u16) -> Self {
        if self.status_code != expected_status {
            panic!("Expected status {}, got {}", expected_status, self.status_code);
        }
        self
    }
    
    /// Assert the response status is successful (2xx)
    pub fn assert_success(self) -> Self {
        if self.status_code < 200 || self.status_code >= 300 {
            panic!("Expected successful status, got {}", self.status_code);
        }
        self
    }
    
    /// Assert response header value
    pub fn assert_header(self, name: &str, expected_value: &str) -> Self {
        if let Some(value) = self.headers.get(name) {
            if value != expected_value {
                panic!("Expected header '{}' to be '{}', got '{}'", name, expected_value, value);
            }
        } else {
            panic!("Expected header '{}' not found", name);
        }
        self
    }
    
    /// Assert response header exists
    pub fn assert_header_exists(self, name: &str) -> Self {
        if !self.headers.contains_key(name) {
            panic!("Expected header '{}' to exist", name);
        }
        self
    }
    
    /// Assert JSON response contains specific fields/values
    pub fn assert_json_contains(self, expected: JsonValue) -> TestResult<Self> {
        let actual_json = self.json()?;
        
        if !json_contains(&actual_json, &expected) {
            return Err(TestError::Assertion {
                message: format!("Expected JSON to contain: {}, got: {}", expected, actual_json),
            });
        }
        
        Ok(self)
    }
    
    /// Assert JSON response equals expected value
    pub fn assert_json_equals(self, expected: JsonValue) -> TestResult<Self> {
        let actual_json = self.json()?;
        
        if actual_json != expected {
            return Err(TestError::Assertion {
                message: format!("Expected JSON: {}, got: {}", expected, actual_json),
            });
        }
        
        Ok(self)
    }
    
    /// Assert response body contains text
    pub fn assert_body_contains(self, expected_text: &str) -> TestResult<Self> {
        let body = self.body();
        
        if !body.contains(expected_text) {
            return Err(TestError::Assertion {
                message: format!("Expected body to contain '{}', got: {}", expected_text, body),
            });
        }
        
        Ok(self)
    }
    
    /// Assert validation error for specific field
    pub fn assert_validation_error(self, field: &str, _error_type: &str) -> TestResult<Self> {
        let json = self.json()?;
        
        // Check if it's a validation error response
        if let Some(errors) = json.get("errors") {
            if let Some(field_errors) = errors.get(field) {
                if field_errors.as_array().map_or(false, |arr| !arr.is_empty()) {
                    return Ok(self);
                }
            }
        }
        
        Err(TestError::Assertion {
            message: format!("Expected validation error for field '{}', got: {}", field, json),
        })
    }
}

/// Helper function to check if JSON contains expected values
fn json_contains(actual: &JsonValue, expected: &JsonValue) -> bool {
    match (actual, expected) {
        (JsonValue::Object(actual_map), JsonValue::Object(expected_map)) => {
            for (key, expected_value) in expected_map {
                if let Some(actual_value) = actual_map.get(key) {
                    if !json_contains(actual_value, expected_value) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        },
        (JsonValue::Array(actual_arr), JsonValue::Array(expected_arr)) => {
            // For arrays, check if all expected items exist in actual array
            expected_arr.iter().all(|expected_item| {
                actual_arr.iter().any(|actual_item| json_contains(actual_item, expected_item))
            })
        },
        _ => actual == expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_client_creation() {
        let client = TestClient::new();
        assert_eq!(client.base_url, "http://localhost:3000");
        assert!(client.headers.is_empty());
    }
    
    #[test]
    fn test_client_with_custom_url() {
        let client = TestClient::with_base_url("http://example.com");
        assert_eq!(client.base_url, "http://example.com");
    }
    
    #[test]
    fn test_client_headers() {
        let client = TestClient::new()
            .header("X-Test", "value");
        assert_eq!(client.headers.get("X-Test"), Some(&"value".to_string()));
    }
    
    #[test]
    fn test_json_contains() {
        let actual = json!({"name": "John", "age": 30, "active": true});
        let expected = json!({"name": "John"});
        
        assert!(json_contains(&actual, &expected));
        
        let expected_false = json!({"name": "Jane"});
        assert!(!json_contains(&actual, &expected_false));
    }
    
    #[test]
    fn test_json_contains_nested() {
        let actual = json!({
            "user": {
                "name": "John",
                "profile": {
                    "email": "john@example.com"
                }
            }
        });
        let expected = json!({
            "user": {
                "name": "John"
            }
        });
        
        assert!(json_contains(&actual, &expected));
    }
    
    #[test]
    fn test_json_contains_array() {
        let actual = json!({"items": ["a", "b", "c"]});
        let expected = json!({"items": ["a", "c"]});
        
        assert!(json_contains(&actual, &expected));
    }
}