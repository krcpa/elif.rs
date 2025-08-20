//! Request abstraction for handling HTTP requests
//! 
//! Provides rich request parsing and data extraction capabilities.

use std::collections::HashMap;
use std::any::{Any, TypeId};
use axum::{
    http::Uri,
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
    pub extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
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
            extensions: HashMap::new(),
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
    
    /// Parse JSON body to specified type (async version for consistency)
    pub async fn json_async<T: DeserializeOwned>(&self) -> HttpResult<T> {
        self.json()
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
    
    /// Get a query parameter as a specific type
    pub fn query_param_as<T>(&self, name: &str) -> HttpResult<Option<T>>
    where 
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        match self.query_param(name) {
            Some(param) => {
                let parsed = param.parse::<T>()
                    .map_err(|e| HttpError::bad_request(format!("Invalid {} query parameter '{}': {}", name, param, e)))?;
                Ok(Some(parsed))
            }
            None => Ok(None)
        }
    }

    /// Get User-Agent header
    pub fn user_agent(&self) -> Option<String> {
        self.header_string("user-agent").unwrap_or(None)
    }

    /// Get Authorization header
    pub fn authorization(&self) -> Option<String> {
        self.header_string("authorization").unwrap_or(None)
    }

    /// Extract Bearer token from Authorization header
    pub fn bearer_token(&self) -> Option<String> {
        if let Some(auth) = self.authorization() {
            if auth.starts_with("Bearer ") {
                Some(auth[7..].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get request IP address from headers or connection
    pub fn client_ip(&self) -> Option<String> {
        // Try common forwarded headers first
        if let Ok(Some(forwarded)) = self.header_string("x-forwarded-for") {
            // Take first IP if multiple
            if let Some(ip) = forwarded.split(',').next() {
                return Some(ip.trim().to_string());
            }
        }
        
        if let Ok(Some(real_ip)) = self.header_string("x-real-ip") {
            return Some(real_ip);
        }
        
        // Could extend with connection info if available
        None
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

    /// Get a reference to the extensions map for reading middleware-added data
    pub fn extensions(&self) -> &HashMap<TypeId, Box<dyn Any + Send + Sync>> {
        &self.extensions
    }

    /// Get a mutable reference to the extensions map for adding middleware data
    pub fn extensions_mut(&mut self) -> &mut HashMap<TypeId, Box<dyn Any + Send + Sync>> {
        &mut self.extensions
    }

    /// Insert typed data into request extensions (helper for middleware)
    pub fn insert_extension<T: Send + Sync + 'static>(&mut self, data: T) {
        self.extensions.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Get typed data from request extensions (helper for middleware)
    pub fn get_extension<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|any| any.downcast_ref::<T>())
    }
}

/// Enhanced parameter extraction methods with better error handling and type safety
impl ElifRequest {
    /// Extract and parse a path parameter with proper error handling
    /// 
    /// This is the preferred method for extracting path parameters as it provides
    /// better error messages and type safety compared to the legacy methods.
    pub fn path_param_typed<T>(&self, name: &str) -> Result<T, crate::request::pipeline::ParamError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Debug + std::fmt::Display,
    {
        crate::request::pipeline::parameter_extraction::extract_path_param(self, name)
    }

    /// Extract and parse a path parameter as i32
    pub fn path_param_int(&self, name: &str) -> Result<i32, crate::request::pipeline::ParamError> {
        self.path_param_typed(name)
    }

    /// Extract and parse a path parameter as u32
    pub fn path_param_u32(&self, name: &str) -> Result<u32, crate::request::pipeline::ParamError> {
        self.path_param_typed(name)
    }

    /// Extract and parse a path parameter as i64
    pub fn path_param_i64(&self, name: &str) -> Result<i64, crate::request::pipeline::ParamError> {
        self.path_param_typed(name)
    }

    /// Extract and parse a path parameter as u64
    pub fn path_param_u64(&self, name: &str) -> Result<u64, crate::request::pipeline::ParamError> {
        self.path_param_typed(name)
    }

    /// Extract and parse a path parameter as UUID
    pub fn path_param_uuid(&self, name: &str) -> Result<uuid::Uuid, crate::request::pipeline::ParamError> {
        self.path_param_typed(name)
    }

    /// Extract and parse a path parameter as String (validates non-empty)
    pub fn path_param_string(&self, name: &str) -> Result<String, crate::request::pipeline::ParamError> {
        let value: String = self.path_param_typed(name)?;
        if value.is_empty() {
            return Err(crate::request::pipeline::ParamError::ParseError {
                param: name.to_string(),
                value: value.clone(),
                error: "Parameter cannot be empty".to_string(),
            });
        }
        Ok(value)
    }

    /// Extract and parse an optional query parameter with proper error handling
    pub fn query_param_typed_new<T>(&self, name: &str) -> Result<Option<T>, crate::request::pipeline::ParamError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Debug + std::fmt::Display,
    {
        crate::request::pipeline::parameter_extraction::extract_query_param(self, name)
    }

    /// Extract and parse a required query parameter with proper error handling  
    pub fn query_param_required_typed<T>(&self, name: &str) -> Result<T, crate::request::pipeline::ParamError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Debug + std::fmt::Display,
    {
        crate::request::pipeline::parameter_extraction::extract_required_query_param(self, name)
    }

    /// Extract query parameter as optional i32
    pub fn query_param_int_new(&self, name: &str) -> Result<Option<i32>, crate::request::pipeline::ParamError> {
        self.query_param_typed_new(name)
    }

    /// Extract query parameter as required i32
    pub fn query_param_int_required(&self, name: &str) -> Result<i32, crate::request::pipeline::ParamError> {
        self.query_param_required_typed(name)
    }

    /// Extract query parameter as optional u32
    pub fn query_param_u32_new(&self, name: &str) -> Result<Option<u32>, crate::request::pipeline::ParamError> {
        self.query_param_typed_new(name)
    }

    /// Extract query parameter as required u32
    pub fn query_param_u32_required(&self, name: &str) -> Result<u32, crate::request::pipeline::ParamError> {
        self.query_param_required_typed(name)
    }

    /// Extract query parameter as optional bool
    pub fn query_param_bool_new(&self, name: &str) -> Result<Option<bool>, crate::request::pipeline::ParamError> {
        self.query_param_typed_new(name)
    }

    /// Extract query parameter as required bool
    pub fn query_param_bool_required(&self, name: &str) -> Result<bool, crate::request::pipeline::ParamError> {
        self.query_param_required_typed(name)
    }

    /// Extract query parameter as optional String
    pub fn query_param_string_new(&self, name: &str) -> Result<Option<String>, crate::request::pipeline::ParamError> {
        self.query_param_typed_new(name)
    }

    /// Extract query parameter as required String
    pub fn query_param_string_required(&self, name: &str) -> Result<String, crate::request::pipeline::ParamError> {
        self.query_param_required_typed(name)
    }

    /// Validate that path parameter exists and is not empty
    pub fn validate_path_param(&self, name: &str) -> Result<&String, crate::request::pipeline::ParamError> {
        let param = self.path_params.get(name)
            .ok_or_else(|| crate::request::pipeline::ParamError::Missing(name.to_string()))?;
        
        if param.is_empty() {
            return Err(crate::request::pipeline::ParamError::ParseError {
                param: name.to_string(),
                value: param.clone(),
                error: "Parameter cannot be empty".to_string(),
            });
        }
        
        Ok(param)
    }

    /// Check if request has a specific path parameter
    pub fn has_path_param(&self, name: &str) -> bool {
        self.path_params.contains_key(name)
    }

    /// Check if request has a specific query parameter
    pub fn has_query_param(&self, name: &str) -> bool {
        self.query_params.contains_key(name)
    }

    /// Get all path parameter names
    pub fn path_param_names(&self) -> Vec<&String> {
        self.path_params.keys().collect()
    }

    /// Get all query parameter names
    pub fn query_param_names(&self) -> Vec<&String> {
        self.query_params.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Uri;
    use crate::response::ElifHeaderMap;
    use std::collections::HashMap;

    #[test]
    fn test_new_path_param_methods() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("slug".to_string(), "test-post".to_string());

        let mut request = ElifRequest::new(
            ElifMethod::GET,
            "/users/123/posts/test-post".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        request.path_params = params;

        // Test existing convenient methods
        assert_eq!(request.path_param("id"), Some(&"123".to_string()));
        assert_eq!(request.path_param("slug"), Some(&"test-post".to_string()));
        assert_eq!(request.path_param("nonexistent"), None);

        // Test typed path params (existing method)
        let id: u32 = request.path_param_parsed("id").unwrap();
        assert_eq!(id, 123);
        
        let slug: String = request.path_param_parsed("slug").unwrap();
        assert_eq!(slug, "test-post");
        
        // Test error on invalid type conversion
        assert!(request.path_param_parsed::<u32>("slug").is_err());
    }
    
    #[test]
    fn test_query_param_methods() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "2".to_string());
        query_params.insert("search".to_string(), "hello world".to_string());

        let mut request = ElifRequest::new(
            ElifMethod::GET,
            "/search?page=2&search=hello%20world".parse().unwrap(),
            ElifHeaderMap::new(),
        );
        request.query_params = query_params;

        // Test query param access
        assert_eq!(request.query_param("page"), Some(&"2".to_string()));
        assert_eq!(request.query_param("search"), Some(&"hello world".to_string()));
        assert_eq!(request.query_param("nonexistent"), None);

        // Test typed query params
        let page: Option<u32> = request.query_param_as("page").unwrap();
        assert_eq!(page, Some(2));
        
        let nonexistent: Option<u32> = request.query_param_as("nonexistent").unwrap();
        assert_eq!(nonexistent, None);
        
        // Test error on invalid type conversion
        assert!(request.query_param_as::<u32>("search").is_err());
    }
    
    #[test]
    fn test_header_method() {
        let mut headers = ElifHeaderMap::new();
        headers.insert("Content-Type".parse().unwrap(), "application/json".parse().unwrap());
        headers.insert("User-Agent".parse().unwrap(), "test-client/1.0".parse().unwrap());

        let request = ElifRequest::new(
            ElifMethod::POST,
            "/api/test".parse().unwrap(),
            headers,
        );

        // Test header access
        assert_eq!(request.header_string("content-type").unwrap(), Some("application/json".to_string()));
        assert_eq!(request.header_string("user-agent").unwrap(), Some("test-client/1.0".to_string()));
        assert_eq!(request.header_string("nonexistent").unwrap(), None);
    }

    #[test]
    fn test_query_param_extraction() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "2".to_string());
        query_params.insert("per_page".to_string(), "25".to_string());
        query_params.insert("search".to_string(), "rust".to_string());

        let request = ElifRequest::new(
            ElifMethod::GET,
            "/posts?page=2&per_page=25&search=rust".parse().unwrap(),
            ElifHeaderMap::new(),
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
        let mut headers = ElifHeaderMap::new();
        let header_name = crate::response::ElifHeaderName::from_str("content-type").unwrap();
        let header_value = crate::response::ElifHeaderValue::from_str("application/json").unwrap();
        headers.insert(header_name, header_value);

        let request = ElifRequest::new(
            ElifMethod::POST,
            "/api/users".parse().unwrap(),
            headers,
        );

        assert!(request.is_json());
    }

    #[test]
    fn test_bearer_token_extraction() {
        let mut headers = ElifHeaderMap::new();
        let header_name = crate::response::ElifHeaderName::from_str("authorization").unwrap();
        let header_value = crate::response::ElifHeaderValue::from_str("Bearer abc123xyz").unwrap();
        headers.insert(header_name, header_value);

        let request = ElifRequest::new(
            ElifMethod::GET,
            "/api/protected".parse().unwrap(),
            headers,
        );

        let token = request.bearer_token().unwrap();
        assert_eq!(token, "abc123xyz");
    }

    #[test]
    fn test_extract_elif_request() {
        let method = ElifMethod::POST;
        let uri: Uri = "/test".parse().unwrap();
        let headers = ElifHeaderMap::new();
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
        
        let middleware_header = crate::response::headers::ElifHeaderName::from_str("x-middleware").unwrap();
        let custom_header = crate::response::headers::ElifHeaderName::from_str("x-custom").unwrap();
        assert!(request.headers.contains_key(&middleware_header));
        assert!(request.headers.contains_key(&custom_header));
        assert_eq!(request.headers.get(&middleware_header).unwrap().to_str().unwrap(), "processed");
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
        let request_id_header = crate::response::headers::ElifHeaderName::from_str("x-request-id").unwrap();
        assert!(request.headers.contains_key(&request_id_header));
        assert_eq!(request.path_param("user_id"), Some(&"456".to_string()));
        assert_eq!(request.query_param("enriched"), Some(&"true".to_string()));
    }
}