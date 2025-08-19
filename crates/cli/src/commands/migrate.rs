use elif_core::ElifError;
use elif_orm::{MigrationManager, MigrationRunner, MigrationRollback};

pub async fn create(name: &str) -> Result<(), ElifError> {
    let manager = MigrationManager::new();
    
    match manager.create_migration(name).await {
        Ok(filename) => {
            println!("✓ Created migration: {}", filename);
            Ok(())
        }
        Err(e) => Err(ElifError::Database { message: format!("Failed to create migration: {}", e) })
    }
}

pub async fn run() -> Result<(), ElifError> {
    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());
    
    // Create migration manager and runner
    let manager = MigrationManager::new();
    let runner = MigrationRunner::from_url(manager, &database_url).await
        .map_err(|e| ElifError::Database { message: format!("Failed to create migration runner: {}", e) })?;
    
    println!("🚀 Running database migrations...");
    
    match runner.run_migrations().await {
        Ok(result) => {
            if result.applied_count == 0 {
                println!("✓ No pending migrations found. Database is up to date.");
                if result.skipped_count > 0 {
                    println!("  {} migrations already applied.", result.skipped_count);
                }
            } else {
                println!("✓ Applied {} migration(s) successfully:", result.applied_count);
                for migration_id in &result.applied_migrations {
                    println!("  - {}", migration_id);
                }
                println!("  Execution time: {}ms", result.execution_time_ms);
            }
            Ok(())
        }
        Err(e) => Err(ElifError::Database { message: format!("Migration failed: {}", e) })
    }
}

pub async fn rollback() -> Result<(), ElifError> {
    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());
    
    // Create migration manager and runner
    let manager = MigrationManager::new();
    let runner = MigrationRunner::from_url(manager, &database_url).await
        .map_err(|e| ElifError::Database { message: format!("Failed to create migration runner: {}", e) })?;
    
    println!("🔄 Rolling back last batch of migrations...");
    
    match runner.rollback_last_batch().await {
        Ok(result) => {
            if result.rolled_back_count == 0 {
                println!("✓ No migrations to rollback.");
            } else {
                println!("✓ Rolled back {} migration(s) successfully:", result.rolled_back_count);
                for migration_id in &result.rolled_back_migrations {
                    println!("  - {}", migration_id);
                }
                println!("  Execution time: {}ms", result.execution_time_ms);
            }
            Ok(())
        }
        Err(e) => Err(ElifError::Database { message: format!("Rollback failed: {}", e) })
    }
}

pub async fn status() -> Result<(), ElifError> {
    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());
    
    // Create migration manager and runner
    let manager = MigrationManager::new();
    let runner = MigrationRunner::from_url(manager, &database_url).await
        .map_err(|e| ElifError::Database { message: format!("Failed to create migration runner: {}", e) })?;
    
    println!("Migration Status:");
    println!("================");
    
    match runner.get_migration_status().await {
        Ok(status_list) => {
            if status_list.is_empty() {
                println!("No migrations found");
            } else {
                for (migration, is_applied) in &status_list {
                    let status_icon = if *is_applied { "✅" } else { "⏳" };
                    let status_text = if *is_applied { "Applied" } else { "Pending" };
                    
                    println!("  {} {} - {} ({})", 
                        status_icon, 
                        migration.id, 
                        migration.name,
                        status_text
                    );
                    
                    if *is_applied {
                        continue;
                    }
                    
                    // Show a preview of pending migration
                    if !migration.up_sql.trim().is_empty() {
                        let preview = migration.up_sql
                            .lines()
                            .take(2)
                            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with("--"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        if !preview.is_empty() {
                            println!("     Preview: {}", 
                                if preview.len() > 60 { 
                                    format!("{}...", &preview[..60]) 
                                } else { 
                                    preview 
                                }
                            );
                        }
                    }
                }
                
                let applied_count = status_list.iter().filter(|(_, applied)| *applied).count();
                let pending_count = status_list.len() - applied_count;
                
                println!();
                println!("Summary: {} applied, {} pending", applied_count, pending_count);
            }
            Ok(())
        }
        Err(e) => Err(ElifError::Database { message: format!("Failed to get migration status: {}", e) })
    }
}