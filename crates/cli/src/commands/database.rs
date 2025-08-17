use elif_core::ElifError;
use elif_orm::{SeederManager, Environment, factory_registry_mut};
use sqlx::postgres::PgPoolOptions;
use url::Url;

pub async fn seed(env: Option<String>, force: bool, verbose: bool) -> Result<(), ElifError> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://elif:elif@localhost:5432/elif_dev".to_string());

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| ElifError::Database(format!("Failed to connect to database: {}", e)))?;

    // Determine environment
    let environment = if let Some(env_str) = env {
        Environment::from_str(&env_str)
    } else {
        SeederManager::current_environment()
    };

    if verbose {
        println!("🌱 Database Seeding");
        println!("==================");
        println!("Environment: {}", environment.as_str());
        println!("Database URL: {}", mask_database_url(&database_url));
        println!();
    }

    // Create seeder manager
    let seeder_manager = SeederManager::new();

    // Run seeders based on environment
    match environment {
        Environment::Production if !force => {
            return Err(ElifError::Database(
                "Cannot run seeders in production environment without --force flag".to_string()
            ));
        }
        Environment::Production => {
            println!("⚠️  WARNING: Running seeders in PRODUCTION environment!");
            println!("This operation will modify production data.");
            println!();
            
            if verbose {
                println!("🔄 Running production seeders (forced)...");
            }
            
            seeder_manager.run_production_force(&pool).await
                .map_err(|e| ElifError::Database(format!("Production seeding failed: {}", e)))?;
        }
        _ => {
            if verbose {
                println!("🔄 Running seeders for {} environment...", environment.as_str());
            }
            
            seeder_manager.run_for_environment(&pool, &environment).await
                .map_err(|e| ElifError::Database(format!("Seeding failed: {}", e)))?;
        }
    }

    println!("✅ Database seeding completed successfully");
    Ok(())
}

pub async fn factory_status() -> Result<(), ElifError> {
    println!("🏭 Factory System Status");
    println!("========================");
    
    // Get factory registry information
    let registry = factory_registry_mut();
    let factory_count = registry.factory_count();
    
    println!("Registered Factories: {}", factory_count);
    
    if factory_count == 0 {
        println!("⚠️  No factories are currently registered.");
        println!("   Consider creating model factories in your application.");
    } else {
        println!("✅ Factory system is operational");
    }
    
    // Show factory configuration
    println!();
    println!("Factory Configuration:");
    let config = elif_orm::factory_config();
    println!("  - Validate models: {}", config.validate_models);
    println!("  - Use transactions: {}", config.use_transactions);
    println!("  - Max batch size: {}", config.max_batch_size);
    println!("  - Realistic timestamps: {}", config.realistic_timestamps);
    println!("  - Seed: {:?}", config.seed);

    Ok(())
}

pub async fn seed_status() -> Result<(), ElifError> {
    println!("🌱 Seeding Status");
    println!("=================");
    
    let current_env = SeederManager::current_environment();
    println!("Current Environment: {}", current_env.as_str());
    println!("Environment Safety Check: {}", 
        if current_env.is_safe_for_seeding() { "✅ Safe" } else { "⚠️  Unsafe (requires --force)" }
    );
    
    // Environment variable checks
    println!();
    println!("Environment Variables:");
    
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        println!("  DATABASE_URL: ✅ {}", mask_database_url(&database_url));
    } else {
        println!("  DATABASE_URL: ⚠️  Not set (using default)");
    }
    
    for env_var in ["ELIF_ENV", "ENV", "ENVIRONMENT"] {
        if let Ok(value) = std::env::var(env_var) {
            println!("  {}: ✅ {}", env_var, value);
        } else {
            println!("  {}: ❌ Not set", env_var);
        }
    }
    
    Ok(())
}

pub async fn factory_test(count: usize) -> Result<(), ElifError> {
    println!("🧪 Testing Factory System");
    println!("=========================");
    
    // Test fake data generation
    println!("Testing fake data generators:");
    
    for i in 0..count.min(5) {
        println!("  Sample #{}: ", i + 1);
        println!("    Name: {}", elif_orm::fake_name());
        println!("    Email: {}", elif_orm::fake_email());
        println!("    Company: {}", elif_orm::fake_company());
        println!("    Phone: {}", elif_orm::fake_phone());
        println!("    Address: {}", elif_orm::fake_address());
    }
    
    println!();
    println!("✅ Fake data generation is working correctly");
    Ok(())
}

fn mask_database_url(url_str: &str) -> String {
    if let Ok(mut url) = Url::parse(url_str) {
        if url.password().is_some() {
            // The unwrap is safe because we've just checked that there is a password.
            url.set_password(Some("****")).unwrap();
        }
        url.to_string()
    } else {
        // Fallback for invalid URLs, though this should ideally not happen with valid DATABASE_URL.
        url_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_database_url() {
        let url = "postgresql://user:password@localhost:5432/database";
        let masked = mask_database_url(url);
        assert_eq!(masked, "postgresql://user:****@localhost:5432/database");
        
        let url_no_password = "postgresql://localhost:5432/database";
        let masked_no_pw = mask_database_url(url_no_password);
        assert_eq!(masked_no_pw, "postgresql://localhost:5432/database");

        let url_user_only = "postgresql://user@localhost:5432/database";
        let masked_user_only = mask_database_url(url_user_only);
        assert_eq!(masked_user_only, "postgresql://user@localhost:5432/database");

        let url_invalid = "not a valid url";
        let masked_invalid = mask_database_url(url_invalid);
        assert_eq!(masked_invalid, "not a valid url");
    }
}