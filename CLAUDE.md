# CLAUDE.md — elif.rs

## What is elif?
A Rust web framework designed for both AI agents and developers. Think Laravel or NestJS but for Rust - simple, intuitive, productive.

## Core Philosophy
- **Developer Experience First**: APIs should be obvious, like `handle(req, next)` for middleware
- **AI-Friendly**: LLMs can understand and generate code easily
- **Pure Framework Types**: Never expose internal dependencies (Axum, Hyper) to users
- **Spec-First**: Generate code from specifications, not the other way around

## Quick Start (New Session)
```bash
# Check what needs to be done
gh issue list --repo krcpa/elif.rs --state open --limit 5

# Validate code state
cargo build && cargo test

# Start working on first open issue
gh issue view <number> --repo krcpa/elif.rs
```

## Working on Tasks
1. **Always use GitHub issues** - Never work without an issue number
2. **Small, focused commits** - One feature/fix per commit
3. **Use TodoWrite tool** - Track progress within Claude
4. **Test before commit** - `cargo test && cargo clippy`
5. **Close with summary** - `gh issue close <number> --comment "..."`

## Testing Strategy
- **Unit tests**: Basic functionality validation
- **Integration tests**: Real usage scenarios
- **UI tests with trybuild**: Compile-time macro validation (for proc macros)
  - Pass tests: Valid macro usage compiles successfully
  - Fail tests: Invalid usage produces meaningful error messages
- **Example**: `elif-http-derive` has comprehensive test coverage with 9 UI test scenarios

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

## Key Rules
- **MARKER blocks**: Only edit inside `// <<<ELIF:BEGIN ...>>>` markers
- **SQL safety**: Always use parameters (`$1, $2`), never string concat
- **Type wrapping**: Wrap all external types (Request → ElifRequest)
- **Error format**: `{ "error": { "code": "...", "message": "...", "hint": "..." } }`
- **Controller macros**: Use `#[controller("/path")]` for declarative routing (requires derive feature)
- **Builder patterns**: Use `#[builder]` macro from service-builder 0.3.0
  - Use `#[builder(optional)]` for `Option<T>` fields that default to `None`
  - Use `#[builder(default)]` for fields using `Default::default()`
  - Use `#[builder(default = "expression")]` for custom default values
  - Use `#[builder(getter)]` for external field access
  - Use `#[builder(setter)]` for runtime field modification
  - Add convenience methods via `impl ConfigBuilder` blocks
  - Use `build_with_defaults()` for configuration patterns
  - Use `build()?` for service construction patterns

## Known Limitations
- **Response body caching**: Not possible yet - bodies can only be read once (see #130, #131)
- **Controller macros**: Current implementation is foundational - runtime route registration and advanced features coming in future releases (Epic #236, tasks 8-12)
- **Middleware complexity**: Current trait system too complex, needs simplification

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
