//! HTTP testing client and response utilities
//!
//! Provides a fluent API for making HTTP requests in tests and
//! comprehensive assertions for validating responses.

use std::collections::HashMap;
use serde_json::{Value as JsonValue, json};
use service_builder::builder;
use crate::{TestError, TestResult};

/// HTTP test client for making requests in tests
#[derive(Debug, Clone)]
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

/// Configuration for Request builder
#[derive(Debug, Clone)]
#[builder]
pub struct RequestBuilderConfig {
    pub client: TestClient,
    pub method: String,
    pub path: String,
    
    #[builder(default)]
    pub headers: HashMap<String, String>,
    
    #[builder(optional)]
    pub body: Option<String>,
    
    #[builder(default)]
    pub query_params: HashMap<String, String>,
}

impl RequestBuilderConfig {
    /// Build the final request
    pub fn build_request(self) -> RequestBuilder {
        RequestBuilder {
            builder_config: self,
        }
    }
}

// Add convenience methods to the generated builder
impl RequestBuilderConfigBuilder {
    /// Add a header
    pub fn add_header(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let mut headers = self.headers.clone().unwrap_or_default();
        headers.insert(name.into(), value.into());
        self.headers(headers)
    }
    
    /// Add query parameter
    pub fn add_query(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut query_params = self.query_params.clone().unwrap_or_default();
        query_params.insert(key.into(), value.into());
        self.query_params(query_params)
    }
    
    /// Add multiple query parameters
    pub fn add_queries(self, params: HashMap<String, String>) -> Self {
        let mut query_params = self.query_params.clone().unwrap_or_default();
        query_params.extend(params);
        self.query_params(query_params)
    }
    
    /// Set JSON body and content type
    pub fn with_json_body<T: serde::Serialize>(self, data: &T) -> Self {
        match serde_json::to_string(data) {
            Ok(json_str) => {
                self.body(Some(json_str))
                    .add_header("Content-Type", "application/json")
            },
            Err(_) => self,
        }
    }
    
    /// Set form body and content type
    pub fn with_form_body(self, data: HashMap<String, String>) -> Self {
        let form_data = data.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        self.body(Some(form_data))
            .add_header("Content-Type", "application/x-www-form-urlencoded")
    }
    
    pub fn build_config(self) -> RequestBuilderConfig {
        self.build_with_defaults().unwrap()
    }
}

/// Request builder for fluent API
pub struct RequestBuilder {
    builder_config: RequestBuilderConfig,
}

impl RequestBuilder {
    fn new(client: TestClient, method: String, path: String) -> Self {
        Self {
            builder_config: RequestBuilderConfig::builder()
                .client(client)
                .method(method)
                .path(path)
                .build_config(),
        }
    }
    
    /// Set a request header
    pub fn header(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let config = self.builder_config;
        let builder = RequestBuilderConfig::builder()
            .client(config.client)
            .method(config.method)
            .path(config.path)
            .headers(config.headers)
            .body(config.body)
            .query_params(config.query_params)
            .add_header(name, value)
            .build_config();
        
        Self {
            builder_config: builder,
        }
    }
    
    /// Set JSON body for the request
    pub fn json<T: serde::Serialize>(self, data: &T) -> Self {
        let config = self.builder_config;
        let builder = RequestBuilderConfig::builder()
            .client(config.client)
            .method(config.method)
            .path(config.path)
            .headers(config.headers)
            .body(config.body)
            .query_params(config.query_params)
            .with_json_body(data)
            .build_config();
        
        Self {
            builder_config: builder,
        }
    }
    
    /// Set form data body
    pub fn form(self, data: HashMap<String, String>) -> Self {
        let config = self.builder_config;
        let builder = RequestBuilderConfig::builder()
            .client(config.client)
            .method(config.method)
            .path(config.path)
            .headers(config.headers)
            .body(config.body)
            .query_params(config.query_params)
            .with_form_body(data)
            .build_config();
        
        Self {
            builder_config: builder,
        }
    }
    
    /// Set plain text body
    pub fn body(self, body: impl Into<String>) -> Self {
        let config = self.builder_config;
        let builder = RequestBuilderConfig::builder()
            .client(config.client)
            .method(config.method)
            .path(config.path)
            .headers(config.headers)
            .body(Some(body.into()))
            .query_params(config.query_params)
            .build_config();
        
        Self {
            builder_config: builder,
        }
    }
    
    /// Add query parameter
    pub fn query(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let config = self.builder_config;
        let builder = RequestBuilderConfig::builder()
            .client(config.client)
            .method(config.method)
            .path(config.path)
            .headers(config.headers)
            .body(config.body)
            .query_params(config.query_params)
            .add_query(key, value)
            .build_config();
        
        Self {
            builder_config: builder,
        }
    }
    
    /// Add multiple query parameters
    pub fn queries(self, params: HashMap<String, String>) -> Self {
        let config = self.builder_config;
        let builder = RequestBuilderConfig::builder()
            .client(config.client)
            .method(config.method)
            .path(config.path)
            .headers(config.headers)
            .body(config.body)
            .query_params(config.query_params)
            .add_queries(params)
            .build_config();
        
        Self {
            builder_config: builder,
        }
    }
    
    /// Send the request and return a test response
    pub async fn send(self) -> TestResult<TestResponse> {
        let config = &self.builder_config;
        
        // Build the full URL
        let mut url = format!("{}{}", config.client.base_url, config.path);
        if !config.query_params.is_empty() {
            let query_string = config.query_params.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url.push_str(&format!("?{}", query_string));
        }
        
        // In a real implementation, this would make an HTTP request
        // For now, we'll create a mock response
        let response = TestResponse {
            status_code: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            },
            body: json!({"message": "Test response", "method": config.method, "path": config.path}).to_string(),
        };
        
        Ok(response)
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