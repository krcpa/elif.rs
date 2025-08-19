//! Request data extractors

use serde::de::DeserializeOwned;
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