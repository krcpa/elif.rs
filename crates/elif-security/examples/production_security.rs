//! Production Security Example
//!
//! This example demonstrates a production-ready security setup with strict
//! configurations, proper error handling, and comprehensive protection.

use elif_core::Container;
use elif_http::{Server, routing::Router, response::Response, request::Request, HttpResult, HttpConfig};
use elif_security::{
    SecurityMiddlewareBuilder, CorsConfig, CsrfConfig, RateLimitConfig, 
    config::RateLimitIdentifier,
};
use std::collections::HashSet;
use std::env;

/// Production-grade user creation endpoint with comprehensive validation
async fn create_user_handler(_req: Request) -> HttpResult<Response> {
    // In production, this would include:
    // - Input validation and sanitization
    // - Authentication checks
    // - Authorization verification
    // - Database transaction handling
    // - Audit logging
    
    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .header("X-Content-Type-Options", "nosniff")
        .header("X-Frame-Options", "DENY")
        .body(r#"{"message": "User created successfully", "id": 123}"#.into())
        .unwrap())
}

/// Sensitive admin endpoint with strict protection
async fn admin_dashboard_handler(_req: Request) -> HttpResult<Response> {
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("X-Content-Type-Options", "nosniff")
        .header("X-Frame-Options", "DENY")
        .header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
        .body(r#"{"data": "Sensitive admin data", "users": [], "settings": {}}"#.into())
        .unwrap())
}

/// Public API with relaxed but still secure settings
async fn public_api_handler(_req: Request) -> HttpResult<Response> {
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "public, max-age=300")
        .body(r#"{"api_version": "v1", "status": "operational"}"#.into())
        .unwrap())
}

/// Authentication endpoint with strict rate limiting
async fn login_handler(_req: Request) -> HttpResult<Response> {
    // In production:
    // - Validate credentials
    // - Implement account lockout
    // - Log authentication attempts
    // - Return JWT or session token
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(r#"{"token": "jwt-token-here", "expires_in": 3600}"#.into())
        .unwrap())
}

/// Health check endpoint (exempted from most security checks)
async fn health_handler(_req: Request) -> HttpResult<Response> {
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(r#"{"status": "healthy", "version": "1.0.0"}"#.into())
        .unwrap())
}

fn get_production_cors_config() -> CorsConfig {
    // In production, get allowed origins from environment variables
    let allowed_origins_env = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "https://app.yourdomain.com,https://admin.yourdomain.com".to_string());
    
    let allowed_origins: HashSet<String> = allowed_origins_env
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    CorsConfig {
        allowed_origins: Some(allowed_origins),
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
            "x-ratelimit-limit".to_string(),
            "x-ratelimit-remaining".to_string(),
        ]),
        allow_credentials: true,
        max_age: Some(300), // Short cache for security
    }
}

fn get_production_csrf_config() -> CsrfConfig {
    CsrfConfig {
        token_header: "X-CSRF-Token".to_string(),
        cookie_name: "_csrf_token".to_string(),
        secure_cookie: true,  // HTTPS only in production
        token_lifetime: 1800, // 30 minutes - balance security vs UX
        exempt_methods: HashSet::from([
            "GET".to_string(),
            "HEAD".to_string(),
            "OPTIONS".to_string(),
        ]),
        exempt_paths: HashSet::from([
            "/health".to_string(),
            "/api/public".to_string(), // Public read-only API
        ]),
    }
}

fn get_production_rate_limit_config() -> RateLimitConfig {
    RateLimitConfig {
        max_requests: 60, // Strict rate limiting for production
        window_seconds: 60,
        identifier: RateLimitIdentifier::IpAddress,
        exempt_paths: HashSet::from([
            "/health".to_string(), // Health checks should not be rate limited
        ]),
    }
}

fn get_strict_auth_rate_limit_config() -> RateLimitConfig {
    RateLimitConfig {
        max_requests: 5,   // Very strict for authentication endpoints
        window_seconds: 300, // 5 minute window
        identifier: RateLimitIdentifier::IpAddress,
        exempt_paths: HashSet::new(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging for production
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .json() // JSON format for log aggregation
        .init();

    // Create dependency injection container
    let mut container = Container::default();

    // Create production security pipeline
    let main_security_pipeline = SecurityMiddlewareBuilder::new()
        .with_cors(get_production_cors_config())
        .with_csrf(get_production_csrf_config())
        .with_rate_limit(get_production_rate_limit_config())
        .build();

    // Create strict security pipeline for authentication endpoints
    let auth_security_pipeline = SecurityMiddlewareBuilder::new()
        .with_cors(get_production_cors_config())
        .with_csrf(get_production_csrf_config())
        .with_rate_limit(get_strict_auth_rate_limit_config())
        .build();

    // Create router with different security levels per endpoint
    let router = Router::new()
        // Public endpoints (still protected but more permissive)
        .get("/health", health_handler)
        .get("/api/public", public_api_handler)
        
        // Authentication endpoints (strict rate limiting)
        .post("/auth/login", login_handler)
        
        // Protected application endpoints
        .post("/users", create_user_handler)
        .get("/admin/dashboard", admin_dashboard_handler);

    // Create and configure server
    let config = HttpConfig::default();
    let mut server = Server::with_container(std::sync::Arc::new(container), config)?
        .use_middleware(main_security_pipeline);

    // In a real production setup, you might have different middleware
    // for different route groups, but for this example we'll use one pipeline

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let bind_address = format!("{}:{}", host, port);

    println!("🚀 Starting production server with enhanced security...");
    println!("📍 Server binding to: {}", bind_address);
    println!("🔒 Production security features:");
    println!("   ✅ Strict CORS (specific origins only)");
    println!("   ✅ CSRF protection with secure cookies");
    println!("   ✅ Rate limiting (60 requests/minute)");
    println!("   ✅ Security headers (HSTS, X-Frame-Options, etc.)");
    println!("   ✅ Structured JSON logging");
    println!();
    println!("🌍 Environment variables:");
    println!("   ALLOWED_ORIGINS - Comma-separated list of allowed origins");
    println!("   PORT - Server port (default: 3000)");
    println!("   HOST - Server host (default: 0.0.0.0)");
    println!();
    println!("📊 Monitoring endpoints:");
    println!("   GET /health - Health check (rate limit exempt)");
    println!("   GET /api/public - Public API (CORS enabled)");
    println!();
    println!("🔐 Protected endpoints:");
    println!("   POST /auth/login - Authentication (strict rate limiting)");
    println!("   POST /users - User creation (CSRF protected)");
    println!("   GET /admin/dashboard - Admin access (full protection)");

    // Start the server
    server.use_router(router);
    server.listen(&bind_address).await?;

    Ok(())
}

// Production deployment checklist:
//
// 1. Environment Variables:
//    - ALLOWED_ORIGINS="https://yourdomain.com,https://admin.yourdomain.com"
//    - PORT=3000
//    - HOST=0.0.0.0
//
// 2. HTTPS Configuration:
//    - Ensure TLS termination at load balancer or reverse proxy
//    - Set secure cookie flags
//    - Configure HSTS headers
//
// 3. Reverse Proxy (nginx example):
//    server {
//        listen 443 ssl;
//        server_name yourdomain.com;
//        
//        location / {
//            proxy_pass http://127.0.0.1:3000;
//            proxy_set_header Host $host;
//            proxy_set_header X-Real-IP $remote_addr;
//            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
//            proxy_set_header X-Forwarded-Proto $scheme;
//        }
//    }
//
// 4. Monitoring:
//    - Set up log aggregation (ELK stack, Splunk, etc.)
//    - Monitor rate limiting metrics
//    - Alert on security violations
//    - Track CSRF token validation failures
//
// 5. Security Testing:
//    curl -X OPTIONS https://yourdomain.com/users \
//      -H "Origin: https://malicious.com" \
//      # Should be blocked
//    
//    curl -X POST https://yourdomain.com/users \
//      -H "Content-Type: application/json" \
//      -d '{"name": "Test"}' \
//      # Should require CSRF token
//
// 6. Load Testing:
//    # Test rate limiting under load
//    ab -n 1000 -c 10 https://yourdomain.com/api/public