//! Request data extractors and Simple input helpers

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::str::FromStr;
use crate::errors::{HttpError, HttpResult};
use crate::request::ElifRequest;

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
        let data = request.path_params::<T>()?;
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

// Simple input helpers for ElifRequest
impl ElifRequest {
    /// Simple input extraction with default value
    /// 
    /// Searches query parameters first, then path parameters.
    /// Returns the default value if the parameter is missing or can't be parsed.
    /// 
    /// Simple equivalent: `$request->input('page', 1)`
    pub fn input<T>(&self, key: &str, default: T) -> T 
    where 
        T: FromStr + Clone,
        T::Err: std::fmt::Debug,
    {
        self.query_param(key)
            .or_else(|| self.path_param(key))
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Simple input extraction that returns Option
    /// 
    /// Simple equivalent: `$request->input('search')`
    pub fn input_optional<T>(&self, key: &str) -> Option<T>
    where 
        T: FromStr,
        T::Err: std::fmt::Debug,
    {
        self.query_param(key)
            .or_else(|| self.path_param(key))
            .and_then(|s| s.parse().ok())
    }

    /// Extract a string input with default
    /// 
    /// Simple equivalent: `$request->input('name', 'default')`
    pub fn string(&self, key: &str, default: &str) -> String {
        self.query_param(key)
            .or_else(|| self.path_param(key))
            .map(|s| s.clone())
            .unwrap_or_else(|| default.to_string())
    }

    /// Extract an optional string input
    /// 
    /// Simple equivalent: `$request->input('search')`
    pub fn string_optional(&self, key: &str) -> Option<String> {
        self.query_param(key)
            .or_else(|| self.path_param(key))
            .map(|s| s.clone())
    }

    /// Extract an integer input with default
    /// 
    /// Simple equivalent: `$request->input('page', 1)`
    pub fn integer(&self, key: &str, default: i64) -> i64 {
        self.input(key, default)
    }

    /// Extract an optional integer input
    /// 
    /// Simple equivalent: `$request->input('limit')`
    pub fn integer_optional(&self, key: &str) -> Option<i64> {
        self.input_optional(key)
    }

    /// Extract a boolean input with default
    /// 
    /// Recognizes: "true", "1", "on", "yes" as true (case-insensitive)
    /// Simple equivalent: `$request->boolean('active', false)`
    pub fn boolean(&self, key: &str, default: bool) -> bool {
        self.query_param(key)
            .or_else(|| self.path_param(key))
            .map(|s| {
                match s.to_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => true,
                    "false" | "0" | "off" | "no" => false,
                    _ => default,
                }
            })
            .unwrap_or(default)
    }

    /// Extract multiple inputs at once as a HashMap
    /// 
    /// Simple equivalent: `$request->only(['name', 'email', 'age'])`
    pub fn inputs(&self, keys: &[&str]) -> HashMap<String, String> {
        keys.iter()
            .filter_map(|&key| {
                self.query_param(key)
                    .or_else(|| self.path_param(key))
                    .map(|val| (key.to_string(), val.clone()))
            })
            .collect()
    }

    /// Extract all query parameters as HashMap
    /// 
    /// Simple equivalent: `$request->query()`
    pub fn all_query(&self) -> HashMap<String, String> {
        self.query_params
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if a parameter exists (in query or path)
    /// 
    /// Simple equivalent: `$request->has('search')`
    pub fn has(&self, key: &str) -> bool {
        self.query_param(key).is_some() || self.path_param(key).is_some()
    }

    /// Check if a parameter exists and is not empty
    /// 
    /// Simple equivalent: `$request->filled('search')`
    pub fn filled(&self, key: &str) -> bool {
        self.query_param(key)
            .or_else(|| self.path_param(key))
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    }

    /// Get a parameter as array (comma-separated or multiple values)
    /// 
    /// Simple equivalent: `$request->input('tags', [])`
    pub fn array(&self, key: &str) -> Vec<String> {
        if let Some(value) = self.query_param(key).or_else(|| self.path_param(key)) {
            // Split by comma and clean up
            value.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Extract pagination parameters with sensible defaults
    /// 
    /// Returns (page, per_page) with defaults of (1, 10)
    /// Simple equivalent: `[$page, $perPage] = [$request->input('page', 1), $request->input('per_page', 10)]`
    pub fn pagination(&self) -> (u32, u32) {
        let page = self.input("page", 1u32).max(1);
        let per_page = self.input("per_page", 10u32).clamp(1, 100);
        (page, per_page)
    }

    /// Extract sorting parameters
    /// 
    /// Returns (sort_field, sort_direction) with defaults
    /// Simple equivalent: `[$sort, $order] = [$request->input('sort', 'id'), $request->input('order', 'asc')]`
    pub fn sorting(&self, default_field: &str) -> (String, String) {
        let sort = self.string("sort", default_field);
        let order = self.string("order", "asc");
        let direction = match order.to_lowercase().as_str() {
            "desc" | "descending" | "down" => "desc".to_string(),
            _ => "asc".to_string(),
        };
        (sort, direction)
    }

    /// Extract search and filtering parameters
    /// 
    /// Returns a HashMap of common filter parameters
    /// Simple equivalent: `$filters = $request->only(['search', 'status', 'category'])`
    pub fn filters(&self) -> HashMap<String, String> {
        self.inputs(&[
            "search", "q", "query",
            "status", "state", 
            "category", "type",
            "filter", "filters"
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize};
    use crate::request::ElifRequest;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestQuery {
        name: String,
        age: Option<u32>,
        active: Option<bool>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestPath {
        id: u32,
        slug: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestAppState {
        database_url: String,
        api_key: String,
    }

    fn create_test_request_with_query(query: &str) -> ElifRequest {
        use axum::extract::Request;
        use axum::body::Body;

        let uri = if query.is_empty() {
            "/test".to_string()
        } else {
            format!("/test?{}", query)
        };

        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        let (parts, _body) = request.into_parts();
        ElifRequest::extract_elif_request(
            crate::request::ElifMethod::from_axum(parts.method),
            parts.uri, 
            crate::response::headers::ElifHeaderMap::from_axum(parts.headers),
            None
        )
    }

    #[test]
    fn test_elif_query_extraction_success() {
        let request = create_test_request_with_query("name=John&age=30&active=true");
        let result: Result<ElifQuery<TestQuery>, _> = ElifQuery::from_request(&request);
        
        assert!(result.is_ok());
        let query = result.unwrap();
        assert_eq!(query.0.name, "John");
        assert_eq!(query.0.age, Some(30));
        assert_eq!(query.0.active, Some(true));
    }

    #[test]
    fn test_elif_query_extraction_partial() {
        let request = create_test_request_with_query("name=Alice");
        let result: Result<ElifQuery<TestQuery>, _> = ElifQuery::from_request(&request);
        
        assert!(result.is_ok());
        let query = result.unwrap();
        assert_eq!(query.0.name, "Alice");
        assert_eq!(query.0.age, None);
        assert_eq!(query.0.active, None);
    }

    #[test]
    fn test_elif_query_extraction_empty() {
        let request = create_test_request_with_query("");
        let result: Result<ElifQuery<TestQuery>, _> = ElifQuery::from_request(&request);
        
        // Should fail because 'name' is required
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpError::BadRequest { .. }));
    }

    #[test]
    fn test_elif_query_extraction_invalid_format() {
        let request = create_test_request_with_query("name=John&age=not_a_number");
        let result: Result<ElifQuery<TestQuery>, _> = ElifQuery::from_request(&request);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpError::BadRequest { .. }));
    }

    #[test]
    fn test_elif_query_url_decoding() {
        let request = create_test_request_with_query("name=John%20Doe&active=true");
        let result: Result<ElifQuery<TestQuery>, _> = ElifQuery::from_request(&request);
        
        assert!(result.is_ok());
        let query = result.unwrap();
        assert_eq!(query.0.name, "John Doe");
    }

    #[test]
    fn test_elif_state_creation_and_access() {
        let state = TestAppState {
            database_url: "postgres://localhost:5432/test".to_string(),
            api_key: "secret_key_123".to_string(),
        };
        
        let elif_state = ElifState::new(state.clone());
        
        // Test inner reference
        assert_eq!(elif_state.inner(), &state);
        
        // Test into_inner 
        let recovered_state = elif_state.into_inner();
        assert_eq!(recovered_state, state);
    }

    #[test]
    fn test_elif_state_clone_requirement() {
        #[derive(Clone, PartialEq, Debug)]
        struct CloneableState {
            value: i32,
        }
        
        let state = CloneableState { value: 42 };
        let elif_state = ElifState::new(state.clone());
        
        assert_eq!(elif_state.inner().value, 42);
        assert_eq!(elif_state.into_inner(), state);
    }

    #[test]
    fn test_elif_query_debug_impl() {
        let query = ElifQuery(TestQuery {
            name: "Test".to_string(),
            age: Some(25),
            active: Some(false),
        });
        
        let debug_string = format!("{:?}", query);
        assert!(debug_string.contains("ElifQuery"));
        assert!(debug_string.contains("Test"));
    }

    #[test]
    fn test_elif_path_debug_impl() {
        let path = ElifPath(TestPath {
            id: 123,
            slug: "test-slug".to_string(),
        });
        
        let debug_string = format!("{:?}", path);
        assert!(debug_string.contains("ElifPath"));
        assert!(debug_string.contains("123"));
        assert!(debug_string.contains("test-slug"));
    }

    #[test]
    fn test_elif_state_debug_impl() {
        let state = ElifState::new(TestAppState {
            database_url: "postgres://localhost:5432/test".to_string(),
            api_key: "secret_key_123".to_string(),
        });
        
        let debug_string = format!("{:?}", state);
        assert!(debug_string.contains("ElifState"));
    }
}