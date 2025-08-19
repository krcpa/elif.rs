//! Request abstraction for handling HTTP requests
//! 
//! Provides rich request parsing and data extraction capabilities.

use std::collections::HashMap;
use axum::{
    http::{HeaderValue, Uri},
    body::Bytes,
};
use serde::de::DeserializeOwned;
use crate::errors::{HttpError, HttpResult};
use super::ElifMethod;
use crate::response::ElifHeaderMap;

/// Request abstraction that wraps Axum's request types
/// with additional parsing and extraction capabilities
#[derive(Debug)]
pub struct ElifRequest {
    pub method: ElifMethod,
    pub uri: Uri,
    pub headers: ElifHeaderMap,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    body_bytes: Option<Bytes>,
}

impl ElifRequest {
    /// Create new ElifRequest from Axum components
    pub fn new(
        method: ElifMethod,
        uri: Uri,
        headers: ElifHeaderMap,
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

    /// Extract ElifRequest from request components
    pub fn extract_elif_request(
        method: ElifMethod,
        uri: Uri,
        headers: ElifHeaderMap,
        body: Option<Bytes>,
    ) -> ElifRequest {
        let mut request = ElifRequest::new(method, uri, headers);
        if let Some(body) = body {
            request = request.with_body(body);
        }
        request
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

    /// Set request body bytes (consuming)
    pub fn with_body(mut self, body: Bytes) -> Self {
        self.body_bytes = Some(body);
        self
    }

    /// Set request body bytes (borrowing - for middleware use)
    pub fn set_body(&mut self, body: Bytes) {
        self.body_bytes = Some(body);
    }

    /// Add header to request (for middleware use)
    pub fn add_header<K, V>(&mut self, key: K, value: V) -> HttpResult<()>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        use crate::response::{ElifHeaderName, ElifHeaderValue};
        
        let header_name = ElifHeaderName::from_str(key.as_ref())
            .map_err(|e| HttpError::bad_request(format!("Invalid header name: {}", e)))?;
        let header_value = ElifHeaderValue::from_str(value.as_ref())
            .map_err(|e| HttpError::bad_request(format!("Invalid header value: {}", e)))?;
        
        self.headers.insert(header_name, header_value);
        Ok(())
    }

    /// Add path parameter (for middleware use)
    pub fn add_path_param<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.path_params.insert(key.into(), value.into());
    }

    /// Add query parameter (for middleware use) 
    pub fn add_query_param<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.query_params.insert(key.into(), value.into());
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
    pub fn header(&self, name: &str) -> Option<&crate::response::ElifHeaderValue> {
        self.headers.get_str(name)
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

    /// Parse query parameters to specified type
    pub fn query<T: DeserializeOwned>(&self) -> HttpResult<T> {
        let query_str = self.query_string().unwrap_or("");
        serde_urlencoded::from_str::<T>(query_str)
            .map_err(|e| HttpError::bad_request(format!("Invalid query parameters: {}", e)))
    }

    /// Parse path parameters to specified type
    pub fn path_params<T: DeserializeOwned>(&self) -> HttpResult<T> {
        let json_value = serde_json::to_value(&self.path_params)
            .map_err(|e| HttpError::internal(format!("Failed to serialize path params: {}", e)))?;
        
        serde_json::from_value::<T>(json_value)
            .map_err(|e| HttpError::bad_request(format!("Invalid path parameters: {}", e)))
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

    /// Convert ElifRequest to Axum Request for backward compatibility
    pub(crate) fn into_axum_request(self) -> axum::extract::Request {
        use axum::body::Body;
        
        let body = match self.body_bytes {
            Some(bytes) => Body::from(bytes),
            None => Body::empty(),
        };
        
        let mut builder = axum::extract::Request::builder()
            .method(self.method.to_axum())
            .uri(self.uri);
        
        // Add headers one by one
        for (key, value) in self.headers.iter() {
            builder = builder.header(key.to_axum(), value.to_axum());
        }
        
        builder.body(body)
            .expect("Failed to construct Axum request")
    }

    /// Convert Axum Request to ElifRequest for backward compatibility
    pub(crate) async fn from_axum_request(request: axum::extract::Request) -> Self {
        use axum::body::Body;
        use axum::extract::Request;
        
        let (parts, body) = request.into_parts();
        
        // Extract body bytes
        let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => Some(bytes),
            Err(_) => None,
        };
        
        Self::extract_elif_request(
            ElifMethod::from_axum(parts.method),
            parts.uri,
            ElifHeaderMap::from_axum(parts.headers),
            body_bytes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Uri;
    use crate::response::ElifHeaderMap;
    use std::collections::HashMap;

    #[test]
    fn test_path_param_extraction() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("slug".to_string(), "test-post".to_string());

        let request = ElifRequest::new(
            ElifElifMethod::GET,
            "/users/123/posts/test-post".parse().unwrap(),
            ElifElifHeaderMap::new(),
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
            ElifElifMethod::GET,
            "/posts?page=2&per_page=25&search=rust".parse().unwrap(),
            ElifElifHeaderMap::new(),
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
        let mut headers = ElifElifHeaderMap::new();
        let header_name = crate::response::ElifHeaderName::from_str("content-type").unwrap();
        let header_value = crate::response::ElifHeaderValue::from_str("application/json").unwrap();
        headers.insert(header_name, header_value);

        let request = ElifRequest::new(
            ElifElifMethod::POST,
            "/api/users".parse().unwrap(),
            headers,
        );

        assert!(request.is_json());
    }

    #[test]
    fn test_bearer_token_extraction() {
        let mut headers = ElifElifHeaderMap::new();
        let header_name = crate::response::ElifHeaderName::from_str("authorization").unwrap();
        let header_value = crate::response::ElifHeaderValue::from_str("Bearer abc123xyz").unwrap();
        headers.insert(header_name, header_value);

        let request = ElifRequest::new(
            ElifElifMethod::GET,
            "/api/protected".parse().unwrap(),
            headers,
        );

        let token = request.bearer_token().unwrap().unwrap();
        assert_eq!(token, "abc123xyz");
    }

    #[test]
    fn test_extract_elif_request() {
        let method = ElifElifMethod::POST;
        let uri: Uri = "/test".parse().unwrap();
        let headers = ElifElifHeaderMap::new();
        let body = Some(Bytes::from("test body"));

        let request = ElifRequest::extract_elif_request(method.clone(), uri.clone(), headers.clone(), body.clone());

        assert_eq!(request.method, method);
        assert_eq!(request.uri, uri);
        assert_eq!(request.body_bytes(), body.as_ref());
    }

    #[test]
    fn test_borrowing_api_headers() {
        let mut request = ElifRequest::new(
            ElifMethod::GET,
            "/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        // Test adding headers with borrowing API
        request.add_header("x-middleware", "processed").unwrap();
        request.add_header("x-custom", "value").unwrap();
        
        assert!(request.headers.contains_key("x-middleware"));
        assert!(request.headers.contains_key("x-custom"));
        assert_eq!(request.headers.get("x-middleware").unwrap(), "processed");
    }

    #[test]
    fn test_borrowing_api_params() {
        let mut request = ElifRequest::new(
            ElifMethod::GET,
            "/users/123?page=2".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        // Test adding parameters with borrowing API
        request.add_path_param("id", "123");
        request.add_path_param("section", "profile");
        request.add_query_param("page", "2");
        request.add_query_param("limit", "10");
        
        assert_eq!(request.path_param("id"), Some(&"123".to_string()));
        assert_eq!(request.path_param("section"), Some(&"profile".to_string()));
        assert_eq!(request.query_param("page"), Some(&"2".to_string()));
        assert_eq!(request.query_param("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_borrowing_api_body() {
        let mut request = ElifRequest::new(
            ElifMethod::POST,
            "/test".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        let body_data = Bytes::from("test body content");
        request.set_body(body_data.clone());
        
        assert_eq!(request.body_bytes(), Some(&body_data));
    }

    #[test]
    fn test_borrowing_api_middleware_pattern() {
        let mut request = ElifRequest::new(
            ElifMethod::GET,
            "/api/users".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        
        // Simulate middleware adding context data
        request.add_header("x-request-id", "req-123").unwrap();
        request.add_path_param("user_id", "456");
        request.add_query_param("enriched", "true");
        
        // Verify all modifications were applied
        assert!(request.headers.contains_key("x-request-id"));
        assert_eq!(request.path_param("user_id"), Some(&"456".to_string()));
        assert_eq!(request.query_param("enriched"), Some(&"true".to_string()));
    }
}