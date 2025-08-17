//! Custom test assertions for HTTP testing

use crate::{ElifResponse, errors::HttpError};
use axum::http::StatusCode;

pub trait HttpAssertions {
    fn assert_ok(&self);
    fn assert_status(&self, expected: StatusCode);
    fn assert_json_contains(&self, key: &str, value: &str);
}

impl HttpAssertions for ElifResponse {
    fn assert_ok(&self) {
        self.assert_status(StatusCode::OK);
    }

    fn assert_status(&self, expected: StatusCode) {
        assert_eq!(self.status_code(), expected, "Response status mismatch");
    }

    fn assert_json_contains(&self, _key: &str, _value: &str) {
        // Future implementation for JSON assertion
    }
}

pub trait ErrorAssertions {
    fn assert_error_code(&self, expected: &str);
    fn assert_status_code(&self, expected: StatusCode);
}

impl ErrorAssertions for HttpError {
    fn assert_error_code(&self, expected: &str) {
        assert_eq!(self.error_code(), expected, "Error code mismatch");
    }

    fn assert_status_code(&self, expected: StatusCode) {
        assert_eq!(self.status_code(), expected, "Error status code mismatch");
    }
}