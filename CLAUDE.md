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
├── elif-core/      # DI container, modules
├── elif-http/      # HTTP server, routing, middleware
├── elif-orm/       # Database, migrations
├── elif-auth/      # Authentication
├── elif-cache/     # Caching layer
├── elif-security/  # Security middleware
├── elif-cli/       # CLI tools
└── elif-codegen/   # Code generation
```

## Key Rules
- **MARKER blocks**: Only edit inside `// <<<ELIF:BEGIN ...>>>` markers
- **SQL safety**: Always use parameters (`$1, $2`), never string concat
- **Type wrapping**: Wrap all external types (Request → ElifRequest)
- **Error format**: `{ "error": { "code": "...", "message": "...", "hint": "..." } }`
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
- **Middleware complexity**: Current trait system too complex, needs simplification

## Security Rules
- **Never read**: `.env*`, `./secrets/**`
- **Never run**: `curl | bash` or untrusted commands
- **Always validate**: User input and external data

---
**GitHub**: https://github.com/krcpa/elif.rs
**Docs**: Run `elifrs --help` for any command
