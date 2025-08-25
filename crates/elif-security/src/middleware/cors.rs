//! CORS (Cross-Origin Resource Sharing) middleware implementation
//!
//! Provides secure cross-origin request handling with configurable policies.

use crate::{SecurityError, SecurityResult};
use elif_http::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::{ElifMethod, ElifRequest},
    response::{ElifResponse, ElifStatusCode},
};
use std::collections::HashSet;

pub use crate::config::CorsConfig;

/// CORS middleware that handles cross-origin requests
#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    /// Create new CORS middleware with configuration
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }

    /// Create CORS middleware with default permissive settings
    pub fn permissive() -> Self {
        Self::new(CorsConfig {
            allowed_origins: None, // Allow all origins
            allow_credentials: false,
            ..CorsConfig::default()
        })
    }

    /// Create CORS middleware with strict settings  
    pub fn strict() -> Self {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("https://localhost:3000".to_string());

        Self::new(CorsConfig {
            allowed_origins: Some(allowed_origins),
            allow_credentials: true,
            max_age: Some(300), // 5 minutes
            ..CorsConfig::default()
        })
    }

    /// Builder method to set allowed origins
    pub fn allow_origin(mut self, origin: &str) -> Self {
        let origins = self.config.allowed_origins.get_or_insert_with(HashSet::new);
        origins.insert(origin.to_string());
        self
    }

    /// Builder method to allow all origins (not recommended for production)
    pub fn allow_any_origin(mut self) -> Self {
        self.config.allowed_origins = None;
        self
    }

    /// Builder method to set allowed methods
    pub fn allow_methods(mut self, methods: Vec<ElifMethod>) -> Self {
        self.config.allowed_methods = methods.into_iter().map(|m| m.to_string()).collect();
        self
    }

    /// Builder method to set allowed headers
    pub fn allow_headers(mut self, headers: Vec<&str>) -> Self {
        self.config.allowed_headers = headers.into_iter().map(|h| h.to_lowercase()).collect();
        self
    }

    /// Builder method to expose headers
    pub fn expose_headers(mut self, headers: Vec<&str>) -> Self {
        self.config.exposed_headers = headers.into_iter().map(|h| h.to_lowercase()).collect();
        self
    }

    /// Builder method to allow credentials
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.config.allow_credentials = allow;
        self
    }

    /// Builder method to set max age for preflight cache
    pub fn max_age(mut self, seconds: u32) -> Self {
        self.config.max_age = Some(seconds);
        self
    }

    /// Check if the request origin is allowed (static version)
    fn is_origin_allowed_static(origin: &str, config: &CorsConfig) -> bool {
        match &config.allowed_origins {
            None => true, // Allow all origins
            Some(origins) => origins.contains(origin) || origins.contains("*"),
        }
    }

    /// Check if the request origin is allowed
    fn is_origin_allowed(&self, origin: &str) -> bool {
        Self::is_origin_allowed_static(origin, &self.config)
    }

    /// Check if the request method is allowed
    fn is_method_allowed(&self, method: &str) -> bool {
        self.config.allowed_methods.contains(method)
    }

    /// Check if the request headers are allowed
    fn are_headers_allowed(&self, headers: &elif_http::response::ElifHeaderMap) -> bool {
        if let Some(requested_headers) = headers.get_str("access-control-request-headers") {
            if let Ok(requested_headers_str) = requested_headers.to_str() {
                for header in requested_headers_str.split(',') {
                    let header = header.trim().to_lowercase();
                    if !self.config.allowed_headers.contains(&header) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Handle preflight OPTIONS request
    fn handle_preflight(&self, request: &ElifRequest) -> SecurityResult<ElifResponse> {
        // Check if request method is allowed for preflight
        if let Some(request_method) = request.headers.get_str("access-control-request-method") {
            if let Ok(method_str) = request_method.to_str() {
                if !self.is_method_allowed(method_str) {
                    return Err(SecurityError::CorsViolation {
                        message: format!("Method '{}' not allowed", method_str),
                    });
                }
            }
        }

        // Check if request headers are allowed
        if !self.are_headers_allowed(&request.headers) {
            return Err(SecurityError::CorsViolation {
                message: "Headers not allowed".to_string(),
            });
        }

        // Create preflight response
        let mut response = ElifResponse::with_status(ElifStatusCode::OK);

        // Add preflight headers
        self.add_cors_headers(
            &mut response,
            request
                .headers
                .get_str("origin")
                .and_then(|h| h.to_str().ok()),
        )?;

        // Add method headers
        if !self.config.allowed_methods.is_empty() {
            let methods = self
                .config
                .allowed_methods
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            if response
                .add_header("access-control-allow-methods", &methods)
                .is_err()
            {
                return Err(SecurityError::CorsViolation {
                    message: "Failed to add allowed methods header".to_string(),
                });
            }
        }

        // Add headers header
        if !self.config.allowed_headers.is_empty() {
            let headers = self
                .config
                .allowed_headers
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            if response
                .add_header("access-control-allow-headers", &headers)
                .is_err()
            {
                return Err(SecurityError::CorsViolation {
                    message: "Failed to add allowed headers header".to_string(),
                });
            }
        }

        // Add max age
        if let Some(max_age) = self.config.max_age {
            if response
                .add_header("access-control-max-age", max_age.to_string())
                .is_err()
            {
                return Err(SecurityError::CorsViolation {
                    message: "Failed to add max age header".to_string(),
                });
            }
        }

        Ok(response)
    }

    /// Add CORS headers to response (static version for async contexts)
    fn add_cors_headers_to_response(
        response: &mut ElifResponse,
        origin: Option<&str>,
        config: &CorsConfig,
    ) {
        if Self::add_cors_headers_impl(response, origin, config).is_err() {
            log::warn!("Failed to add CORS headers to response");
        }
    }

    /// Add CORS headers to response
    fn add_cors_headers(
        &self,
        response: &mut ElifResponse,
        origin: Option<&str>,
    ) -> SecurityResult<()> {
        Self::add_cors_headers_impl(response, origin, &self.config)
    }

    /// Internal implementation for adding CORS headers
    fn add_cors_headers_impl(
        response: &mut ElifResponse,
        origin: Option<&str>,
        config: &CorsConfig,
    ) -> SecurityResult<()> {
        // Add Access-Control-Allow-Origin
        if let Some(origin_str) = origin {
            if Self::is_origin_allowed_static(origin_str, config) {
                let origin_header = if config.allowed_origins.is_none()
                    || config.allowed_origins.as_ref().unwrap().contains("*")
                {
                    "*"
                } else {
                    origin_str
                };

                response
                    .add_header("access-control-allow-origin", origin_header)
                    .map_err(|_| SecurityError::CorsViolation {
                        message: "Failed to add origin header".to_string(),
                    })?;
            }
        }

        // Add Access-Control-Allow-Credentials
        if config.allow_credentials {
            response
                .add_header("access-control-allow-credentials", "true")
                .map_err(|_| SecurityError::CorsViolation {
                    message: "Failed to add credentials header".to_string(),
                })?;
        }

        // Add Access-Control-Expose-Headers
        if !config.exposed_headers.is_empty() {
            let exposed = config
                .exposed_headers
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            response
                .add_header("access-control-expose-headers", &exposed)
                .map_err(|_| SecurityError::CorsViolation {
                    message: "Failed to add exposed headers".to_string(),
                })?;
        }

        Ok(())
    }
}

impl Middleware for CorsMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let config = self.config.clone();

        // Extract origin header before async move
        let origin = request
            .headers
            .get_str("origin")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Handle preflight OPTIONS request before async move
        if request.method == ElifMethod::OPTIONS {
            let preflight_result = self.handle_preflight(&request);
            return Box::pin(async move {
                match preflight_result {
                    Ok(response) => response,
                    Err(_) => ElifResponse::with_status(ElifStatusCode::FORBIDDEN)
                        .text("CORS policy violation"),
                }
            });
        }

        // Check origin for non-preflight requests before async move
        if let Some(ref origin_str) = origin {
            if !self.is_origin_allowed(origin_str) {
                return Box::pin(async move {
                    ElifResponse::with_status(ElifStatusCode::FORBIDDEN)
                        .text("CORS policy violation: origin not allowed")
                });
            }
        }

        Box::pin(async move {
            // Request is valid, proceed to next middleware/handler
            let mut response = next.run(request).await;

            // Add CORS headers to response (inline the logic)
            CorsMiddleware::add_cors_headers_to_response(&mut response, origin.as_deref(), &config);

            response
        })
    }

    fn name(&self) -> &'static str {
        "CorsMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::middleware::v2::MiddlewarePipelineV2;
    use elif_http::request::ElifRequest;
    use elif_http::response::ElifHeaderMap;

    #[tokio::test]
    async fn test_cors_preflight_request() {
        let cors = CorsMiddleware::new(CorsConfig::default());

        // Create preflight OPTIONS request
        let mut headers = ElifHeaderMap::new();
        headers.insert(
            "origin".parse().unwrap(),
            "https://example.com".parse().unwrap(),
        );
        headers.insert(
            "access-control-request-method".parse().unwrap(),
            "GET".parse().unwrap(),
        );

        let request = ElifRequest::new(ElifMethod::OPTIONS, "/".parse().unwrap(), headers);

        // Test preflight handling directly
        match cors.handle_preflight(&request) {
            Ok(response) => {
                assert_eq!(response.status_code(), ElifStatusCode::OK);
                // Test that response would have CORS headers (implementation specific)
            }
            Err(e) => panic!("Preflight request should succeed: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_cors_middleware_v2() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        let pipeline = MiddlewarePipelineV2::new().add(cors);

        // Test normal request
        let mut headers = ElifHeaderMap::new();
        headers.insert(
            "origin".parse().unwrap(),
            "https://example.com".parse().unwrap(),
        );

        let request = ElifRequest::new(ElifMethod::GET, "/".parse().unwrap(), headers);

        let response = pipeline
            .execute(request, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Hello World") })
            })
            .await;

        // Response should be ok with CORS headers
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_preflight_v2() {
        let cors = CorsMiddleware::new(CorsConfig::default());
        let pipeline = MiddlewarePipelineV2::new().add(cors);

        // Create preflight OPTIONS request
        let mut headers = ElifHeaderMap::new();
        headers.insert(
            "origin".parse().unwrap(),
            "https://example.com".parse().unwrap(),
        );
        headers.insert(
            "access-control-request-method".parse().unwrap(),
            "GET".parse().unwrap(),
        );

        let request = ElifRequest::new(ElifMethod::OPTIONS, "/".parse().unwrap(), headers);

        let response = pipeline
            .execute(request, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Should not reach handler") })
            })
            .await;

        // Preflight should return success response
        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_origin_not_allowed() {
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("https://trusted.com".to_string());

        let config = CorsConfig {
            allowed_origins: Some(allowed_origins),
            ..CorsConfig::default()
        };

        let cors = CorsMiddleware::new(config);
        let pipeline = MiddlewarePipelineV2::new().add(cors);

        // Test request from disallowed origin
        let mut headers = ElifHeaderMap::new();
        headers.insert(
            "origin".parse().unwrap(),
            "https://evil.com".parse().unwrap(),
        );

        let request = ElifRequest::new(ElifMethod::GET, "/".parse().unwrap(), headers);

        let response = pipeline
            .execute(request, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Should not reach handler") })
            })
            .await;

        // Should be rejected with 403
        assert_eq!(response.status_code(), ElifStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_cors_builder_methods() {
        let cors = CorsMiddleware::new(CorsConfig::default())
            .allow_origin("https://example.com")
            .allow_methods(vec![ElifMethod::GET, ElifMethod::POST])
            .allow_headers(vec!["content-type", "authorization"])
            .allow_credentials(true)
            .max_age(3600);

        assert!(cors
            .config
            .allowed_origins
            .as_ref()
            .unwrap()
            .contains("https://example.com"));
        assert!(cors.config.allowed_methods.contains("GET"));
        assert!(cors.config.allowed_methods.contains("POST"));
        assert!(cors.config.allowed_headers.contains("content-type"));
        assert!(cors.config.allow_credentials);
        assert_eq!(cors.config.max_age, Some(3600));
    }

    #[tokio::test]
    async fn test_cors_method_not_allowed() {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert("GET".to_string());

        let config = CorsConfig {
            allowed_methods: allowed_methods,
            ..CorsConfig::default()
        };

        let cors = CorsMiddleware::new(config);

        // Test preflight for disallowed method
        let mut headers = ElifHeaderMap::new();
        headers.insert(
            "origin".parse().unwrap(),
            "https://example.com".parse().unwrap(),
        );
        headers.insert(
            "access-control-request-method".parse().unwrap(),
            "DELETE".parse().unwrap(),
        );

        let request = ElifRequest::new(ElifMethod::OPTIONS, "/".parse().unwrap(), headers);

        // Test preflight handling directly
        match cors.handle_preflight(&request) {
            Ok(_) => panic!("Preflight for disallowed method should fail"),
            Err(_) => {
                // Should be rejected
                // The error handling will convert this to a 403 response in the middleware
            }
        }
    }
}
