# Phase 3: Essential Middleware & Validation + Architectural Consistency 🛡️

**Duration**: 3-4 weeks  
**Goal**: Secure web server with pure framework architecture  
**Status**: ✅ **3.1-3.2 & 3.7-3.9 Complete** | 🚨 **3.18 CRITICAL** | Phase 2 Complete

## Overview

Phase 3 adds essential security middleware and input validation to create a production-ready web server. This includes CORS, CSRF protection, rate limiting, comprehensive input validation, and security-focused middleware.

## Dependencies

- **Phase 2**: ✅ HTTP server, routing, middleware pipeline, controllers

## Key Components

### 1. Security Middleware Collection
**File**: `crates/elif-security/src/middleware/mod.rs`

Production-ready security middleware for common web vulnerabilities.

**Requirements**:
- ✅ **CORS (Cross-Origin Resource Sharing) middleware** - **COMPLETED Phase 3.1**
- 🚧 **CSRF (Cross-Site Request Forgery) protection** - **IN PROGRESS Phase 3.2**  
- Rate limiting with multiple strategies
- Security headers middleware (HSTS, X-Frame-Options, etc.)
- Request size limiting
- IP whitelisting/blacklisting

**API Design**:
```rust
// ✅ CORS Middleware (IMPLEMENTED Phase 3.1)
CorsMiddleware::new(CorsConfig::default())
    .allow_origin("https://example.com")
    .allow_methods(vec![Method::GET, Method::POST])
    .allow_headers(vec!["Authorization", "Content-Type"])
    .allow_credentials(true)
    .max_age(3600);
    
// Usage with Tower/Axum
let app = Router::new()
    .route("/", get(handler))
    .layer(CorsLayer::new(cors_config));

// Rate Limiting
RateLimitMiddleware::new()
    .requests_per_minute(60)
    .per_ip(true)
    .with_redis("redis://localhost") // or in-memory
    .custom_key_fn(|req| format!("user:{}", req.user_id()));

// CSRF Protection  
CsrfMiddleware::new()
    .token_header("X-CSRF-Token")
    .cookie_name("_csrf")
    .exclude_routes(vec!["/api/webhook"]);
```

### 2. Input Validation System
**File**: `crates/elif-validation/src/lib.rs`

Comprehensive input validation with derive macros and custom validators.

**Requirements**:
- Validation derive macro for structs
- Built-in validators (required, email, min/max, regex, etc.)
- Custom validator support
- Nested validation for complex objects
- Conditional validation rules
- Internationalized error messages

**API Design**:
```rust
#[derive(Validate, Deserialize)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 100), custom = "validate_username")]
    pub username: String,
    
    #[validate(email, length(max = 255))]
    pub email: String,
    
    #[validate(length(min = 8), custom = "validate_password_strength")]
    pub password: String,
    
    #[validate(range(min = 18, max = 120))]
    pub age: Option<u8>,
    
    #[validate(nested)]
    pub profile: CreateProfileRequest,
}

// Custom validator
fn validate_username(username: &str) -> Result<(), ValidationError> {
    if username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_username"))
    }
}

// Usage in controller
impl UserController {
    async fn store(&self, mut request: Request) -> Response {
        let user_data: CreateUserRequest = request.validate_json()?;
        // ... rest of the logic
    }
}
```

### 3. Request Sanitization
**File**: `crates/elif-validation/src/sanitization.rs`

Input sanitization to prevent XSS and injection attacks.

**Requirements**:
- HTML sanitization and escaping
- SQL injection prevention (already handled by ORM)
- NoSQL injection prevention
- Path traversal prevention
- Script tag removal
- Whitespace normalization

**API Design**:
```rust
#[derive(Sanitize, Deserialize)]
pub struct BlogPostRequest {
    #[sanitize(trim, html_escape)]
    pub title: String,
    
    #[sanitize(html_clean, whitespace_normalize)]
    pub content: String,
    
    #[sanitize(path_safe)]
    pub slug: String,
    
    #[sanitize(array(item = "trim, lowercase"))]
    pub tags: Vec<String>,
}
```

### 4. Logging & Request Tracing Middleware
**File**: `crates/elif-http/src/middleware/logging.rs`

Structured logging with request tracing and correlation IDs.

**Requirements**:
- Request logging with timing
- Correlation ID generation and tracking
- Structured logging (JSON format)
- Log level configuration
- Request/response body logging (configurable)
- Error logging with stack traces

**API Design**:
```rust
LoggingMiddleware::new()
    .log_requests(true)
    .log_responses(false) // don't log response bodies
    .log_errors(true)
    .exclude_paths(vec!["/health", "/metrics"])
    .correlation_header("X-Correlation-ID")
    .format(LogFormat::Json);

// Log output example
{
  "timestamp": "2025-01-13T10:30:00Z",
  "level": "INFO",
  "correlation_id": "abc123",
  "method": "POST",
  "path": "/api/users",
  "status": 201,
  "duration_ms": 45.2,
  "user_id": "user_456"
}
```

### 5. Request/Response Transformation Pipeline
**File**: `crates/elif-http/src/middleware/transform.rs`

Middleware for transforming requests and responses.

**Requirements**:
- Content-Type negotiation and transformation
- Compression middleware (gzip, brotli)
- Response caching headers
- API versioning support
- Request/response interceptors

**API Design**:
```rust
// Compression
CompressionMiddleware::new()
    .enable_gzip()
    .enable_brotli()
    .min_size(1024); // Don't compress small responses

// API Versioning
ApiVersionMiddleware::new()
    .header_name("Api-Version")
    .default_version("v1")
    .supported_versions(vec!["v1", "v2"]);

// Response transformation
ResponseTransformMiddleware::new()
    .wrap_responses(true) // Wrap all responses in {"data": ..., "meta": ...}
    .add_timestamp(true)
    .add_request_id(true);
```

### 6. Health Check & Monitoring
**File**: `crates/elif-http/src/health.rs`

Built-in health check endpoints and monitoring capabilities.

**Requirements**:
- Health check endpoint (/health)
- Readiness/liveness probes
- Database connection health
- Dependency health checks
- Metrics collection integration
- Status page functionality

**API Design**:
```rust
HealthCheckMiddleware::new()
    .endpoint("/health")
    .check_database(true)
    .check_redis(true)
    .custom_check("external_api", || {
        // Custom health check logic
        async { Ok(()) }
    });

// Health response format
{
  "status": "healthy",
  "timestamp": "2025-01-13T10:30:00Z",
  "checks": {
    "database": {"status": "healthy", "response_time_ms": 2.1},
    "redis": {"status": "healthy", "response_time_ms": 0.5},
    "external_api": {"status": "degraded", "error": "High latency"}
  }
}
```

## Implementation Plan

### Week 1: Security Middleware Foundation
- [x] **CORS middleware with full configuration** ✅ **Phase 3.1 Complete (Issue #29)**
  - Full Tower service integration
  - Builder pattern API (.allow_origin, .allow_methods, .allow_credentials)
  - Preflight request handling
  - Production-ready security defaults
  - 5 comprehensive tests
- [ ] **CSRF protection with token generation/validation** 🚧 **Phase 3.2 In Progress (Issue #30)**
- [ ] Basic rate limiting with in-memory storage (Issue #31)
- [ ] Security headers middleware (Issue #33)

### Week 2: Validation & Sanitization System
- [ ] Validation derive macro and built-in validators
- [ ] Input sanitization for XSS prevention
- [ ] Integration with request parsing
- [ ] Custom validator framework

### Week 3: Advanced Features & Polish
- [ ] Advanced rate limiting with Redis backend
- [ ] Comprehensive logging and tracing
- [ ] Health check system
- [ ] Request/response transformation pipeline
- [ ] Integration testing and documentation

### Week 4: Architectural Consistency (CRITICAL)
- [ ] **Phase 3.18: HTTP Server Architecture Cleanup - Pure Framework Implementation** 🚨 **CRITICAL (Issue #57)**
  - Consolidate 5 server implementations to 1-2 maximum
  - Remove direct Axum imports from all server implementations  
  - Create pure framework server using ElifRouter, ElifRequest, ElifResponse exclusively
  - Update examples to demonstrate framework types only
  - Ensure Axum is purely implementation detail
  - **Priority**: Must complete before any additional middleware work

## Testing Strategy

### Unit Tests
- Individual middleware functionality
- Validation rules and error messages
- Sanitization effectiveness
- Rate limiting accuracy

### Integration Tests
- Full middleware pipeline processing
- Security vulnerability testing
- Performance under rate limiting
- Health check endpoint functionality

### Security Tests
- CORS policy enforcement
- CSRF attack prevention
- XSS prevention through sanitization
- Rate limiting bypass attempts

## Success Criteria

### Security Requirements
- [ ] CORS policies prevent unauthorized cross-origin requests
- [ ] CSRF protection blocks forged requests
- [ ] Rate limiting prevents abuse
- [ ] Input validation prevents malformed data
- [ ] Sanitization prevents XSS attacks

### Performance Requirements
- [ ] Middleware overhead <1ms per request
- [ ] Rate limiting doesn't impact normal traffic
- [ ] Validation processes <100 fields in <1ms

### Usability Requirements
- [ ] Clear validation error messages
- [ ] Easy middleware configuration
- [ ] Comprehensive logging for debugging

## Deliverables

1. **Security Middleware Suite**:
   - CORS, CSRF, rate limiting, security headers
   - Configurable and production-ready

2. **Validation Framework**:
   - Derive macro for automatic validation
   - Comprehensive built-in validators
   - Custom validator support

3. **Monitoring & Observability**:
   - Request logging and tracing
   - Health check system
   - Performance monitoring hooks

4. **Documentation & Examples**:
   - Security best practices guide
   - Middleware configuration examples
   - Validation patterns and recipes

## Files Structure
```
crates/elif-security/
├── src/
│   ├── lib.rs              # Public exports
│   ├── middleware/
│   │   ├── mod.rs          # Middleware collection
│   │   ├── cors.rs         # CORS middleware
│   │   ├── csrf.rs         # CSRF protection
│   │   ├── rate_limit.rs   # Rate limiting
│   │   └── headers.rs      # Security headers
│   └── config.rs           # Security configuration
└── Cargo.toml

crates/elif-validation/
├── src/
│   ├── lib.rs              # Public exports
│   ├── validators/         # Built-in validators
│   ├── sanitization.rs     # Input sanitization
│   ├── errors.rs           # Validation errors
│   └── macros.rs           # Derive macros
└── Cargo.toml

crates/elif-http/src/middleware/
├── logging.rs              # Request logging
├── transform.rs            # Request/response transformation
├── compression.rs          # Response compression
└── health.rs               # Health checks
```

This phase ensures that elif.rs applications are secure by default and provide enterprise-grade validation and monitoring capabilities.