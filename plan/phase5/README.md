# Phase 5: Authentication & Authorization 🔐

**Duration**: 3-4 weeks  
**Goal**: Complete user management system  
**Status**: Ready after Phase 4

## Overview

Phase 5 implements a complete authentication and authorization system with JWT/session support, role-based permissions, and integration with our existing middleware pipeline. This provides modern web framework authentication capabilities with enterprise-grade security.

## Dependencies

- **Phase 3**: ✅ Security middleware and validation system
- **Phase 4**: ✅ Database operations and transactions

## Key Components

### 1. Authentication System Core
**File**: `crates/elif-auth/src/auth.rs`

Multi-strategy authentication system supporting various auth methods.

**Requirements**:
- JWT token authentication
- Session-based authentication  
- API key authentication
- Password hashing with bcrypt/argon2
- Token refresh and blacklisting
- Multi-factor authentication (MFA) support

**API Design**:
```rust
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, credentials: Credentials) -> Result<AuthUser, AuthError>;
    async fn validate_token(&self, token: &str) -> Result<AuthUser, AuthError>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair, AuthError>;
    async fn revoke_token(&self, token: &str) -> Result<(), AuthError>;
}

// JWT Provider
pub struct JwtAuthProvider {
    secret: String,
    issuer: String,
    expiry: Duration,
}

// Session Provider
pub struct SessionAuthProvider {
    store: Box<dyn SessionStore>,
    cookie_config: CookieConfig,
}

// Usage
#[derive(Deserialize)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 1))]
    pub password: String,
    
    pub remember: Option<bool>,
}

impl AuthController {
    async fn login(&self, mut request: Request) -> Response {
        let login_data: LoginRequest = request.validate_json()?;
        
        let user = self.auth.authenticate(Credentials::Password {
            email: login_data.email,
            password: login_data.password,
        }).await?;
        
        let tokens = self.auth.generate_tokens(&user).await?;
        
        Response::json(json!({
            "user": user,
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "expires_in": 3600
        }))
    }
}
```

### 2. Role-Based Authorization System
**File**: `crates/elif-auth/src/authorization.rs`

Flexible role and permission system with policy support.

**Requirements**:
- Role and permission models
- Policy-based authorization
- Resource-based permissions
- Permission inheritance and hierarchies
- Authorization middleware
- Route-level permission requirements

**API Design**:
```rust
// Authorization models
#[derive(Model)]
pub struct Role {
    pub id: u64,
    pub name: String,
    pub guard_name: String,
    pub permissions: HasManyThrough<Permission, RolePermission>,
}

#[derive(Model)]
pub struct Permission {
    pub id: u64,
    pub name: String,
    pub guard_name: String,
}

// Policy trait
pub trait Policy<T>: Send + Sync {
    async fn view(&self, user: &AuthUser, resource: &T) -> bool { false }
    async fn create(&self, user: &AuthUser) -> bool { false }
    async fn update(&self, user: &AuthUser, resource: &T) -> bool { false }
    async fn delete(&self, user: &AuthUser, resource: &T) -> bool { false }
}

// User model with authorization
impl User {
    pub async fn has_permission(&self, permission: &str) -> Result<bool, AuthError>;
    pub async fn has_role(&self, role: &str) -> Result<bool, AuthError>;
    pub async fn can<T>(&self, action: &str, resource: Option<&T>) -> Result<bool, AuthError>
    where T: Model;
    
    pub async fn assign_role(&mut self, role: &str) -> Result<(), AuthError>;
    pub async fn revoke_role(&mut self, role: &str) -> Result<(), AuthError>;
}

// Authorization middleware
AuthorizationMiddleware::new()
    .require_permission("posts.create")
    .require_role("admin")
    .policy::<Post>(PostPolicy::new());

// Route-level authorization
Route::post("/posts", PostController::create)
    .middleware(RequirePermission::new("posts.create"));

Route::put("/posts/{id}", PostController::update)
    .middleware(RequirePolicy::<Post>::new("update"));
```

### 3. Authentication Middleware Integration
**File**: `crates/elif-auth/src/middleware.rs`

Authentication middleware that integrates with our existing middleware pipeline.

**Requirements**:
- Token extraction from headers/cookies
- User resolution and injection into request
- Optional vs required authentication
- Authentication strategy selection
- Rate limiting integration for auth endpoints

**API Design**:
```rust
// Authentication middleware
AuthMiddleware::new()
    .strategies(vec![
        Box::new(JwtStrategy::new(jwt_config)),
        Box::new(SessionStrategy::new(session_config)),
        Box::new(ApiKeyStrategy::new(api_config)),
    ])
    .optional(false) // Require authentication
    .user_resolver(DatabaseUserResolver::new(user_service));

// In request handling
impl Request {
    pub fn user(&self) -> Option<&AuthUser>;
    pub fn require_user(&self) -> Result<&AuthUser, AuthError>;
    pub fn can<T>(&self, action: &str, resource: Option<&T>) -> Result<bool, AuthError>;
}

// Controller usage
impl PostController {
    async fn update(&self, request: Request) -> Response {
        let user = request.require_user()?;
        let post_id: u64 = request.param("id")?.parse()?;
        let post = Post::find(self.pool(), post_id).await?;
        
        if !user.can("update", Some(&post)).await? {
            return Response::forbidden();
        }
        
        // Update logic...
    }
}
```

### 4. Password Management System
**File**: `crates/elif-auth/src/passwords.rs`

Secure password handling with reset functionality.

**Requirements**:
- Password hashing (Argon2, bcrypt)
- Password strength validation
- Password reset token generation
- Secure password reset flow
- Password history tracking
- Account lockout after failed attempts

**API Design**:
```rust
pub struct PasswordManager {
    hasher: Box<dyn PasswordHasher>,
    reset_token_store: Box<dyn TokenStore>,
    config: PasswordConfig,
}

impl PasswordManager {
    pub async fn hash_password(&self, password: &str) -> Result<String, AuthError>;
    pub async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError>;
    
    pub async fn create_reset_token(&self, user: &User) -> Result<String, AuthError>;
    pub async fn reset_password(&self, token: &str, new_password: &str) -> Result<(), AuthError>;
    
    pub async fn change_password(&self, user: &mut User, current: &str, new: &str) -> Result<(), AuthError>;
}

// Password validation
#[derive(Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1))]
    pub current_password: String,
    
    #[validate(length(min = 8), custom = "validate_password_strength")]
    pub new_password: String,
    
    #[validate(must_match(other = "new_password"))]
    pub new_password_confirmation: String,
}
```

### 5. Session Management
**File**: `crates/elif-auth/src/sessions.rs`

Session storage and management with multiple backends.

**Requirements**:
- Session storage (Redis, database, file)
- Session lifecycle management
- Concurrent session limits
- Session security (regeneration, fixation prevention)
- Remember me functionality
- Session cleanup and garbage collection

**API Design**:
```rust
pub trait SessionStore: Send + Sync {
    async fn create(&self, session_data: SessionData) -> Result<String, SessionError>;
    async fn read(&self, session_id: &str) -> Result<Option<SessionData>, SessionError>;
    async fn update(&self, session_id: &str, data: SessionData) -> Result<(), SessionError>;
    async fn destroy(&self, session_id: &str) -> Result<(), SessionError>;
    async fn cleanup_expired(&self) -> Result<(), SessionError>;
}

// Redis session store
pub struct RedisSessionStore {
    redis: Arc<RedisPool>,
    ttl: Duration,
}

// Database session store  
pub struct DatabaseSessionStore {
    pool: Arc<Pool<Postgres>>,
}

// Session middleware
SessionMiddleware::new()
    .store(RedisSessionStore::new(redis_pool))
    .cookie_name("elif_session")
    .secure(true)
    .http_only(true)
    .same_site(SameSite::Strict);
```

### 6. Multi-Factor Authentication (MFA)
**File**: `crates/elif-auth/src/mfa.rs`

Two-factor authentication with TOTP and backup codes.

**Requirements**:
- TOTP (Time-based One-Time Password) support
- QR code generation for authenticator apps
- Backup recovery codes
- MFA enforcement policies
- MFA challenge/response flow

**API Design**:
```rust
pub struct MfaManager {
    totp_generator: TotpGenerator,
    backup_codes: BackupCodeManager,
}

impl MfaManager {
    pub async fn generate_secret(&self, user: &User) -> Result<MfaSecret, AuthError>;
    pub async fn verify_totp(&self, user: &User, code: &str) -> Result<bool, AuthError>;
    pub async fn generate_backup_codes(&self, user: &User) -> Result<Vec<String>, AuthError>;
    pub async fn verify_backup_code(&self, user: &User, code: &str) -> Result<bool, AuthError>;
}

// MFA middleware
MfaMiddleware::new()
    .require_for_sensitive_routes()
    .challenge_endpoint("/auth/mfa/challenge")
    .bypass_for_trusted_devices();
```

## Implementation Plan

### Week 1: Authentication Core
- [ ] JWT and session authentication providers
- [ ] Password hashing and management
- [ ] Token generation and validation
- [ ] Basic authentication middleware

### Week 2: Authorization System
- [ ] Role and permission models
- [ ] Policy-based authorization
- [ ] Authorization middleware
- [ ] Route-level permission controls

### Week 3: Advanced Features
- [ ] Password reset functionality
- [ ] Session management with multiple stores
- [ ] Multi-factor authentication (TOTP)
- [ ] Account lockout and security features

### Week 4: Integration & Polish
- [ ] Integration with existing middleware pipeline
- [ ] Authentication controllers and routes
- [ ] Security hardening and testing
- [ ] Documentation and examples

## Testing Strategy

### Unit Tests
- Password hashing and verification
- Token generation and validation
- Permission checking logic
- Policy enforcement

### Integration Tests
- Full authentication flows
- Authorization middleware behavior
- Session management across requests
- MFA challenge/response

### Security Tests
- Token injection and manipulation
- Session fixation attacks
- Brute force protection
- Permission bypass attempts

## Success Criteria

### Functional Requirements
- [ ] JWT and session authentication work end-to-end
- [ ] Role-based permissions control access correctly
- [ ] Password reset flow is secure and functional
- [ ] MFA provides additional security layer

### Security Requirements
- [ ] Passwords are securely hashed (Argon2/bcrypt)
- [ ] Sessions are secure (regeneration, CSRF protection)
- [ ] Tokens expire and can be revoked
- [ ] Authorization prevents privilege escalation

### Performance Requirements
- [ ] Authentication middleware <1ms overhead
- [ ] Permission checks <0.5ms average
- [ ] Session operations <2ms average

## Deliverables

1. **Authentication System**:
   - JWT and session providers
   - Password management with secure hashing
   - Token refresh and revocation

2. **Authorization Framework**:
   - Role and permission system
   - Policy-based authorization
   - Middleware integration

3. **Security Features**:
   - Multi-factor authentication
   - Account lockout protection
   - Secure session management

4. **Documentation & Examples**:
   - Authentication setup guide
   - Authorization patterns and examples
   - Security best practices

## Files Structure
```
crates/elif-auth/
├── src/
│   ├── lib.rs              # Public exports
│   ├── auth.rs             # Authentication core
│   ├── authorization.rs    # Role-based authorization
│   ├── middleware.rs       # Auth middleware
│   ├── passwords.rs        # Password management
│   ├── sessions.rs         # Session management
│   ├── mfa.rs              # Multi-factor authentication
│   ├── models.rs           # Auth-related models
│   └── errors.rs           # Authentication errors
├── migrations/
│   ├── create_users_table.sql
│   ├── create_roles_table.sql
│   ├── create_permissions_table.sql
│   └── create_sessions_table.sql
└── Cargo.toml

examples/auth-demo/
├── src/
│   ├── main.rs             # Demo application
│   ├── controllers/
│   │   ├── auth.rs         # Auth controller
│   │   └── users.rs        # Protected user controller
│   └── policies/
│       └── user_policy.rs  # Example policy
└── Cargo.toml
```

This phase provides a complete authentication and authorization system that rivals modern web frameworks while maintaining Rust's security and performance characteristics.