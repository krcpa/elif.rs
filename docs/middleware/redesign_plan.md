# Elif.rs Middleware Redesign Plan

## Executive Summary
This plan outlines a complete redesign of the elif.rs middleware system to achieve Laravel-style simplicity while maintaining Rust's performance and type safety. The goal is to make middleware creation intuitive for developers coming from web frameworks like Laravel, NestJS, or Express.

## Current State Analysis

### Problems with Current Implementation
1. **Complex Trait Definition**: Two separate methods (`process_request` and `process_response`) with boxed futures
2. **No Next Pattern**: Missing the intuitive `handle(request, next)` pattern
3. **Axum Leakage**: Internal dependencies (Axum types) exposed in some middleware
4. **Body Consumption**: Response bodies can only be read once, preventing caching middleware
5. **Poor Developer Experience**: Too much boilerplate for simple middleware

### Current Middleware Trait
```rust
pub trait Middleware: Send + Sync {
    fn process_request<'a>(&'a self, request: Request) -> BoxFuture<'a, Result<Request, Response>>;
    fn process_response<'a>(&'a self, response: Response) -> BoxFuture<'a, Response>;
    fn name(&self) -> &'static str;
}
```

## Proposed Design

### New Middleware Trait
```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn handle(&self, request: ElifRequest, next: Next) -> ElifResponse;
}
```

### Next Type Implementation
```rust
pub struct Next {
    inner: Box<dyn Future<Output = ElifResponse> + Send>,
}

impl Next {
    pub async fn run(self, request: ElifRequest) -> ElifResponse {
        // Execute the rest of the middleware chain
    }
}
```

### Simple Middleware Example (Target DX)
```rust
pub struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn handle(&self, request: ElifRequest, next: Next) -> ElifResponse {
        // Before request
        let start = Instant::now();
        let method = request.method().clone();
        let path = request.path().to_string();
        
        // Pass to next middleware
        let response = next.run(request).await;
        
        // After response
        let duration = start.elapsed();
        info!("{} {} - {} - {:?}", method, path, response.status(), duration);
        
        response
    }
}
```

## Implementation Phases

### Phase 1: Core Redesign (9.8.1)
- Create new middleware trait with `handle(request, next)` pattern
- Implement `Next` type for middleware chaining
- Create middleware adapter for backward compatibility
- Implement body buffering for response manipulation

### Phase 2: Framework Integration (9.8.2)
- Update `ElifRouter` to use new middleware system
- Create middleware registration API
- Implement middleware ordering and priorities
- Add route-specific middleware support

### Phase 3: Built-in Middleware Migration (9.8.3)
- Migrate all existing middleware to new pattern
- Implement common middleware (CORS, Auth, Rate Limit, etc.)
- Create middleware composition utilities
- Add middleware testing utilities

### Phase 4: Advanced Features (9.8.4)
- Implement conditional middleware execution
- Add middleware groups and named stacks
- Create middleware factories for common patterns
- Performance optimizations

### Phase 5: Developer Tools (9.8.5)
- CLI generator for middleware (`elifrs generate middleware`)
- Middleware debugging and introspection
- Documentation and examples
- Migration guide from old system

## Key Design Decisions

### 1. Single Async Method
- Simpler mental model
- Matches Laravel/Express patterns
- Easier to generate and understand

### 2. Framework Types Only
- Use `ElifRequest` and `ElifResponse`
- No Axum/Hyper types exposed
- Clean abstraction boundaries

### 3. Body Buffering
- Implement automatic body buffering for middleware that needs it
- Opt-in mechanism to avoid performance overhead
- Solves the response caching problem

### 4. Middleware Composition
```rust
// Laravel-style middleware groups
app.middleware_group("api", vec![
    RateLimitMiddleware::new(100),
    AuthMiddleware::new(),
    JsonMiddleware::new(),
]);

// Route-specific middleware
app.route("/admin/*")
   .middleware(AdminAuthMiddleware::new())
   .get(admin_handler);
```

## Success Criteria
- [ ] Developers can create middleware in < 10 lines of code
- [ ] AI can generate middleware from simple prompts
- [ ] No Axum/Hyper types in user code
- [ ] Response body can be read/modified by middleware
- [ ] Performance overhead < 5% vs current system
- [ ] Full backward compatibility during migration

## Example: Complete Auth Middleware
```rust
use elif::prelude::*;

pub struct AuthMiddleware {
    jwt_secret: String,
}

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(&self, mut request: ElifRequest, next: Next) -> ElifResponse {
        // Extract token
        let token = match request.header("Authorization") {
            Some(h) if h.starts_with("Bearer ") => &h[7..],
            _ => return ElifResponse::unauthorized().json(json!({
                "error": "Missing or invalid authorization header"
            })),
        };
        
        // Validate token
        match validate_jwt(token, &self.jwt_secret) {
            Ok(claims) => {
                // Add user to request context
                request.set_context("user", claims);
                next.run(request).await
            }
            Err(e) => ElifResponse::unauthorized().json(json!({
                "error": format!("Invalid token: {}", e)
            })),
        }
    }
}
```

## Migration Strategy
1. Implement new system alongside old
2. Provide adapter for old middleware
3. Gradually migrate built-in middleware
4. Deprecate old system in 10.0
5. Remove old system in 11.0

## Performance Considerations
- Use `Arc` for shared middleware state
- Minimize allocations in hot path
- Lazy body buffering only when needed
- Benchmark against current system

## AI/LLM Considerations
- Simple, predictable patterns
- Clear naming conventions
- Minimal boilerplate
- Easy to generate from specs

## Timeline
- Phase 1: 2 days
- Phase 2: 2 days
- Phase 3: 3 days
- Phase 4: 2 days
- Phase 5: 1 day

Total: 10 days (can be parallelized to 5-6 days)