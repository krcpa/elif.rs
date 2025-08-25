//! HTTP error response formatting

use super::HttpError;
use crate::response::{ElifResponse, ElifStatusCode, IntoElifResponse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

impl HttpError {
    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> ElifStatusCode {
        match self {
            HttpError::StartupFailed { .. } => ElifStatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ShutdownFailed { .. } => ElifStatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ConfigError { .. } => ElifStatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ServiceResolutionFailed { .. } => ElifStatusCode::INTERNAL_SERVER_ERROR,
            HttpError::RequestTimeout => ElifStatusCode::REQUEST_TIMEOUT,
            HttpError::RequestTooLarge { .. } => ElifStatusCode::PAYLOAD_TOO_LARGE,
            HttpError::BadRequest { .. } => ElifStatusCode::BAD_REQUEST,
            HttpError::InternalError { .. } => ElifStatusCode::INTERNAL_SERVER_ERROR,
            HttpError::HealthCheckFailed { .. } => ElifStatusCode::SERVICE_UNAVAILABLE,
            HttpError::DatabaseError { .. } => ElifStatusCode::INTERNAL_SERVER_ERROR,
            HttpError::ValidationError { .. } => ElifStatusCode::UNPROCESSABLE_ENTITY,
            HttpError::NotFound { .. } => ElifStatusCode::NOT_FOUND,
            HttpError::Conflict { .. } => ElifStatusCode::CONFLICT,
            HttpError::Unauthorized => ElifStatusCode::UNAUTHORIZED,
            HttpError::Forbidden { .. } => ElifStatusCode::FORBIDDEN,
        }
    }

    /// Get error hint for user guidance
    pub fn error_hint(&self) -> Option<&'static str> {
        match &self {
            HttpError::RequestTooLarge { .. } => Some("Reduce request payload size"),
            HttpError::RequestTimeout => Some("Retry the request"),
            HttpError::BadRequest { .. } => Some("Check request format and parameters"),
            HttpError::HealthCheckFailed { .. } => {
                Some("Server may be starting up or experiencing issues")
            }
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

        ElifResponse::with_status(self.status_code()).json_value(body)
    }
}

// Implement IntoResponse for automatic HTTP error responses (Axum compatibility)
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = self.status_code().to_axum(); // Convert to Axum StatusCode
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
        assert_eq!(
            HttpError::bad_request("test").status_code(),
            crate::response::status::ElifStatusCode::BAD_REQUEST
        );
        assert_eq!(
            HttpError::RequestTimeout.status_code(),
            crate::response::status::ElifStatusCode::REQUEST_TIMEOUT
        );
        assert_eq!(
            HttpError::RequestTooLarge {
                size: 100,
                limit: 50
            }
            .status_code(),
            crate::response::status::ElifStatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            HttpError::health_check("Database unavailable").status_code(),
            crate::response::status::ElifStatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_validation_error_status_code() {
        let validation_error = HttpError::validation_error("Field is required");
        assert_eq!(
            validation_error.status_code(),
            crate::response::status::ElifStatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn test_error_hints() {
        let timeout_error = HttpError::RequestTimeout;
        assert_eq!(timeout_error.error_hint(), Some("Retry the request"));

        let large_request_error = HttpError::RequestTooLarge {
            size: 100,
            limit: 50,
        };
        assert_eq!(
            large_request_error.error_hint(),
            Some("Reduce request payload size")
        );

        let not_found_error = HttpError::not_found("User");
        assert_eq!(not_found_error.error_hint(), None);
    }

    #[test]
    fn test_error_response_format_consistency() {
        use axum::response::IntoResponse as AxumIntoResponse;

        let error = HttpError::not_found("User");
        let response = AxumIntoResponse::into_response(error);

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
