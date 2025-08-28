# Database Seeding

Populate your development and test databases with realistic data using intelligent seeders, factories, and dependency resolution. elif.rs makes seeding as elegant as Laravel's system, but with Rust's safety and performance.

## Why Seeding Matters

**The Problem**: Fresh databases are empty. Testing with empty data is useless. Setting up test data manually is tedious and error-prone.

**The Solution**: Smart seeders that understand dependencies, generate realistic data, and adapt to different environments automatically.

```bash
# One command, perfect database
elifrs db:fresh --seed
```

## Quick Start

### 1. Generate Your First Seeder

```bash
# Create a basic seeder
elifrs make:seeder UserSeeder

# Create a seeder for a specific table  
elifrs make:seeder UserSeeder --table users

# Create a seeder with factory integration
elifrs make:seeder ProductSeeder --table products --factory
```

### 2. Edit Your Seeder

```rust
// database/seeders/user_seeder.rs
use elif_orm::Database;

pub struct UserSeeder;

impl UserSeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        println!("🌱 Running UserSeeder...");
        
        // Simple, clean data insertion
        db.query("INSERT INTO users (name, email) VALUES ($1, $2)")
            .bind("Alice Johnson")
            .bind("alice@example.com")
            .execute()
            .await?;
        
        println!("✅ UserSeeder completed");
        Ok(())
    }
    
    // Define dependencies (run RoleSeeder first)
    pub fn dependencies() -> Vec<&'static str> {
        vec!["RoleSeeder"]
    }
}
```

### 3. Run Your Seeders

```bash
# Run all seeders
elifrs db:seed

# Run seeders for test environment
elifrs db:seed --env test

# Fresh database with seeds
elifrs db:fresh --seed
```

## Core Concepts

### Dependency Resolution

elif.rs automatically orders your seeders based on dependencies. No more guessing, no more failures due to missing data.

```rust
// UserSeeder depends on RoleSeeder
pub fn dependencies() -> Vec<&'static str> {
    vec!["RoleSeeder"]  // Roles must exist before users
}
```

**What happens**: elif.rs builds a dependency graph, performs topological sorting, and runs seeders in the correct order. Circular dependencies are detected and prevented.

### Environment Safety

Different environments need different data. elif.rs protects production while enabling rich development data.

```bash
# Safe for development and testing
elifrs db:seed --env dev

# Requires explicit confirmation for production
elifrs db:seed --env production --force
```

### Smart Templates

Three seeder templates for different needs:

- **Basic**: Custom seeding logic, full control
- **Table-Targeted**: Pre-filled sample data for quick setup  
- **Factory-Integrated**: Automated realistic data generation

## Advanced Features

### Factory-Powered Seeders

Generate thousands of realistic records effortlessly:

```rust
use elif_orm::{Database, factory::Factory};
use serde_json::json;

pub struct ProductSeeder;

impl ProductSeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let factory = Factory::new();
        
        // Generate 100 realistic products
        for i in 1..=100 {
            let data = factory
                .for_table("products")
                .with_attributes(json!({
                    "name": format!("Product {}", i),
                    "price": (1000 + i * 50) as f64 / 100.0,
                    "category": match i % 3 {
                        0 => "Electronics",
                        1 => "Books", 
                        _ => "Clothing"
                    },
                    "created_at": chrono::Utc::now(),
                }))
                .build();
            
            factory.insert(db, "products", &data).await?;
        }
        
        Ok(())
    }
}
```

### Relationship Seeding

Seed related data with intelligent relationships:

```rust
pub struct OrderSeeder;

impl OrderSeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        // Get existing users and products
        let users = db.query("SELECT id FROM users LIMIT 10").fetch_all().await?;
        let products = db.query("SELECT id FROM products LIMIT 50").fetch_all().await?;
        
        // Create orders with realistic relationships
        for user in users {
            let user_id: i32 = user.get("id");
            
            // Each user gets 1-5 orders
            for _ in 0..rand::random::<u8>() % 5 + 1 {
                let order_id = db.query("INSERT INTO orders (user_id, status) VALUES ($1, $2) RETURNING id")
                    .bind(user_id)
                    .bind("pending")
                    .fetch_one()
                    .await?
                    .get::<i32, _>("id");
                
                // Each order gets 1-3 products
                for _ in 0..rand::random::<u8>() % 3 + 1 {
                    let product = &products[rand::random::<usize>() % products.len()];
                    let product_id: i32 = product.get("id");
                    
                    db.query("INSERT INTO order_items (order_id, product_id, quantity) VALUES ($1, $2, $3)")
                        .bind(order_id)
                        .bind(product_id)
                        .bind(rand::random::<u8>() % 5 + 1)
                        .execute()
                        .await?;
                }
            }
        }
        
        Ok(())
    }
    
    pub fn dependencies() -> Vec<&'static str> {
        vec!["UserSeeder", "ProductSeeder"]  // Need both users and products
    }
}
```

## Database Lifecycle Commands

### Complete Database Management

elif.rs provides Laravel-level database lifecycle commands:

```bash
# Create fresh database with seeds
elifrs db:fresh --seed

# Reset database (rollback + migrate + seed)  
elifrs db:reset --with-seeds

# Run only seeders
elifrs db:seed

# Environment-specific seeding
elifrs db:seed --env test
elifrs db:seed --env production --force

# Verbose output for debugging
elifrs db:seed --verbose
```

### Development Workflow

**Daily Development**:
```bash
# Fresh start every morning
elifrs db:fresh --seed
```

**Feature Development**:
```bash
# Add new seeder
elifrs make:seeder FeatureSeeder --table features

# Test with fresh data
elifrs db:reset --with-seeds
```

**Testing**:
```bash
# Clean test environment
elifrs db:fresh --seed --env test
```

## Best Practices

### 1. Organize by Domain

```
database/seeders/
├── core/
│   ├── role_seeder.rs      # Base roles
│   └── user_seeder.rs      # Base users
├── content/
│   ├── post_seeder.rs      # Blog posts
│   └── comment_seeder.rs   # Comments
└── ecommerce/
    ├── product_seeder.rs   # Products
    └── order_seeder.rs     # Orders
```

### 2. Use Clear Dependencies

```rust
// ✅ Good: Clear, explicit dependencies
pub fn dependencies() -> Vec<&'static str> {
    vec!["RoleSeeder", "CategorySeeder"]
}

// ❌ Bad: No dependencies when needed
pub fn dependencies() -> Vec<&'static str> {
    vec![]  // But the seeder needs roles!
}
```

### 3. Environment-Aware Data

```rust
pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let env = std::env::var("ELIF_ENV").unwrap_or("development".to_string());
    
    let user_count = match env.as_str() {
        "test" => 3,        // Minimal data for fast tests
        "development" => 50, // Rich data for development
        "staging" => 100,   // Production-like volume
        _ => 0              // No seeding in production
    };
    
    // Seed appropriate amount of data
    // ...
}
```

### 4. Idempotent Seeders

Make seeders safe to run multiple times:

```rust
pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    // Check if already seeded
    let existing = db.query("SELECT COUNT(*) as count FROM users WHERE email = $1")
        .bind("admin@example.com")
        .fetch_one()
        .await?
        .get::<i64, _>("count");
    
    if existing > 0 {
        println!("ℹ️  Admin user already exists, skipping");
        return Ok(());
    }
    
    // Safe to seed admin user
    // ...
}
```

## Error Handling

### Dependency Validation

elif.rs validates dependencies before running:

```
❌ Error: Seeder 'UserSeeder' depends on 'RoleSeeder', but 'RoleSeeder' was not found

💡 Fix: Create RoleSeeder or remove dependency
```

### Circular Dependency Detection

```
❌ Error: Circular dependency detected in seeders: UserSeeder, RoleSeeder

💡 Fix: Review dependencies in UserSeeder and RoleSeeder
```

### Environment Safety

```
⚠️  WARNING: Running seeders on production environment!
   This operation will permanently add data.
   Are you sure you want to continue? (y/N): 
```

## Comparison with Other Frameworks

| Feature | elif.rs | Laravel | Rails | Django |
|---------|---------|---------|-------|---------|
| Dependency Resolution | ✅ Automatic | ❌ Manual | ❌ Manual | ❌ Manual |
| Circular Dependency Detection | ✅ Yes | ❌ No | ❌ No | ❌ No |
| Environment Safety | ✅ Built-in | ✅ Yes | ✅ Yes | ✅ Yes |
| Factory Integration | ✅ Built-in | ✅ Yes | ✅ Yes | ✅ Yes |
| Type Safety | ✅ Compile-time | ❌ Runtime | ❌ Runtime | ❌ Runtime |
| Performance | ✅ Rust speed | ❌ PHP/Ruby | ❌ Ruby | ❌ Python |

## Troubleshooting

### Common Issues

**Seeder not found**:
```bash
# Ensure seeder is in database/seeders/ directory
# Check mod.rs includes your seeder module
```

**Permission denied in production**:
```bash
# Use --force flag for production (use carefully!)
elifrs db:seed --env production --force
```

**Slow seeding**:
```bash
# Use batch inserts for large data sets
# Consider factory patterns for realistic data
```

### Performance Tips

1. **Batch Operations**: Use transactions for multiple inserts
2. **Factory Generation**: Let factories generate realistic data
3. **Environment Sizing**: Adjust data volume per environment
4. **Dependency Optimization**: Minimize seeder dependencies

## What's Next?

- Learn about [Database Migrations](./migrations.md) for schema management
- Explore [Model Relationships](./relationships.md) for complex data structures  
- Read about [Testing](../testing/database-testing.md) with seeded data
- Discover [CLI Commands](../cli/commands.md) for more database tools

elif.rs seeding makes database population **simple**, **safe**, and **smart**. Your future self will thank you for the clean, realistic test data! 🌱

