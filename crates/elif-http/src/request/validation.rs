//! Request validation utilities

use crate::errors::{HttpError, HttpResult};

/// Validation trait for request data
pub trait Validate {
    fn validate(&self) -> HttpResult<()>;
}

/// Helper functions for common validation patterns
pub fn validate_required<T>(field: &Option<T>, field_name: &str) -> HttpResult<()> {
    if field.is_none() {
        return Err(HttpError::bad_request(format!("{} is required", field_name)));
    }
    Ok(())
}

pub fn validate_min_length(value: &str, min: usize, field_name: &str) -> HttpResult<()> {
    if value.len() < min {
        return Err(HttpError::bad_request(format!("{} must be at least {} characters long", field_name, min)));
    }
    Ok(())
}

pub fn validate_max_length(value: &str, max: usize, field_name: &str) -> HttpResult<()> {
    if value.len() > max {
        return Err(HttpError::bad_request(format!("{} must be at most {} characters long", field_name, max)));
    }
    Ok(())
}

pub fn validate_email(email: &str, field_name: &str) -> HttpResult<()> {
    if !email.contains('@') || !email.contains('.') {
        return Err(HttpError::bad_request(format!("{} must be a valid email address", field_name)));
    }
    Ok(())
}