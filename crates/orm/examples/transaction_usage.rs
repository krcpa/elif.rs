//! Transaction Usage Examples
//! 
//! This example demonstrates how to use the transaction system in elif-orm.

use elif_orm::{
    transaction::{Transaction, TransactionConfig, IsolationLevel, with_transaction, with_transaction_default},
    database::{ManagedPool, PoolConfig},
    error::ModelError,
};

// This is a basic example showing transaction API usage
// Note: This requires a real database connection to run

fn main() {
    println!("Transaction usage examples (see source code for implementation details)");
}

// Example usage (would require a real database connection)
#[allow(dead_code)]
async fn transaction_examples() -> Result<(), ModelError> {
    // This is pseudo-code showing how transactions would be used
    // In real usage, you'd have a properly configured database pool
    
    // Example 1: Basic transaction with manual commit/rollback
    let pool = create_mock_pool().await?;
    
    let mut tx = Transaction::begin_default(&pool).await?;
    
    // Perform database operations here
    // If everything succeeds:
    tx.commit().await?;
    
    // Example 2: Transaction with specific isolation level
    let config = TransactionConfig {
        isolation_level: Some(IsolationLevel::Serializable),
        auto_retry: true,
        max_retries: 3,
        ..Default::default()
    };
    
    let mut tx = Transaction::begin(&pool, config).await?;
    
    // Perform operations that need serializable isolation
    // The transaction will auto-retry on serialization failures
    tx.commit().await?;
    
    // Example 3: Read-only transaction
    let mut tx = Transaction::begin_read_only(&pool).await?;
    
    // Perform read-only operations
    tx.commit().await?;
    
    // Example 4: Automatic transaction management
    let result = with_transaction_default(&pool, || {
        Box::pin(async {
            // Your database operations here
            // If this returns Ok, transaction commits automatically
            // If this returns Err, transaction rolls back automatically
            Ok("Operation completed successfully".to_string())
        })
    }).await?;
    
    println!("Result: {}", result);
    
    // Example 5: Transaction with custom configuration and auto-retry
    let serializable_config = TransactionConfig {
        isolation_level: Some(IsolationLevel::Serializable),
        auto_retry: true,
        max_retries: 5,
        ..Default::default()
    };
    
    let result = with_transaction(&pool, serializable_config, || {
        Box::pin(async {
            // Operations that might face serialization conflicts
            // Will be automatically retried up to 5 times
            perform_complex_operation().await
        })
    }).await?;
    
    println!("Complex operation result: {}", result);
    
    Ok(())
}

#[allow(dead_code)]
async fn create_mock_pool() -> Result<ManagedPool, ModelError> {
    // This would create a real database pool in actual usage
    Err(ModelError::Connection("Mock pool - database not configured".to_string()))
}

#[allow(dead_code)]
async fn perform_complex_operation() -> Result<String, ModelError> {
    // Simulate a complex database operation that might face serialization conflicts
    Ok("Complex operation completed".to_string())
}

// Usage patterns for different scenarios:

/// Example: E-commerce order processing with transactions
#[allow(dead_code)]
async fn process_order_with_transaction(pool: &ManagedPool, order_id: i32) -> Result<(), ModelError> {
    with_transaction_default(pool, || {
        Box::pin(async move {
            // 1. Validate inventory
            // 2. Reserve items
            // 3. Process payment
            // 4. Update order status
            // 5. Send confirmation
            
            // All operations are atomic - if any step fails, all changes roll back
            println!("Processing order {}", order_id);
            Ok(())
        })
    }).await
}

/// Example: Bank transfer with serializable isolation
#[allow(dead_code)]
async fn bank_transfer(
    pool: &ManagedPool, 
    from_account: i32, 
    to_account: i32, 
    amount: i64
) -> Result<(), ModelError> {
    let config = TransactionConfig {
        isolation_level: Some(IsolationLevel::Serializable),
        auto_retry: true,
        max_retries: 3,
        ..Default::default()
    };
    
    with_transaction(pool, config, || {
        Box::pin(async move {
            // 1. Check from_account balance
            // 2. Debit from_account
            // 3. Credit to_account
            // 4. Log transaction
            
            // Serializable isolation ensures no concurrent transfers can cause
            // inconsistent state, with automatic retry on conflicts
            println!("Transferring {} from account {} to account {}", amount, from_account, to_account);
            Ok(())
        })
    }).await
}

/// Example: Bulk data processing with manual transaction control
#[allow(dead_code)]
async fn bulk_data_import(pool: &ManagedPool, data: Vec<String>) -> Result<usize, ModelError> {
    let mut tx = Transaction::begin_default(pool).await?;
    let mut processed = 0;
    
    for item in data {
        // Process each item
        // If processing fails, we can decide whether to rollback or continue
        match process_item(&item).await {
            Ok(_) => {
                processed += 1;
                
                // Commit every 100 items to avoid long-running transactions
                if processed % 100 == 0 {
                    tx.commit().await?;
                    tx = Transaction::begin_default(pool).await?;
                }
            }
            Err(e) => {
                eprintln!("Failed to process item {}: {}", item, e);
                // Continue processing other items
            }
        }
    }
    
    // Commit any remaining changes
    if tx.is_active() {
        tx.commit().await?;
    }
    
    Ok(processed)
}

#[allow(dead_code)]
async fn process_item(_item: &str) -> Result<(), ModelError> {
    // Mock item processing
    Ok(())
}