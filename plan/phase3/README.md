# Phase 3: Security Core

**Duration**: Months 7-9 (12 weeks)  
**Team**: 2-3 developers  
**Goal**: Enterprise-grade security matching Laravel's Guard system

## Overview

Phase 3 implements comprehensive security features including authentication, authorization, input validation, and security middleware. This phase focuses on making the framework secure by default while providing flexibility for different authentication methods.

## Dependencies

- **Phase 1**: DI container for service registration and middleware system
- **Phase 2**: Database layer for user storage and session management
- **External**: JWT libraries, password hashing, validation libraries

## Key Components

### 1. Authentication System
**File**: `crates/elif-auth/src/guard.rs`

Multi-provider authentication system supporting various auth methods.

**Requirements**:
- Multiple authentication guards (JWT, session, API token)
- User provider abstraction for different user sources
- Authentication middleware for route protection
- Password verification and hashing
- Remember me functionality
- Account lockout and rate limiting

**API Design**:
```rust
pub trait AuthGuard: Send + Sync {
    type User: User;
    
    async fn attempt(&self, credentials: Credentials) -> Result<Self::User, AuthError>;
    async fn user(&self, request: &Request) -> Result<Option<Self::User>, AuthError>;
    async fn login(&self, user: Self::User, remember: bool) -> Result<AuthToken, AuthError>;
    async fn logout(&self, request: &Request) -> Result<(), AuthError>;
    async fn refresh(&self, token: &str) -> Result<AuthToken, AuthError>;
}

// JWT Guard Implementation
pub struct JwtGuard<U: User, P: UserProvider<U>> {
    user_provider: P,
    jwt_config: JwtConfig,
    _phantom: PhantomData<U>,
}

impl<U: User, P: UserProvider<U>> AuthGuard for JwtGuard<U, P> {
    type User = U;
    
    async fn attempt(&self, credentials: Credentials) -> Result<U, AuthError> {
        let user = self.user_provider
            .retrieve_by_credentials(&credentials)
            .await?;
            
        if self.verify_password(&credentials.password, &user.password()).await? {
            Ok(user)
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_token: Option<String>,
}

pub struct Credentials {
    pub email: String,
    pub password: String,
}
```

### 2. Authorization System
**File**: `crates/elif-auth/src/authorization.rs`

Policy-based authorization with roles and permissions.

**Requirements**:
- Policy definition and registration
- Gate system for ability checking
- Role-based access control (RBAC)
- Resource-based permissions
- Authorization middleware
- Super user bypass mechanism

**API Design**:
```rust
pub trait Gate: Send + Sync {
    async fn allows<U: User>(
        &self,
        user: &U,
        ability: &str,
        resource: Option<&dyn Any>
    ) -> bool;
    
    async fn denies<U: User>(
        &self,
        user: &U,
        ability: &str,
        resource: Option<&dyn Any>
    ) -> bool {
        !self.allows(user, ability, resource).await
    }
    
    fn define<F>(&mut self, ability: &str, callback: F)
    where
        F: Fn(&dyn User, Option<&dyn Any>) -> bool + Send + Sync + 'static;
}

// Policy trait for resource-specific authorization
pub trait Policy<T>: Send + Sync {
    async fn view(&self, user: &dyn User, resource: &T) -> bool { false }
    async fn create(&self, user: &dyn User) -> bool { false }
    async fn update(&self, user: &dyn User, resource: &T) -> bool { false }
    async fn delete(&self, user: &dyn User, resource: &T) -> bool { false }
}

// Usage example
pub struct PostPolicy;

impl Policy<Post> for PostPolicy {
    async fn view(&self, user: &dyn User, post: &Post) -> bool {
        post.published || post.user_id == user.id()
    }
    
    async fn update(&self, user: &dyn User, post: &Post) -> bool {
        post.user_id == user.id() || user.has_role("admin")
    }
}

// Authorization macro for easy checking
#[macro_export]
macro_rules! authorize {
    ($gate:expr, $user:expr, $ability:expr) => {
        if !$gate.allows($user, $ability, None).await {
            return Err(AuthError::Unauthorized);
        }
    };
    ($gate:expr, $user:expr, $ability:expr, $resource:expr) => {
        if !$gate.allows($user, $ability, Some($resource)).await {
            return Err(AuthError::Unauthorized);
        }
    };
}
```

### 3. Input Validation
**File**: `crates/elif-validation/src/validator.rs`

Comprehensive input validation with custom rules and error messages.

**Requirements**:
- Built-in validation rules (required, email, min/max length, etc.)
- Custom validation rules
- Nested validation for complex structures
- Localized error messages
- Validation middleware for automatic request validation
- File upload validation

**API Design**:
```rust
pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

// Derive macro for automatic validation
#[derive(Validate, Deserialize)]
pub struct CreateUserRequest {
    #[validate(email, message = "Invalid email address")]
    pub email: String,
    
    #[validate(length(min = 8, max = 255), message = "Password must be 8-255 characters")]
    pub password: String,
    
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    
    #[validate(custom = "validate_age")]
    pub age: u8,
}

fn validate_age(age: u8) -> Result<(), ValidationError> {
    if age < 18 {
        Err(ValidationError::new("age", "Must be at least 18 years old"))
    } else {
        Ok(())
    }
}

// Usage in controllers
pub async fn create_user(
    Json(request): Json<CreateUserRequest>
) -> Result<Json<User>, ValidationError> {
    request.validate()?; // Automatic validation
    
    // Create user logic...
    Ok(Json(user))
}
```

### 4. Security Middleware
**File**: `crates/elif-auth/src/middleware.rs`

Essential security middleware for common attack prevention.

**Requirements**:
- CORS middleware with configurable policies
- CSRF protection with token verification
- Rate limiting per IP and per user
- Request size limiting
- Security headers (HSTS, CSP, X-Frame-Options)
- IP filtering and geo-blocking

**API Design**:
```rust
// CORS Middleware
pub struct CorsMiddleware {
    config: CorsConfig,
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub max_age: Duration,
    pub allow_credentials: bool,
}

impl Middleware for CorsMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Result<Response, MiddlewareError> {
        let origin = request.headers().get("Origin");
        
        if self.is_preflight(&request) {
            return Ok(self.handle_preflight(origin));
        }
        
        let mut response = next.run(request).await?;
        self.add_cors_headers(&mut response, origin);
        Ok(response)
    }
}

// Rate Limiting Middleware
pub struct RateLimitMiddleware {
    store: Box<dyn RateLimitStore>,
    config: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_attempts: u32,
    pub window_seconds: u64,
    pub key_generator: KeyGenerator,
}

pub enum KeyGenerator {
    IpAddress,
    UserId,
    Custom(Box<dyn Fn(&Request) -> String + Send + Sync>),
}

// CSRF Middleware
pub struct CsrfMiddleware {
    config: CsrfConfig,
}

impl Middleware for CsrfMiddleware {
    async fn handle(&self, request: Request, next: Next) -> Result<Response, MiddlewareError> {
        if self.should_verify(&request) {
            self.verify_token(&request)?;
        }
        
        let response = next.run(request).await?;
        Ok(self.add_csrf_token(response))
    }
}
```

### 5. Password Security
**File**: `crates/elif-auth/src/password.rs`

Secure password hashing and verification.

**Requirements**:
- Multiple hashing algorithms (Argon2, bcrypt)
- Automatic algorithm upgrading
- Password strength validation
- Secure random salt generation
- Timing attack prevention
- Password history tracking

**API Design**:
```rust
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> Result<String, PasswordError>;
    async fn verify(&self, password: &str, hash: &str) -> Result<bool, PasswordError>;
    fn needs_rehash(&self, hash: &str) -> bool;
}

pub struct Argon2Hasher {
    config: Argon2Config,
}

impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, password: &str) -> Result<String, PasswordError> {
        let salt = generate_salt();
        let hash = argon2::hash_encoded(
            password.as_bytes(),
            &salt,
            &self.config.into()
        )?;
        Ok(hash)
    }
    
    async fn verify(&self, password: &str, hash: &str) -> Result<bool, PasswordError> {
        // Timing-safe verification
        Ok(argon2::verify_encoded(hash, password.as_bytes())?)
    }
}

// Password strength validation
pub struct PasswordValidator {
    min_length: usize,
    require_uppercase: bool,
    require_lowercase: bool,
    require_numbers: bool,
    require_symbols: bool,
    check_common_passwords: bool,
}

impl PasswordValidator {
    pub fn validate(&self, password: &str) -> Result<(), PasswordError> {
        if password.len() < self.min_length {
            return Err(PasswordError::TooShort(self.min_length));
        }
        
        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(PasswordError::MissingUppercase);
        }
        
        // Additional validations...
        
        Ok(())
    }
}
```

## Implementation Plan

### Week 1-2: Authentication Foundation
- [ ] Define authentication traits and interfaces
- [ ] Implement JWT authentication guard
- [ ] Add session-based authentication
- [ ] Password hashing and verification
- [ ] Basic user provider abstraction

### Week 3-4: Authorization System
- [ ] Policy trait and registration system
- [ ] Gate implementation for ability checking
- [ ] Role-based access control
- [ ] Authorization middleware
- [ ] Permission caching for performance

### Week 5-6: Input Validation
- [ ] Validation trait and derive macro
- [ ] Built-in validation rules library
- [ ] Custom validation rule support
- [ ] Validation error handling and messages
- [ ] File upload validation

### Week 7-8: Security Middleware
- [ ] CORS middleware implementation
- [ ] CSRF protection system
- [ ] Rate limiting middleware
- [ ] Security headers middleware
- [ ] Request size limiting

### Week 9-10: Advanced Security Features
- [ ] API token authentication
- [ ] Account lockout mechanisms
- [ ] Security event logging
- [ ] Password strength validation
- [ ] Two-factor authentication foundation

### Week 11-12: Testing & Hardening
- [ ] Comprehensive security testing
- [ ] Penetration testing simulation
- [ ] Performance benchmarks for auth operations
- [ ] Security audit and vulnerability assessment
- [ ] Documentation and security guidelines

## Security Considerations

### Authentication Security:
- Timing-safe password verification
- Secure session token generation
- JWT token expiration and refresh
- Account lockout after failed attempts
- Password complexity requirements

### Authorization Security:
- Principle of least privilege
- Policy-based access control
- Resource-level permissions
- Role hierarchy validation
- Permission caching security

### Input Validation Security:
- SQL injection prevention
- XSS attack prevention
- File upload security
- Size limiting and validation
- Content type validation

### General Security:
- HTTPS enforcement
- Secure cookie settings
- CSRF token validation
- Rate limiting implementation
- Security header configuration

## Performance Requirements

### Authentication Performance:
- Password hashing: <200ms per operation
- JWT verification: <1ms per token
- Session lookup: <5ms per request
- User resolution: <10ms per request

### Authorization Performance:
- Policy evaluation: <1ms per check
- Role verification: <0.5ms per check
- Permission caching: 99%+ hit rate
- Gate resolution: <2ms per ability check

## Testing Strategy

### Security Tests:
- Authentication bypass attempts
- Authorization escalation tests
- Input validation bypass tests
- CSRF attack simulations
- Rate limiting effectiveness

### Performance Tests:
- Authentication throughput testing
- Authorization performance under load
- Validation performance with large inputs
- Middleware overhead measurement

### Integration Tests:
- Full authentication flow testing
- Authorization with database integration
- Validation with complex nested structures
- Middleware chain interaction

## Success Criteria

### Functional Requirements:
- [ ] Users can authenticate via multiple methods (JWT, session, API token)
- [ ] Authorization policies enforce access control correctly
- [ ] Input validation prevents malicious data submission
- [ ] Security middleware blocks common attacks
- [ ] Password security meets industry standards

### Security Requirements:
- [ ] Passes OWASP security checklist
- [ ] Resistant to common attacks (CSRF, XSS, injection)
- [ ] Timing attack resistant authentication
- [ ] Secure by default configuration
- [ ] Comprehensive security logging

### Performance Requirements:
- [ ] Authentication operations complete within 200ms
- [ ] Authorization checks complete within 2ms
- [ ] Middleware adds <1ms overhead per request
- [ ] Can handle 1000+ auth operations per second

## Deliverables

1. **Core Crates**:
   - `elif-auth` - Authentication and authorization
   - `elif-validation` - Input validation system
   - `elif-security` - Security middleware collection

2. **Documentation**:
   - Authentication setup guide
   - Authorization policy documentation
   - Validation rules reference
   - Security best practices guide

3. **Examples**:
   - Multi-provider authentication setup
   - Complex authorization policies
   - Custom validation rules
   - Security middleware configuration

4. **Security Tools**:
   - Security audit commands
   - Permission debugging utilities
   - Validation testing helpers

## File Structure
```
crates/elif-auth/
├── src/
│   ├── lib.rs                # Public API exports
│   ├── guard.rs             # Authentication guards
│   ├── authorization.rs      # Authorization system
│   ├── password.rs          # Password hashing
│   ├── middleware.rs        # Auth middleware
│   └── error.rs            # Auth error types
└── Cargo.toml

crates/elif-validation/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── validator.rs        # Validation system
│   ├── rules.rs           # Built-in validation rules
│   ├── custom.rs          # Custom rule support
│   └── error.rs          # Validation errors
└── Cargo.toml

crates/elif-security/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── cors.rs            # CORS middleware
│   ├── csrf.rs            # CSRF protection
│   ├── rate_limit.rs      # Rate limiting
│   ├── headers.rs         # Security headers
│   └── encryption.rs      # Encryption utilities
└── Cargo.toml
```

This phase creates enterprise-grade security capabilities that protect applications by default while providing the flexibility needed for complex authorization scenarios.