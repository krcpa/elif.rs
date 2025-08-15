# Security Middleware Documentation

## Overview

The `elif-security` crate provides comprehensive security middleware for the elif.rs web framework. It implements essential web security features including CORS, CSRF protection, and rate limiting to help protect web applications from common attacks.

## Architecture

The security middleware is built using pure framework abstractions and integrates seamlessly with the elif.rs middleware pipeline. The security suite consists of:

- **CORS Middleware**: Cross-Origin Resource Sharing protection
- **CSRF Middleware**: Cross-Site Request Forgery protection  
- **Rate Limiting Middleware**: Request rate limiting and abuse prevention
- **Security Integration Builder**: Unified configuration and integration

## Quick Start

### Basic Security Setup

```rust
use elif_security::basic_security_pipeline;
use elif_http::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a server with basic security middleware
    let security_pipeline = basic_security_pipeline();
    
    let server = Server::new()
        .use_middleware(security_pipeline)
        .listen("127.0.0.1:3000")
        .await?;
    
    Ok(())
}
```

### Custom Security Configuration

```rust
use elif_security::{SecurityMiddlewareBuilder, CorsConfig, CsrfConfig, RateLimitConfig};
use std::collections::HashSet;

let cors_config = CorsConfig {
    allowed_origins: Some(["https://yourdomain.com".to_string()].into_iter().collect()),
    allow_credentials: true,
    max_age: Some(3600),
    ..CorsConfig::default()
};

let csrf_config = CsrfConfig {
    secure_cookie: true,
    token_lifetime: 1800, // 30 minutes
    ..CsrfConfig::default()
};

let rate_limit_config = RateLimitConfig {
    max_requests: 100,
    window_seconds: 60,
    identifier: elif_security::RateLimitIdentifier::IpAddress,
    exempt_paths: ["/health".to_string()].into_iter().collect(),
};

let security_pipeline = SecurityMiddlewareBuilder::new()
    .with_cors(cors_config)
    .with_csrf(csrf_config)
    .with_rate_limit(rate_limit_config)
    .build();
```

## CORS Middleware

### Configuration

```rust
use elif_security::CorsConfig;
use std::collections::HashSet;

let cors_config = CorsConfig {
    allowed_origins: Some(HashSet::from([
        "https://app.yourdomain.com".to_string(),
        "https://admin.yourdomain.com".to_string(),
    ])),
    allowed_methods: HashSet::from([
        "GET".to_string(),
        "POST".to_string(), 
        "PUT".to_string(),
        "DELETE".to_string(),
    ]),
    allowed_headers: HashSet::from([
        "content-type".to_string(),
        "authorization".to_string(),
        "x-csrf-token".to_string(),
    ]),
    allow_credentials: true,
    max_age: Some(86400), // 24 hours
};
```

### Security Considerations

- **Origin Validation**: Always specify explicit allowed origins in production
- **Credentials**: Only enable `allow_credentials` when necessary
- **Preflight Caching**: Use appropriate `max_age` values to balance security and performance
- **Wildcard Origins**: Never use wildcard origins (`*`) with credentials enabled

### Common Patterns

**Development Configuration**:
```rust
let cors_config = CorsConfig {
    allowed_origins: None, // Allow all origins (development only)
    allow_credentials: false,
    ..CorsConfig::default()
};
```

**Production Configuration**:
```rust
let cors_config = CorsConfig {
    allowed_origins: Some(HashSet::from([
        "https://yourdomain.com".to_string(),
    ])),
    allow_credentials: true,
    max_age: Some(300), // Short cache in production
    ..CorsConfig::default()
};
```

## CSRF Middleware

### Configuration

```rust
use elif_security::CsrfConfig;

let csrf_config = CsrfConfig {
    token_header: "X-CSRF-Token".to_string(),
    cookie_name: "_csrf_token".to_string(),
    secure_cookie: true,  // HTTPS only
    token_lifetime: 3600, // 1 hour
    exempt_methods: HashSet::from([
        "GET".to_string(),
        "HEAD".to_string(),
        "OPTIONS".to_string(),
    ]),
    exempt_paths: HashSet::from([
        "/api/public".to_string(),
    ]),
};
```

### Implementation Details

The CSRF middleware:
1. Generates cryptographically secure tokens
2. Validates tokens on state-changing requests (POST, PUT, DELETE, PATCH)
3. Supports both header and form-based token submission
4. Implements double-submit cookie pattern
5. Provides token refresh on successful validation

### Security Features

- **Token Binding**: Tokens are bound to user sessions and user-agent strings
- **Automatic Cleanup**: Expired tokens are automatically cleaned up
- **Secure Defaults**: Secure cookie settings enabled by default for HTTPS
- **Flexible Exemptions**: Configure exempt methods and paths as needed

### Usage in Frontend

```javascript
// Get CSRF token from cookie or meta tag
const csrfToken = getCsrfToken();

// Include in requests
fetch('/api/data', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
        'X-CSRF-Token': csrfToken
    },
    body: JSON.stringify({...})
});
```

## Rate Limiting Middleware

### Configuration

```rust
use elif_security::{RateLimitConfig, RateLimitIdentifier};
use std::collections::HashSet;

let rate_limit_config = RateLimitConfig {
    max_requests: 100,        // 100 requests
    window_seconds: 60,       // per minute
    identifier: RateLimitIdentifier::IpAddress,
    exempt_paths: HashSet::from([
        "/health".to_string(),
        "/metrics".to_string(),
    ]),
};
```

### Rate Limiting Strategies

**By IP Address**:
```rust
identifier: RateLimitIdentifier::IpAddress,
```

**By User ID** (requires authentication middleware):
```rust
identifier: RateLimitIdentifier::UserId,
```

### IP Address Detection

⚠️ **Security Note**: The current implementation uses header-based IP detection for compatibility with reverse proxies:

1. Checks `X-Forwarded-For` header
2. Checks `X-Real-IP` header  
3. Falls back to connection IP

**Production Considerations**:
- Ensure your reverse proxy (nginx, Cloudflare, etc.) sets trusted IP headers
- Consider using connection-based detection if not behind a proxy
- Validate proxy configuration to prevent header spoofing

### Rate Limiting Levels

**Development** (permissive):
```rust
RateLimitConfig {
    max_requests: 1000,
    window_seconds: 60,
    ...
}
```

**Production** (moderate):
```rust
RateLimitConfig {
    max_requests: 100,
    window_seconds: 60,
    ...
}
```

**Strict** (high security):
```rust
RateLimitConfig {
    max_requests: 30,
    window_seconds: 60,
    ...
}
```

## Security Pipeline Builder

### Pre-configured Pipelines

**Basic Security** (development-friendly):
```rust
use elif_security::basic_security_pipeline;

let pipeline = basic_security_pipeline();
// Includes: Permissive CORS + Moderate rate limiting + Default CSRF
```

**Strict Security** (production-ready):
```rust
use elif_security::strict_security_pipeline;

let allowed_origins = vec!["https://yourdomain.com".to_string()];
let pipeline = strict_security_pipeline(allowed_origins);
// Includes: Strict CORS + Aggressive rate limiting + Secure CSRF
```

**Development Security** (local development):
```rust
use elif_security::development_security_pipeline;

let pipeline = development_security_pipeline();
// Includes: Permissive CORS + Relaxed rate limiting + Non-HTTPS CSRF
```

### Custom Pipeline Builder

```rust
use elif_security::SecurityMiddlewareBuilder;

let pipeline = SecurityMiddlewareBuilder::new()
    .with_cors_permissive()           // Quick CORS setup
    .with_csrf_default()              // Default CSRF settings
    .with_rate_limit_strict()         // Strict rate limiting
    .build();
```

### Middleware Ordering

The security pipeline applies middleware in optimal order:

1. **CORS Middleware** - Handles preflight requests early
2. **Rate Limiting** - Prevents abuse before processing
3. **CSRF Middleware** - Validates tokens after rate limiting

This ordering ensures maximum security and performance.

## Error Handling

### Error Types

```rust
use elif_security::SecurityError;

match security_result {
    Err(SecurityError::CorsViolation { message }) => {
        // Handle CORS rejection
    },
    Err(SecurityError::CsrfValidationFailed) => {
        // Handle CSRF token validation failure
    },
    Err(SecurityError::RateLimitExceeded { limit, window_seconds }) => {
        // Handle rate limiting
    },
    Ok(response) => {
        // Request passed security checks
    }
}
```

### HTTP Status Codes

- **CORS Violations**: `403 Forbidden`
- **CSRF Failures**: `403 Forbidden`  
- **Rate Limiting**: `429 Too Many Requests`
- **Configuration Errors**: `500 Internal Server Error`

### Response Headers

Rate limiting middleware adds informative headers:
- `X-RateLimit-Limit`: Maximum requests allowed
- `X-RateLimit-Remaining`: Requests remaining in window
- `X-RateLimit-Reset`: Unix timestamp of window reset

## Testing

### Unit Testing

```rust
use elif_security::{SecurityMiddlewareBuilder, CorsConfig};
use axum::{extract::Request, http::Method, body::Body};

#[tokio::test]
async fn test_cors_security() {
    let cors_config = CorsConfig {
        allowed_origins: Some(["https://trusted.com".to_string()].into_iter().collect()),
        ..CorsConfig::default()
    };
    
    let pipeline = SecurityMiddlewareBuilder::new()
        .with_cors(cors_config)
        .build();
    
    // Test allowed origin
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/data")
        .header("Origin", "https://trusted.com")
        .body(Body::empty())
        .unwrap();
    
    let result = pipeline.process_request(request).await;
    assert!(result.is_ok());
    
    // Test blocked origin
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/data")
        .header("Origin", "https://malicious.com")
        .body(Body::empty())
        .unwrap();
    
    let result = pipeline.process_request(request).await;
    assert!(result.is_err());
}
```

### Integration Testing

The crate includes comprehensive integration tests covering:
- Multi-middleware interaction
- Attack simulation scenarios
- Edge case handling
- Error propagation
- Configuration validation

Run tests with:
```bash
cargo test -p elif-security
cargo test -p elif-security --test security_attacks
```

## Production Deployment

### Security Checklist

- [ ] **HTTPS Only**: Enable secure cookies and HTTPS enforcement
- [ ] **Origin Validation**: Specify explicit allowed origins (no wildcards)
- [ ] **CSRF Protection**: Enable on all state-changing endpoints
- [ ] **Rate Limiting**: Configure appropriate limits for your use case
- [ ] **Proxy Configuration**: Ensure reverse proxy sets correct IP headers
- [ ] **Monitoring**: Set up alerts for security violations
- [ ] **Token Management**: Use secure token lifetime values

### Environment Configuration

```rust
use elif_security::SecurityMiddlewareBuilder;

let is_production = std::env::var("ENVIRONMENT") == Ok("production".to_string());

let cors_config = if is_production {
    CorsConfig {
        allowed_origins: Some(allowed_origins_from_env()),
        allow_credentials: true,
        max_age: Some(300),
        ..CorsConfig::default()
    }
} else {
    CorsConfig::default() // Development settings
};

let csrf_config = CsrfConfig {
    secure_cookie: is_production,
    token_lifetime: if is_production { 1800 } else { 7200 },
    ..CsrfConfig::default()
};
```

### Monitoring and Observability

The middleware integrates with the framework's logging system:

- Security violations are logged with appropriate levels
- Rate limiting information is included in response headers
- Metrics are available for monitoring dashboards

## Advanced Configuration

### Custom CORS Logic

```rust
use elif_security::CorsConfig;

let cors_config = CorsConfig {
    allowed_origins: Some(get_dynamic_allowed_origins()),
    allowed_methods: HashSet::from([
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "PATCH".to_string(),
    ]),
    allowed_headers: HashSet::from([
        "accept".to_string(),
        "authorization".to_string(),
        "content-type".to_string(),
        "x-csrf-token".to_string(),
        "x-requested-with".to_string(),
    ]),
    expose_headers: HashSet::from([
        "x-csrf-token".to_string(),
        "x-ratelimit-remaining".to_string(),
    ]),
    allow_credentials: true,
    max_age: Some(86400),
};
```

### Rate Limiting Strategies

**API Tier-based Limiting**:
```rust
let api_rate_config = RateLimitConfig {
    max_requests: get_user_rate_limit(), // Based on user tier
    window_seconds: 60,
    identifier: RateLimitIdentifier::UserId,
    exempt_paths: premium_user_exempt_paths(),
};
```

**Path-specific Limiting**:
```rust
let rate_configs = vec![
    ("/api/auth/login", RateLimitConfig { max_requests: 5, window_seconds: 300, .. }),
    ("/api/public", RateLimitConfig { max_requests: 1000, window_seconds: 60, .. }),
    ("/*", RateLimitConfig { max_requests: 100, window_seconds: 60, .. }),
];
```

## Security Best Practices

### CORS Best Practices
1. **Never use wildcard origins** (`*`) in production
2. **Minimize allowed headers** to only what's needed
3. **Use short max-age values** in production for flexibility
4. **Enable credentials only when necessary**
5. **Regularly audit allowed origins**

### CSRF Best Practices
1. **Use HTTPS in production** for secure cookies
2. **Set appropriate token lifetimes** (balance security vs UX)
3. **Implement proper token refresh** on the frontend
4. **Monitor for suspicious token failures**
5. **Use double-submit cookie pattern** (automatically implemented)

### Rate Limiting Best Practices
1. **Set realistic limits** based on actual usage patterns
2. **Implement graduated responses** (warnings before blocking)
3. **Use appropriate identifiers** (IP vs User ID)
4. **Monitor rate limiting effectiveness**
5. **Provide clear error messages** to legitimate users

### General Security Practices
1. **Defense in Depth**: Use multiple security layers
2. **Regular Updates**: Keep dependencies updated
3. **Security Testing**: Include security tests in CI/CD
4. **Monitoring**: Implement comprehensive security monitoring
5. **Documentation**: Keep security documentation current

## Troubleshooting

### Common Issues

**CORS preflight failures**:
- Verify `allowed_methods` includes the request method
- Check `allowed_headers` includes all request headers
- Ensure `max_age` is not too short for caching

**CSRF token validation failures**:
- Verify token is included in request headers
- Check token hasn't expired
- Ensure secure cookie settings match HTTPS usage

**Rate limiting false positives**:
- Review IP detection logic for proxy environments
- Adjust limits based on actual usage patterns
- Consider exempting health check endpoints

### Debug Logging

Enable debug logging to troubleshoot security issues:

```rust
use tracing::Level;

tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .init();
```

Security middleware emits detailed logs at debug level for troubleshooting.

## Migration Guide

### From Basic HTTP to Secured

```rust
// Before: Basic server
let server = Server::new()
    .listen("127.0.0.1:3000")
    .await?;

// After: Secured server  
let security_pipeline = basic_security_pipeline();

let server = Server::new()
    .use_middleware(security_pipeline)
    .listen("127.0.0.1:3000")  
    .await?;
```

### Upgrading Security Levels

```rust
// Development
let pipeline = development_security_pipeline();

// Production (gradual migration)
let pipeline = SecurityMiddlewareBuilder::new()
    .with_cors(production_cors_config())
    .with_rate_limit_default()  // Start with default
    .with_csrf(production_csrf_config())
    .build();

// Production (fully secured)
let pipeline = strict_security_pipeline(production_origins());
```

## Contributing

Security is critical to web applications. When contributing:

1. **Security-first mindset**: Consider attack vectors and edge cases
2. **Comprehensive testing**: Include both positive and negative test cases  
3. **Documentation**: Update security documentation with changes
4. **Backward compatibility**: Maintain secure defaults
5. **Code review**: All security changes require thorough review

For security vulnerabilities, please follow responsible disclosure practices.

## License

This crate is part of the elif.rs framework and is licensed under the MIT license.