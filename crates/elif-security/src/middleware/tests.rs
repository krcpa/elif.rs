//! Comprehensive tests for request sanitization and security headers middleware

#[cfg(feature = "legacy-tests")]
use super::{
    sanitization::{SanitizationMiddleware, SanitizationConfig},
    security_headers::{SecurityHeadersMiddleware, SecurityHeadersConfig},
};
#[cfg(feature = "legacy-tests")]
use crate::config::SecurityConfig;
#[cfg(feature = "legacy-tests")]
use axum::{extract::Request, response::Response, body::Body, http::StatusCode};
#[cfg(feature = "legacy-tests")]
use elif_http::middleware::Middleware;
#[cfg(feature = "legacy-tests")]
use std::collections::HashMap;

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_sanitization_middleware_strict_mode() {
    let middleware = SanitizationMiddleware::strict();
    
    // Test normal request should pass
    let request = Request::builder()
        .method("GET")
        .uri("/api/test")
        .header("User-Agent", "Mozilla/5.0 (compatible; Test)")
        .body(Body::empty())
        .unwrap();
    
    let result = middleware.process_request(request).await;
    assert!(result.is_ok());
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_sanitization_middleware_blocks_malicious_user_agents() {
    let middleware = SanitizationMiddleware::strict();
    
    let malicious_agents = ["sqlmap", "nikto", "nmap", "masscan", "wget", "curl", "python-requests"];
    
    for agent in &malicious_agents {
        let request = Request::builder()
            .method("POST")
            .uri("/api/test")
            .header("User-Agent", format!("MyBot/{} 1.0", agent))
            .body(Body::empty())
            .unwrap();
        
        let result = middleware.process_request(request).await;
        assert!(result.is_err(), "Should block malicious user agent: {}", agent);
        
        if let Err(response) = result {
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_sanitization_middleware_request_size_limit() {
    let config = SanitizationConfig {
        max_request_size: Some(100), // 100 bytes limit
        ..SanitizationConfig::default()
    };
    let middleware = SanitizationMiddleware::new(config);
    
    // Test request within limit
    let small_request = Request::builder()
        .method("POST")
        .uri("/api/test")
        .header("Content-Length", "50")
        .body(Body::empty())
        .unwrap();
    
    let result = middleware.process_request(small_request).await;
    assert!(result.is_ok());
    
    // Test request exceeding limit
    let large_request = Request::builder()
        .method("POST")
        .uri("/api/test")
        .header("Content-Length", "200")
        .body(Body::empty())
        .unwrap();
    
    let result = middleware.process_request(large_request).await;
    assert!(result.is_err());
    
    if let Err(response) = result {
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_sanitization_middleware_permissive_mode() {
    let middleware = SanitizationMiddleware::permissive();
    
    // Test request with curl user agent (should be blocked even in permissive mode for XSS protection)
    let request = Request::builder()
        .method("GET")
        .uri("/api/test")
        .header("User-Agent", "curl/7.68.0")
        .body(Body::empty())
        .unwrap();
    
    let result = middleware.process_request(request).await;
    assert!(result.is_err()); // Still blocks malicious agents
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_headers_middleware_strict() {
    let middleware = SecurityHeadersMiddleware::strict();
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("Hello, World!"))
        .unwrap();
    
    let result = middleware.process_response(response).await;
    let headers = result.headers();
    
    // Verify all security headers are present
    assert!(headers.contains_key("content-security-policy"));
    assert!(headers.contains_key("strict-transport-security"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-xss-protection"));
    assert!(headers.contains_key("referrer-policy"));
    assert!(headers.contains_key("permissions-policy"));
    assert!(headers.contains_key("cross-origin-embedder-policy"));
    assert!(headers.contains_key("cross-origin-opener-policy"));
    assert!(headers.contains_key("cross-origin-resource-policy"));
    
    // Verify header values
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-xss-protection").unwrap(), "1; mode=block");
    assert_eq!(headers.get("cross-origin-opener-policy").unwrap(), "same-origin");
    assert_eq!(headers.get("cross-origin-resource-policy").unwrap(), "same-origin");
    
    // Verify CSP is restrictive
    let csp = headers.get("content-security-policy").unwrap().to_str().unwrap();
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("script-src 'self'"));
    assert!(csp.contains("object-src 'none'"));
    
    // Verify HSTS is long-term
    let hsts = headers.get("strict-transport-security").unwrap().to_str().unwrap();
    assert!(hsts.contains("max-age=63072000")); // 2 years
    assert!(hsts.contains("includeSubDomains"));
    assert!(hsts.contains("preload"));
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_headers_middleware_development() {
    let middleware = SecurityHeadersMiddleware::development();
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("Hello, World!"))
        .unwrap();
    
    let result = middleware.process_response(response).await;
    let headers = result.headers();
    
    // Verify development-friendly settings
    assert_eq!(headers.get("x-frame-options").unwrap(), "SAMEORIGIN");
    
    let csp = headers.get("content-security-policy").unwrap().to_str().unwrap();
    assert!(csp.contains("unsafe-inline"));
    assert!(csp.contains("unsafe-eval"));
    
    let hsts = headers.get("strict-transport-security").unwrap().to_str().unwrap();
    assert!(hsts.contains("max-age=31536000")); // 1 year, shorter than strict
    assert!(!hsts.contains("preload")); // No preload in development
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_headers_middleware_api_focused() {
    let middleware = SecurityHeadersMiddleware::api_focused();
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(r#"{"data": "test"}"#))
        .unwrap();
    
    let result = middleware.process_response(response).await;
    let headers = result.headers();
    
    // API-focused CSP should be very restrictive
    let csp = headers.get("content-security-policy").unwrap().to_str().unwrap();
    assert_eq!(csp, "default-src 'none'; frame-ancestors 'none'");
    
    // Should deny all framing
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    
    // Should have no-referrer policy for APIs
    assert_eq!(headers.get("referrer-policy").unwrap(), "no-referrer");
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_headers_middleware_custom_headers() {
    let mut custom_headers = HashMap::new();
    custom_headers.insert("X-API-Version".to_string(), "v1.0".to_string());
    custom_headers.insert("X-Rate-Limit-Policy".to_string(), "strict".to_string());
    
    let config = SecurityHeadersConfig {
        custom_headers,
        remove_server_header: true,
        remove_x_powered_by: true,
        ..SecurityHeadersConfig::default()
    };
    
    let middleware = SecurityHeadersMiddleware::new(config);
    
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("API Response"))
        .unwrap();
    
    // Add headers that should be removed
    response.headers_mut().insert("server", "nginx/1.20".parse().unwrap());
    response.headers_mut().insert("x-powered-by", "PHP/8.0".parse().unwrap());
    
    let result = middleware.process_response(response).await;
    let headers = result.headers();
    
    // Verify custom headers are added
    assert_eq!(headers.get("x-api-version").unwrap(), "v1.0");
    assert_eq!(headers.get("x-rate-limit-policy").unwrap(), "strict");
    
    // Verify server headers are removed
    assert!(!headers.contains_key("server"));
    assert!(!headers.contains_key("x-powered-by"));
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_headers_middleware_error_handling() {
    // Test with invalid header values
    let config = SecurityHeadersConfig {
        content_security_policy: Some("invalid\nCSP\nheader".to_string()),
        ..SecurityHeadersConfig::default()
    };
    
    let middleware = SecurityHeadersMiddleware::new(config);
    
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("Test"))
        .unwrap();
    
    let result = middleware.process_response(response).await;
    
    // Should handle error gracefully and return error response
    assert_eq!(result.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(result.headers().contains_key("x-security-error"));
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_sanitization_config_validation() {
    // Test blocked patterns work correctly
    let config = SanitizationConfig {
        blocked_patterns: vec![
            r"<script[^>]*>.*?</script>".to_string(),
            r"javascript:".to_string(),
            r"on\w+\s*=".to_string(),
        ],
        enable_xss_protection: true,
        ..SanitizationConfig::default()
    };
    
    let middleware = SanitizationMiddleware::new(config);
    
    // Test XSS detection through User-Agent (simplified test)
    let request = Request::builder()
        .method("GET")
        .uri("/test?param=%3Cscript%3Ealert('xss')%3C/script%3E")
        .header("User-Agent", "Mozilla/5.0")
        .body(Body::empty())
        .unwrap();
    
    let result = middleware.process_request(request).await;
    assert!(result.is_ok()); // URL encoding not directly checked in current implementation
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_config_integration() {
    let security_config = SecurityConfig {
        sanitization: Some(SanitizationConfig {
            enable_xss_protection: true,
            enable_sql_injection_protection: true,
            max_request_size: Some(2048),
            ..SanitizationConfig::default()
        }),
        security_headers: Some(SecurityHeadersConfig {
            content_security_policy: Some("default-src 'self'".to_string()),
            x_frame_options: Some("DENY".to_string()),
            ..SecurityHeadersConfig::default()
        }),
        ..SecurityConfig::default()
    };
    
    // Test that config structures are properly defined
    assert!(security_config.sanitization.is_some());
    assert!(security_config.security_headers.is_some());
    
    let sanitization_config = security_config.sanitization.unwrap();
    assert_eq!(sanitization_config.max_request_size, Some(2048));
    assert!(sanitization_config.enable_xss_protection);
    assert!(sanitization_config.enable_sql_injection_protection);
    
    let headers_config = security_config.security_headers.unwrap();
    assert_eq!(headers_config.content_security_policy, Some("default-src 'self'".to_string()));
    assert_eq!(headers_config.x_frame_options, Some("DENY".to_string()));
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_middleware_naming_consistency() {
    let sanitization_middleware = SanitizationMiddleware::strict();
    let security_headers_middleware = SecurityHeadersMiddleware::strict();
    
    assert_eq!(sanitization_middleware.name(), "SanitizationMiddleware");
    assert_eq!(security_headers_middleware.name(), "SecurityHeadersMiddleware");
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_comprehensive_security_pipeline_integration() {
    // This test would be in integration.rs but we'll test the basic middleware interaction here
    let sanitization = SanitizationMiddleware::strict();
    let security_headers = SecurityHeadersMiddleware::strict();
    
    // Simulate request processing
    let request = Request::builder()
        .method("POST")
        .uri("/api/secure")
        .header("User-Agent", "Mozilla/5.0 (compatible; SecurityTest)")
        .header("Content-Length", "100")
        .body(Body::from(r#"{"data": "test"}"#))
        .unwrap();
    
    // Process through sanitization first
    let request_result = sanitization.process_request(request).await;
    assert!(request_result.is_ok());
    
    // Create response for security headers processing
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(r#"{"result": "success"}"#))
        .unwrap();
    
    // Process response through security headers
    let response_result = security_headers.process_response(response).await;
    assert_eq!(response_result.status(), StatusCode::OK);
    
    // Verify security headers were added
    let headers = response_result.headers();
    assert!(headers.contains_key("content-security-policy"));
    assert!(headers.contains_key("x-frame-options"));
}

// Performance and edge case tests

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_sanitization_performance_with_large_headers() {
    let middleware = SanitizationMiddleware::permissive();
    
    // Test with large user agent string
    let large_ua = "A".repeat(8192); // 8KB user agent
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .header("User-Agent", large_ua)
        .body(Body::empty())
        .unwrap();
    
    let start = std::time::Instant::now();
    let result = middleware.process_request(request).await;
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100); // Should process quickly
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_security_headers_with_empty_response() {
    let middleware = SecurityHeadersMiddleware::strict();
    
    let response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap();
    
    let result = middleware.process_response(response).await;
    
    // Should still add security headers to empty responses
    assert_eq!(result.status(), StatusCode::NO_CONTENT);
    assert!(result.headers().contains_key("x-frame-options"));
    assert!(result.headers().contains_key("content-security-policy"));
}

#[cfg(feature = "legacy-tests")]
#[ignore]
#[tokio::test]
async fn test_error_response_security_headers() {
    let middleware = SecurityHeadersMiddleware::strict();
    
    let response = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Error occurred"))
        .unwrap();
    
    let result = middleware.process_response(response).await;
    
    // Security headers should be added even to error responses
    assert_eq!(result.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(result.headers().contains_key("x-frame-options"));
    assert!(result.headers().contains_key("x-content-type-options"));
}