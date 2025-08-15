# Phase 4: Database Operations 💾

**Duration**: 2-3 weeks  
**Goal**: Complete database transaction story  
**Status**: Ready after Phase 3

## Overview

Phase 4 completes our database layer with enterprise-grade connection pooling, transaction management, and advanced query builder features. This builds on our existing ORM foundation to provide modern web framework database capabilities.

## Dependencies

- **Phase 1**: ✅ Complete (DI container, config)
- **Phase 2.1**: ✅ Complete (ORM foundation)
- **Phase 3**: ✅ Complete (Security & validation)

## Key Components

### 1. Connection Pooling & Management
**File**: `crates/elif-orm/src/connection.rs`

Enterprise-grade database connection pooling with monitoring.

**Requirements**:
- Configurable connection pools (min/max connections)
- Connection health monitoring and recovery
- Read/write splitting for scaled databases
- Connection idle timeout and cleanup
- Pool metrics and monitoring
- Multiple database support

**API Design**:
```rust
#[derive(Config, Debug)]
pub struct DatabaseConfig {
    #[config(env = "DATABASE_URL")]
    pub url: String,
    
    #[config(env = "DATABASE_POOL_MIN", default = 1)]
    pub pool_min: u32,
    
    #[config(env = "DATABASE_POOL_MAX", default = 10)]
    pub pool_max: u32,
    
    #[config(env = "DATABASE_TIMEOUT", default = 30)]
    pub timeout_seconds: u64,
    
    #[config(env = "DATABASE_READ_URLS", default = "")]
    pub read_urls: Vec<String>, // Read replicas
}

pub struct ConnectionManager {
    write_pool: Pool<Postgres>,
    read_pools: Vec<Pool<Postgres>>,
    config: DatabaseConfig,
    metrics: PoolMetrics,
}

impl ConnectionManager {
    pub async fn write_connection(&self) -> Result<PoolConnection<Postgres>, DatabaseError>;
    pub async fn read_connection(&self) -> Result<PoolConnection<Postgres>, DatabaseError>;
    pub fn metrics(&self) -> PoolMetrics;
}
```

### 2. Transaction Management System
**File**: `crates/elif-orm/src/transaction.rs`

Robust transaction handling with automatic rollback and nested transactions.

**Requirements**:
- Transaction begin/commit/rollback
- Automatic rollback on panic or error
- Nested transaction support (savepoints)
- Transaction timeout handling
- Distributed transaction support (future)
- Transaction-scoped model operations

**API Design**:
```rust
pub struct Transaction<'a> {
    conn: &'a mut PgConnection,
    committed: bool,
}

impl<'a> Transaction<'a> {
    pub async fn commit(mut self) -> Result<(), DatabaseError>;
    pub async fn rollback(mut self) -> Result<(), DatabaseError>;
    pub async fn savepoint(&mut self, name: &str) -> Result<Savepoint<'_>, DatabaseError>;
}

// Usage patterns
// Automatic rollback on drop
async fn transfer_money(pool: &Pool<Postgres>, from: u64, to: u64, amount: Decimal) -> Result<(), DatabaseError> {
    let mut tx = pool.begin().await?;
    
    Account::decrement_balance(&mut tx, from, amount).await?;
    Account::increment_balance(&mut tx, to, amount).await?;
    
    tx.commit().await?; // Explicit commit
    Ok(())
}

// With middleware
DatabaseTransactionMiddleware::new()
    .auto_begin_on_write_requests()
    .timeout(Duration::from_secs(30));
```

### 3. Advanced Query Builder Features
**File**: `crates/elif-orm/src/query/advanced.rs`

Enhanced query builder with complex SQL features.

**Requirements**:
- UNION and UNION ALL queries
- Complex subqueries (EXISTS, NOT EXISTS, IN subqueries)
- Window functions (ROW_NUMBER, RANK, etc.)
- Common Table Expressions (CTE)
- Lateral joins and advanced join types
- Query optimization hints

**API Design**:
```rust
// UNION queries
let active_users = User::query().where_eq("status", "active");
let premium_users = User::query().where_eq("subscription", "premium");

let combined = active_users.union(premium_users)
    .order_by("created_at")
    .limit(100);

// Complex subqueries
let users_with_posts = User::query()
    .where_exists(|sub| {
        sub.select("1")
           .from("posts")
           .where_raw("posts.user_id = users.id")
    })
    .where_not_in("id", |sub| {
        sub.select("user_id")
           .from("banned_users")
    });

// Window functions
let ranked_posts = Post::query()
    .select("*, ROW_NUMBER() OVER (PARTITION BY category_id ORDER BY views DESC) as rank")
    .where_raw("rank <= 5");

// CTE (Common Table Expressions)
let recursive_categories = Category::query()
    .with_recursive("category_tree", |cte| {
        cte.select("id, parent_id, name, 0 as level")
           .from("categories") 
           .where_null("parent_id")
           .union_all(|union| {
               union.select("c.id, c.parent_id, c.name, ct.level + 1")
                    .from("categories c")
                    .join("category_tree ct", "c.parent_id", "ct.id")
           })
    });
```

### 4. Database Migration System
**File**: `crates/elif-orm/src/migrations/mod.rs`

Complete migration system with schema building and version control.

**Requirements**:
- Migration creation and running
- Schema builder with type-safe operations
- Migration rollback capabilities
- Migration status tracking
- Batch migration operations
- Migration dependency management

**API Design**:
```rust
pub trait Migration: Send + Sync {
    fn up(&self) -> Schema;
    fn down(&self) -> Schema;
    fn dependencies(&self) -> Vec<&'static str> { vec![] }
}

// Schema builder
pub struct Schema;

impl Schema {
    pub fn create_table<F>(name: &str, builder: F) -> Self 
    where F: FnOnce(&mut TableBuilder);
    
    pub fn alter_table<F>(name: &str, builder: F) -> Self
    where F: FnOnce(&mut TableBuilder);
    
    pub fn drop_table(name: &str) -> Self;
    pub fn create_index(table: &str, columns: &[&str]) -> Self;
}

// Example migration
pub struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn up(&self) -> Schema {
        Schema::create_table("users", |table| {
            table.id(); // Auto-increment primary key
            table.uuid("uuid").unique(); // UUID for external references
            table.string("email").unique().index();
            table.string("name");
            table.text("bio").nullable();
            table.timestamp("email_verified_at").nullable();
            table.timestamps(); // created_at, updated_at
            table.soft_deletes(); // deleted_at
        })
    }
    
    fn down(&self) -> Schema {
        Schema::drop_table("users")
    }
}
```

### 5. Query Performance Optimization
**File**: `crates/elif-orm/src/performance.rs`

Query optimization, caching, and performance monitoring.

**Requirements**:
- Query result caching (Redis/in-memory)
- Query execution plan analysis
- Slow query logging and alerting
- Connection pool monitoring
- Query batching for N+1 prevention
- Database index recommendations

**API Design**:
```rust
// Query caching
User::query()
    .where_eq("status", "active")
    .cache_for(Duration::from_mins(30))
    .cache_key("active_users")
    .get()
    .await?;

// Query batching (N+1 prevention)
let posts = Post::query().limit(10).get().await?;
let users = User::query()
    .where_in("id", posts.iter().map(|p| p.user_id))
    .batch_load() // Indicates this is a batch load
    .get()
    .await?;

// Performance monitoring
QueryPerformanceMiddleware::new()
    .slow_query_threshold(Duration::from_millis(100))
    .log_slow_queries(true)
    .alert_on_slow_queries(Duration::from_secs(1));
```

### 6. Database Seeding Foundation
**File**: `crates/elif-orm/src/seeding.rs`

Basic seeding system (full factory system in Phase 9).

**Requirements**:
- Seed runner and management
- Environment-specific seeds
- Seed dependencies and ordering
- Data truncation and cleanup
- Production seed safety

**API Design**:
```rust
pub trait Seeder: Send + Sync {
    async fn run(&self, pool: &Pool<Postgres>) -> Result<(), SeedError>;
    fn dependencies(&self) -> Vec<&'static str> { vec![] }
    fn environments(&self) -> Vec<Environment> { vec![Environment::Development] }
}

// Example seeder
pub struct UserSeeder;

impl Seeder for UserSeeder {
    async fn run(&self, pool: &Pool<Postgres>) -> Result<(), SeedError> {
        let admin = User {
            email: "admin@example.com".to_string(),
            name: "Administrator".to_string(),
            ..Default::default()
        };
        
        User::create(pool, admin).await?;
        Ok(())
    }
}
```

## Current Status (2025-08-15)

**COMPLETED TASKS:**
- ✅ **Phase 4.1**: Database Service Integration (Issue #60)
- ✅ **Phase 4.2**: Basic Connection Pool Management (Issue #61)

**ACTIVE TASKS:**
- **Phase 4.7**: Database Architecture Refactor - Move Database Services to elif-orm (Issue #66)

## Task Breakdown

### Phase 4.1: Database Service Integration ✅ COMPLETED
- ✅ Complete DatabaseServiceProvider integration with DI container
- ✅ Basic pool setup and service registration  
- ✅ Service provider lifecycle management

### Phase 4.2: Basic Connection Pool Management ✅ COMPLETED  
- ✅ Enhanced PoolConfig with comprehensive configuration options
- ✅ ManagedPool wrapper with statistics tracking and health monitoring
- ✅ Advanced error handling with dedicated PoolError types
- ✅ Pool registry system for multi-database support
- ✅ Connection acquisition/release with performance monitoring

### Phase 4.3: Simple Transaction Support (Issue #62)
- [ ] Basic transaction begin/commit/rollback functionality
- [ ] Auto-rollback on panic or error
- [ ] Transaction-scoped model operations
- [ ] Error handling and recovery

### Phase 4.4: Basic Migration System (Issue #63)  
- [ ] Migration creation and execution
- [ ] Simple schema operations (create/drop tables)
- [ ] Migration status tracking
- [ ] CLI integration for migration commands

### Phase 4.5: Model-Database Integration (Issue #64)
- [ ] Enhanced model operations with pool integration
- [ ] Connection management in model methods
- [ ] Transaction support in model operations
- [ ] Performance optimizations

### Phase 4.6: Basic CRUD Operations (Issue #65)
- [ ] Complete CRUD implementation with new pool system
- [ ] Error handling integration
- [ ] Performance monitoring and logging
- [ ] Integration testing

### Phase 4.7: Database Architecture Refactor (Issue #66) 🆕  
**Priority**: Medium - **Duration**: 2 days
- [ ] Move database functionality from elif-http to elif-orm
- [ ] Update dependencies and import paths
- [ ] Fix service provider integration
- [ ] Update tests and documentation
- [ ] Ensure clean architectural separation

## Implementation Plan

### Week 1: Architecture & Transaction Foundation
- [ ] **Phase 4.7**: Complete database architecture refactor (2 days)
- [ ] **Phase 4.3**: Simple transaction support implementation (3 days)
- [ ] Integration testing and validation

### Week 2: Migration System & Model Integration  
- [ ] **Phase 4.4**: Basic migration system (4 days)
- [ ] **Phase 4.5**: Model-database integration (3 days)

### Week 2: Advanced Query Features
- [ ] UNION and complex subquery support
- [ ] Window functions and CTEs
- [ ] Query optimization and caching
- [ ] Performance monitoring integration

### Week 3: Migrations & Polish
- [ ] Complete migration system with schema builder
- [ ] Migration rollback and dependency management
- [ ] Basic seeding system
- [ ] Integration testing and documentation

## Testing Strategy

### Unit Tests
- Connection pool behavior under load
- Transaction rollback scenarios
- Advanced query SQL generation
- Migration up/down operations

### Integration Tests
- Database operations under concurrent load
- Transaction isolation testing
- Migration system end-to-end
- Performance benchmarking

### Performance Tests
- Connection pool performance under load
- Query caching effectiveness
- Transaction overhead measurement
- Migration execution time

## Success Criteria

### Functional Requirements
- [ ] Connection pooling handles concurrent requests efficiently
- [ ] Transactions provide ACID guarantees with auto-rollback
- [ ] Advanced queries generate correct SQL
- [ ] Migrations can modify schemas safely

### Performance Requirements
- [ ] Connection acquisition <1ms average
- [ ] Transaction overhead <0.1ms
- [ ] Query caching reduces database load by >50%
- [ ] Pool handles 1000+ concurrent connections

### Reliability Requirements
- [ ] Connection recovery after database downtime
- [ ] Transaction rollback on panic/error
- [ ] Migration rollback works correctly
- [ ] No connection leaks under load

## Deliverables

1. **Database Infrastructure**:
   - Connection pooling with monitoring
   - Transaction management system
   - Performance optimization layer

2. **Advanced Query System**:
   - Extended query builder features
   - Query caching and optimization
   - Performance monitoring

3. **Migration Framework**:
   - Complete migration system
   - Schema builder with type safety
   - Migration management CLI

4. **Documentation & Examples**:
   - Database configuration guide
   - Transaction patterns and best practices
   - Performance optimization guide

## Files Structure
```
crates/elif-orm/src/
├── connection.rs           # Connection pooling and management
├── transaction.rs          # Transaction handling
├── query/
│   ├── mod.rs             # Existing query builder
│   ├── advanced.rs        # Advanced query features
│   ├── caching.rs         # Query result caching
│   └── optimization.rs    # Query optimization
├── migrations/
│   ├── mod.rs             # Migration framework
│   ├── schema.rs          # Schema builder
│   └── runner.rs          # Migration runner
├── seeding.rs             # Basic seeding system
└── performance.rs         # Performance monitoring

crates/elif-cli/src/commands/
├── migrate.rs             # Migration commands
└── seed.rs               # Seeding commands
```

This phase completes the database layer foundation, providing enterprise-grade capabilities that rival modern web frameworks while maintaining Rust's performance and safety guarantees.