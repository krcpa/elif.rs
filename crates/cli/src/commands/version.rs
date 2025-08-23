use elif_core::ElifError;

pub async fn create(version: &str, description: Option<&str>) -> Result<(), ElifError> {
    println!("📦 Creating API version: {}", version);
    if let Some(desc) = description {
        println!("   Description: {}", desc);
    }
    
    // Delegate to existing implementation for now
    crate::commands::api_version::create_version(version, description.map(|s| s.to_string())).await
}

pub async fn deprecate(version: &str, message: Option<&str>, sunset_date: Option<&str>) -> Result<(), ElifError> {
    println!("⚠️ Deprecating API version: {}", version);
    
    // Delegate to existing implementation for now
    crate::commands::api_version::deprecate_version(version, message.map(|s| s.to_string()), sunset_date.map(|s| s.to_string())).await
}

pub async fn list() -> Result<(), ElifError> {
    // Delegate to existing implementation for now
    crate::commands::api_version::list_versions().await
}

pub async fn migrate(from: &str, to: &str) -> Result<(), ElifError> {
    // Delegate to existing implementation for now
    crate::commands::api_version::generate_migration_guide(from, to).await
}

pub async fn validate() -> Result<(), ElifError> {
    // Delegate to existing implementation for now
    crate::commands::api_version::validate_versions().await
}