# Phase 10: Macro System Design Specification

## Overview
Technical specification for the decorator/attribute macro system that will transform elif.rs from verbose trait-based controllers to elegant, Laravel/NestJS-style development experience.

## Core Macro Architecture

### 1. Controller Registration Macros

#### `#[controller]` - Primary Controller Macro
```rust
#[controller]
#[route("/api/users")]  // Optional base route
#[inject(user_service: UserService, email_service: EmailService)]
#[middleware(auth, cors)]  // Applied to all routes
impl UserController {
    // Controller methods
}
```

**Generated Code:**
- Implements `Controller` trait automatically
- Creates DI container integration methods
- Registers all routes with Axum router
- Sets up middleware pipeline

#### `#[route("/path")]` - Base Route Prefix
```rust
#[controller]
#[route("/api/v1/users")]
impl UserController {
    #[get("/{id}")]  // Results in: GET /api/v1/users/{id}
    async fn show(&self, id: Uuid) -> Result<User, HttpError> {
        // ...
    }
}
```

### 2. HTTP Method Macros

#### Route Method Decorators
```rust
#[get("/path")]           // GET request
#[post("/path")]          // POST request  
#[put("/path")]           // PUT request
#[patch("/path")]         // PATCH request
#[delete("/path")]        // DELETE request
#[options("/path")]       // OPTIONS request
#[head("/path")]          // HEAD request

// Shorthand versions (inherit from controller base route)
#[get]                    // GET to base route
#[post]                   // POST to base route
#[put("/{id}")]          // PUT to base route + /{id}
```

**Generated Handler:**
```rust
// From:
#[get("/{id}")]
async fn show(&self, id: Uuid) -> Result<User, HttpError> {
    self.user_service.find(id).await
}

// Generates:
pub fn show_handler() -> impl Handler {
    |State(container): State<Arc<Container>>, Path(id): Path<Uuid>| async move {
        let controller = Self::from_container(&container)?;
        controller.show(id).await
    }
}
```

### 3. Parameter Extraction Macros

#### `#[path]` - Path Parameters
```rust
#[get("/users/{id}/posts/{post_id}")]
async fn show(&self, 
    #[path] id: Uuid,           // Extracts {id}
    #[path] post_id: Uuid       // Extracts {post_id}
) -> Result<Post, HttpError> {
    // ...
}
```

#### `#[query]` - Query Parameters  
```rust
#[get("/users")]
async fn index(&self,
    #[query] page: Option<u32>,
    #[query] search: Option<String>,
    #[query] params: QueryParams,  // Struct extraction
) -> Result<Vec<User>, HttpError> {
    // ...
}
```

#### `#[body]` - Request Body
```rust
#[post("/users")]
async fn create(&self, #[body] user_data: CreateUserRequest) -> Result<User, HttpError> {
    // user_data is automatically deserialized and validated
}
```

#### `#[header]` - Header Extraction
```rust
#[get("/users")]
async fn index(&self,
    #[header("Authorization")] auth: Option<String>,
    #[header] user_agent: Option<String>,  // Auto-maps to User-Agent
) -> Result<Vec<User>, HttpError> {
    // ...
}
```

### 4. Service Injection Macros

#### `#[inject]` - Dependency Injection
```rust
#[controller]
#[inject(
    user_service: UserService,
    email_service: EmailService,
    cache: RedisService
)]
impl UserController {
    // Services automatically available as self.user_service, etc.
    
    #[post]
    async fn create(&self, #[body] data: CreateUserRequest) -> Result<User, HttpError> {
        let user = self.user_service.create(data).await?;
        self.email_service.send_welcome(&user.email).await?;
        Ok(user)
    }
}
```

**Generated Structure:**
```rust
impl UserController {
    user_service: Arc<UserService>,
    email_service: Arc<EmailService>, 
    cache: Arc<RedisService>,
    
    pub fn from_container(container: &Container) -> Result<Self, HttpError> {
        Ok(Self {
            user_service: container.get("user_service")?,
            email_service: container.get("email_service")?,
            cache: container.get("cache")?,
        })
    }
}
```

### 5. Validation Macros

#### `#[validate]` - Request Validation
```rust
#[derive(Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    name: String,
    
    #[validate(email)]
    email: String,
    
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[post("/users")]
#[validate(CreateUserRequest)]
async fn create(&self, #[body] data: CreateUserRequest) -> Result<User, HttpError> {
    // data is guaranteed to be valid
}
```

**Generated Validation Logic:**
```rust
// Macro generates:
pub fn create_handler() -> impl Handler {
    |State(container): State<Arc<Container>>, Json(data): Json<Value>| async move {
        let data: CreateUserRequest = serde_json::from_value(data)
            .map_err(|e| HttpError::bad_request(&format!("Invalid JSON: {}", e)))?;
            
        data.validate()
            .map_err(|e| HttpError::validation_error(&format!("Validation failed: {}", e)))?;
            
        let controller = Self::from_container(&container)?;
        controller.create(data).await
    }
}
```

### 6. Authentication & Authorization Macros

#### `#[auth]` - Authentication Requirements
```rust
#[controller]
#[route("/api/users")]
impl UserController {
    #[get]
    #[auth(optional)]  // Optional authentication
    async fn index(&self, #[user] user: Option<AuthUser>) -> Result<Vec<User>, HttpError> {
        // ...
    }
    
    #[post]
    #[auth(required)]  // Required authentication
    async fn create(&self, #[user] user: AuthUser, #[body] data: CreateUserRequest) -> Result<User, HttpError> {
        // user is guaranteed to exist
    }
    
    #[delete("/{id}")]
    #[auth(required)]
    #[roles("admin", "moderator")]  // Role-based access
    async fn delete(&self, #[user] user: AuthUser, #[path] id: Uuid) -> Result<(), HttpError> {
        // user has admin or moderator role
    }
}
```

#### `#[user]` - User Injection
```rust
#[get("/profile")]
#[auth(required)]
async fn profile(&self, #[user] user: AuthUser) -> Result<UserProfile, HttpError> {
    // user extracted from JWT/session automatically
}
```

### 7. Resource Transformation Macros

#### `#[resource]` - Response Transformation
```rust
#[derive(Resource)]
#[resource(User)]  // Links to User model
struct UserResource {
    id: Uuid,
    name: String,
    email: String,
    // password field omitted for security
    
    #[resource(nested)]
    posts: Vec<PostResource>,  // Auto-load related data
}

#[get("/{id}")]
#[resource(UserResource)]  // Auto-transform response
async fn show(&self, #[path] id: Uuid) -> Result<User, HttpError> {
    self.user_service.find(id).await  // Returns User, transformed to UserResource
}
```

### 8. CRUD Generation Macros

#### `#[crud]` - Auto-generate CRUD Operations
```rust
#[controller]
#[route("/api/users")]
#[inject(user_service: UserService)]
impl UserController {
    #[crud(User, resource = UserResource)]
    // Generates: index, show, create, update, delete methods
    
    // Custom methods can still be added:
    #[post("/{id}/activate")]
    async fn activate(&self, #[path] id: Uuid) -> Result<User, HttpError> {
        self.user_service.activate(id).await
    }
}
```

**Generated CRUD Methods:**
```rust
// Auto-generated by #[crud] macro:
#[get]
async fn index(&self, #[query] params: QueryParams) -> Result<Vec<UserResource>, HttpError> {
    let users = self.user_service.find_all(params).await?;
    Ok(users.into_iter().map(UserResource::from).collect())
}

#[get("/{id}")]
async fn show(&self, #[path] id: Uuid) -> Result<UserResource, HttpError> {
    let user = self.user_service.find(id).await?;
    Ok(UserResource::from(user))
}

#[post]
#[validate(CreateUserRequest)]
async fn create(&self, #[body] data: CreateUserRequest) -> Result<UserResource, HttpError> {
    let user = self.user_service.create(data.into()).await?;
    Ok(UserResource::from(user))
}

// ... etc
```

### 9. Middleware Macros

#### `#[middleware]` - Apply Middleware
```rust
#[controller]
#[middleware(cors, rate_limit("100/hour"))]  // Controller-level
impl UserController {
    #[get]
    #[middleware(cache("5m"))]  // Method-level
    async fn index(&self) -> Result<Vec<User>, HttpError> {
        // ...
    }
}
```

### 10. Response Macros

#### `#[cache]` - Response Caching
```rust
#[get]
#[cache(ttl = "5m", key = "users:all")]
async fn index(&self) -> Result<Vec<User>, HttpError> {
    // Response cached for 5 minutes
}
```

#### `#[rate_limit]` - Rate Limiting  
```rust
#[post]
#[rate_limit("5/minute")]  // Max 5 requests per minute
async fn create(&self, #[body] data: CreateUserRequest) -> Result<User, HttpError> {
    // ...
}
```

## Implementation Strategy

### Phase 1: Core Route Macros
1. `#[controller]` - Basic controller registration
2. `#[get]`, `#[post]`, etc. - HTTP method routing
3. `#[path]`, `#[query]`, `#[body]` - Parameter extraction
4. Integration with Axum router

### Phase 2: Service Injection
1. `#[inject]` - DI container integration
2. Auto-generate service field access
3. Container resolution at runtime

### Phase 3: Validation System
1. `#[validate]` - Request validation
2. Integration with `validator` crate
3. Custom validation error responses

### Phase 4: Authentication
1. `#[auth]` - Authentication middleware
2. `#[user]` - User extraction
3. `#[roles]` - Role-based authorization

### Phase 5: Resources & CRUD
1. `#[resource]` - Response transformation
2. `#[crud]` - Auto-generate CRUD operations
3. Resource relationship handling

## Error Handling Strategy

### Macro Error Messages
```rust
// Good error messages for common mistakes:

#[get("/users/{id}")]
async fn show(&self, id: String) -> Result<User, HttpError> {
    //                   ^^^^^^
    //                   Expected Uuid, got String
    //                   Help: Use #[path] id: Uuid or parse manually
}

#[inject(unknown_service: UnknownService)]
//       ^^^^^^^^^^^^^^^
//       Service 'unknown_service' not registered in container
//       Available services: user_service, email_service, cache
```

### Compile-time Validation
- Verify service types exist in container at compile time
- Validate route path parameters match function parameters  
- Ensure authentication decorators are consistent
- Check resource field mappings

## Performance Considerations

### Code Generation Optimization
- Generate minimal boilerplate code
- Avoid unnecessary allocations
- Compile-time route table generation
- Efficient service resolution

### Runtime Performance
- Zero-cost abstractions where possible
- Lazy service initialization
- Cached container lookups
- Optimized serialization paths

## Testing Strategy

### Macro Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_controller_macro_expansion() {
        // Test macro expansion results
        let expanded = quote! {
            #[controller]
            #[route("/users")]
            impl UserController {
                #[get("/{id}")]
                async fn show(&self, #[path] id: Uuid) -> Result<User, HttpError> {
                    Ok(User::default())
                }
            }
        };
        
        // Verify generated code
        assert_contains!(expanded, "pub fn show_handler");
        assert_contains!(expanded, "Path(id): Path<Uuid>");
    }
}
```

### Integration Testing
- Full controller lifecycle tests
- Service injection verification  
- Route registration validation
- Error handling verification

This macro system will transform elif.rs into the most ergonomic web framework in Rust while maintaining all type safety and performance benefits.