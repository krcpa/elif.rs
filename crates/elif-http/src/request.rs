//! Request abstraction for handling HTTP requests
//! 
//! Provides rich request parsing and data extraction capabilities.

use std::collections::HashMap;
use axum::{
    http::{HeaderMap, HeaderValue, Method, Uri},
    body::Bytes,
};
use serde::de::DeserializeOwned;
use crate::error::{HttpError, HttpResult};

/// Request abstraction that wraps Axum's request types
/// with additional parsing and extraction capabilities
#[derive(Debug)]
pub struct ElifRequest {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    body_bytes: Option<Bytes>,
}

impl ElifRequest {
    /// Create new ElifRequest from Axum components
    pub fn new(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
    ) -> Self {
        Self {
            method,
            uri,
            headers,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            body_bytes: None,
        }
    }

    /// Set path parameters extracted from route
    pub fn with_path_params(mut self, params: HashMap<String, String>) -> Self {
        self.path_params = params;
        self
    }

    /// Set query parameters
    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    /// Set request body bytes
    pub fn with_body(mut self, body: Bytes) -> Self {
        self.body_bytes = Some(body);
        self
    }

    /// Get path parameter by name
    pub fn path_param(&self, name: &str) -> Option<&String> {
        self.path_params.get(name)
    }

    /// Get path parameter by name, parsed to specific type
    pub fn path_param_parsed<T>(&self, name: &str) -> HttpResult<T>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        let param = self.path_param(name)
            .ok_or_else(|| HttpError::bad_request(format!("Missing path parameter: {}", name)))?;
        
        param.parse::<T>()
            .map_err(|e| HttpError::bad_request(format!("Invalid path parameter {}: {}", name, e)))
    }

    /// Get query parameter by name
    pub fn query_param(&self, name: &str) -> Option<&String> {
        self.query_params.get(name)
    }

    /// Get query parameter by name, parsed to specific type
    pub fn query_param_parsed<T>(&self, name: &str) -> HttpResult<Option<T>>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        if let Some(param) = self.query_param(name) {
            let parsed = param.parse::<T>()
                .map_err(|e| HttpError::bad_request(format!("Invalid query parameter {}: {}", name, e)))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get required query parameter by name, parsed to specific type
    pub fn query_param_required<T>(&self, name: &str) -> HttpResult<T>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        self.query_param_parsed(name)?
            .ok_or_else(|| HttpError::bad_request(format!("Missing required query parameter: {}", name)))
    }

    /// Get header value by name
    pub fn header(&self, name: &str) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Get header value as string
    pub fn header_string(&self, name: &str) -> HttpResult<Option<String>> {
        if let Some(value) = self.header(name) {
            let str_value = value.to_str()
                .map_err(|_| HttpError::bad_request(format!("Invalid header value for {}", name)))?;
            Ok(Some(str_value.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Get Content-Type header
    pub fn content_type(&self) -> HttpResult<Option<String>> {
        self.header_string("content-type")
    }

    /// Check if request has JSON content type
    pub fn is_json(&self) -> bool {
        if let Ok(Some(content_type)) = self.content_type() {
            content_type.contains("application/json")
        } else {
            false
        }
    }

    /// Get request body as bytes
    pub fn body_bytes(&self) -> Option<&Bytes> {
        self.body_bytes.as_ref()
    }

    /// Parse JSON body to specified type
    pub fn json<T: DeserializeOwned>(&self) -> HttpResult<T> {
        let bytes = self.body_bytes()
            .ok_or_else(|| HttpError::bad_request("No request body".to_string()))?;
        
        serde_json::from_slice(bytes)
            .map_err(|e| HttpError::bad_request(format!("Invalid JSON body: {}", e)))
    }

    /// Parse form data body to specified type
    pub fn form<T: DeserializeOwned>(&self) -> HttpResult<T> {
        let bytes = self.body_bytes()
            .ok_or_else(|| HttpError::bad_request("No request body".to_string()))?;
        
        let body_str = std::str::from_utf8(bytes)
            .map_err(|_| HttpError::bad_request("Invalid UTF-8 in form body".to_string()))?;
        
        serde_urlencoded::from_str(body_str)
            .map_err(|e| HttpError::bad_request(format!("Invalid form data: {}", e)))
    }

    /// Get User-Agent header
    pub fn user_agent(&self) -> HttpResult<Option<String>> {
        self.header_string("user-agent")
    }

    /// Get Authorization header
    pub fn authorization(&self) -> HttpResult<Option<String>> {
        self.header_string("authorization")
    }

    /// Extract Bearer token from Authorization header
    pub fn bearer_token(&self) -> HttpResult<Option<String>> {
        if let Some(auth) = self.authorization()? {
            if auth.starts_with("Bearer ") {
                Ok(Some(auth[7..].to_string()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Get request IP address from headers or connection
    pub fn client_ip(&self) -> HttpResult<Option<String>> {
        // Try common forwarded headers first
        if let Some(forwarded) = self.header_string("x-forwarded-for")? {
            // Take first IP if multiple
            if let Some(ip) = forwarded.split(',').next() {
                return Ok(Some(ip.trim().to_string()));
            }
        }
        
        if let Some(real_ip) = self.header_string("x-real-ip")? {
            return Ok(Some(real_ip));
        }
        
        // Could extend with connection info if available
        Ok(None)
    }

    /// Check if request is HTTPS
    pub fn is_secure(&self) -> bool {
        self.uri.scheme()
            .map(|s| s == &axum::http::uri::Scheme::HTTPS)
            .unwrap_or(false)
    }

    /// Get request host
    pub fn host(&self) -> Option<&str> {
        self.uri.host()
    }

    /// Get request path
    pub fn path(&self) -> &str {
        self.uri.path()
    }

    /// Get query string
    pub fn query_string(&self) -> Option<&str> {
        self.uri.query()
    }
}

/// Helper trait for extracting ElifRequest from Axum request parts
pub trait RequestExtractor {
    /// Extract ElifRequest from request components
    fn extract_elif_request(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Option<Bytes>,
    ) -> ElifRequest {
        let mut request = ElifRequest::new(method, uri, headers);
        if let Some(body) = body {
            request = request.with_body(body);
        }
        request
    }
}

impl RequestExtractor for ElifRequest {}

/// Framework-native Query extractor - use instead of axum::extract::Query
#[derive(Debug)]
pub struct ElifQuery<T>(pub T);

impl<T: DeserializeOwned> ElifQuery<T> {
    /// Extract and deserialize query parameters from request
    pub fn from_request(request: &ElifRequest) -> HttpResult<Self> {
        let query_str = request.query_string().unwrap_or("");
        let data = serde_urlencoded::from_str::<T>(query_str)
            .map_err(|e| HttpError::bad_request(format!("Invalid query parameters: {}", e)))?;
        Ok(ElifQuery(data))
    }
}

/// Framework-native Path extractor - use instead of axum::extract::Path  
#[derive(Debug)]
pub struct ElifPath<T>(pub T);

impl<T: DeserializeOwned> ElifPath<T> {
    /// Extract and deserialize path parameters from request
    pub fn from_request(request: &ElifRequest) -> HttpResult<Self> {
        // Convert HashMap to JSON for deserialization
        let json_value = serde_json::to_value(&request.path_params)
            .map_err(|e| HttpError::internal_server_error(format!("Failed to serialize path params: {}", e)))?;
        
        let data = serde_json::from_value::<T>(json_value)
            .map_err(|e| HttpError::bad_request(format!("Invalid path parameters: {}", e)))?;
        Ok(ElifPath(data))
    }
}

/// Framework-native State extractor - use instead of axum::extract::State
#[derive(Debug)]  
pub struct ElifState<T>(pub T);

impl<T: Clone> ElifState<T> {
    /// Extract state from application context
    pub fn new(state: T) -> Self {
        ElifState(state)
    }
    
    /// Get reference to inner state
    pub fn inner(&self) -> &T {
        &self.0
    }
    
    /// Get owned copy of inner state (requires Clone)
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Uri}; // TODO: Replace with framework types when available
    use std::collections::HashMap;

    #[test]
    fn test_path_param_extraction() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("slug".to_string(), "test-post".to_string());

        let request = ElifRequest::new(
            Method::GET,
            "/users/123/posts/test-post".parse().unwrap(),
            HeaderMap::new(),
        ).with_path_params(params);

        assert_eq!(request.path_param("id"), Some(&"123".to_string()));
        assert_eq!(request.path_param("slug"), Some(&"test-post".to_string()));
        assert_eq!(request.path_param("nonexistent"), None);

        // Test parsed path param
        let id: u32 = request.path_param_parsed("id").unwrap();
        assert_eq!(id, 123);
    }

    #[test]
    fn test_query_param_extraction() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "2".to_string());
        query_params.insert("per_page".to_string(), "25".to_string());
        query_params.insert("search".to_string(), "rust".to_string());

        let request = ElifRequest::new(
            Method::GET,
            "/posts?page=2&per_page=25&search=rust".parse().unwrap(),
            HeaderMap::new(),
        ).with_query_params(query_params);

        assert_eq!(request.query_param("page"), Some(&"2".to_string()));
        let page: u32 = request.query_param_required("page").unwrap();
        assert_eq!(page, 2);

        let per_page: Option<u32> = request.query_param_parsed("per_page").unwrap();
        assert_eq!(per_page, Some(25));

        assert!(request.query_param_parsed::<u32>("search").is_err()); // Should fail parsing
    }

    #[test]
    fn test_json_detection() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        let request = ElifRequest::new(
            Method::POST,
            "/api/users".parse().unwrap(),
            headers,
        );

        assert!(request.is_json());
    }

    #[test]
    fn test_bearer_token_extraction() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer abc123xyz".parse().unwrap());

        let request = ElifRequest::new(
            Method::GET,
            "/api/protected".parse().unwrap(),
            headers,
        );

        let token = request.bearer_token().unwrap().unwrap();
        assert_eq!(token, "abc123xyz");
    }
}