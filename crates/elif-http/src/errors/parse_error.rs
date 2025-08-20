//! Framework-native parsing error types
//! 
//! These errors replace Axum error type exposures in public APIs.
//! They represent parsing/validation failures that occur during request processing.

use thiserror::Error;

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Framework-native parsing errors (replaces Axum error exposures)
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid HTTP method: {method}")]
    InvalidMethod { method: String },
    
    #[error("Invalid header name: {name}")]
    InvalidHeaderName { name: String },
    
    #[error("Invalid header value: {value}")]
    InvalidHeaderValue { value: String },
    
    #[error("Header value contains non-ASCII characters")]
    HeaderToStrError,
    
    #[error("Invalid status code: {code}")]
    InvalidStatusCode { code: u16 },
    
    #[error("JSON parsing failed: {message}")]
    JsonRejection { message: String },
}

impl ParseError {
    /// Create an invalid method error
    pub fn invalid_method<T: Into<String>>(method: T) -> Self {
        ParseError::InvalidMethod { 
            method: method.into() 
        }
    }
    
    /// Create an invalid header name error
    pub fn invalid_header_name<T: Into<String>>(name: T) -> Self {
        ParseError::InvalidHeaderName { 
            name: name.into() 
        }
    }
    
    /// Create an invalid header value error
    pub fn invalid_header_value<T: Into<String>>(value: T) -> Self {
        ParseError::InvalidHeaderValue { 
            value: value.into() 
        }
    }
    
    /// Create a header to string error
    pub fn header_to_str_error() -> Self {
        ParseError::HeaderToStrError
    }
    
    /// Create an invalid status code error
    pub fn invalid_status_code(code: u16) -> Self {
        ParseError::InvalidStatusCode { code }
    }
    
    /// Create a JSON rejection error
    pub fn json_rejection<T: Into<String>>(message: T) -> Self {
        ParseError::JsonRejection { 
            message: message.into() 
        }
    }
}

// Convert from Axum error types to framework-native errors
impl From<axum::http::method::InvalidMethod> for ParseError {
    fn from(err: axum::http::method::InvalidMethod) -> Self {
        ParseError::InvalidMethod { 
            method: err.to_string() 
        }
    }
}

impl From<axum::http::header::InvalidHeaderName> for ParseError {
    fn from(err: axum::http::header::InvalidHeaderName) -> Self {
        ParseError::InvalidHeaderName { 
            name: err.to_string() 
        }
    }
}

impl From<axum::http::header::InvalidHeaderValue> for ParseError {
    fn from(err: axum::http::header::InvalidHeaderValue) -> Self {
        ParseError::InvalidHeaderValue { 
            value: err.to_string() 
        }
    }
}

impl From<axum::http::header::ToStrError> for ParseError {
    fn from(_: axum::http::header::ToStrError) -> Self {
        ParseError::HeaderToStrError
    }
}

// Note: From<InvalidStatusCode> implementation removed because it hardcodes code: 0
// which produces incorrect error messages. Use ParseError::invalid_status_code(actual_code) instead.

impl From<axum::extract::rejection::JsonRejection> for ParseError {
    fn from(err: axum::extract::rejection::JsonRejection) -> Self {
        ParseError::JsonRejection { 
            message: err.to_string() 
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_creation() {
        let error = ParseError::invalid_method("INVALID");
        assert!(matches!(error, ParseError::InvalidMethod { .. }));
        assert_eq!(error.to_string(), "Invalid HTTP method: INVALID");
    }

    #[test]
    fn test_header_errors() {
        let name_error = ParseError::invalid_header_name("bad name");
        let value_error = ParseError::invalid_header_value("bad\x00value");
        let str_error = ParseError::header_to_str_error();
        
        assert!(matches!(name_error, ParseError::InvalidHeaderName { .. }));
        assert!(matches!(value_error, ParseError::InvalidHeaderValue { .. }));
        assert!(matches!(str_error, ParseError::HeaderToStrError));
    }

    #[test]
    fn test_status_code_error() {
        let error = ParseError::invalid_status_code(999);
        assert!(matches!(error, ParseError::InvalidStatusCode { code: 999 }));
    }

    #[test]
    fn test_json_error() {
        let error = ParseError::json_rejection("Invalid JSON syntax");
        assert!(matches!(error, ParseError::JsonRejection { .. }));
    }

    #[test]
    fn test_axum_conversions() {
        // Test that we can convert from Axum errors
        let axum_method_err = axum::http::Method::from_bytes(b"INVALID METHOD WITH SPACES").unwrap_err();
        let parse_err: ParseError = axum_method_err.into();
        assert!(matches!(parse_err, ParseError::InvalidMethod { .. }));
    }
}