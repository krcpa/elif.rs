//! Request data extractors for handler parameters

use crate::foundation::RequestExtractor;
use crate::request::ElifRequest;
use crate::errors::HttpError;

/// Extract query parameters from the request
pub struct QueryExtractor<T> {
    data: T,
}

impl<T> RequestExtractor for QueryExtractor<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Error = HttpError;

    fn extract(request: &ElifRequest) -> Result<Self, Self::Error> {
        let data = request.query()
            .map_err(|e| HttpError::bad_request(format!("Invalid query parameters: {}", e)))?;
        Ok(QueryExtractor { data })
    }
}

impl<T> std::ops::Deref for QueryExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// Extract path parameters from the request
pub struct PathExtractor<T> {
    data: T,
}

impl<T> RequestExtractor for PathExtractor<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Error = HttpError;

    fn extract(request: &ElifRequest) -> Result<Self, Self::Error> {
        let data = request.path_params()?;
        Ok(PathExtractor { data })
    }
}

impl<T> std::ops::Deref for PathExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// Extract JSON body from the request
pub struct JsonExtractor<T> {
    data: T,
}

impl<T> RequestExtractor for JsonExtractor<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Error = HttpError;

    fn extract(request: &ElifRequest) -> Result<Self, Self::Error> {
        let data = request.json()
            .map_err(|e| HttpError::bad_request(format!("Invalid JSON body: {}", e)))?;
        Ok(JsonExtractor { data })
    }
}

impl<T> std::ops::Deref for JsonExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}