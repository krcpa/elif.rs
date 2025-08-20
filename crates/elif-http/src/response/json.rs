//! JSON handling utilities for requests and responses

use axum::{
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::ops::{Deref, DerefMut};
use crate::errors::HttpResult;
use crate::response::{ElifResponse, IntoElifResponse, ElifStatusCode};

/// Enhanced JSON extractor with better error handling
#[derive(Debug)]
pub struct ElifJson<T>(pub T);

impl<T> ElifJson<T> {
    /// Create new ElifJson wrapper
    pub fn new(data: T) -> Self {
        Self(data)
    }

    /// Extract inner data
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ElifJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ElifJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for ElifJson<T> {
    fn from(data: T) -> Self {
        Self(data)
    }
}

/// JSON request extraction with enhanced error handling
#[axum::async_trait]
impl<T, S> FromRequest<S> for ElifJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(data)) => Ok(ElifJson(data)),
            Err(rejection) => Err(JsonError::from_axum_json_rejection(rejection)),
        }
    }
}

/// ElifJson to ElifResponse implementation
impl<T> IntoElifResponse for ElifJson<T>
where
    T: Serialize,
{
    fn into_response(self) -> ElifResponse {
        match ElifResponse::ok().json(&self.0) {
            Ok(response) => response,
            Err(_) => ElifResponse::internal_server_error().text("JSON serialization failed"),
        }
    }
}

/// JSON response implementation
impl<T> IntoResponse for ElifJson<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(bytes) => {
                let mut response = Response::new(bytes.into());
                response.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/json"),
                );
                response
            }
            Err(err) => {
                tracing::error!("JSON serialization failed: {}", err);
                let mut response = Response::new("Internal server error: JSON serialization failed".into());
                *response.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                response.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("text/plain"),
                );
                response
            }
        }
    }
}

/// Enhanced JSON error handling
#[derive(Debug)]
pub struct JsonError {
    pub status: ElifStatusCode,
    pub message: String,
    pub details: Option<String>,
}

impl JsonError {
    /// Create new JSON error
    pub fn new(status: ElifStatusCode, message: String) -> Self {
        Self {
            status,
            message,
            details: None,
        }
    }

    /// Create JSON error with details
    pub fn with_details(status: ElifStatusCode, message: String, details: String) -> Self {
        Self {
            status,
            message,
            details: Some(details),
        }
    }

    /// Create from Axum JSON rejection (internal use only)
    pub(crate) fn from_axum_json_rejection(rejection: axum::extract::rejection::JsonRejection) -> Self {
        use axum::extract::rejection::JsonRejection::*;
        
        match rejection {
            JsonDataError(err) => {
                Self::with_details(
                    ElifStatusCode::BAD_REQUEST,
                    "Invalid JSON data".to_string(),
                    err.to_string(),
                )
            }
            JsonSyntaxError(err) => {
                Self::with_details(
                    ElifStatusCode::BAD_REQUEST,
                    "JSON syntax error".to_string(),
                    err.to_string(),
                )
            }
            MissingJsonContentType(_) => {
                Self::new(
                    ElifStatusCode::BAD_REQUEST,
                    "Missing 'Content-Type: application/json' header".to_string(),
                )
            }
            BytesRejection(err) => {
                Self::with_details(
                    ElifStatusCode::BAD_REQUEST,
                    "Failed to read request body".to_string(),
                    err.to_string(),
                )
            }
            _ => {
                Self::new(
                    ElifStatusCode::BAD_REQUEST,
                    "Invalid JSON request".to_string(),
                )
            }
        }
    }
}

impl IntoResponse for JsonError {
    fn into_response(self) -> Response {
        let error_body = if let Some(details) = self.details {
            serde_json::json!({
                "error": {
                    "code": self.status.as_u16(),
                    "message": self.message,
                    "details": details
                }
            })
        } else {
            serde_json::json!({
                "error": {
                    "code": self.status.as_u16(),
                    "message": self.message
                }
            })
        };

        match serde_json::to_vec(&error_body) {
            Ok(bytes) => {
                let mut response = Response::new(bytes.into());
                *response.status_mut() = self.status.to_axum();
                response.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("application/json"),
                );
                response
            }
            Err(_) => {
                // Fallback error response
                let mut response = Response::new(self.message.into());
                *response.status_mut() = self.status.to_axum();
                response.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("text/plain"),
                );
                response
            }
        }
    }
}

/// JSON response helpers
pub struct JsonResponse;

impl JsonResponse {
    /// Create successful JSON response
    pub fn ok<T: Serialize>(data: &T) -> HttpResult<Response> {
        ElifResponse::json_ok(data)
    }

    /// Create JSON response with custom status
    pub fn with_status<T: Serialize>(status: ElifStatusCode, data: &T) -> HttpResult<Response> {
        ElifResponse::with_status(status).json(data)?.build()
    }

    /// Create paginated JSON response
    pub fn paginated<T: Serialize>(
        data: &[T],
        page: u32,
        per_page: u32,
        total: u64,
    ) -> HttpResult<Response> {
        let total_pages = (total as f64 / per_page as f64).ceil() as u32;
        
        let response_data = serde_json::json!({
            "data": data,
            "pagination": {
                "page": page,
                "per_page": per_page,
                "total": total,
                "total_pages": total_pages,
                "has_next": page < total_pages,
                "has_prev": page > 1
            }
        });

        ElifResponse::ok().json_value(response_data).build()
    }

    /// Create error response with JSON body
    pub fn error(status: ElifStatusCode, message: &str) -> HttpResult<Response> {
        ElifResponse::json_error(status, message)
    }

    /// Create validation error response
    pub fn validation_error<T: Serialize>(errors: &T) -> HttpResult<Response> {
        ElifResponse::validation_error(errors)
    }

    /// Create API success response with message
    pub fn success_message(message: &str) -> HttpResult<Response> {
        let response_data = serde_json::json!({
            "success": true,
            "message": message
        });

        ElifResponse::ok().json_value(response_data).build()
    }

    /// Create created resource response
    pub fn created<T: Serialize>(data: &T) -> HttpResult<Response> {
        ElifResponse::created().json(data)?.build()
    }

    /// Create no content response (for DELETE operations)
    pub fn no_content() -> HttpResult<Response> {
        ElifResponse::no_content().build()
    }
}

/// Validation error types for JSON responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationErrors {
    pub errors: std::collections::HashMap<String, Vec<String>>,
}

impl ValidationErrors {
    /// Create new validation errors container
    pub fn new() -> Self {
        Self {
            errors: std::collections::HashMap::new(),
        }
    }

    /// Add error for a field
    pub fn add_error(&mut self, field: String, error: String) {
        self.errors.entry(field).or_insert_with(Vec::new).push(error);
    }

    /// Add multiple errors for a field
    pub fn add_errors(&mut self, field: String, errors: Vec<String>) {
        self.errors.entry(field).or_insert_with(Vec::new).extend(errors);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.values().map(|v| v.len()).sum()
    }

    /// Convert to JSON response
    pub fn to_response(self) -> HttpResult<Response> {
        JsonResponse::validation_error(&self)
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// API response wrapper for consistent JSON responses
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub errors: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create successful API response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            errors: None,
        }
    }

    /// Create successful API response with message
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            errors: None,
        }
    }

    /// Create error API response
    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
            errors: None,
        }
    }

    /// Create error API response with validation errors
    pub fn validation_error(message: String, errors: serde_json::Value) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
            errors: Some(errors),
        }
    }

    /// Convert to HTTP response
    pub fn to_response(self) -> HttpResult<Response> {
        let status = if self.success {
            ElifStatusCode::OK
        } else {
            ElifStatusCode::BAD_REQUEST
        };

        ElifResponse::with_status(status).json(&self)?.build()
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        match self.to_response() {
            Ok(response) => response,
            Err(e) => {
                tracing::error!("Failed to create API response: {}", e);
                (ElifStatusCode::INTERNAL_SERVER_ERROR.to_axum(), "Internal server error").into_response()
            }
        }
    }
}