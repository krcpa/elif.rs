use crate::{
    errors::{HttpError, HttpResult},
    request::ElifRequest,
    response::{ElifResponse, ElifHeaderMap, ElifStatusCode},
    middleware::{Middleware, BoxFuture},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use service_builder::builder;
use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use once_cell::sync::Lazy;

// Static regex patterns compiled once for performance
static URL_PATH_VERSION_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"/api/v?(\d+(?:\.\d+)?)/").expect("Invalid URL path version regex")
});

static ACCEPT_HEADER_VERSION_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"version=([^;,\s]+)").expect("Invalid Accept header version regex")
});

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
#[derive(Debug, Clone, Default)]
#[builder]
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
    
    /// Get a specific version
    pub fn get_version(&self, version: &str) -> Option<&ApiVersion> {
        self.versions.get(version)
    }
    
    /// Get the versioning strategy
    pub fn get_strategy(&self) -> &VersionStrategy {
        &self.strategy
    }
    
    /// Get all versions
    pub fn get_versions(&self) -> &HashMap<String, ApiVersion> {
        &self.versions
    }
    
    /// Get all versions as mutable reference
    pub fn get_versions_mut(&mut self) -> &mut HashMap<String, ApiVersion> {
        &mut self.versions
    }
    
    /// Clone all configuration for rebuilding
    pub fn clone_config(&self) -> (HashMap<String, ApiVersion>, VersionStrategy, Option<String>, bool, String, String, bool) {
        (
            self.versions.clone(),
            self.strategy.clone(),
            self.default_version.clone(),
            self.include_deprecation_headers,
            self.version_header_name.clone(),
            self.version_param_name.clone(),
            self.strict_validation,
        )
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
                if let Some(captures) = URL_PATH_VERSION_REGEX.captures(path) {
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
                        if let Some(captures) = ACCEPT_HEADER_VERSION_REGEX.captures(accept_str) {
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
            .ok_or_else(|| HttpError::internal(format!("Version configuration not found: {}", version_key)))?;

        Ok(VersionInfo {
            version: version_key,
            is_deprecated: api_version.deprecated,
            api_version: api_version.clone(),
        })
    }

    /// Add deprecation headers to response
    fn add_deprecation_headers(&self, mut response: ElifResponse, version_info: &VersionInfo) -> HttpResult<ElifResponse> {
        if self.config.include_deprecation_headers && version_info.is_deprecated {
            // Add deprecation header
            response = response.header("Deprecation", "true")?;
            
            // Add warning message if available
            if let Some(message) = &version_info.api_version.deprecation_message {
                response = response.header("Warning", format!("299 - \"{}\"", message))?;
            }
            
            // Add sunset date if available
            if let Some(sunset) = &version_info.api_version.sunset_date {
                response = response.header("Sunset", sunset)?;
            }
        }
        
        Ok(response)
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
                if let Ok(value) = default_version.parse() {
                    headers.insert("X-Api-Default-Version", value);
                }
            }
            
            // Add supported versions list
            let supported_versions: Vec<String> = config.versions.keys().cloned().collect();
            if !supported_versions.is_empty() {
                let versions_str = supported_versions.join(",");
                if let Ok(value) = versions_str.parse() {
                    headers.insert("X-Api-Supported-Versions", value);
                }
            }
            
            response
        })
    }
    
    fn name(&self) -> &'static str {
        "VersioningMiddleware"
    }
}

/// Tower Layer implementation for VersioningMiddleware
#[derive(Debug, Clone)]
pub struct VersioningLayer {
    config: VersioningConfig,
}

impl VersioningLayer {
    /// Create a new versioning layer
    pub fn new(config: VersioningConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for VersioningLayer {
    type Service = VersioningService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        VersioningService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Tower Service implementation for versioning
#[derive(Debug, Clone)]
pub struct VersioningService<S> {
    inner: S,
    config: VersioningConfig,
}

impl<S> Service<axum::extract::Request> for VersioningService<S>
where
    S: Service<axum::extract::Request, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    type Response = axum::response::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: axum::extract::Request) -> Self::Future {
        let config = self.config.clone();
        let mut inner = self.inner.clone();
        
        Box::pin(async move {
            // Extract version from request
            let extracted_version = match Self::extract_version_from_request(&config, &request) {
                Ok(version) => version,
                Err(error_response) => return Ok(error_response),
            };
            
            let version_info = match Self::resolve_version(&config, extracted_version) {
                Ok(info) => info,
                Err(error_response) => return Ok(error_response),
            };
            
            // Store version info in request extensions
            request.extensions_mut().insert(version_info.clone());
            
            // Call the inner service
            let mut response = inner.call(request).await.map_err(|e| e)?;
            
            // Add versioning headers to response
            Self::add_version_headers(&config, &version_info, &mut response);
            
            Ok(response)
        })
    }
}

impl<S> VersioningService<S> {
    /// Extract version from request based on strategy
    fn extract_version_from_request(
        config: &VersioningConfig,
        request: &axum::extract::Request,
    ) -> Result<Option<String>, axum::response::Response> {
        // Local static regex definitions for better encapsulation and performance
        static URL_PATH_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
            regex::Regex::new(r"/api/v?(\d+(?:\.\d+)?)/").expect("Failed to compile URL path regex")
        });
        static ACCEPT_HEADER_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
            regex::Regex::new(r"version=([^;,\s]+)").expect("Failed to compile Accept header regex")
        });
        
        let extracted = match &config.strategy {
            VersionStrategy::UrlPath => {
                // Extract version from URL path (e.g., /api/v1/users -> v1)
                let path = request.uri().path();
                if let Some(captures) = URL_PATH_REGEX.captures(path) {
                    if let Some(version) = captures.get(1) {
                        Some(format!("v{}", version.as_str()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            VersionStrategy::Header(header_name) => {
                request.headers()
                    .get(header_name)
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            },
            VersionStrategy::QueryParam(param_name) => {
                // Parse query parameters from URI
                if let Some(query) = request.uri().query() {
                    if let Ok(params) = serde_urlencoded::from_str::<HashMap<String, String>>(query) {
                        params.get(param_name).map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            VersionStrategy::AcceptHeader => {
                if let Some(accept) = request.headers().get("accept") {
                    if let Ok(accept_str) = accept.to_str() {
                        // Parse Accept header for version (e.g., application/vnd.api+json;version=1)
                        if let Some(captures) = ACCEPT_HEADER_REGEX.captures(accept_str) {
                            if let Some(version) = captures.get(1) {
                                Some(format!("v{}", version.as_str()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };
        
        Ok(extracted)
    }

    /// Resolve version to API version configuration
    fn resolve_version(
        config: &VersioningConfig,
        requested_version: Option<String>,
    ) -> Result<VersionInfo, axum::response::Response> {
        let version_key = if let Some(version) = requested_version {
            if config.versions.contains_key(&version) {
                version
            } else if config.strict_validation {
                let error_response = axum::response::Response::builder()
                    .status(400)
                    .body(axum::body::Body::from(format!("Unsupported API version: {}", version)))
                    .unwrap();
                return Err(error_response);
            } else if let Some(default) = &config.default_version {
                default.clone()
            } else {
                let error_response = axum::response::Response::builder()
                    .status(400)
                    .body(axum::body::Body::from("No valid API version specified and no default available"))
                    .unwrap();
                return Err(error_response);
            }
        } else if let Some(default) = &config.default_version {
            default.clone()
        } else {
            let error_response = axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from("API version is required"))
                .unwrap();
            return Err(error_response);
        };

        let api_version = config.versions.get(&version_key)
            .ok_or_else(|| {
                axum::response::Response::builder()
                    .status(500)
                    .body(axum::body::Body::from(format!("Version configuration not found: {}", version_key)))
                    .unwrap()
            })?;

        Ok(VersionInfo {
            version: version_key,
            is_deprecated: api_version.deprecated,
            api_version: api_version.clone(),
        })
    }

    /// Add version headers to response
    fn add_version_headers(
        config: &VersioningConfig,
        version_info: &VersionInfo,
        response: &mut axum::response::Response,
    ) {
        let headers = response.headers_mut();
        
        // Add current version header
        if let Ok(value) = version_info.version.parse() {
            headers.insert("X-Api-Version", value);
        }
        
        // Add API version support information
        if let Some(default_version) = &config.default_version {
            if let Ok(value) = default_version.parse() {
                headers.insert("X-Api-Default-Version", value);
            }
        }
        
        // Add supported versions list
        let supported_versions: Vec<String> = config.versions.keys().cloned().collect();
        if !supported_versions.is_empty() {
            let versions_str = supported_versions.join(",");
            if let Ok(value) = versions_str.parse() {
                headers.insert("X-Api-Supported-Versions", value);
            }
        }
        
        // Add deprecation headers if needed
        if config.include_deprecation_headers && version_info.is_deprecated {
            // Use from_static for known static values
            headers.insert("Deprecation", axum::http::HeaderValue::from_static("true"));
            
            // Handle dynamic warning message safely
            if let Some(message) = &version_info.api_version.deprecation_message {
                let warning_value = format!("299 - \"{}\"", message);
                if let Ok(value) = warning_value.parse() {
                    headers.insert("Warning", value);
                }
            }
            
            // Handle dynamic sunset date safely
            if let Some(sunset) = &version_info.api_version.sunset_date {
                if let Ok(value) = sunset.parse() {
                    headers.insert("Sunset", value);
                }
            }
        }
    }
}

/// Convenience functions for creating versioning middleware
pub fn versioning_middleware(config: VersioningConfig) -> VersioningMiddleware {
    VersioningMiddleware::new(config)
}

/// Create versioning layer for use with axum routers
pub fn versioning_layer(config: VersioningConfig) -> VersioningLayer {
    VersioningLayer::new(config)
}

/// Create versioning middleware with default configuration
pub fn default_versioning_middleware() -> VersioningMiddleware {
    let mut config = VersioningConfig {
        versions: HashMap::new(),
        strategy: VersionStrategy::UrlPath,
        default_version: Some("v1".to_string()),
        include_deprecation_headers: true,
        version_header_name: "Api-Version".to_string(),
        version_param_name: "version".to_string(),
        strict_validation: true,
    };

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
        let config = VersioningConfig::builder()
            .strategy(VersionStrategy::Header("X-Api-Version".to_string()))
            .default_version(Some("v2".to_string()))
            .strict_validation(false)
            .build().unwrap();

        assert!(!config.strict_validation);
        assert_eq!(config.default_version, Some("v2".to_string()));
        match config.strategy {
            VersionStrategy::Header(name) => assert_eq!(name, "X-Api-Version"),
            _ => panic!("Expected Header strategy"),
        }
    }

    #[test]
    fn test_version_deprecation() {
        let mut config = VersioningConfig::builder().build().unwrap();
        
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
        let config = VersioningConfig::builder()
            .strategy(VersionStrategy::UrlPath)
            .build().unwrap();
            
        let middleware = VersioningMiddleware::new(config);
        
        // Test URL path extraction logic would go here
        // This is a simplified test structure
    }
}