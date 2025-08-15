# elif-orm

[![Crates.io](https://img.shields.io/crates/v/elif-orm.svg)](https://crates.io/crates/elif-orm)
[![Documentation](https://docs.rs/elif-orm/badge.svg)](https://docs.rs/elif-orm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Production-ready **Object-Relational Mapping (ORM)** for Rust, designed as the database layer for the [elif.rs](https://github.com/krcpa/elif.rs) web framework. Provides type-safe, async database operations with migrations, connection pooling, transactions, and an intuitive query builder.

## 🚀 Features

### **Core ORM Capabilities**
- **Model System**: Type-safe model definitions with automatic CRUD operations
- **Query Builder**: Fluent, type-safe query construction with compile-time validation
- **Relationships**: Foreign keys, joins, and relationship loading (planned)
- **Primary Key Support**: UUID, integer, and composite primary keys
- **Timestamps**: Automatic `created_at`/`updated_at` management
- **Soft Deletes**: Logical deletion with `deleted_at` timestamps

### **Database Operations**  
- **Connection Pooling**: Production-ready PostgreSQL connection management
- **Transactions**: ACID transactions with configurable isolation levels
- **Migrations**: Schema versioning and database evolution
- **Raw SQL**: Escape hatch for complex queries when needed

### **Production Features**
- **Async/Await**: Built on `sqlx` and `tokio` for high-performance async I/O
- **SQL Injection Protection**: All queries use parameterized statements
- **Error Handling**: Comprehensive error types with detailed context
- **Logging**: Structured logging with `tracing` integration
- **Type Safety**: Compile-time query validation and type checking

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
elif-orm = "0.5.0"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }
```

## 🏃 Quick Start

### 1. Define Your Models

```rust
use elif_orm::{Model, ModelResult};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Option<Uuid>,
    pub email: String,
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Model for User {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str { "users" }
    fn uses_timestamps() -> bool { true }
    fn uses_soft_deletes() -> bool { true }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        self.id
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = Some(key);
    }

    // Implement timestamp methods...
    fn created_at(&self) -> Option<DateTime<Utc>> { self.created_at }
    fn updated_at(&self) -> Option<DateTime<Utc>> { self.updated_at }
    fn deleted_at(&self) -> Option<DateTime<Utc>> { self.deleted_at }

    fn set_created_at(&mut self, timestamp: DateTime<Utc>) {
        self.created_at = Some(timestamp);
    }

    fn set_updated_at(&mut self, timestamp: DateTime<Utc>) {
        self.updated_at = Some(timestamp);
    }

    fn set_deleted_at(&mut self, timestamp: Option<DateTime<Utc>>) {
        self.deleted_at = timestamp;
    }

    // Field serialization for database operations
    fn to_fields(&self) -> HashMap<String, serde_json::Value> {
        let mut fields = HashMap::new();
        if let Some(id) = self.id {
            fields.insert("id".to_string(), serde_json::Value::String(id.to_string()));
        }
        fields.insert("email".to_string(), serde_json::Value::String(self.email.clone()));
        fields.insert("name".to_string(), serde_json::Value::String(self.name.clone()));
        fields
    }

    // Row deserialization from database
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(User {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            name: row.try_get("name")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            deleted_at: row.try_get("deleted_at")?,
        })
    }
}
```

### 2. Set Up Database Connection

```rust
use elif_orm::database::{create_database_pool, DatabaseServiceProvider};
use sqlx::Pool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create connection pool
    let database_url = "postgresql://username:password@localhost:5432/database";
    let pool = create_database_pool(database_url).await?;

    // Use the pool for operations...
    basic_crud_operations(&pool).await?;
    
    Ok(())
}
```

### 3. Perform CRUD Operations

```rust
async fn basic_crud_operations(pool: &Pool<sqlx::Postgres>) -> ModelResult<()> {
    // CREATE - Insert new user
    let mut new_user = User {
        id: None,
        email: "john@example.com".to_string(),
        name: "John Doe".to_string(),
        created_at: None,
        updated_at: None,
        deleted_at: None,
    };
    
    let created_user = User::create(pool, new_user).await?;
    println!("Created user: {:?}", created_user);

    // READ - Find user by ID
    let user_id = created_user.primary_key().unwrap();
    if let Some(found_user) = User::find(pool, user_id).await? {
        println!("Found user: {}", found_user.name);
    }

    // UPDATE - Modify user
    let mut user_to_update = found_user.unwrap();
    user_to_update.name = "John Smith".to_string();
    user_to_update.update(pool).await?;
    println!("Updated user name");

    // DELETE - Remove user (soft delete)
    user_to_update.delete(pool).await?;
    println!("Deleted user (soft delete)");

    // QUERY - Get all active users
    let all_users = User::all(pool).await?;
    println!("Active users count: {}", all_users.len());

    // COUNT - Get total count
    let user_count = User::count(pool).await?;
    println!("Total active users: {}", user_count);

    Ok(())
}
```

### 4. Advanced Querying

```rust
use elif_orm::QueryBuilder;

async fn advanced_queries(pool: &Pool<sqlx::Postgres>) -> ModelResult<()> {
    // Find users by specific field
    let users_by_email = User::where_field(pool, "email", "john@example.com").await?;
    
    // Find first user matching criteria
    let first_user = User::first_where(pool, "name", "John Doe").await?;
    
    // Using QueryBuilder for complex queries
    let query = User::query()
        .select("name, email, created_at")
        .where_like("name", "%John%")
        .where_gte("created_at", "2024-01-01")
        .order_by("created_at", OrderDirection::Desc)
        .limit(10);
    
    let sql = query.to_sql();
    println!("Generated SQL: {}", sql);

    Ok(())
}
```

### 5. Transactions

```rust
use elif_orm::transaction::{Transaction, IsolationLevel, with_transaction};

async fn transaction_example(pool: &Pool<sqlx::Postgres>) -> ModelResult<()> {
    // Automatic transaction with default settings
    with_transaction(pool, |tx| async move {
        // All operations within this block are transactional
        let user = User::create(pool, new_user).await?;
        user.update(pool).await?;
        // Automatically commits on success, rolls back on error
        Ok(())
    }).await?;

    // Manual transaction control
    let mut tx = Transaction::begin_with_isolation(pool, IsolationLevel::ReadCommitted).await?;
    
    // Perform operations...
    let result = perform_complex_operations(&tx).await;
    
    match result {
        Ok(_) => tx.commit().await?,
        Err(_) => tx.rollback().await?,
    }

    Ok(())
}
```

## 🏗️ Architecture

### Core Components

```
elif-orm/
├── model.rs           # Model trait and CRUD operations
├── query.rs          # QueryBuilder for complex queries  
├── database.rs       # Connection pooling and management
├── transaction.rs    # Transaction support
├── migration.rs      # Schema migration system
├── migration_runner.rs # Migration execution engine
└── error.rs         # Comprehensive error handling
```

### Model System

The `Model` trait is the core abstraction:

```rust
pub trait Model: Send + Sync + Debug + Serialize + for<'de> Deserialize<'de> {
    type PrimaryKey: Clone + Send + Sync + Debug + std::fmt::Display;

    // Required implementations
    fn table_name() -> &'static str;
    fn primary_key(&self) -> Option<Self::PrimaryKey>;
    fn set_primary_key(&mut self, key: Self::PrimaryKey);
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self>;
    fn to_fields(&self) -> HashMap<String, serde_json::Value>;

    // Provided CRUD methods
    async fn find(pool: &Pool<Postgres>, id: Self::PrimaryKey) -> ModelResult<Option<Self>>;
    async fn create(pool: &Pool<Postgres>, model: Self) -> ModelResult<Self>;
    async fn update(&mut self, pool: &Pool<Postgres>) -> ModelResult<()>;
    async fn delete(self, pool: &Pool<Postgres>) -> ModelResult<()>;
    async fn all(pool: &Pool<Postgres>) -> ModelResult<Vec<Self>>;
    async fn count(pool: &Pool<Postgres>) -> ModelResult<i64>;
    // ... and more
}
```

### QueryBuilder

Type-safe, fluent query construction:

```rust
let query = QueryBuilder::<User>::new()
    .select("id, name, email")
    .from("users")
    .where_eq("is_active", true)
    .where_like("name", "%search%")
    .join_inner("profiles", "users.id", "profiles.user_id")
    .order_by("created_at", OrderDirection::Desc)
    .limit(50)
    .offset(100);

let sql = query.to_sql();
let bindings = query.bindings();
```

### Connection Management

Production-ready connection pooling:

```rust
use elif_orm::database::{DatabaseServiceProvider, PoolConfig};

let config = PoolConfig {
    max_connections: 20,
    min_connections: 5,
    acquire_timeout: 30,
    idle_timeout: Some(600),
    max_lifetime: Some(1800),
    test_before_acquire: true,
};

let provider = DatabaseServiceProvider::new(database_url)
    .with_config(config)
    .with_service_name("main_db".to_string());

let pool = provider.create_managed_pool().await?;
```

## 🔧 Configuration

### Environment Variables

```bash
DATABASE_URL=postgresql://username:password@localhost:5432/database
DATABASE_POOL_MAX_CONNECTIONS=20
DATABASE_POOL_MIN_CONNECTIONS=5
DATABASE_ACQUIRE_TIMEOUT=30
```

### Programmatic Configuration

```rust
use elif_orm::database::{DatabaseServiceProvider, PoolConfig};

let provider = DatabaseServiceProvider::new(database_url)
    .with_max_connections(20)
    .with_min_connections(5)
    .with_acquire_timeout(30)
    .with_idle_timeout(Some(600))
    .with_max_lifetime(Some(1800))
    .with_test_before_acquire(true);
```

## 🧪 Testing

elif-orm includes comprehensive test coverage:

```bash
# Run all tests
cargo test -p elif-orm

# Run specific test modules
cargo test -p elif-orm model_tests
cargo test -p elif-orm query_builder_tests  
cargo test -p elif-orm database_tests

# Run with integration tests (requires database)
cargo test -p elif-orm --ignored
```

### Test Categories

- **Unit Tests**: Model trait, query builder, error handling
- **Integration Tests**: Database operations, transactions, migrations  
- **Performance Tests**: Query performance, connection pooling
- **Security Tests**: SQL injection prevention, parameter binding

## 📊 Performance

Benchmarks on standard hardware (results may vary):

| Operation | Performance | Notes |
|-----------|-------------|--------|
| Simple SELECT | < 1ms | With connection pooling |
| Complex JOIN | < 10ms | Multi-table queries |
| Bulk INSERT | > 1000 records/sec | Batch operations |
| Transaction | < 5ms overhead | ACID compliance |
| Connection Acquire | < 1ms | From healthy pool |

## 🔒 Security Features

- **SQL Injection Prevention**: All queries use parameterized statements
- **Type Safety**: Compile-time validation prevents runtime errors
- **Connection Security**: TLS support for encrypted connections
- **Audit Logging**: Comprehensive operation logging
- **Access Control**: Integration with authentication systems

## 🗂️ Migration System

```rust
use elif_orm::migration::{Migration, MigrationManager, SchemaBuilder};

// Define migrations
struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn name(&self) -> &str { "20241215000001_create_users_table" }
    
    fn up(&self, schema: &mut SchemaBuilder) -> Result<(), String> {
        schema.create_table("users", |table| {
            table.uuid("id").primary_key();
            table.string("email").not_null().unique();
            table.string("name").not_null();
            table.timestamps();
            table.soft_deletes();
        })
    }
    
    fn down(&self, schema: &mut SchemaBuilder) -> Result<(), String> {
        schema.drop_table("users")
    }
}

// Run migrations
let manager = MigrationManager::new();
manager.register(Box::new(CreateUsersTable));
manager.run_pending_migrations(&pool).await?;
```

## 🚀 Integration with elif.rs Framework

elif-orm integrates seamlessly with the elif.rs web framework:

```rust
use elif_core::{Container, ServiceProvider};
use elif_orm::database::DatabaseServiceProvider;

// In your elif.rs application
let mut container = Container::builder()
    .register(DatabaseServiceProvider::new(database_url))
    .build()?;

// Models are automatically available in controllers
#[controller("/api/users")]
impl UserController {
    async fn index(&self, pool: &Pool<Postgres>) -> Result<Vec<User>, HttpError> {
        let users = User::all(pool).await?;
        Ok(users)
    }
}
```

## 📚 Examples

Check the [`examples/`](./examples/) directory for comprehensive usage examples:

- [`advanced_queries.rs`](./examples/advanced_queries.rs) - Complex queries and joins
- [`transaction_usage.rs`](./examples/transaction_usage.rs) - Transaction patterns

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
git clone https://github.com/krcpa/elif.rs
cd elif.rs/crates/orm

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## 🔗 Related Projects

- [elif.rs](https://github.com/krcpa/elif.rs) - The main web framework
- [sqlx](https://github.com/launchbadge/sqlx) - The underlying database driver
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime

## 📞 Support

- **Documentation**: [docs.rs/elif-orm](https://docs.rs/elif-orm)
- **Issues**: [GitHub Issues](https://github.com/krcpa/elif.rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/krcpa/elif.rs/discussions)

---

**Made with ❤️ by the elif.rs team**