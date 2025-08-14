# Phase 7: Developer Experience 🚀

**Duration**: 4-5 weeks  
**Goal**: Modern framework productivity and developer experience  
**Status**: Ready after Phase 6

## Overview

Phase 7 focuses on developer productivity, providing modern framework tooling and code generation capabilities. This includes enhanced scaffolding, API documentation generation, testing framework, and comprehensive CLI tooling that makes building applications fast and enjoyable.

## Dependencies

- **Phase 2**: ✅ HTTP server and routing
- **Phase 6**: ✅ Complete ORM with relationships

## Key Components

### 1. Enhanced Code Generation & Scaffolding
**File**: `crates/elif-cli/src/generators/mod.rs`

Advanced code generation system that creates complete application scaffolding.

**Requirements**:
- Resource scaffolding (model, controller, migration, tests)
- API resource generation with OpenAPI specs
- Authentication scaffolding (login, register, middleware)
- CRUD controller generation with validation
- Migration generation from model changes
- Policy and authorization scaffolding

**API Design**:
```bash
# Generate complete resource with relationships
elifrs make:resource Post --fields "title:string,content:text,user_id:integer" \
  --relationships "user:belongs_to,comments:has_many,tags:belongs_to_many" \
  --api --tests --policy

# Generated files:
# ├── src/models/post.rs              # Model with relationships  
# ├── src/controllers/post_controller.rs # Full CRUD controller
# ├── src/policies/post_policy.rs     # Authorization policy
# ├── migrations/create_posts_table.sql
# ├── tests/models/post_test.rs       # Model tests
# ├── tests/controllers/post_controller_test.rs # Controller tests
# └── openapi/post_api.yml            # OpenAPI specification

# Generate authentication system
elifrs make:auth --jwt --mfa --password-reset

# Generate API with versioning
elifrs make:api v2 --from-openapi api_spec.yml

# Generate complete application structure
elifrs new blog-app --template api --with-auth --with-admin
```

**Generated Code Quality**:
```rust
// Generated controller with proper validation and error handling
#[controller]
pub struct PostController {
    post_service: Arc<PostService>,
}

impl PostController {
    // <<<ELIF:BEGIN agent-editable:post-index>>>
    pub async fn index(&self, request: Request) -> Result<Response, HttpError> {
        let query = Post::query().with("user").with_count("comments");
        
        let posts = query.paginate(request.per_page()).await?;
        
        Ok(Response::json(PostCollection::new(posts)))
    }
    // <<<ELIF:END agent-editable:post-index>>>

    // <<<ELIF:BEGIN agent-editable:post-store>>>
    pub async fn store(&self, mut request: Request) -> Result<Response, HttpError> {
        let user = request.require_user()?;
        let data: CreatePostRequest = request.validate_json()?;
        
        user.can("create", Post::class())?;
        
        let post = Post::create(request.database(), Post {
            title: data.title,
            content: data.content,
            user_id: user.id,
            ..Default::default()
        }).await?;
        
        Ok(Response::json(PostResource::new(post)).status(201))
    }
    // <<<ELIF:END agent-editable:post-store>>>
}
```

### 2. API Documentation Generation (OpenAPI)
**File**: `crates/elif-openapi/src/lib.rs`

Automatic OpenAPI/Swagger documentation generation from code annotations.

**Requirements**:
- OpenAPI 3.0 specification generation
- Automatic endpoint discovery from routes
- Request/response schema generation from structs
- Authentication scheme documentation
- Interactive API documentation (Swagger UI)
- Postman collection export

**API Design**:
```rust
// API documentation annotations
#[openapi(
    tag = "Posts",
    summary = "Create a new post",
    description = "Creates a new blog post with the given title and content",
)]
pub async fn store(&self, request: Request) -> Result<Response, HttpError> {
    // Implementation
}

#[derive(Serialize, Deserialize, OpenApiSchema)]
#[openapi(
    description = "Request payload for creating a new post",
    example = r#"{"title": "My Post", "content": "Hello world!"}"#
)]
pub struct CreatePostRequest {
    #[openapi(description = "Post title", max_length = 255)]
    pub title: String,
    
    #[openapi(description = "Post content in Markdown")]  
    pub content: String,
    
    #[openapi(description = "Tags for the post", nullable = true)]
    pub tags: Option<Vec<String>>,
}

// CLI commands
elifrs openapi:generate --output docs/api.yml
elifrs openapi:serve --port 8080  // Serve Swagger UI
elifrs openapi:export --format postman --output api.postman.json
```

### 3. Testing Framework & Utilities
**File**: `crates/elif-testing/src/lib.rs`

Comprehensive testing framework with utilities for all types of testing.

**Requirements**:
- Test database management (transactions, seeding)
- HTTP testing utilities (test client, assertions)
- Factory system for test data generation
- Mocking and stubbing utilities
- Performance and load testing tools
- Integration with existing test runners

**API Design**:
```rust
// Test utilities
#[cfg(test)]
mod tests {
    use elif_testing::prelude::*;
    
    #[test_database]
    async fn test_post_creation() {
        // Automatic test database setup and cleanup
        let user = UserFactory::new().create().await?;
        
        let post_data = CreatePostRequest {
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
        };
        
        let response = TestClient::new()
            .authenticated_as(&user)
            .post("/api/posts")
            .json(&post_data)
            .send()
            .await?;
            
        response.assert_status(201)
               .assert_json_contains(json!({"title": "Test Post"}));
               
        // Verify database changes
        assert_database_has("posts", |post: Post| {
            post.title == "Test Post" && post.user_id == user.id
        }).await?;
    }
    
    #[test]
    async fn test_post_validation() {
        let response = TestClient::new()
            .post("/api/posts")
            .json(&json!({"title": ""}))  // Invalid: empty title
            .send()
            .await?;
            
        response.assert_validation_error("title", "required");
    }
}

// Factory definitions
#[factory]
pub struct UserFactory {
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl UserFactory {
    pub fn admin(mut self) -> Self {
        self.email = format!("admin+{}@example.com", Uuid::new_v4());
        self
    }
    
    pub fn with_posts(self, count: usize) -> Self {
        self.after_create(move |user| {
            PostFactory::new().count(count).for_user(user.id).create()
        })
    }
}
```

### 4. CLI Command System
**File**: `crates/elif-cli/src/commands/mod.rs`

Extensible CLI system for application management and development tasks.

**Requirements**:
- Custom command creation and registration
- Command argument and option parsing
- Interactive commands with prompts
- Scheduled command execution (cron-like)
- Command progress bars and output formatting
- Built-in development server commands

**API Design**:
```rust
// Custom command definition
#[derive(Command)]
#[command(
    name = "posts:cleanup",
    description = "Clean up old published posts",
    help = "Removes posts older than the specified number of days"
)]
pub struct PostCleanupCommand {
    #[arg(long, short, default_value = "30")]
    days: u32,
    
    #[arg(long)]
    dry_run: bool,
}

impl CommandHandler for PostCleanupCommand {
    async fn handle(&self) -> Result<(), CommandError> {
        let cutoff_date = Utc::now() - Duration::days(self.days as i64);
        
        let query = Post::query()
            .where_lt("created_at", cutoff_date)
            .where_eq("status", "published");
            
        if self.dry_run {
            let count = query.count().await?;
            println!("Would delete {} posts", count);
            return Ok(());
        }
        
        let progress = ProgressBar::new("Deleting posts");
        let deleted = query.delete().await?;
        progress.finish_with_message(format!("Deleted {} posts", deleted));
        
        Ok(())
    }
}

// Built-in commands
elifrs serve --port 3000 --hot-reload    // Development server
elifrs queue:work --queue high,default   // Queue worker
elifrs schedule:run                       // Run scheduled commands
elifrs posts:cleanup --days 60           // Custom command
```

### 5. Hot Reload Development Server
**File**: `crates/elif-dev-server/src/lib.rs`

Development server with hot reloading and enhanced development experience.

**Requirements**:
- File watching and automatic rebuilds
- Hot reload for code changes
- Request logging with detailed debugging
- Error pages with stack traces
- Development-specific middleware
- Database query logging and profiling

**API Design**:
```rust
// Development server configuration
#[derive(Config)]
pub struct DevServerConfig {
    #[config(default = 3000)]
    pub port: u16,
    
    #[config(default = true)]
    pub hot_reload: bool,
    
    #[config(default = true)]
    pub debug_queries: bool,
    
    #[config(default = "src/**/*.rs")]
    pub watch_patterns: Vec<String>,
}

// Enhanced error pages in development
DevErrorMiddleware::new()
    .show_source_code(true)
    .show_environment_variables(false)  // Security
    .show_request_data(true)
    .syntax_highlighting(true);

// Query debugging
QueryDebugMiddleware::new()
    .log_slow_queries(Duration::from_millis(10))
    .show_query_plans(true)
    .highlight_n_plus_one_queries(true);
```

### 6. Performance Profiling & Monitoring
**File**: `crates/elif-profiler/src/lib.rs`

Built-in profiling and performance monitoring for development.

**Requirements**:
- Request timing and profiling
- Database query performance tracking
- Memory usage monitoring
- CPU profiling integration
- Custom metrics and tracing
- Performance dashboard for development

**API Design**:
```rust
// Profiling middleware
ProfilerMiddleware::new()
    .profile_requests(true)
    .profile_database_queries(true)
    .profile_memory_usage(true)
    .dashboard_endpoint("/debug/profiler");

// Custom profiling  
#[profile("user_service::create_user")]
async fn create_user(&self, data: CreateUserRequest) -> Result<User, UserError> {
    let _span = tracing::span!("user_creation").enter();
    
    // Track specific metrics
    metrics::counter!("users.created").increment(1);
    metrics::histogram!("user_creation.duration").record(start.elapsed());
    
    // Implementation
}

// Performance dashboard
GET /debug/profiler         # Performance dashboard
GET /debug/queries          # Database query analysis  
GET /debug/memory           # Memory usage breakdown
GET /debug/routes           # Route performance metrics
```

## Implementation Plan

### Week 1: Enhanced Code Generation
- [ ] Advanced resource scaffolding system
- [ ] Template-based code generation engine
- [ ] CRUD controller and validation generation
- [ ] Migration generation from model definitions

### Week 2: API Documentation System
- [ ] OpenAPI specification generation
- [ ] Request/response schema extraction
- [ ] Swagger UI integration
- [ ] Export to various formats (Postman, etc.)

### Week 3: Testing Framework
- [ ] Test database management utilities
- [ ] HTTP testing client and assertions
- [ ] Factory system for test data
- [ ] Integration with standard test runners

### Week 4: CLI & Development Tools  
- [ ] Custom command system
- [ ] Development server with hot reload
- [ ] Performance profiling and monitoring
- [ ] Enhanced debugging utilities

### Week 5: Integration & Polish
- [ ] Integration between all development tools
- [ ] Comprehensive documentation and tutorials
- [ ] Example applications showcasing features
- [ ] Performance optimization and testing

## Testing Strategy

### Unit Tests
- Code generation template processing
- OpenAPI schema generation accuracy
- Test utility functionality
- CLI command argument parsing

### Integration Tests
- End-to-end scaffolding workflows
- Generated code compilation and functionality
- Testing framework with real applications
- Development server hot reload behavior

### User Experience Tests
- Documentation completeness and accuracy
- CLI usability and error messages
- Generated code quality and patterns
- Development workflow efficiency

## Success Criteria

### Productivity Requirements
- [ ] Generate complete CRUD resource in <30 seconds
- [ ] Hot reload rebuilds in <2 seconds for typical changes
- [ ] OpenAPI docs accurately reflect API behavior
- [ ] Testing utilities reduce test code by >60%

### Quality Requirements
- [ ] Generated code follows best practices and is LLM-editable
- [ ] Documentation is comprehensive and up-to-date
- [ ] Error messages are helpful and actionable
- [ ] CLI commands have intuitive interfaces

### Feature Completeness
- [ ] Modern CLI command system feature parity
- [ ] Complete CRUD scaffolding with relationships
- [ ] Comprehensive testing utilities
- [ ] Professional API documentation generation

## Deliverables

1. **Advanced Code Generation**:
   - Resource scaffolding with relationships
   - Authentication system generation
   - API scaffolding with OpenAPI integration

2. **Developer Tools**:
   - CLI command system with custom commands
   - Hot reload development server
   - Performance profiling dashboard

3. **Testing Framework**:
   - Complete testing utilities
   - Factory system for test data
   - HTTP testing client

4. **Documentation System**:
   - OpenAPI generation and Swagger UI
   - Comprehensive guides and tutorials
   - Best practices documentation

## Files Structure
```
crates/elif-cli/src/generators/
├── mod.rs                  # Generator system core
├── resource.rs             # Resource scaffolding
├── auth.rs                 # Authentication scaffolding
├── api.rs                  # API generation
└── templates/              # Code generation templates

crates/elif-openapi/
├── src/
│   ├── lib.rs              # OpenAPI generation
│   ├── schema.rs           # Schema extraction
│   ├── endpoints.rs        # Endpoint documentation
│   └── export.rs           # Export utilities
└── templates/swagger-ui/   # Swagger UI assets

crates/elif-testing/
├── src/
│   ├── lib.rs              # Testing utilities
│   ├── database.rs         # Test database management
│   ├── client.rs           # HTTP test client
│   ├── factories.rs        # Factory system
│   └── assertions.rs       # Custom assertions

crates/elif-dev-server/
├── src/
│   ├── lib.rs              # Development server
│   ├── hot_reload.rs       # File watching and reloading
│   ├── error_pages.rs      # Enhanced error pages
│   └── middleware.rs       # Development middleware

examples/generated-blog/    # Example of generated application
├── generated_by_elif.md    # Generation audit trail
├── src/
│   ├── models/             # Generated models
│   ├── controllers/        # Generated controllers
│   └── policies/           # Generated policies
├── tests/                  # Generated tests
└── openapi/               # Generated API docs
```

This phase transforms elif.rs into a highly productive development framework that provides excellent developer experience while maintaining Rust's performance and safety benefits.