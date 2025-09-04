# elif.rs Documentation

This directory contains comprehensive documentation for the elif.rs web framework.

## Directory Structure

### `/getting-started/`
Getting started guides and tutorials:
- `introduction.md` - What makes elif.rs special
- `installation.md` - Setup and installation guide
- `zero-boilerplate-quickstart.md` - 5-minute API with zero boilerplate 🚀
- `bootstrap-macro.md` - Complete guide to #[elif::bootstrap] macro
- `quickstart-no-rust.md` - Build APIs using only CLI commands
- `configuration.md` - Application configuration
- `project-structure.md` - Understanding generated projects

### `/basics/`
Core framework concepts:
- `controllers.md` - Declarative HTTP controllers
- `routing.md` - Request routing and path parameters
- `requests.md` - Request handling and parsing
- `responses.md` - Response building and formatting
- `middleware.md` - Cross-cutting concerns and middleware
- `dependency-injection.md` - IoC container and service management
- `error-handling.md` - Error handling patterns
- `validation.md` - Request validation

### `/database/`
Database and ORM documentation:
- `introduction.md` - Database integration overview
- `configuration.md` - Database setup and connection
- `models.md` - ORM models and field definitions
- `query-builder.md` - Building database queries
- `relationships.md` - Model relationships
- `migrations.md` - Database schema management
- `transactions.md` - Transaction handling
- `seeding.md` - Database seeding

### `/middleware/`
Middleware system documentation:
- `integration_example.md` - Complete middleware v2 router integration example and status
- `redesign_plan.md` - Original middleware redesign plan and architecture

### `/development/`
Development and project management documentation:
- `phase2_refactor_progress.md` - Progress tracking for Phase 2 refactor
- `prd.md` - Product Requirements Document

### Root Level
- `API_VERSIONING.md` - API versioning implementation guide

## Framework Documentation

For framework usage documentation, see:
- `CLAUDE.md` in the root directory - Primary development guide
- Individual crate READMEs in `crates/*/README.md`
- Examples in `examples/` directory

## Contributing

When adding new documentation:
1. Place technical documentation in the appropriate subdirectory
2. Keep user-facing guides in the root or examples
3. Update this README when adding new categories
4. Use clear, descriptive filenames with underscores instead of hyphens