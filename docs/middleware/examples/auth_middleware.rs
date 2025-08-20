use elif::prelude::*;
use elif_http::middleware::v2::{Middleware, Next, NextFuture};
use elif_http::request::ElifRequest;
use elif_http::response::ElifResponse;
use serde_json::json;

/// JWT Authentication Middleware
/// 
/// This middleware validates JWT tokens in the Authorization header
/// and rejects requests with invalid or missing tokens.
#[derive(Debug)]
pub struct JwtAuthMiddleware {
    secret: String,
    skip_paths: Vec<String>,
}

impl JwtAuthMiddleware {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            skip_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/public".to_string(),
            ],
        }
    }
    
    pub fn skip_paths(mut self, paths: Vec<String>) -> Self {
        self.skip_paths = paths;
        self
    }
    
    fn should_skip(&self, path: &str) -> bool {
        self.skip_paths.iter().any(|skip_path| {
            path.starts_with(skip_path)
        })
    }
    
    fn extract_token(&self, request: &ElifRequest) -> Option<String> {
        request
            .header("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|auth_header| {
                if auth_header.starts_with("Bearer ") {
                    Some(auth_header[7..].to_string())
                } else {
                    None
                }
            })
    }
    
    fn validate_token(&self, token: &str) -> bool {
        // In a real implementation, you would:
        // 1. Decode the JWT
        // 2. Verify signature with the secret
        // 3. Check expiration
        // 4. Validate claims
        
        // For this example, we just check if token matches secret
        token == self.secret
    }
}

impl Middleware for JwtAuthMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let skip = self.should_skip(request.path());
        let secret = self.secret.clone();
        
        Box::pin(async move {
            // Skip authentication for public paths
            if skip {
                return next.run(request).await;
            }
            
            // Extract token from Authorization header
            let token = match extract_token_from_request(&request) {
                Some(t) => t,
                None => {
                    return ElifResponse::unauthorized()
                        .json_value(json!({
                            "error": {
                                "code": "missing_token",
                                "message": "Missing or invalid Authorization header",
                                "hint": "Include 'Authorization: Bearer <token>' header"
                            }
                        }));
                }
            };
            
            // Validate token
            if !validate_jwt_token(&token, &secret) {
                return ElifResponse::unauthorized()
                    .json_value(json!({
                        "error": {
                            "code": "invalid_token",
                            "message": "Invalid or expired token",
                            "hint": "Obtain a new token from /auth/login"
                        }
                    }));
            }
            
            // Token is valid, proceed to next middleware
            let response = next.run(request).await;
            
            // Optionally add authentication info to response headers
            match response.header("X-Authenticated", "true") {
                Ok(authenticated_response) => authenticated_response,
                Err(_) => response, // If adding header fails, return original response
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "JwtAuthMiddleware"
    }
}

// Helper functions (in real implementation, these might be in a separate auth module)
fn extract_token_from_request(request: &ElifRequest) -> Option<String> {
    request
        .header("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|auth_header| {
            if auth_header.starts_with("Bearer ") {
                Some(auth_header[7..].to_string())
            } else {
                None
            }
        })
}

fn validate_jwt_token(token: &str, secret: &str) -> bool {
    // Simplified validation - in real implementation use a JWT library
    // like `jsonwebtoken` crate
    token == secret
}

// Usage example
#[allow(dead_code)]
fn usage_example() -> Result<(), Box<dyn std::error::Error>> {
    use elif_http::{Server, HttpConfig};
    use elif_core::Container;
    
    let container = Container::new();
    let mut server = Server::new(container, HttpConfig::default())?;
    
    // Add JWT authentication middleware
    server.use_middleware(
        JwtAuthMiddleware::new("your-secret-key".to_string())
            .skip_paths(vec![
                "/health".to_string(),
                "/auth".to_string(), // Allow authentication endpoints
                "/public".to_string(), // Allow public assets
            ])
    );
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::request::{ElifMethod, ElifRequest};
    use elif_http::response::headers::ElifHeaderMap;
    use elif_http::middleware::v2::MiddlewarePipelineV2;
    
    #[tokio::test]
    async fn test_auth_middleware_with_valid_token() {
        let middleware = JwtAuthMiddleware::new("secret123".to_string());
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        // Create request with valid token
        let mut headers = ElifHeaderMap::new();
        headers.insert("authorization".parse().unwrap(), "Bearer secret123".parse().unwrap());
        let request = ElifRequest::new(ElifMethod::GET, "/protected".parse().unwrap(), headers);
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().text("Protected content")
            })
        }).await;
        
        assert_eq!(response.status_code(), elif_http::response::status::ElifStatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_auth_middleware_with_invalid_token() {
        let middleware = JwtAuthMiddleware::new("secret123".to_string());
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        // Create request with invalid token
        let mut headers = ElifHeaderMap::new();
        headers.insert("authorization".parse().unwrap(), "Bearer wrong-token".parse().unwrap());
        let request = ElifRequest::new(ElifMethod::GET, "/protected".parse().unwrap(), headers);
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().text("Should not reach here")
            })
        }).await;
        
        assert_eq!(response.status_code(), elif_http::response::status::ElifStatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_auth_middleware_skips_public_paths() {
        let middleware = JwtAuthMiddleware::new("secret123".to_string());
        let pipeline = MiddlewarePipelineV2::new().add(middleware);
        
        // Create request to public path without token
        let headers = ElifHeaderMap::new();
        let request = ElifRequest::new(ElifMethod::GET, "/health".parse().unwrap(), headers);
        
        let response = pipeline.execute(request, |_req| {
            Box::pin(async {
                ElifResponse::ok().text("Healthy")
            })
        }).await;
        
        assert_eq!(response.status_code(), elif_http::response::status::ElifStatusCode::OK);
    }
}