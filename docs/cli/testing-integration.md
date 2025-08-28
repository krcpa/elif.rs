# Testing Integration CLI Guide

The elif.rs CLI provides a powerful, module-aware testing system that makes running tests intelligent, fast, and productive. This guide covers every aspect of the testing integration system introduced in Epic 6 Phase 4.

## Why elif.rs Testing is Special

**🎯 Module Awareness**: Automatically discovers tests by module structure  
**👀 Smart Watch Mode**: Runs only affected tests when files change  
**🔗 Integration Ready**: Seamless database and environment setup  
**📊 Coverage Reporting**: Built-in coverage with llvm-cov integration  
**⚡ Performance**: Parallel execution with intelligent filtering  
**🏗️ Laravel Familiarity**: Commands you already know and love

## Quick Start

### Basic Test Execution

```bash
# Run all tests (unit + integration)
elifrs test

# Run only unit tests
elifrs test --unit

# Run only integration tests  
elifrs test --integration

# Generate coverage report
elifrs test --coverage
```

### Module-Specific Testing

```bash
# Run tests for specific module
elifrs test --module UserModule

# Run unit tests for user-related code
elifrs test --unit --module user

# Watch mode for continuous testing
elifrs test --watch --module auth
```

### Watch Mode (Continuous Testing)

```bash
# Watch all files and run all tests
elifrs test --watch

# Watch for unit test changes only
elifrs test --watch --unit

# Watch specific module
elifrs test --watch --module UserModule
```

## Core Testing Commands

### `elifrs test`

Run the comprehensive testing suite with module awareness and intelligent discovery.

**Basic Usage:**
```bash
elifrs test [FLAGS] [OPTIONS]
```

**Flags:**
```bash
--unit              Run unit tests only
--integration       Run integration tests only  
--watch            Enable continuous testing (watch mode)
--coverage         Generate coverage reporting
```

**Options:**
```bash
--module <name>    Focus on specific module
```

**Examples:**

```bash
# Run all tests with full discovery
elifrs test

# Unit tests only (fast feedback)
elifrs test --unit

# Integration tests with coverage
elifrs test --integration --coverage

# Focus on UserModule tests
elifrs test --module UserModule

# Watch mode for rapid development
elifrs test --watch --unit
```

## Module-Aware Test Discovery

### How It Works

The testing system intelligently discovers tests across your project:

1. **Unit Tests**: Found in source files with `#[test]` or `#[tokio::test]`
2. **Integration Tests**: Discovered in `tests/` directory
3. **Module Mapping**: Links tests to their corresponding modules
4. **Smart Filtering**: Runs only relevant tests based on changes

### Test Structure Recognition

```rust
// Unit test in src/modules/user.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        // Test code
    }

    #[tokio::test] 
    async fn test_async_user_service() {
        // Async test code
    }
}
```

```rust
// Integration test in tests/user_integration_test.rs
use your_app::modules::UserModule;

#[tokio::test]
async fn test_full_user_workflow() {
    // Integration test code
}
```

### Discovery Output

```
🔍 Discovering tests with module awareness...
✅ Test discovery completed:
   Unit tests: 15
   Integration tests: 8
   Filtered by module: UserModule

📋 Test Execution Plan:
   🧪 Unit Tests: 15 tests across 3 modules
     • UserModule: 8 tests
     • AuthModule: 4 tests  
     • lib: 3 tests
   🔗 Integration Tests: 8 tests across 2 files
     • user_integration_test: 5 tests
     • auth_integration_test: 3 tests
```

## Watch Mode - Continuous Testing

### Smart File Watching

Watch mode monitors your project and automatically runs relevant tests:

```bash
# Start watch mode
elifrs test --watch

👀 Starting continuous testing mode...
   Press Ctrl+C to stop

🚀 Running initial test suite...
# ... initial test results ...

👀 Watching for file changes...

# When you save a file:
🔄 Files changed: ["user.rs"]
🎯 Running tests for affected modules...
# ... targeted test results ...
```

### Intelligent Change Detection

The system watches:
- Source files (`src/**/*.rs`)
- Test files (`tests/**/*.rs`) 
- Configuration (`Cargo.toml`)
- Module directories (`modules/`, `services/`, `controllers/`)

### Smart Test Selection

When files change, the system:
1. **Analyzes** which modules are affected
2. **Filters** to relevant tests only
3. **Runs** targeted test suite
4. **Provides** fast feedback

## Integration Test Environment

### Database Setup

For integration tests requiring databases:

```bash
# Set up test database environment
export TEST_DATABASE_URL="postgresql://localhost/myapp_test"

# Run integration tests (automatic setup)
elifrs test --integration
```

**Environment Setup Output:**
```
🔧 Setting up test environment...
   Database: postgresql://localhost/myapp_test
   Running test database migrations...
   Test data directory: available
✅ Test environment ready
```

### Test Environment Features

- **Automatic migration running** for test databases
- **Environment variable detection** (`DATABASE_URL`, `TEST_DATABASE_URL`)
- **Test data directory support** (`test-data/`)
- **Cleanup and isolation** between test runs

## Coverage Reporting

### Generate Coverage Reports

```bash
# Run tests with coverage
elifrs test --coverage

📊 Generating coverage report...
✅ Coverage report generated: target/llvm-cov/html/index.html
```

### Coverage Requirements

Install cargo-llvm-cov for coverage reporting:

```bash
cargo install cargo-llvm-cov
```

### Coverage Output

```
📊 Test Results Summary
============================================================
🧪 Unit Tests:
   ✅ Passed: 15
   
🔗 Integration Tests:
   ✅ Passed: 8

🎯 Overall Results:
   Total: 23
   ✅ Passed: 23

🎉 All tests passed!
============================================================

📊 Coverage report generated: target/llvm-cov/html/index.html
   Open in browser: file:///.../target/llvm-cov/html/index.html
```

## Advanced Features

### Module Filtering

Target specific parts of your application:

```bash
# Test everything user-related
elifrs test --module user

# Test authentication module only
elifrs test --unit --module AuthModule

# Watch auth module in development  
elifrs test --watch --module auth --unit
```

### Parallel Execution

Tests run in parallel for maximum performance:
- **Unit tests**: Run together with `cargo test --lib`
- **Integration tests**: Run separately per file for isolation
- **Module filtering**: Applied to reduce test surface area

### Smart Error Handling

The system handles common issues gracefully:

```
❌ Project compilation failed:
error[E0425]: cannot find value `undefined_var` in this scope
 --> src/user.rs:23:9

💡 Fix compilation errors before running tests
```

```
📭 No tests found matching the criteria.

💡 Getting Started with Testing:
   • Add #[test] functions to your source files
   • Create integration tests in tests/ directory  
   • Use #[tokio::test] for async tests

📖 Example:
   #[test]
   fn test_user_creation() {
       // Your test code here
   }
```

## Test Organization Patterns

### Module-Based Testing

```
src/
├── modules/
│   ├── user.rs              # Contains unit tests
│   └── auth.rs              # Contains unit tests
├── lib.rs                   # Contains lib-level tests
└── ...

tests/
├── user_integration_test.rs # Integration tests  
├── auth_integration_test.rs # Integration tests
└── ...
```

### Test Naming Conventions

```rust
// Unit tests within modules
#[cfg(test)]
mod tests {
    #[test]
    fn test_create_user() { }
    
    #[test] 
    fn test_validate_email() { }
    
    #[tokio::test]
    async fn test_async_operation() { }
}

// Integration test files
// tests/user_workflow_test.rs
#[tokio::test]
async fn test_complete_user_registration_flow() { }

#[tokio::test]  
async fn test_user_authentication_flow() { }
```

## Performance Optimization

### Fast Feedback Loop

1. **Unit tests first**: Run with `--unit` for immediate feedback
2. **Module filtering**: Use `--module` to focus on changes
3. **Watch mode**: Get continuous feedback during development
4. **Parallel execution**: Tests run concurrently when possible

### Recommended Workflow

```bash
# During development (fast)
elifrs test --watch --unit --module UserModule

# Before commit (comprehensive)  
elifrs test --coverage

# CI/CD (complete)
elifrs test --unit && elifrs test --integration
```

## Integration with Development Workflow

### With Module System

The testing system integrates seamlessly with the elif.rs module system:

```bash
# Validate modules and run tests
elifrs module validate && elifrs test

# Create module with tests  
elifrs make module UserModule --services=UserService
# Then add tests and run:
elifrs test --module UserModule
```

### With Database Seeding

Combine with database commands for complete testing:

```bash
# Fresh database with seeds, then test
elifrs db fresh --seed && elifrs test --integration

# Reset database and test specific module
elifrs db reset --with-seeds && elifrs test --integration --module UserModule
```

## Troubleshooting

### Common Issues

**Tests not discovered:**
```bash
# Verify test discovery
elifrs test --unit

# Check if tests are properly annotated with #[test]
```

**Watch mode not detecting changes:**
```bash
# Check that you're in a Rust project with Cargo.toml
# Verify files are in watched directories (src/, tests/, modules/)
```

**Coverage reports not generating:**
```bash
# Install required tool
cargo install cargo-llvm-cov

# Verify it's available
cargo llvm-cov --version
```

**Integration tests failing:**
```bash
# Check database environment
echo $DATABASE_URL
echo $TEST_DATABASE_URL

# Verify migrations exist  
ls migrations/
```

### Debug Mode

For debugging test discovery and execution:

```bash
# Add RUST_LOG for detailed output
RUST_LOG=debug elifrs test --unit --module UserModule
```

## Best Practices

### Test Organization
- **Keep unit tests close** to the code they test
- **Group integration tests** by feature or workflow
- **Use descriptive names** that explain what is being tested

### Module Testing
- **Test at module boundaries** to verify interfaces
- **Use integration tests** for cross-module interactions
- **Focus unit tests** on individual functions and methods

### Watch Mode Usage
- **Start with `--unit`** for fastest feedback during development
- **Add `--module`** filtering when working on specific features
- **Use full test suite** before committing changes

### Coverage Goals
- **Aim for high coverage** of business logic
- **Focus on critical paths** rather than 100% coverage
- **Use coverage reports** to find untested code paths

## Examples

### Basic Test Setup

```rust
// src/modules/user.rs
pub struct User {
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(name: String, email: String) -> Result<Self, &'static str> {
        if email.contains('@') {
            Ok(User { name, email })
        } else {
            Err("Invalid email")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_user_creation() {
        let user = User::new("Alice".to_string(), "alice@example.com".to_string());
        assert!(user.is_ok());
    }
    
    #[test]
    fn test_invalid_email_rejected() {
        let user = User::new("Bob".to_string(), "invalid-email".to_string());
        assert!(user.is_err());
    }
}
```

### Integration Test Example

```rust
// tests/user_integration_test.rs
use your_app::modules::UserModule;
use your_app::database::Database;

#[tokio::test]
async fn test_user_registration_workflow() {
    // Setup test database
    let db = Database::new_test().await;
    
    // Test complete workflow
    let user_service = UserModule::new(db);
    let result = user_service.register("Alice", "alice@example.com").await;
    
    assert!(result.is_ok());
    
    // Verify in database
    let saved_user = user_service.find_by_email("alice@example.com").await;
    assert!(saved_user.is_some());
}
```

### Watch Mode Session

```bash
$ elifrs test --watch --unit --module user

👀 Starting continuous testing mode...
   Press Ctrl+C to stop

🚀 Running initial test suite...
🔍 Discovering tests with module awareness...
✅ Test discovery completed:
   Unit tests: 8
   Integration tests: 0
   Filtered by module: user

📋 Test Execution Plan:
   🧪 Unit Tests: 8 tests across 1 modules
     • user: 8 tests

🧪 Running Unit Tests...
test user::tests::test_valid_user_creation ... ok
test user::tests::test_invalid_email_rejected ... ok
test user::tests::test_user_serialization ... ok
# ... more tests ...

📊 Test Results Summary
============================================================
🧪 Unit Tests:
   ✅ Passed: 8

🎯 Overall Results:
   Total: 8
   ✅ Passed: 8

🎉 All tests passed!
============================================================

👀 Watching for file changes...

# ... You edit src/modules/user.rs ...

🔄 Files changed: ["user.rs"]
🎯 Running tests for affected modules...
# ... runs tests again with new changes ...
```

This comprehensive testing system ensures your elif.rs applications are well-tested, maintainable, and production-ready with minimal configuration and maximum developer productivity.