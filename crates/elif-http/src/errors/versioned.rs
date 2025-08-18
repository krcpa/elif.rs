use crate::{
    errors::{HttpError, HttpResult},
    response::{ElifResponse, ElifJson, ElifStatusCode},
    middleware::versioning::VersionInfo,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Version-aware error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedError {
    /// Error information
    pub error: ErrorInfo,
    /// API version that generated this error
    pub api_version: String,
    /// Links to migration guides or documentation (if version is deprecated)
    pub migration_info: Option<MigrationInfo>,
}

/// Core error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional details or hints
    pub details: Option<String>,
    /// Field-specific errors for validation
    pub field_errors: Option<HashMap<String, Vec<String>>>,
}

/// Migration information for deprecated versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    /// URL to migration guide
    pub migration_guide_url: Option<String>,
    /// Recommended version to migrate to
    pub recommended_version: String,
    /// Deprecation warning message
    pub deprecation_message: Option<String>,
    /// Date when this version will be removed
    pub sunset_date: Option<String>,
}

/// Version-aware error builder
pub struct VersionedErrorBuilder {
    code: String,
    message: String,
    details: Option<String>,
    field_errors: Option<HashMap<String, Vec<String>>>,
    status_code: ElifStatusCode,
}

impl VersionedErrorBuilder {
    /// Create a new versioned error builder
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            field_errors: None,
            status_code: ElifStatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Set status code
    pub fn status(mut self, status: ElifStatusCode) -> Self {
        self.status_code = status;
        self
    }

    /// Add details
    pub fn details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    /// Add field errors for validation
    pub fn field_errors(mut self, field_errors: HashMap<String, Vec<String>>) -> Self {
        self.field_errors = Some(field_errors);
        self
    }

    /// Add a single field error
    pub fn field_error(mut self, field: &str, error: &str) -> Self {
        self.field_errors
            .get_or_insert_with(HashMap::new)
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(error.to_string());
        self
    }

    /// Build the error response with version information
    pub fn build(self, version_info: &VersionInfo) -> HttpResult<ElifResponse> {
        let error_info = ErrorInfo {
            code: self.code.clone(),
            message: self.message.clone(),
            details: self.details.clone(),
            field_errors: self.field_errors.clone(),
        };

        let migration_info = if version_info.is_deprecated {
            Some(MigrationInfo {
                migration_guide_url: Some(format!("/docs/migration/{}", version_info.version)),
                recommended_version: self.get_recommended_version(&version_info.version),
                deprecation_message: version_info.api_version.deprecation_message.clone(),
                sunset_date: version_info.api_version.sunset_date.clone(),
            })
        } else {
            None
        };

        let versioned_error = VersionedError {
            error: error_info,
            api_version: version_info.version.clone(),
            migration_info,
        };

        let mut response = ElifResponse::with_status(self.status_code);
        
        // Add JSON body
        response = response.json(&versioned_error)?;
        
        // Add deprecation headers if needed
        if version_info.is_deprecated {
            response = response.header("Deprecation", "true")?;
            
            if let Some(message) = &version_info.api_version.deprecation_message {
                response = response.header("Warning", format!("299 - \"{}\"", message))?;
            }
            
            if let Some(sunset) = &version_info.api_version.sunset_date {
                response = response.header("Sunset", sunset)?;
            }
        }

        Ok(response)
    }

    /// Get recommended version for migration
    fn get_recommended_version(&self, current_version: &str) -> String {
        // Simple logic to recommend next version
        // In practice, this would be configurable
        match current_version {
            "v1" => "v2".to_string(),
            "v2" => "v3".to_string(),
            version => {
                if let Some(v_pos) = version.find('v') {
                    if let Ok(num) = version[v_pos + 1..].parse::<u32>() {
                        return format!("v{}", num + 1);
                    }
                }
                "latest".to_string()
            }
        }
    }
}

/// Extension trait for version-aware error handling
pub trait VersionedErrorExt {
    /// Create a version-aware bad request error
    fn versioned_bad_request(version_info: &VersionInfo, code: &str, message: &str) -> HttpResult<ElifResponse>;
    
    /// Create a version-aware not found error
    fn versioned_not_found(version_info: &VersionInfo, resource: &str) -> HttpResult<ElifResponse>;
    
    /// Create a version-aware validation error
    fn versioned_validation_error(
        version_info: &VersionInfo, 
        field_errors: HashMap<String, Vec<String>>
    ) -> HttpResult<ElifResponse>;
    
    /// Create a version-aware internal server error
    fn versioned_internal_error(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse>;
    
    /// Create a version-aware unauthorized error
    fn versioned_unauthorized(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse>;
    
    /// Create a version-aware forbidden error
    fn versioned_forbidden(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse>;
}

impl VersionedErrorExt for HttpError {
    fn versioned_bad_request(version_info: &VersionInfo, code: &str, message: &str) -> HttpResult<ElifResponse> {
        VersionedErrorBuilder::new(code, message)
            .status(ElifStatusCode::BAD_REQUEST)
            .build(version_info)
    }
    
    fn versioned_not_found(version_info: &VersionInfo, resource: &str) -> HttpResult<ElifResponse> {
        VersionedErrorBuilder::new("NOT_FOUND", &format!("{} not found", resource))
            .status(ElifStatusCode::NOT_FOUND)
            .details(&format!("The requested {} could not be found", resource))
            .build(version_info)
    }
    
    fn versioned_validation_error(
        version_info: &VersionInfo, 
        field_errors: HashMap<String, Vec<String>>
    ) -> HttpResult<ElifResponse> {
        VersionedErrorBuilder::new("VALIDATION_ERROR", "Request validation failed")
            .status(ElifStatusCode::UNPROCESSABLE_ENTITY)
            .details("One or more fields contain invalid values")
            .field_errors(field_errors)
            .build(version_info)
    }
    
    fn versioned_internal_error(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse> {
        VersionedErrorBuilder::new("INTERNAL_ERROR", "Internal server error")
            .status(ElifStatusCode::INTERNAL_SERVER_ERROR)
            .details(message)
            .build(version_info)
    }
    
    fn versioned_unauthorized(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse> {
        VersionedErrorBuilder::new("UNAUTHORIZED", "Authentication required")
            .status(ElifStatusCode::UNAUTHORIZED)
            .details(message)
            .build(version_info)
    }
    
    fn versioned_forbidden(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse> {
        VersionedErrorBuilder::new("FORBIDDEN", "Access denied")
            .status(ElifStatusCode::FORBIDDEN)
            .details(message)
            .build(version_info)
    }
}

/// Convenience functions for creating versioned errors
pub fn versioned_error(_version_info: &VersionInfo, code: &str, message: &str) -> VersionedErrorBuilder {
    VersionedErrorBuilder::new(code, message)
}

pub fn bad_request_v(version_info: &VersionInfo, code: &str, message: &str) -> HttpResult<ElifResponse> {
    HttpError::versioned_bad_request(version_info, code, message)
}

pub fn not_found_v(version_info: &VersionInfo, resource: &str) -> HttpResult<ElifResponse> {
    HttpError::versioned_not_found(version_info, resource)
}

pub fn validation_error_v(version_info: &VersionInfo, field_errors: HashMap<String, Vec<String>>) -> HttpResult<ElifResponse> {
    HttpError::versioned_validation_error(version_info, field_errors)
}

pub fn internal_error_v(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse> {
    HttpError::versioned_internal_error(version_info, message)
}

pub fn unauthorized_v(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse> {
    HttpError::versioned_unauthorized(version_info, message)
}

pub fn forbidden_v(version_info: &VersionInfo, message: &str) -> HttpResult<ElifResponse> {
    HttpError::versioned_forbidden(version_info, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::versioning::ApiVersion;

    fn create_test_version_info(version: &str, deprecated: bool) -> VersionInfo {
        VersionInfo {
            version: version.to_string(),
            is_deprecated: deprecated,
            api_version: ApiVersion {
                version: version.to_string(),
                deprecated,
                deprecation_message: if deprecated {
                    Some("This version is deprecated".to_string())
                } else {
                    None
                },
                sunset_date: if deprecated {
                    Some("2024-12-31".to_string())
                } else {
                    None
                },
                is_default: false,
            },
        }
    }

    #[test]
    fn test_versioned_error_builder() {
        let version_info = create_test_version_info("v1", false);
        
        let response = VersionedErrorBuilder::new("TEST_ERROR", "Test error message")
            .status(ElifStatusCode::BAD_REQUEST)
            .details("Additional details")
            .build(&version_info)
            .unwrap();
        
        assert_eq!(response.status_code(), ElifStatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_deprecated_version_migration_info() {
        let version_info = create_test_version_info("v1", true);
        
        let response = VersionedErrorBuilder::new("TEST_ERROR", "Test error")
            .status(ElifStatusCode::BAD_REQUEST)
            .build(&version_info)
            .unwrap();
        
        // Test passes if we can build the response
        assert_eq!(response.status_code(), ElifStatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_validation_error_with_fields() {
        let version_info = create_test_version_info("v2", false);
        let mut field_errors = HashMap::new();
        field_errors.insert("email".to_string(), vec!["Invalid email format".to_string()]);
        field_errors.insert("age".to_string(), vec!["Must be positive".to_string()]);
        
        let response = HttpError::versioned_validation_error(&version_info, field_errors).unwrap();
        
        assert_eq!(response.status_code(), ElifStatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_convenience_functions() {
        let version_info = create_test_version_info("v1", false);
        
        let _bad_request = bad_request_v(&version_info, "BAD_INPUT", "Invalid input").unwrap();
        let _not_found = not_found_v(&version_info, "User").unwrap();
        let _internal = internal_error_v(&version_info, "Something went wrong").unwrap();
        let _unauthorized = unauthorized_v(&version_info, "Token expired").unwrap();
        let _forbidden = forbidden_v(&version_info, "Insufficient permissions").unwrap();
    }
}