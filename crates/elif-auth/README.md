# elif-auth

[![Crates.io](https://img.shields.io/crates/v/elif-auth)](https://crates.io/crates/elif-auth)
[![Documentation](https://docs.rs/elif-auth/badge.svg)](https://docs.rs/elif-auth)
[![License](https://img.shields.io/crates/l/elif-auth)](https://github.com/krcpa/elif.rs)

Authentication and authorization system for the [elif.rs](https://github.com/krcpa/elif.rs) LLM-friendly web framework.

## Features

- **🔐 JWT Authentication** - Complete JWT token management with signing, validation, and refresh
- **📝 Session-Based Auth** - Cookie-based sessions with multiple storage backends  
- **🔒 Password Security** - Argon2 and bcrypt password hashing with strength validation
- **🛡️ CSRF Protection** - Session integration with CSRF tokens for enhanced security
- **⚡ Multiple Storage** - Memory, database, and Redis session storage (extensible)
- **🔑 Role-Based Access** - User roles and permissions with flexible authorization
- **🚀 Production Ready** - Configurable security settings for development vs production

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
elif-auth = "0.1.0"
```

### JWT Authentication

```rust
use elif_auth::{JwtProvider, JwtConfig, JwtUser};

// Configure JWT provider
let config = JwtConfig {
    secret: "your-secret-key".to_string(),
    algorithm: "HS256".to_string(),
    access_token_expiry: 900,  // 15 minutes
    refresh_token_expiry: 604800,  // 1 week
    issuer: "your-app".to_string(),
    audience: Some("your-users".to_string()),
    allow_refresh: true,
};

let jwt_provider = JwtProvider::new(config)?;

// Generate tokens for a user
let user = JwtUser {
    id: "123".to_string(),
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
    roles: vec!["user".to_string()],
    permissions: vec!["read".to_string(), "write".to_string()],
    // ... other fields
};

let access_token = jwt_provider.generate_token(&user, "access")?;
let (access, refresh) = jwt_provider.generate_token_pair(&user)?;

// Validate tokens
let claims = jwt_provider.validate_token_claims(&access_token)?;
println!("User: {} ({})", claims.username, claims.sub);
```

### Session Authentication

```rust
use elif_auth::{SessionProvider, MemorySessionStorage, SessionData};
use chrono::Duration;

// Create session storage and provider
let storage = MemorySessionStorage::new();
let session_provider = SessionProvider::with_default_config(storage);

// Create a session for authenticated user
let session_id = session_provider.create_session(
    &user,
    Some("csrf_token".to_string()),
    Some("192.168.1.1".to_string()),
    Some("Mozilla/5.0...".to_string()),
).await?;

// Validate session
let session_data = session_provider.validate_session(&session_id).await?;
println!("Session for: {}", session_data.username);

// Clean up expired sessions
let cleaned = session_provider.cleanup_expired().await?;
println!("Cleaned {} expired sessions", cleaned);
```

### Password Security

```rust
use elif_auth::{CryptoUtils, Argon2Hasher, PasswordHasher};

// Hash password with default settings
let password = "user_password123";
let hash = CryptoUtils::hash_password(password)?;

// Verify password
let is_valid = CryptoUtils::verify_password(password, &hash)?;

// Use specific hasher
let hasher = Argon2Hasher::production(); // High security settings
let hash = hasher.hash_password(password)?;
let is_valid = hasher.verify_password(password, &hash)?;

// Validate password strength
CryptoUtils::validate_password_strength(
    password,
    8,    // min length
    128,  // max length  
    true, // require uppercase
    true, // require lowercase
    true, // require numbers
    true, // require special chars
)?;
```

### Middleware Integration

```rust
use elif_auth::{JwtMiddleware, SessionMiddleware, SessionMiddlewareConfig};

// JWT Middleware
let jwt_middleware = JwtMiddleware::new(jwt_provider)
    .skip_path("/health")
    .optional(); // Don't fail on missing tokens

// Session Middleware  
let session_config = SessionMiddlewareConfig::production()
    .cookie_name("app_session")
    .cookie_secure(true);

let session_middleware = SessionMiddleware::new(session_provider, session_config);

// Extract and validate session from cookie
if let Some(session_id) = session_middleware.extract_session_id_from_cookie(cookie_header) {
    let session_data = session_middleware.validate_session(&session_id).await?;
    let user_context = session_middleware.create_user_context(&session_data);
    println!("Authenticated user: {}", user_context.username);
}
```

## Configuration

### JWT Configuration

```rust
use elif_auth::JwtConfig;

let config = JwtConfig {
    secret: env::var("JWT_SECRET")?,
    algorithm: "HS256".to_string(),
    access_token_expiry: 900,      // 15 minutes
    refresh_token_expiry: 604800,  // 1 week
    issuer: "myapp".to_string(),
    audience: Some("users".to_string()),
    allow_refresh: true,
};
```

### Session Configuration

```rust
use elif_auth::{SessionMiddlewareConfig, CookieSameSite};

// Production configuration
let config = SessionMiddlewareConfig::production()
    .cookie_name("secure_session")
    .cookie_domain("example.com")
    .cookie_secure(true)
    .cookie_same_site(CookieSameSite::Strict)
    .require_csrf(true);

// Development configuration  
let dev_config = SessionMiddlewareConfig::development()
    .cookie_secure(false)
    .require_csrf(false);
```

## Storage Backends

### Memory Storage (Development)

```rust
use elif_auth::MemorySessionStorage;

let storage = MemorySessionStorage::new();
// ⚠️  Sessions lost on restart - development only
```

### Custom Storage Implementation

Implement the `SessionStorage` trait for custom backends:

```rust
use elif_auth::{SessionStorage, SessionId, SessionData};
use async_trait::async_trait;

struct DatabaseSessionStorage {
    pool: sqlx::Pool<sqlx::Postgres>,
}

#[async_trait]
impl SessionStorage for DatabaseSessionStorage {
    type SessionId = SessionId;
    type SessionData = SessionData;
    
    async fn create_session(
        &self, 
        data: SessionData,
        expires_at: DateTime<Utc>
    ) -> AuthResult<SessionId> {
        // Implementation for database storage
        todo!()
    }
    
    // ... implement other required methods
}
```

## Security Features

### Password Hashing

- **Argon2id** - Recommended for new applications (memory-hard)
- **bcrypt** - Compatible with existing systems
- **Configurable costs** - Development vs production settings
- **Password strength validation** - Customizable requirements

### Session Security

- **Secure session IDs** - Cryptographically secure random generation
- **Cookie security** - HttpOnly, Secure, SameSite attributes
- **CSRF protection** - Integrated CSRF token management
- **Automatic cleanup** - Expired session removal
- **IP and User-Agent binding** - Session hijacking protection

### JWT Security

- **HMAC signing** - HS256, HS384, HS512 algorithms
- **Token validation** - Expiration, issuer, audience checks
- **Refresh tokens** - Secure token renewal
- **Claims validation** - Custom claim verification

## Error Handling

```rust
use elif_auth::{AuthError, AuthResult};

match jwt_provider.generate_token(&user, "access") {
    Ok(token) => println!("Token: {}", token.token),
    Err(AuthError::Configuration { message }) => {
        eprintln!("Config error: {}", message);
    }
    Err(AuthError::Token { message }) => {
        eprintln!("Token error: {}", message);  
    }
    Err(AuthError::Crypto { message }) => {
        eprintln!("Crypto error: {}", message);
    }
    Err(err) => eprintln!("Other error: {}", err),
}
```

## Feature Flags

```toml
[dependencies]
elif-auth = { version = "0.1.0", features = ["argon2", "bcrypt", "jwt"] }

# Or selectively:
elif-auth = { version = "0.1.0", features = ["jwt"], default-features = false }
```

Available features:
- `argon2` - Argon2 password hashing (enabled by default)
- `bcrypt` - bcrypt password hashing (enabled by default)  
- `jwt` - JWT token support (enabled by default)
- `session` - Session-based authentication (always available)

## Testing

The crate includes comprehensive tests covering:

- JWT token generation and validation
- Session lifecycle management
- Password hashing and verification
- Middleware functionality
- Security configurations

Run tests with:

```bash
cargo test --package elif-auth
```

## Examples

See the [examples](../../examples/) directory for complete working examples:

- JWT authentication flow
- Session-based login/logout
- Password management
- Middleware integration
- Custom storage backends

## Framework Integration

This crate is part of the [elif.rs](https://github.com/krcpa/elif.rs) framework ecosystem:

- **[elif-core](../core)** - Dependency injection and configuration
- **[elif-http](../elif-http)** - HTTP server and routing
- **[elif-security](../elif-security)** - Security middleware (CORS, CSRF, rate limiting)
- **[elif-orm](../orm)** - Database ORM and query builder
- **[elif-validation](../elif-validation)** - Input validation

## Contributing

Contributions are welcome! Please see the [contributing guidelines](../../CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT OR Apache-2.0 license. See [LICENSE](../../LICENSE) files for details.

## Changelog

### 0.1.0 - Initial Release

- JWT authentication provider with token management
- Session-based authentication with multiple storage backends
- Password hashing with Argon2 and bcrypt support
- Authentication middleware for HTTP requests
- Comprehensive security configurations
- Role-based access control foundations
- 51 passing tests with full coverage