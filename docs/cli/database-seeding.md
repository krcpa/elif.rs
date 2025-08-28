# Database Seeding CLI Guide

The elif.rs CLI provides Laravel-inspired database seeding commands that make populating your database with test data simple, safe, and intelligent. This guide covers every aspect of the database seeding system.

## Why elif.rs Seeding is Special

**🎯 Intelligent Dependency Resolution**: Seeders run in correct order automatically
**🛡️ Environment Safety**: Built-in protection against accidental production seeding  
**🏭 Factory Integration**: Generate realistic test data effortlessly
**⚡ Performance**: Rust-powered speed with compile-time safety
**🔄 Laravel Familiarity**: Commands you already know and love

## Quick Start

### Generate Your First Seeder

```bash
# Basic seeder generation
elifrs make:seeder UserSeeder

# Table-specific seeder with sample data
elifrs make:seeder UserSeeder --table users

# Factory-powered seeder for realistic data
elifrs make:seeder ProductSeeder --table products --factory
```

### Run Your Seeders

```bash
# Run all seeders with dependency resolution
elifrs db:seed

# Fresh database with seeds (development workflow)
elifrs db:fresh --seed

# Reset and reseed (testing workflow)  
elifrs db:reset --with-seeds
```

## Seeder Generation Commands

### `elifrs make:seeder <name>`

Generate a database seeder with intelligent templates.

**Basic Usage:**
```bash
elifrs make:seeder UserSeeder
```

**Options:**
- `--table <table>`: Target specific table with sample data
- `--factory`: Include factory integration for realistic data generation

**Examples:**

```bash
# Basic seeder (full control)
elifrs make:seeder AdminSeeder

# Table-targeted seeder (quick setup)  
elifrs make:seeder CategorySeeder --table categories

# Factory-powered seeder (realistic data)
elifrs make:seeder CustomerSeeder --table customers --factory

# Complex seeder with dependencies
elifrs make:seeder OrderSeeder --table orders
# Then edit to add dependencies: vec!["CustomerSeeder", "ProductSeeder"]
```

**Generated Files:**
- Basic: Custom seeding logic template
- Table: Pre-filled with sample INSERT statements  
- Factory: Automated data generation with realistic patterns

## Database Lifecycle Commands

### `elifrs db:fresh [--seed]`

Create a completely fresh database with optional seeding.

**What it does:**
1. Drop all tables (if exist)
2. Run all migrations from scratch
3. Run seeders (if `--seed` flag provided)

```bash
# Fresh database only
elifrs db:fresh

# Fresh database with seeders
elifrs db:fresh --seed

# Fresh database for specific environment
elifrs db:fresh --seed --env test
```

**Perfect for:** Daily development, feature branches, clean testing

### `elifrs db:reset [--with-seeds]`

Reset database by rolling back migrations and re-running them.

**What it does:**
1. Rollback all applied migrations
2. Re-run all migrations
3. Run seeders (if `--with-seeds` flag provided)

```bash
# Reset database schema
elifrs db:reset

# Reset and reseed
elifrs db:reset --with-seeds

# Reset with confirmation (production)
elifrs db:reset --with-seeds --force
```

**Perfect for:** Schema changes, migration testing, data refresh

### `elifrs db:seed [options]`

Run database seeders with intelligent dependency resolution.

**Options:**
- `--env <environment>`: Target specific environment (dev, test, staging, prod)
- `--force`: Force run in production (requires confirmation)
- `--verbose`: Show detailed seeding progress

```bash
# Run all seeders
elifrs db:seed

# Environment-specific seeding
elifrs db:seed --env test
elifrs db:seed --env staging

# Production seeding (use carefully!)
elifrs db:seed --env production --force

# Verbose output for debugging
elifrs db:seed --verbose
```

**Features:**
- **Dependency Resolution**: Runs seeders in correct order based on dependencies
- **Circular Detection**: Prevents infinite loops from circular dependencies  
- **Environment Safety**: Protects production with confirmation prompts
- **Error Recovery**: Clear error messages with suggested fixes

## Database Management Commands

### `elifrs db:setup`

Initialize and validate database connection with health checks.

```bash
elifrs db:setup
```

**Output:**
```
🗄️ Database Setup & Health Check
Connection: postgresql://***@localhost:5432/myapp_dev
Environment: development

✅ Connection: OK (12ms)
✅ Pool Status: 10 total, 0 active, 10 idle  
✅ Schema: Up to date (15 migrations applied)

📊 Database setup completed successfully!
```

### `elifrs db:status`

Check database health and migration status.

```bash
elifrs db:status --verbose
```

**Output:**
```
🗄️ Database Status Check

✅ Connection: postgresql://***@localhost:5432/myapp_dev
✅ Health Check: Passed (8ms)
✅ Pool Status: 10 total, 2 active, 8 idle
📊 Pool Stats: 1,234 acquires, 0.1% error rate
✅ Schema Version: Up to date
✅ Total Migrations: 15 applied

💡 Recommendations:
   • Database is healthy and ready for development
```

### Database Creation and Destruction

```bash
# Create database
elifrs db:create myapp_test --env test

# Drop database (with confirmation)
elifrs db:drop myapp_old --env staging --force
```

## Advanced Seeding Patterns

### Environment-Aware Seeding

Create seeders that adapt to different environments:

```rust
pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let env = std::env::var("ELIF_ENV").unwrap_or("development".to_string());
    
    let record_count = match env.as_str() {
        "test" => 5,         // Fast tests
        "development" => 50,  // Rich development data  
        "staging" => 200,    // Production-like volume
        _ => 0               // No automatic production seeding
    };
    
    for i in 1..=record_count {
        // Seed appropriate amount of data
        db.query("INSERT INTO users (name, email) VALUES ($1, $2)")
            .bind(format!("User {}", i))
            .bind(format!("user{}@example.com", i))
            .execute()
            .await?;
    }
    
    Ok(())
}
```

### Complex Dependency Chains

Handle complex relationships with clear dependencies:

```rust
// Role-based user system
pub struct AdminSeeder;
impl AdminSeeder {
    pub fn dependencies() -> Vec<&'static str> {
        vec!["RoleSeeder"]  // Admin needs roles
    }
}

pub struct UserSeeder;  
impl UserSeeder {
    pub fn dependencies() -> Vec<&'static str> {
        vec!["RoleSeeder", "AdminSeeder"]  // Users need roles and admin
    }
}

pub struct PostSeeder;
impl PostSeeder {
    pub fn dependencies() -> Vec<&'static str> {
        vec!["UserSeeder"]  // Posts need users
    }
}

pub struct CommentSeeder;
impl CommentSeeder {
    pub fn dependencies() -> Vec<&'static str> {
        vec!["UserSeeder", "PostSeeder"]  // Comments need users and posts
    }
}
```

**Execution Order**: RoleSeeder → AdminSeeder → UserSeeder → PostSeeder → CommentSeeder

### Factory-Powered Realistic Data

Generate thousands of realistic records:

```rust
use elif_orm::{Database, factory::Factory};
use serde_json::json;

pub struct CustomerSeeder;

impl CustomerSeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let factory = Factory::new();
        
        // Generate 500 realistic customers
        for i in 1..=500 {
            let data = factory
                .for_table("customers")
                .with_attributes(json!({
                    "name": format!("{} {}", 
                        factory.fake_first_name(), 
                        factory.fake_last_name()
                    ),
                    "email": format!("customer{}@{}.com", i, factory.fake_domain()),
                    "phone": factory.fake_phone(),
                    "address": json!({
                        "street": factory.fake_street_address(),
                        "city": factory.fake_city(),
                        "country": factory.fake_country(),
                        "postal_code": factory.fake_postal_code(),
                    }),
                    "registration_date": factory.fake_date_between(
                        "2020-01-01", "2024-12-31"
                    ),
                    "customer_tier": factory.fake_choice(&[
                        "bronze", "silver", "gold", "platinum"
                    ]),
                }))
                .build();
            
            factory.insert(db, "customers", &data).await?;
        }
        
        Ok(())
    }
}
```

## Development Workflows

### Daily Development

Start each day with fresh, seeded data:

```bash
# Morning routine - fresh everything
elifrs db:fresh --seed

# Quick seed refresh (keep schema)  
elifrs db:seed
```

### Feature Development

Develop features with consistent test data:

```bash
# Create seeder for new feature
elifrs make:seeder FeatureSeeder --table features

# Test feature with fresh data
elifrs db:reset --with-seeds

# Iterate on seeder
# Edit database/seeders/feature_seeder.rs
elifrs db:seed  # Re-run seeders
```

### Testing Workflow

Ensure clean test environment:

```bash
# Before test suite
elifrs db:fresh --seed --env test

# Reset between test runs
elifrs db:reset --with-seeds --env test
```

### Team Collaboration

Share seeded data across team:

```bash
# Team member pulls new seeders
git pull origin main

# Update database with new seeders  
elifrs db:fresh --seed

# Verify everyone has same data
elifrs db:status
```

## Error Handling and Troubleshooting

### Common Error Messages

**Missing Dependency:**
```
❌ Seeder 'UserSeeder' depends on 'RoleSeeder', but 'RoleSeeder' was not found

💡 Fix: Create RoleSeeder with: elifrs make:seeder RoleSeeder --table roles
```

**Circular Dependency:**
```
❌ Circular dependency detected in seeders: UserSeeder, RoleSeeder

💡 Fix: Review dependencies in both seeders - one shouldn't depend on the other
```

**Production Safety:**
```
⚠️  WARNING: Running seeders on production environment!
   This operation will permanently add data.
   Are you sure you want to continue? (y/N): n

Operation cancelled
```

**Database Connection:**
```
❌ Database connection failed: connection refused

💡 Fix: 
  • Check DATABASE_URL environment variable
  • Ensure database server is running
  • Verify connection permissions
```

### Performance Issues

**Slow Seeding:**
```bash
# Use transactions for batch operations
# Generate data in chunks rather than one-by-one
# Consider factory patterns for realistic data

# Example: Batch insert
db.query("INSERT INTO users (name, email) VALUES 
    ($1, $2), ($3, $4), ($5, $6)")
    .bind("User 1").bind("user1@example.com")
    .bind("User 2").bind("user2@example.com")  
    .bind("User 3").bind("user3@example.com")
    .execute()
    .await?;
```

### Debugging Seeders

**Verbose Output:**
```bash
elifrs db:seed --verbose
```

**Output:**
```
🌱 Running Database Seeders
Environment: development

🔄 Resolving seeder dependencies...
✅ Dependency resolution completed
📋 Execution order: RoleSeeder → UserSeeder → PostSeeder

🌱 Running seeder: RoleSeeder
✅ RoleSeeder completed (15ms)
🌱 Running seeder: UserSeeder  
✅ UserSeeder completed (234ms)
🌱 Running seeder: PostSeeder
✅ PostSeeder completed (89ms)

🎉 Database seeding completed successfully!
```

## Integration with Testing

### Test Database Setup

```bash
# Set up test database with seeds
export ELIF_ENV=test
elifrs db:fresh --seed

# Run your tests
cargo test

# Clean up (optional)
elifrs db:drop test_db --env test
```

### Factory Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_user_seeder() {
        let db = setup_test_db().await;
        
        // Run seeder
        UserSeeder::run(&db).await.expect("Seeder should succeed");
        
        // Verify data
        let user_count = db.query("SELECT COUNT(*) as count FROM users")
            .fetch_one()
            .await
            .unwrap()
            .get::<i64, _>("count");
            
        assert!(user_count > 0, "Seeder should create users");
    }
}
```

## Best Practices Summary

### 1. **Clear Dependencies**
```rust
// ✅ Good: Explicit dependencies
pub fn dependencies() -> Vec<&'static str> {
    vec!["RoleSeeder", "CategorySeeder"]
}
```

### 2. **Environment Awareness**  
```bash
# ✅ Good: Environment-specific seeding
elifrs db:seed --env test  # Fast, minimal data
elifrs db:seed --env development  # Rich development data
```

### 3. **Idempotent Seeders**
```rust
// ✅ Good: Safe to run multiple times
let existing = db.query("SELECT COUNT(*) FROM admin_users WHERE email = $1")
    .bind("admin@example.com")
    .fetch_one().await?;
    
if existing.get::<i64, _>("count") == 0 {
    // Safe to create admin user
}
```

### 4. **Realistic Test Data**
```bash
# ✅ Good: Factory-powered realistic data
elifrs make:seeder CustomerSeeder --table customers --factory
```

### 5. **Performance Optimization**
```rust
// ✅ Good: Batch operations in transactions
db.transaction(|txn| async move {
    for chunk in user_data.chunks(100) {
        // Batch insert chunk
    }
    Ok(())
}).await?;
```

## Comparison: elif.rs vs Other Frameworks

| Feature | elif.rs | Laravel | Rails | Django |
|---------|---------|---------|-------|---------|
| **Dependency Resolution** | ✅ Automatic | ❌ Manual | ❌ Manual | ❌ Manual |
| **Circular Detection** | ✅ Built-in | ❌ Manual | ❌ Manual | ❌ Manual |  
| **Type Safety** | ✅ Compile-time | ❌ Runtime | ❌ Runtime | ❌ Runtime |
| **Performance** | ✅ Rust speed | ❌ PHP | ❌ Ruby | ❌ Python |
| **Environment Safety** | ✅ Built-in | ✅ Manual | ✅ Manual | ✅ Manual |
| **Factory Integration** | ✅ Native | ✅ Yes | ✅ Yes | ✅ Yes |
| **CLI Experience** | ✅ Laravel-inspired | ✅ Excellent | ✅ Good | ✅ Good |

## What Makes elif.rs Seeding Special

### **Intelligence Over Manual Work**
- **Automatic dependency resolution** eliminates guesswork
- **Circular dependency detection** prevents infinite loops
- **Environment-aware execution** protects production data

### **Safety Over Speed**
- **Compile-time validation** catches errors before runtime
- **Type-safe database operations** prevent SQL injection
- **Confirmation prompts** protect critical environments

### **Developer Experience**
- **Laravel-familiar commands** reduce learning curve
- **Rich error messages** provide clear next steps  
- **Verbose output** helps debug complex seeding scenarios

## Next Steps

**Learn More:**
- [Database Seeding Guide](../database/seeding.md) - Complete seeding documentation
- [Database Migrations](../database/migrations.md) - Schema management
- [Testing with Databases](../testing/database-testing.md) - Test data strategies

**Try It Out:**
```bash
# Create a new project with seeding
elifrs new my-app --template web
cd my-app

# Generate your first seeder
elifrs make:seeder UserSeeder --table users --factory

# See the magic
elifrs db:fresh --seed
```

elif.rs database seeding: **Simple**, **Safe**, **Smart**. Just the way database population should be! 🌱✨