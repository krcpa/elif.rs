//! HTTP error response formatting

use super::HttpError;
use crate::response::{ElifResponse, IntoElifResponse};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

impl HttpError {
    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            HttpError::StartupFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ShutdownFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ConfigError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ServiceResolutionFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            HttpError::RequestTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            HttpError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            HttpError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::HealthCheckFailed { .. } => StatusCode::SERVICE_UNAVAILABLE,
            HttpError::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ValidationError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            HttpError::NotFound { .. } => StatusCode::NOT_FOUND,
            HttpError::Conflict { .. } => StatusCode::CONFLICT,
            HttpError::Unauthorized => StatusCode::UNAUTHORIZED,
            HttpError::Forbidden { .. } => StatusCode::FORBIDDEN,
        }
    }

    /// Get error hint for user guidance
    pub fn error_hint(&self) -> Option<&'static str> {
        match &self {
            HttpError::RequestTooLarge { .. } => Some("Reduce request payload size"),
            HttpError::RequestTimeout => Some("Retry the request"),
            HttpError::BadRequest { .. } => Some("Check request format and parameters"),
            HttpError::HealthCheckFailed { .. } => Some("Server may be starting up or experiencing issues"),
            _ => None,
        }
    }
}

// Implement IntoElifResponse for HttpError
impl IntoElifResponse for HttpError {
    fn into_response(self) -> ElifResponse {
        let body = json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "hint": self.error_hint()
            }
        });

        ElifResponse::with_status(self.status_code())
            .json_value(body)
    }
}

// Implement IntoResponse for automatic HTTP error responses (Axum compatibility)
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "hint": self.error_hint()
            }
        });

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(HttpError::bad_request("test").status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(HttpError::RequestTimeout.status_code(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(
            HttpError::RequestTooLarge { size: 100, limit: 50 }.status_code(), 
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            HttpError::health_check("Database unavailable").status_code(), 
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_validation_error_status_code() {
        let validation_error = HttpError::validation_error("Field is required");
        assert_eq!(validation_error.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_error_hints() {
        let timeout_error = HttpError::RequestTimeout;
        assert_eq!(timeout_error.error_hint(), Some("Retry the request"));

        let large_request_error = HttpError::RequestTooLarge { size: 100, limit: 50 };
        assert_eq!(large_request_error.error_hint(), Some("Reduce request payload size"));

        let not_found_error = HttpError::not_found("User");
        assert_eq!(not_found_error.error_hint(), None);
    }

    #[test]
    fn test_error_response_format_consistency() {
        let error = HttpError::not_found("User");
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}