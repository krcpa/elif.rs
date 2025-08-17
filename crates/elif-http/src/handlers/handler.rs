//! Handler abstraction for elif.rs framework
//!
//! This module provides a bridge between elif types and Axum's handler system.

use crate::request::ElifRequest;
use crate::response::{ElifResponse, IntoElifResponse};
use crate::errors::{HttpError, HttpResult};
use axum::{
    extract::{Request as AxumRequest},
    response::{Response as AxumResponse, IntoResponse},
    handler::Handler as AxumHandler,
};
use std::future::Future;
use std::collections::HashMap;

/// Trait for elif handlers that work with ElifRequest/ElifResponse
pub trait ElifHandler<T> {
    type Output: IntoElifResponse + Send;
    type Future: Future<Output = HttpResult<Self::Output>> + Send;

    fn call(self, request: ElifRequest) -> Self::Future;
}

/// Implement ElifHandler for functions that take ElifRequest
impl<F, Fut, R> ElifHandler<(ElifRequest,)> for F
where
    F: FnOnce(ElifRequest) -> Fut + Send,
    Fut: Future<Output = HttpResult<R>> + Send,
    R: IntoElifResponse + Send,
{
    type Output = R;
    type Future = Fut;

    fn call(self, request: ElifRequest) -> Self::Future {
        self(request)
    }
}

/// Wrapper struct that implements the Handler trait
pub struct ElifHandlerWrapper<F, Fut, R> 
where
    F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
    Fut: Future<Output = HttpResult<R>> + Send + 'static,
    R: IntoElifResponse + Send + 'static,
{
    handler: F,
}

impl<F, Fut, R> Clone for ElifHandlerWrapper<F, Fut, R>
where
    F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
    Fut: Future<Output = HttpResult<R>> + Send + 'static,
    R: IntoElifResponse + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
        }
    }
}

impl<F, Fut, R, S> AxumHandler<(), S> for ElifHandlerWrapper<F, Fut, R>
where
    F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
    Fut: Future<Output = HttpResult<R>> + Send + 'static,
    R: IntoElifResponse + Send + 'static,
    S: Send + Sync + 'static,
{
    type Future = std::pin::Pin<Box<dyn Future<Output = AxumResponse> + Send>>;

    fn call(self, req: AxumRequest, _state: S) -> Self::Future {
        Box::pin(async move {
            // Convert Axum request to ElifRequest
            let (parts, body) = req.into_parts();
            
            // Extract body bytes
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            };
            
            // Extract query parameters from URI
            let query_params = if let Some(query) = parts.uri.query() {
                serde_urlencoded::from_str::<HashMap<String, String>>(query)
                    .unwrap_or_default()
            } else {
                HashMap::new()
            };
            
            let elif_request = ElifRequest::extract_elif_request(
                parts.method,
                parts.uri,
                parts.headers,
                body_bytes,
            ).with_query_params(query_params);
            
            match (self.handler)(elif_request).await {
                Ok(response) => {
                    let elif_response = response.into_response();
                    convert_elif_to_axum_response(elif_response)
                }
                Err(error) => {
                    let error_response = crate::response::IntoElifResponse::into_response(error);
                    convert_elif_to_axum_response(error_response)
                }
            }
        })
    }
}

/// Convert elif handler to Axum handler for any state
pub fn elif_handler<F, Fut, R>(handler: F) -> ElifHandlerWrapper<F, Fut, R>
where
    F: Fn(ElifRequest) -> Fut + Send + Clone + 'static,
    Fut: Future<Output = HttpResult<R>> + Send + 'static,
    R: IntoElifResponse + Send + 'static,
{
    ElifHandlerWrapper { handler }
}

/// Convert ElifResponse to Axum Response
fn convert_elif_to_axum_response(elif_response: ElifResponse) -> AxumResponse {
    // ElifResponse already implements IntoResponse for Axum
    use axum::response::IntoResponse as AxumIntoResponse;
    AxumIntoResponse::into_response(elif_response)
}

/// Macro to create elif handlers more easily
#[macro_export]
macro_rules! elif_route {
    ($handler:expr) => {
        $crate::handlers::elif_handler($handler)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ElifStatusCode;

    async fn test_handler(_req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("Hello, World!"))
    }

    #[test]
    fn test_elif_handler_conversion() {
        let handler = elif_handler(test_handler);
        
        // This test verifies the handler compiles and can be used
        // Full integration testing would require setting up Axum routing
        assert!(true);
    }
}