# Phase 2: Database Layer

**Duration**: Months 4-6 (12 weeks)  
**Team**: 2-3 developers  
**Goal**: Full-featured ORM matching Eloquent's capabilities

## Overview

Phase 2 builds a complete database layer on top of the architecture foundation from Phase 1. This includes a full ORM with relationships, query builder, migrations, connection management, and model events.

## Dependencies

- **Phase 1**: Requires working DI container and module system for service registration
- **External**: SQLx for database connectivity, async runtime support

## Key Components

### 1. Base Model System
**File**: `crates/elif-db/src/model.rs`

Core model trait and derive macro for database entities.

**Requirements**:
- Model trait with standard CRUD operations
- Derive macro for automatic implementation
- Primary key handling (auto-increment, UUID, composite)
- Timestamps (created_at, updated_at) with automatic management
- Soft deletes support
- Model serialization/deserialization

**API Design**:
```rust
#[derive(Model, Debug, Serialize, Deserialize)]
#[model(table = "users")]
pub struct User {
    #[model(primary_key)]
    pub id: u64,
    
    pub email: String,
    pub name: String,
    
    #[model(relationship = "HasMany", target = "Post", foreign_key = "user_id")]
    pub posts: Lazy<Vec<Post>>,
    
    #[model(relationship = "HasOne", target = "Profile", foreign_key = "user_id")]
    pub profile: Lazy<Profile>,
    
    #[model(timestamps)]
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    #[model(soft_delete)]
    pub deleted_at: Option<DateTime<Utc>>,
}
```

### 2. Query Builder
**File**: `crates/elif-db/src/query.rs`

Type-safe, fluent query builder for complex database operations.

**Requirements**:
- Fluent interface for building queries
- Type safety with compile-time validation
- Support for joins, subqueries, aggregations
- Pagination support (offset, cursor)
- Raw SQL escape hatches
- Query optimization and caching

**API Design**:
```rust
// Basic queries
let users = User::query()
    .where_eq("active", true)
    .where_gt("created_at", yesterday)
    .order_by("name")
    .limit(10)
    .get()
    .await?;

// Complex queries with joins
let posts_with_users = Post::query()
    .with("user")
    .where_in("status", vec!["published", "featured"])
    .order_by_desc("created_at")
    .paginate(15)
    .get()
    .await?;

// Aggregations
let stats = User::query()
    .select("COUNT(*) as total, AVG(age) as avg_age")
    .where_not_null("email_verified_at")
    .group_by("country")
    .get()
    .await?;
```

### 3. Relationships
**File**: `crates/elif-db/src/relationships.rs`

Full relationship system supporting all major relationship types.

**Requirements**:
- HasOne, HasMany relationships
- BelongsTo, BelongsToMany relationships
- Polymorphic relationships
- Eager loading and lazy loading
- Relationship constraints and cascading
- Through relationships (HasManyThrough)

**API Design**:
```rust
// Relationship definitions
impl User {
    pub fn posts(&self) -> HasMany<Post> {
        HasMany::new(self, "user_id")
    }
    
    pub fn profile(&self) -> HasOne<Profile> {
        HasOne::new(self, "user_id")
    }
    
    pub fn roles(&self) -> BelongsToMany<Role> {
        BelongsToMany::new(self, "user_roles", "user_id", "role_id")
    }
}

// Usage
let user = User::with("posts.comments").find(1).await?;
for post in user.posts.await? {
    println!("Post: {}", post.title);
    for comment in post.comments.await? {
        println!("  Comment: {}", comment.content);
    }
}
```

### 4. Migration System
**File**: `crates/elif-db/src/migrations.rs`

Schema management with version control and rollbacks.

**Requirements**:
- Up/down migration support
- Schema builder with type-safe operations
- Migration versioning and tracking
- Batch migration operations
- Migration rollback capabilities
- Fresh database setup

**API Design**:
```rust
pub struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn up(&self) -> Schema {
        Schema::create_table("users", |table| {
            table.id();
            table.string("email").unique();
            table.string("name");
            table.timestamp("email_verified_at").nullable();
            table.timestamps();
            table.soft_deletes();
        })
    }
    
    fn down(&self) -> Schema {
        Schema::drop_table("users")
    }
}
```

### 5. Connection Management
**File**: `crates/elif-db/src/connection.rs`

Database connection pooling and management.

**Requirements**:
- Connection pooling with configurable limits
- Read/write splitting support
- Transaction management
- Connection health monitoring
- Multiple database support
- Connection middleware for logging/metrics

**API Design**:
```rust
#[derive(Config)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_min: u32,
    pub pool_max: u32,
    pub timeout: Duration,
    pub read_urls: Vec<String>, // For read replicas
}

pub struct ConnectionManager {
    write_pool: Pool<Postgres>,
    read_pools: Vec<Pool<Postgres>>,
    config: DatabaseConfig,
}
```

### 6. Model Events & Observers
**File**: `crates/elif-db/src/events.rs`

Event system for model lifecycle hooks.

**Requirements**:
- Model lifecycle events (creating, created, updating, updated, deleting, deleted)
- Observer pattern for event handling
- Async event handlers
- Event propagation control
- Global and model-specific observers

**API Design**:
```rust
pub trait ModelObserver<M: Model> {
    async fn creating(&self, model: &mut M) -> Result<(), EventError> { Ok(()) }
    async fn created(&self, model: &M) -> Result<(), EventError> { Ok(()) }
    async fn updating(&self, model: &mut M) -> Result<(), EventError> { Ok(()) }
    async fn updated(&self, model: &M) -> Result<(), EventError> { Ok(()) }
    async fn deleting(&self, model: &M) -> Result<(), EventError> { Ok(()) }
    async fn deleted(&self, model: &M) -> Result<(), EventError> { Ok(()) }
}

// Usage
pub struct UserObserver;

impl ModelObserver<User> for UserObserver {
    async fn creating(&self, user: &mut User) -> Result<(), EventError> {
        user.email = user.email.to_lowercase();
        Ok(())
    }
    
    async fn created(&self, user: &User) -> Result<(), EventError> {
        // Send welcome email
        EmailService::send_welcome(user).await?;
        Ok(())
    }
}
```

## Implementation Plan

### Week 1-2: Base Model System
- [ ] Define Model trait and core functionality
- [ ] Implement Model derive macro
- [ ] Add primary key and timestamp support
- [ ] Basic CRUD operations (find, create, update, delete)
- [ ] Soft delete functionality

### Week 3-4: Query Builder Foundation
- [ ] Build query builder structure
- [ ] Implement where clauses and operators
- [ ] Add ordering and limiting
- [ ] Basic joins and select operations
- [ ] Integration with Model trait

### Week 5-6: Relationships System
- [ ] Implement HasOne and HasMany relationships
- [ ] Add BelongsTo and BelongsToMany relationships
- [ ] Eager loading and lazy loading mechanisms
- [ ] Relationship constraints and validation

### Week 7-8: Migration System
- [ ] Migration trait and runner
- [ ] Schema builder with table operations
- [ ] Migration version tracking
- [ ] CLI integration for migration commands
- [ ] Rollback functionality

### Week 9-10: Connection Management & Transactions
- [ ] Connection pooling implementation
- [ ] Transaction support with auto-rollback
- [ ] Read/write splitting
- [ ] Connection health monitoring
- [ ] Database configuration management

### Week 11-12: Model Events & Advanced Features
- [ ] Model event system and observers
- [ ] Advanced query builder features (subqueries, unions)
- [ ] Performance optimization and caching
- [ ] Comprehensive testing and benchmarks

## Database Support

### Primary Database: PostgreSQL
- Full feature support including JSON columns, arrays
- Advanced indexing and constraints
- Connection pooling optimized for PostgreSQL

### Secondary Databases (Future):
- SQLite for development and testing
- MySQL for broader compatibility
- Database-specific optimizations

## Performance Requirements

### Query Performance:
- Simple queries: <10ms
- Complex queries with joins: <50ms  
- Bulk operations: >1000 records/second

### Memory Usage:
- Connection pool: <50MB for 100 connections
- Query builder: <1KB overhead per query
- Model instances: <500 bytes overhead per model

### Connection Management:
- Pool warmup: <100ms
- Connection acquisition: <1ms
- Transaction overhead: <0.1ms

## Testing Strategy

### Unit Tests:
- Model CRUD operations
- Query builder functionality
- Relationship loading and constraints
- Migration up/down operations
- Event system triggering

### Integration Tests:
- Full database integration with real PostgreSQL
- Transaction rollback testing
- Connection pool stress testing
- Migration system end-to-end

### Performance Tests:
- Query performance benchmarks
- Connection pool performance
- Memory usage under load
- Concurrent operation testing

## Success Criteria

### Functional Requirements:
- [ ] Can define models with relationships using derive macros
- [ ] Query builder provides fluent, type-safe API
- [ ] All relationship types work with eager/lazy loading
- [ ] Migrations can create and modify complex schemas
- [ ] Connection pooling handles concurrent requests efficiently
- [ ] Model events trigger correctly and support async handlers

### Performance Requirements:
- [ ] Simple queries execute in <10ms
- [ ] Connection pool supports 1000+ concurrent connections
- [ ] Model instantiation: <100μs per model
- [ ] Migration execution: <1s for typical schema changes

### Quality Requirements:
- [ ] >90% test coverage for all ORM functionality
- [ ] No memory leaks in long-running applications
- [ ] Comprehensive error messages for common issues
- [ ] Full API documentation with examples

## Deliverables

1. **Core Crates**:
   - `elif-db` - ORM, query builder, migrations, connections
   - `elif-db-derive` - Derive macros for models
   - `elif-migrations` - Migration CLI and utilities

2. **Documentation**:
   - Model definition guide
   - Query builder reference
   - Relationship documentation
   - Migration system guide

3. **Examples**:
   - Blog application with User/Post/Comment models
   - E-commerce models with complex relationships
   - Migration examples for common scenarios

4. **Tools**:
   - Migration CLI commands
   - Schema inspection utilities
   - Query debugging tools

## File Structure
```
crates/elif-db/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── model.rs              # Model trait and base functionality
│   ├── query.rs              # Query builder implementation
│   ├── relationships.rs       # Relationship system
│   ├── migrations.rs         # Migration system
│   ├── connection.rs         # Connection management
│   ├── events.rs             # Model events and observers
│   ├── schema.rs             # Schema builder
│   └── error.rs              # Database error types
├── tests/
│   ├── model_tests.rs
│   ├── query_tests.rs
│   ├── relationship_tests.rs
│   ├── migration_tests.rs
│   └── integration_tests.rs
└── Cargo.toml

crates/elif-db-derive/
├── src/
│   ├── lib.rs                # Derive macro implementations
│   ├── model.rs              # Model derive macro
│   └── relationship.rs        # Relationship derive helpers
└── Cargo.toml
```

This phase creates a database layer that rivals Laravel's Eloquent ORM while maintaining type safety and performance characteristics expected in Rust applications.