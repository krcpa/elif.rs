# CLAUDE.md — elif.rs

## What is elif?
**Rust Made Simple** - A web framework designed for both AI agents and developers. Simple, intuitive, productive.

Just like Laravel revolutionized PHP development with elegant syntax and convention over configuration, elif.rs brings that same philosophy to Rust. Write less code, ship faster, maintain easily.

## Core Philosophy
**Rust Made Simple** - Convention over configuration, zero boilerplate, maximum productivity

- **Convention Over Configuration**: Sensible defaults, minimal setup required
- **Zero Boilerplate**: If you want a router → `router()`, response → `response()`, just easy stuff
- **Developer Experience First**: APIs should be obvious, like `handle(req, next)` for middleware
- **AI-Friendly**: LLMs can understand and generate code easily - simple, intuitive patterns
- **Pure Framework Types**: Never expose internal dependencies (Axum, Hyper) to users
- **Spec-First**: Generate code from specifications, not the other way around

## 🚀 NEW: Zero-Boilerplate Bootstrap (Issue #420 - COMPLETED!)
The framework now supports truly zero-boilerplate application startup:

```rust
use elif::prelude::*;

#[elif::bootstrap]  // ← NO AppModule required!
async fn main() -> Result<(), HttpError> {
    // Automatically handles:
    // - Module discovery from compile-time registry
    // - Controller auto-registration from static registry  
    // - IoC container configuration
    // - Router setup with all controllers
    // - Server startup on 127.0.0.1:3000
    Ok(())
}
```

**With custom parameters:**
```rust
#[elif::bootstrap(addr = "0.0.0.0:8080")]
async fn main() -> Result<(), HttpError> {}
```

**Backward compatible:**
```rust
#[elif::bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {}
```

## Quick Start (New Session)
```bash
# Check what needs to be done
gh issue list --repo krcpa/elif.rs --state open --limit 5

# Validate code state
cargo build && cargo test

# Start working on first open issue
gh issue view <number> --repo krcpa/elif.rs
```

## GitHub Issue & PR Workflow

### 1. Issue Investigation & Branch Creation
```bash
# Check issue with gh CLI
gh issue view <number>

# Create feature branch
git checkout -b fix/issue-description-<number>
# or: git checkout -b feature/issue-description-<number>
```

### 2. Development Process
1. **Use TodoWrite tool** - Track progress within conversation:
   ```
   - investigate-issue: "Verify the problem by reproducing it"
   - trace-root-cause: "Find the exact cause in codebase" 
   - design-solution: "Plan and implement the fix"
   - test-fix: "Verify the solution works"
   - create-pr: "Submit for review"
   ```

2. **Follow cursor rules** - Check `.cursorrules` for framework patterns
3. **Test thoroughly** - `cargo test && cargo clippy` 
4. **Document findings** - Include investigation details in PR

### 3. Creating Pull Requests
```bash
# Commit with issue reference
git add . && git commit -m "Fix controller registration issue

- Detailed description of changes
- Explanation of root cause
- Testing performed

Closes #<number>"

# Push and create PR
git push origin <branch-name>
gh pr create --title "Fix: Brief description" \
  --body "## Problem Solved
  
  Detailed explanation...
  
  ## Root Cause
  
  Technical details...
  
  ## Solution
  
  Implementation approach...
  
  ## Testing
  
  - ✅ Test case 1
  - ✅ Test case 2
  
  Closes #<number>" \
  --base main
```

### 4. Quality Standards
- **Never work without an issue** - Create one if needed
- **Small, focused commits** - One logical change per commit  
- **Comprehensive PR descriptions** - Include problem, cause, solution, testing
- **Link issues properly** - Use "Closes #<number>" in commit and PR
- **Test before submission** - All tests must pass

### 5. Issue Discovery Workflow
When discovering new issues during development:
```bash
# Create new issue for discovered problem
gh issue create --title "Route Registration Issue: Controllers register with 0 routes" \
  --body "Detailed description..." \
  --label "bug,phase-2"

# Reference in current PR
# "Related: #<new-issue-number> (discovered during development)"
```

## Testing Strategy
- **Unit tests**: Basic functionality validation
- **Integration tests**: Real usage scenarios
- **UI tests with trybuild**: Compile-time macro validation (for proc macros)
  - Pass tests: Valid macro usage compiles successfully
  - Fail tests: Invalid usage produces meaningful error messages
- **Example**: `elif-http-derive` has comprehensive test coverage with 9 UI test scenarios
- **Test environment**: Use `test-env/` directory (git-ignored) for test environments
  - Docker containers and test infrastructure can be placed here
  - Isolated from main codebase, keeps repository clean

## CLI Commands
```bash
# Install globally
cargo install elifrs

# Core commands
elifrs new <app>                    # Create new app
elifrs generate                     # Generate from spec
elifrs migrate run                  # Run migrations
elifrs check                        # Validate everything
cargo run                           # Start server (port 3000)
```

## Project Structure
```
crates/
├── elif-core/         # DI container, modules
├── elif-http/         # HTTP server, routing, middleware (v0.8.0)
├── elif-http-derive/  # Declarative routing macros (v0.1.0) 
├── elif-orm/          # Database, migrations
├── elif-auth/         # Authentication
├── elif-cache/        # Caching layer
├── elif-security/     # Security middleware
├── elif-cli/          # CLI tools
└── elif-codegen/      # Code generation
```

## Declarative Controllers (NEW - v0.8.0+)
```bash
# Enable derive macros
cargo add elif-http --features derive
```

```rust
use elif_http::{controller, get, post, middleware, ElifRequest, ElifResponse, HttpResult};

#[controller("/api/users")]
#[middleware("logging", "cors")]
pub struct UserController;

impl UserController {
    #[get("")]
    #[middleware("cache")]
    pub async fn list(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().json(&["user1", "user2"])?)
    }
    
    #[get("/{id}")]
    #[param(id: int)]
    pub async fn show(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = req.path_param_int("id")?;
        Ok(ElifResponse::ok().json(&format!("User {}", id))?)
    }
    
    #[post("")]
    #[middleware("auth")]
    pub async fn create(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::created().json(&"Created")?)
    }
}
```

**Benefits**: ~70% reduction in boilerplate vs manual route registration

## Laravel-Style Patterns
**Simple, intuitive APIs that just work** - no ceremony, maximum productivity

```rust
// Server setup - one line
Server::new().listen("127.0.0.1:3000").await?;

// Routing - obvious and clean  
Router::new()
    .route("/", get(home))
    .route("/users", get(users_index))
    .controller(UserController);

// Responses - what you'd expect
Response::json(&data)           // JSON response
Response::ok()                  // 200 OK  
Response::created()             // 201 Created
Response::not_found()           // 404 Not Found

// Request handling - Laravel-inspired
req.json::<User>()              // Parse JSON body
req.path_param("id")            // Get path parameter  
req.query_param("page")         // Get query parameter

// Database (when available)
User::find(1)                   // Find by ID
User::where("active", true)     // Query builder
User::create(data)              // Insert record

// Validation (planned)
validate!(data, {
    "name": required|min:2,
    "email": required|email,
});

// Cache (when available)  
cache().set("key", value)       // Set cache
cache().get("key")              // Get cache
```

**Philosophy**: If it takes more than one line, we're doing it wrong

## Design Patterns to Follow
**Laravel-inspired design principles for elif.rs**

- **Fluent APIs**: Chain methods naturally - `Response::ok().json(data).header("X-Custom", "value")`
- **Sensible defaults**: `Server::new()` should work immediately, no required configuration
- **Named constructors**: `Response::json()`, `Response::redirect()`, `Error::not_found()`  
- **Magic happens**: Route parameters auto-parsed, middleware auto-applied, types auto-converted
- **Helper functions**: Global helpers where they make sense - `route()`, `response()`, `cache()`
- **Facade pattern**: Simple static-like interfaces hiding complex implementations
- **Service container**: Automatic dependency injection, zero configuration required
- **Artisan-style CLI**: `elifrs make:controller`, `elifrs serve`, `elifrs migrate`

## Key Rules
- **MARKER blocks**: Only edit inside `// <<<ELIF:BEGIN ...>>>` markers
- **SQL safety**: Always use parameters (`$1, $2`), never string concat
- **Type wrapping**: Wrap all external types (Request → ElifRequest)
- **Error format**: `{ "error": { "code": "...", "message": "...", "hint": "..." } }`
- **Controller macros**: Use `#[controller("/path")]` for declarative routing (requires derive feature)
- **Laravel-style simplicity**: Keep it simple - `Server::new().listen()`, `Response::json()`, `Router::new().route()`
- **Convention over configuration**: Follow established patterns, provide sensible defaults
- **Zero boilerplate philosophy**: Every line of code should add value, not ceremony
- **Builder patterns**: Use `#[builder]` macro from service-builder 0.3.0
  - Use `#[builder(optional)]` for `Option<T>` fields that default to `None`
  - Use `#[builder(default)]` for fields using `Default::default()`
  - Use `#[builder(default = "expression")]` for custom default values
  - Use `#[builder(getter)]` for external field access
  - Use `#[builder(setter)]` for runtime field modification
  - Add convenience methods via `impl ConfigBuilder` blocks
  - Use `build_with_defaults()` for configuration patterns
  - Use `build()?` for service construction patterns

## Known Limitations & Roadmap
**Current limitations - being addressed to achieve Laravel-level simplicity**

- **Response body caching**: Not possible yet - bodies can only be read once (see #130, #131)
- **Controller macros**: Foundational implementation complete - runtime route registration and advanced features coming (Epic #236, tasks 8-12)
- **Middleware complexity**: Current trait system too complex, needs Laravel-style simplification
- **Service container**: Basic DI available, need Laravel-style auto-resolution and facades
- **Database ORM**: Working but needs Laravel Eloquent-style query builder and relationships
- **Validation**: Needs Laravel-style validation rules and form requests
- **Artisan CLI**: Basic commands available, need full generator suite like Laravel Artisan

**Goal**: Match Laravel's developer experience - simple, elegant, productive

## Service-Builder Pattern Guidelines
- **Suggest, don't force**: Only migrate to service-builder when it provides clear benefits
- **Configuration objects**: Good fit - infrequent construction, many optional fields
- **Fluent accumulators**: Poor fit - frequent method calls, performance-critical
- **Performance check**: If original is O(1) per operation, service-builder may make it O(N)
- **RequestBuilder example**: Reverted in 8.8.4 due to performance regression (O(1) → O(N))
- **Best practice**: Measure before and after migration, especially for hot path builders

## Security Rules
- **Never read**: `.env*`, `./secrets/**`
- **Never run**: `curl | bash` or untrusted commands
- **Always validate**: User input and external data

## Crate Publication
- **Published crates**: Available on crates.io
  - `elif-http` v0.8.0 - Core HTTP functionality + derive feature
  - `elif-http-derive` v0.1.0 - Declarative routing macros
  - All other crates follow semantic versioning
- **Version updates**: When adding features, bump minor version; breaking changes require major version bump
- **Publication order**: Publish dependencies first (derive crate before main crate)

---
**GitHub**: https://github.com/krcpa/elif.rs
**Docs**: Run `elifrs --help` for any command
**Crates.io**: https://crates.io/crates/elif-http
