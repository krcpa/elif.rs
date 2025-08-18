use crate::{
    errors::{HttpError, HttpResult},
    request::ElifRequest,
    response::{ElifResponse, ElifHeaderMap, ElifStatusCode},
    middleware::{Middleware, BoxFuture},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use service_builder::builder;

/// API versioning strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionStrategy {
    /// Version specified in URL path (e.g., /api/v1/users)
    UrlPath,
    /// Version specified in header (e.g., Api-Version: v1)
    Header(String),
    /// Version specified in query parameter (e.g., ?version=v1) 
    QueryParam(String),
    /// Version specified in Accept header (e.g., Accept: application/vnd.api+json;version=1)
    AcceptHeader,
}

impl Default for VersionStrategy {
    fn default() -> Self {
        Self::UrlPath
    }
}

/// API version configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    /// Version identifier (e.g., "v1", "v2", "1.0", "2024-01-01")
    pub version: String,
    /// Whether this version is deprecated
    pub deprecated: bool,
    /// Deprecation warning message
    pub deprecation_message: Option<String>,
    /// Date when this version will be removed (ISO 8601)
    pub sunset_date: Option<String>,
    /// Whether this version is the default
    pub is_default: bool,
}

/// API versioning middleware configuration
#[builder]
#[derive(Debug, Clone)]
pub struct VersioningConfig {
    /// Available API versions
    #[builder(default)]
    pub versions: HashMap<String, ApiVersion>,
    /// Versioning strategy to use
    #[builder(default)]
    pub strategy: VersionStrategy,
    /// Default version if none specified
    #[builder(optional)]
    pub default_version: Option<String>,
    /// Whether to include deprecation headers
    #[builder(default = "true")]
    pub include_deprecation_headers: bool,
    /// Custom header name for version (when using Header strategy)
    #[builder(default = "\"Api-Version\".to_string()")]
    pub version_header_name: String,
    /// Custom query parameter name for version (when using QueryParam strategy)
    #[builder(default = "\"version\".to_string()")]
    pub version_param_name: String,
    /// Whether to be strict about version validation
    #[builder(default = "true")]
    pub strict_validation: bool,
}

impl VersioningConfig {
    /// Add a new API version
    pub fn add_version(&mut self, version: String, api_version: ApiVersion) {
        self.versions.insert(version, api_version);
    }

    /// Set a version as deprecated
    pub fn deprecate_version(&mut self, version: &str, message: Option<String>, sunset_date: Option<String>) {
        if let Some(api_version) = self.versions.get_mut(version) {
            api_version.deprecated = true;
            api_version.deprecation_message = message;
            api_version.sunset_date = sunset_date;
        }
    }

    /// Get the default version
    pub fn get_default_version(&self) -> Option<&ApiVersion> {
        if let Some(default_version) = &self.default_version {
            return self.versions.get(default_version);
        }
        
        // Find the version marked as default
        self.versions.values().find(|v| v.is_default)
    }
}

/// Extracted version information from request
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// The requested version
    pub version: String,
    /// The API version configuration
    pub api_version: ApiVersion,
    /// Whether this version is deprecated
    pub is_deprecated: bool,
}

/// API versioning middleware
pub struct VersioningMiddleware {
    config: VersioningConfig,
}

impl VersioningMiddleware {
    /// Create new versioning middleware
    pub fn new(config: VersioningConfig) -> Self {
        Self { config }
    }

    /// Extract version from request based on strategy
    fn extract_version_from_axum(&self, request: &axum::extract::Request) -> HttpResult<Option<String>> {
        match &self.config.strategy {
            VersionStrategy::UrlPath => {
                // Extract version from URL path (e.g., /api/v1/users -> v1)
                let path = request.uri().path();
                if let Some(captures) = regex::Regex::new(r"/api/v?(\d+(?:\.\d+)?)/")
                    .map_err(|e| HttpError::internal_server_error(format!("Version regex error: {}", e)))?
                    .captures(path) 
                {
                    if let Some(version) = captures.get(1) {
                        return Ok(Some(format!("v{}", version.as_str())));
                    }
                }
                Ok(None)
            },
            VersionStrategy::Header(header_name) => {
                Ok(request.headers()
                    .get(header_name)
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string()))
            },
            VersionStrategy::QueryParam(param_name) => {
                // Parse query parameters from URI
                if let Some(query) = request.uri().query() {
                    let params: HashMap<String, String> = serde_urlencoded::from_str(query)
                        .map_err(|e| HttpError::bad_request(format!("Invalid query parameters: {}", e)))?;
                    Ok(params.get(param_name).map(|s| s.to_string()))
                } else {
                    Ok(None)
                }
            },
            VersionStrategy::AcceptHeader => {
                if let Some(accept) = request.headers().get("accept") {
                    if let Ok(accept_str) = accept.to_str() {
                        // Parse Accept header for version (e.g., application/vnd.api+json;version=1)
                        if let Some(captures) = regex::Regex::new(r"version=([^;,\s]+)")
                            .map_err(|e| HttpError::internal_server_error(format!("Version regex error: {}", e)))?
                            .captures(accept_str)
                        {
                            if let Some(version) = captures.get(1) {
                                return Ok(Some(format!("v{}", version.as_str())));
                            }
                        }
                    }
                }
                Ok(None)
            }
        }
    }

    /// Resolve version to API version configuration
    fn resolve_version(&self, requested_version: Option<String>) -> HttpResult<VersionInfo> {
        let version_key = if let Some(version) = requested_version {
            if self.config.versions.contains_key(&version) {
                version
            } else if self.config.strict_validation {
                return Err(HttpError::bad_request(format!("Unsupported API version: {}", version)));
            } else if let Some(default) = &self.config.default_version {
                default.clone()
            } else {
                return Err(HttpError::bad_request("No valid API version specified and no default available".to_string()));
            }
        } else if let Some(default) = &self.config.default_version {
            default.clone()
        } else {
            return Err(HttpError::bad_request("API version is required".to_string()));
        };

        let api_version = self.config.versions.get(&version_key)
            .ok_or_else(|| HttpError::internal_server_error(format!("Version configuration not found: {}", version_key)))?;

        Ok(VersionInfo {
            version: version_key,
            is_deprecated: api_version.deprecated,
            api_version: api_version.clone(),
        })
    }

    /// Add deprecation headers to response
    fn add_deprecation_headers(&self, response: &mut ElifResponse, version_info: &VersionInfo) {
        if self.config.include_deprecation_headers && version_info.is_deprecated {
            let headers = response.headers_mut();
            
            // Add deprecation header
            headers.insert("Deprecation", "true".parse().unwrap());
            
            // Add warning message if available
            if let Some(message) = &version_info.api_version.deprecation_message {
                headers.insert("Warning", format!("299 - \"{}\"", message).parse().unwrap());
            }
            
            // Add sunset date if available
            if let Some(sunset) = &version_info.api_version.sunset_date {
                headers.insert("Sunset", sunset.parse().unwrap());
            }
        }
    }
}

impl Middleware for VersioningMiddleware {
    fn process_request<'a>(
        &'a self,
        mut request: axum::extract::Request,
    ) -> BoxFuture<'a, Result<axum::extract::Request, axum::response::Response>> {
        Box::pin(async move {
            // Extract version from request
            match self.extract_version_from_axum(&request) {
                Ok(extracted_version) => {
                    match self.resolve_version(extracted_version) {
                        Ok(version_info) => {
                            // Store version info in request extensions
                            request.extensions_mut().insert(version_info);
                            Ok(request)
                        }
                        Err(err) => {
                            let response = axum::response::Response::builder()
                                .status(err.status_code())
                                .body(axum::body::Body::from(err.to_string()))
                                .unwrap();
                            Err(response)
                        }
                    }
                }
                Err(err) => {
                    let response = axum::response::Response::builder()
                        .status(err.status_code())
                        .body(axum::body::Body::from(err.to_string()))
                        .unwrap();
                    Err(response)
                }
            }
        })
    }

    fn process_response<'a>(
        &'a self,
        mut response: axum::response::Response,
    ) -> BoxFuture<'a, axum::response::Response> {
        let config = self.config.clone();
        
        Box::pin(async move {
            // For now, we can't easily access the version info from request in response processing
            // In a production implementation, you'd want to store this in response extensions
            // or use a more sophisticated middleware pattern
            
            // Add general API versioning headers
            let headers = response.headers_mut();
            
            // Add API version support information
            if let Some(default_version) = &config.default_version {
                headers.insert("X-Api-Default-Version", default_version.parse().unwrap());
            }
            
            // Add supported versions list
            let supported_versions: Vec<String> = config.versions.keys().cloned().collect();
            if !supported_versions.is_empty() {
                let versions_str = supported_versions.join(",");
                headers.insert("X-Api-Supported-Versions", versions_str.parse().unwrap());
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "VersioningMiddleware"
    }
}

/// Convenience functions for creating versioning middleware
pub fn versioning_middleware(config: VersioningConfig) -> VersioningMiddleware {
    VersioningMiddleware::new(config)
}

/// Create versioning middleware with default configuration
pub fn default_versioning_middleware() -> VersioningMiddleware {
    let mut config = VersioningConfig::build()
        .strategy(VersionStrategy::UrlPath)
        .default_version(Some("v1".to_string()))
        .build_with_defaults();

    // Add default v1 version
    config.add_version("v1".to_string(), ApiVersion {
        version: "v1".to_string(),
        deprecated: false,
        deprecation_message: None,
        sunset_date: None,
        is_default: true,
    });

    VersioningMiddleware::new(config)
}

/// Extension trait to get version info from request
pub trait RequestVersionExt {
    /// Get version information from request
    fn version_info(&self) -> Option<&VersionInfo>;
    
    /// Get current API version string
    fn api_version(&self) -> Option<&str>;
    
    /// Check if current version is deprecated
    fn is_deprecated_version(&self) -> bool;
}

impl RequestVersionExt for axum::extract::Request {
    fn version_info(&self) -> Option<&VersionInfo> {
        self.extensions().get::<VersionInfo>()
    }
    
    fn api_version(&self) -> Option<&str> {
        self.version_info().map(|v| v.version.as_str())
    }
    
    fn is_deprecated_version(&self) -> bool {
        self.version_info().map(|v| v.is_deprecated).unwrap_or(false)
    }
}

impl RequestVersionExt for ElifRequest {
    fn version_info(&self) -> Option<&VersionInfo> {
        // Note: This will need implementation when ElifRequest has extensions support
        None
    }
    
    fn api_version(&self) -> Option<&str> {
        self.version_info().map(|v| v.version.as_str())
    }
    
    fn is_deprecated_version(&self) -> bool {
        self.version_info().map(|v| v.is_deprecated).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestServerBuilder, HttpAssertions};

    #[test]
    fn test_version_config_builder() {
        let config = VersioningConfig::build()
            .strategy(VersionStrategy::Header("X-Api-Version".to_string()))
            .default_version(Some("v2".to_string()))
            .strict_validation(false)
            .build_with_defaults();

        assert!(!config.strict_validation);
        assert_eq!(config.default_version, Some("v2".to_string()));
        match config.strategy {
            VersionStrategy::Header(name) => assert_eq!(name, "X-Api-Version"),
            _ => panic!("Expected Header strategy"),
        }
    }

    #[test]
    fn test_version_deprecation() {
        let mut config = VersioningConfig::build().build_with_defaults();
        
        config.add_version("v1".to_string(), ApiVersion {
            version: "v1".to_string(),
            deprecated: false,
            deprecation_message: None,
            sunset_date: None,
            is_default: false,
        });

        config.deprecate_version("v1", 
            Some("Version v1 is deprecated, please use v2".to_string()),
            Some("2024-12-31".to_string())
        );

        let version = config.versions.get("v1").unwrap();
        assert!(version.deprecated);
        assert_eq!(version.deprecation_message, Some("Version v1 is deprecated, please use v2".to_string()));
    }

    #[tokio::test]
    async fn test_url_path_version_extraction() {
        let config = VersioningConfig::build()
            .strategy(VersionStrategy::UrlPath)
            .build_with_defaults();
            
        let middleware = VersioningMiddleware::new(config);
        
        // Test URL path extraction logic would go here
        // This is a simplified test structure
    }
}