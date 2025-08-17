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