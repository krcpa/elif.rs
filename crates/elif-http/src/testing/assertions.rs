//! Custom test assertions for HTTP testing

use crate::{ElifResponse, response::ElifStatusCode, errors::HttpError};

pub trait HttpAssertions {
    fn assert_ok(&self);
    fn assert_status(&self, expected: ElifStatusCode);
    fn assert_json_contains(&self, key: &str, value: &str);
}

impl HttpAssertions for ElifResponse {
    fn assert_ok(&self) {
        self.assert_status(ElifStatusCode::OK);
    }

    fn assert_status(&self, expected: ElifStatusCode) {
        assert_eq!(self.status_code(), expected, "Response status mismatch");
    }

    fn assert_json_contains(&self, _key: &str, _value: &str) {
        // Future implementation for JSON assertion
    }
}

pub trait ErrorAssertions {
    fn assert_error_code(&self, expected: &str);
    fn assert_status_code(&self, expected: ElifStatusCode);
}

impl ErrorAssertions for HttpError {
    fn assert_error_code(&self, expected: &str) {
        assert_eq!(self.error_code(), expected, "Error code mismatch");
    }

    fn assert_status_code(&self, expected: ElifStatusCode) {
        assert_eq!(self.status_code(), expected, "Error status code mismatch");
    }
}