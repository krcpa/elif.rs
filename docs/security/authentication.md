# Authentication

Two primary strategies are supported: JWT (bearer tokens) and cookie-based sessions. Utilities for password hashing are also included.

Password hashing
```rust
use elif_auth::utils::CryptoUtils;

let hash = CryptoUtils::hash_password("secret")?;
let ok = CryptoUtils::verify_password("secret", &hash)?;
```

JWT provider (from README APIs)
```rust
use elif_auth::JwtProvider;

let config = /* load JwtConfig (issuer, audience, secret, expiries) */;
let jwt = JwtProvider::new(config)?;

// Generate tokens
let (access, refresh) = jwt.generate_token_pair(&user)?;

// Validate
let claims = jwt.validate_token_claims(&access)?;
```

Sessions (outline)
```rust
use elif_auth::SessionProvider;
// let storage = ...; // memory/redis/db-backed storage
let session = SessionProvider::with_default_config(storage);
let id = session.create_session(user_id, Some("csrf".into()), None).await?;
let data = session.validate_session(&id).await?; // check user info
```

Route protection
- Attach JWT or session middleware to protected scopes/groups.
- Extract auth context in handlers and enforce authorization policies.
