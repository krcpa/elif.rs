use elif_core::ElifError;
use elif_orm::{
    database::{create_database_pool, DatabaseServiceProvider, ManagedPool},
    factory::{
        seeder::{Environment, SeederManager},
    },
    migration_runner::{MigrationRollback, MigrationRunner},
    MigrationManager,
    backends::DatabasePoolConfig,
};
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use url::Url;

/// Database manager for comprehensive lifecycle operations
pub struct DatabaseManager {
    database_url: String,
    environment: Environment,
    #[allow(dead_code)]
    verbose: bool,
}

impl DatabaseManager {
    /// Create a new database manager with environment detection
    pub fn new(env: Option<&str>, verbose: bool) -> Self {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());
        
        let environment = env
            .map(|e| Environment::from_str(e))
            .unwrap_or_else(|| SeederManager::current_environment());

        Self {
            database_url,
            environment,
            verbose,
        }
    }

    /// Create a managed pool with health monitoring
    async fn create_managed_pool(&self) -> Result<Arc<ManagedPool>, ElifError> {
        let provider = DatabaseServiceProvider::new(self.database_url.clone())
            .with_config(DatabasePoolConfig::default());
        
        let managed_pool = provider.create_managed_pool().await
            .map_err(|e| ElifError::Database {
                message: format!("Failed to create database pool: {}", e),
            })?;
        
        Ok(Arc::new(managed_pool))
    }

    /// Get database connection information for display
    fn connection_info(&self) -> String {
        // Safely mask password in URL for display using proper URL parsing
        let masked_url = Url::parse(&self.database_url)
            .map(|mut url| {
                if url.password().is_some() {
                    let _ = url.set_password(Some("***"));
                }
                url.to_string()
            })
            .unwrap_or_else(|_| "postgresql://***".to_string());
        masked_url
    }

    /// Check if running in a safe environment
    fn is_safe_environment(&self) -> bool {
        self.environment.is_safe_for_seeding()
    }

    /// Prompt for confirmation on destructive operations
    async fn confirm_destructive_operation(&self, operation: &str, target: &str) -> Result<bool, ElifError> {
        if !self.is_safe_environment() {
            println!("⚠️  WARNING: Running {} on {} in {} environment!", operation, target, self.environment.as_str());
            println!("   This operation will permanently delete data.");
        }

        print!("   Are you sure you want to continue? (y/N): ");
        tokio::io::stdout().flush().await
            .map_err(|e| ElifError::Io(e))?;

        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();
        
        let response = lines.next_line().await
            .map_err(|e| ElifError::Io(e))?
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        Ok(matches!(response.as_str(), "y" | "yes"))
    }
}

/// Database setup and initialization
pub async fn setup(env: Option<&str>, verbose: bool) -> Result<(), ElifError> {
    let manager = DatabaseManager::new(env, verbose);
    
    println!("🗄️ Database Setup & Health Check");
    println!("Connection: {}", manager.connection_info());
    println!("Environment: {}", manager.environment.as_str());
    println!();

    // Test database connection
    let pool = manager.create_managed_pool().await?;
    
    match pool.health_check().await {
        Ok(duration) => {
            println!("✅ Connection: OK ({:?})", duration);
        }
        Err(e) => {
            println!("❌ Connection: FAILED");
            return Err(ElifError::Database {
                message: format!("Database connection failed: {}", e),
            });
        }
    }

    // Check pool statistics
    let stats = pool.stats();
    println!("✅ Pool Status: {} total, {} active, {} idle", 
        stats.total_connections, stats.active_connections, stats.idle_connections);

    // Check migration status
    let migration_manager = MigrationManager::new();
    let migration_runner = MigrationRunner::from_url(migration_manager, &manager.database_url).await
        .map_err(|e| ElifError::Database {
            message: format!("Failed to create migration runner: {}", e),
        })?;

    match migration_runner.get_migration_status().await {
        Ok(status_list) => {
            let applied_count = status_list.iter().filter(|(_, applied)| *applied).count();
            let pending_count = status_list.len() - applied_count;
            
            if pending_count == 0 {
                println!("✅ Schema: Up to date ({} migrations applied)", applied_count);
            } else {
                println!("⚠️  Schema: {} pending migrations", pending_count);
                println!("   Run: elifrs migrate up");
            }
        }
        Err(_) => {
            println!("⚠️  Schema: Unable to check migration status");
        }
    }

    println!();
    println!("📊 Database setup completed successfully!");
    
    if verbose {
        println!();
        println!("💡 Available commands:");
        println!("   elifrs db status     - Check database health");
        println!("   elifrs db seed       - Run database seeders");
        println!("   elifrs db reset      - Reset database with migrations");
        println!("   elifrs db fresh      - Fresh database setup");
    }

    Ok(())
}

/// Database status and health reporting
pub async fn status(env: Option<&str>, verbose: bool) -> Result<(), ElifError> {
    let manager = DatabaseManager::new(env, verbose);
    
    println!("🗄️ Database Status Check");
    println!();

    // Create connection pool
    let pool = manager.create_managed_pool().await?;

    // Health check with detailed reporting
    let health_result = pool.detailed_health_check().await;
    
    match health_result {
        Ok(report) => {
            println!("✅ Connection: {}", manager.connection_info());
            println!("✅ Health Check: Passed ({:?})", report.check_duration);
            println!("✅ Pool Status: {} total, {} active, {} idle", 
                report.pool_size, report.active_connections, report.idle_connections);
            
            if report.total_acquires > 0 {
                println!("📊 Pool Stats: {} acquires, {:.1}% error rate", 
                    report.total_acquires, report.error_rate);
            }
        }
        Err(e) => {
            println!("❌ Connection: {}", manager.connection_info());
            println!("❌ Health Check: FAILED ({})", e);
            return Err(ElifError::Database {
                message: format!("Database health check failed: {}", e),
            });
        }
    }

    // Migration status
    let migration_manager = MigrationManager::new();
    let migration_runner = MigrationRunner::from_url(migration_manager, &manager.database_url).await
        .map_err(|e| ElifError::Database {
            message: format!("Failed to create migration runner: {}", e),
        })?;

    match migration_runner.get_migration_status().await {
        Ok(status_list) => {
            let applied_count = status_list.iter().filter(|(_, applied)| *applied).count();
            let pending_count = status_list.len() - applied_count;
            
            if pending_count == 0 {
                println!("✅ Schema Version: Up to date");
                println!("✅ Total Migrations: {} applied", applied_count);
            } else {
                println!("⚠️  Schema Version: Out of date");
                println!("📊 Migration Status: {} applied, {} pending", applied_count, pending_count);
            }
        }
        Err(e) => {
            println!("⚠️  Schema Version: Unable to determine ({})", e);
        }
    }

    // Database size and record estimates (basic implementation)
    if verbose {
        println!();
        println!("💡 Recommendations:");
        if let Ok(status_list) = migration_runner.get_migration_status().await {
            let pending_count = status_list.len() - status_list.iter().filter(|(_, applied)| *applied).count();
            if pending_count > 0 {
                println!("   • Run: elifrs migrate up");
            }
        }
        println!("   • Run: elifrs db analyze (performance insights)");
    }

    println!();
    Ok(())
}

/// Run database seeders with environment controls
pub async fn seed(env: Option<&str>, force: bool, verbose: bool) -> Result<(), ElifError> {
    let manager = DatabaseManager::new(env, verbose);
    
    println!("🌱 Running Database Seeders");
    println!("Environment: {}", manager.environment.as_str());
    println!();

    // Safety check for production
    if !manager.is_safe_environment() && !force {
        println!("❌ Environment '{}' requires explicit --force flag for seeding", manager.environment.as_str());
        println!("   Use: elifrs db seed --force");
        return Err(ElifError::Database {
            message: "Production seeding requires explicit confirmation".to_string(),
        });
    }

    // Create seeder manager (in real implementation, this would load from database/seeders)
    let seeder_manager = SeederManager::new();

    // For now, create a temporary sqlx pool for seeder compatibility
    let sqlx_pool = sqlx::Pool::<sqlx::Postgres>::connect(&manager.database_url).await
        .map_err(|e| ElifError::Database {
            message: format!("Failed to create PostgreSQL pool for seeding: {}", e),
        })?;

    if force && !manager.is_safe_environment() {
        println!("⚠️  Force running seeders in {} environment", manager.environment.as_str());
        
        match seeder_manager.run_production_force(&sqlx_pool).await {
            Ok(()) => {
                println!("✅ Production seeders completed successfully");
            }
            Err(e) => {
                return Err(ElifError::Database {
                    message: format!("Seeding failed: {}", e),
                });
            }
        }
    } else {
        // Run seeders for current environment
        match seeder_manager.run_for_environment(&sqlx_pool, &manager.environment).await {
            Ok(()) => {
                println!("🎉 Database seeding completed successfully!");
            }
            Err(e) => {
                return Err(ElifError::Database {
                    message: format!("Seeding failed: {}", e),
                });
            }
        }
    }

    if verbose {
        println!();
        println!("💡 Seeder files should be placed in: database/seeders/");
        println!("   Create seeders with: elifrs add seeder <name>");
    }

    Ok(())
}

/// Reset database with fresh migrations and optional seeding
pub async fn reset(with_seeds: bool, env: Option<&str>, force: bool) -> Result<(), ElifError> {
    let manager = DatabaseManager::new(env, false);
    
    println!("🔄 Resetting Database");
    println!("Environment: {}", manager.environment.as_str());
    
    // Confirmation for destructive operation
    if !force && !manager.confirm_destructive_operation("database reset", "all data").await? {
        println!("Operation cancelled");
        return Ok(());
    }

    println!();
    println!("Step 1/3: Rolling back all migrations...");
    
    // Rollback all migrations
    let migration_manager = MigrationManager::new();
    let migration_runner = MigrationRunner::from_url(migration_manager, &manager.database_url).await
        .map_err(|e| ElifError::Database {
            message: format!("Failed to create migration runner: {}", e),
        })?;

    // Get current migration status and rollback all
    let status_list = migration_runner.get_migration_status().await
        .map_err(|e| ElifError::Database {
            message: format!("Failed to get migration status: {}", e),
        })?;

    let applied_migrations: Vec<_> = status_list
        .iter()
        .filter(|(_, applied)| *applied)
        .collect();

    if !applied_migrations.is_empty() {
        println!("   Rolling back {} migrations...", applied_migrations.len());
        
        // Roll back in batches
        for _ in 0..applied_migrations.len() {
            match migration_runner.rollback_last_batch().await {
                Ok(result) => {
                    if result.rolled_back_count > 0 {
                        println!("   ✅ Rolled back {} migration(s)", result.rolled_back_count);
                    } else {
                        break; // No more to rollback
                    }
                }
                Err(e) => {
                    return Err(ElifError::Database {
                        message: format!("Migration rollback failed: {}", e),
                    });
                }
            }
        }
    } else {
        println!("   No migrations to rollback");
    }

    println!("Step 2/3: Running fresh migrations...");
    
    // Run migrations
    match migration_runner.run_migrations().await {
        Ok(result) => {
            if result.applied_count > 0 {
                println!("   ✅ Applied {} migration(s)", result.applied_count);
            } else {
                println!("   No migrations to apply");
            }
        }
        Err(e) => {
            return Err(ElifError::Database {
                message: format!("Migration failed: {}", e),
            });
        }
    }

    // Run seeders if requested
    if with_seeds {
        println!("Step 3/3: Running seeders...");
        seed(Some(manager.environment.as_str()), force, false).await?;
    } else {
        println!("Step 3/3: Skipping seeders");
    }

    println!();
    println!("🎉 Database reset completed successfully!");
    
    Ok(())
}

/// Fresh database setup (equivalent to reset but more explicit)
pub async fn fresh(env: Option<&str>, with_seeds: bool) -> Result<(), ElifError> {
    println!("🆕 Fresh Database Setup");
    println!();
    
    // Fresh is the same as reset with force
    reset(with_seeds, env, true).await
}

/// Database backup functionality
pub async fn backup(path: Option<&str>, compress: bool) -> Result<(), ElifError> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());
    
    // Generate backup filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let extension = if compress { "sql.gz" } else { "sql" };
    let backup_file = path
        .map(|p| p.to_string())
        .unwrap_or_else(|| format!("backup_elif_db_{}.{}", timestamp, extension));

    println!("💾 Creating Database Backup");
    println!("Target: {}", backup_file);
    
    if compress {
        println!("Compression: Enabled");
    }
    
    println!();
    println!("⚠️ Database backup implementation requires pg_dump integration");
    println!("   This feature will be completed in the next iteration");
    
    // TODO: Implement actual backup using pg_dump
    // For now, provide guidance
    println!("💡 Manual backup command:");
    let masked_url = database_url.split('@').last().unwrap_or("localhost:5432/elif_dev");
    println!("   pg_dump postgresql://USER:PASS@{} > {}", masked_url, backup_file);
    
    Ok(())
}

/// Database restore functionality  
pub async fn restore(backup_file: &str) -> Result<(), ElifError> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());

    println!("📤 Restoring Database");
    println!("Source: {}", backup_file);
    println!();

    // Check if backup file exists
    if !std::path::Path::new(backup_file).exists() {
        return Err(ElifError::Database {
            message: format!("Backup file not found: {}", backup_file),
        });
    }

    println!("⚠️ Database restore implementation requires psql integration");
    println!("   This feature will be completed in the next iteration");
    
    // TODO: Implement actual restore using psql
    // For now, provide guidance
    println!("💡 Manual restore command:");
    let masked_url = database_url.split('@').last().unwrap_or("localhost:5432/elif_dev");
    println!("   psql postgresql://USER:PASS@{} < {}", masked_url, backup_file);
    
    Ok(())
}

/// Database analysis and performance insights
pub async fn analyze() -> Result<(), ElifError> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());

    println!("📊 Database Performance Analysis");
    println!();

    // Create connection pool for analysis
    let _pool = create_database_pool(&database_url).await
        .map_err(|e| ElifError::Database {
            message: format!("Failed to create database pool: {}", e),
        })?;

    println!("⚠️ Database analysis implementation coming in next iteration");
    println!();
    println!("💡 Planned analysis features:");
    println!("   • Table sizes and record counts");
    println!("   • Missing indexes detection");
    println!("   • Query performance insights");
    println!("   • Storage optimization recommendations");
    
    Ok(())
}

pub async fn create(name: &str, env: &str) -> Result<(), ElifError> {
    println!("🗄️ Creating database: {} (env: {})", name, env);
    println!("⚠️ Database creation implementation requires admin database connection");
    println!("   This feature will be completed in the next iteration");
    Ok(())
}

pub async fn drop(name: Option<&str>, env: &str, force: bool) -> Result<(), ElifError> {
    println!(
        "🗑️ Dropping database: {:?} (env: {}, force: {})",
        name, env, force
    );
    println!("⚠️ Database dropping implementation requires admin database connection");
    println!("   This feature will be completed in the next iteration");
    Ok(())
}
