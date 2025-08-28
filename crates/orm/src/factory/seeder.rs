//! Database seeding system with environment controls

use super::Factory;
use crate::error::{OrmError, OrmResult};
use crate::model::Model;

/// Environment types for seeding control
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
    Custom(String),
}

impl Environment {
    /// Parse environment from string
    pub fn from_str(env: &str) -> Self {
        match env.to_lowercase().as_str() {
            "development" | "dev" => Environment::Development,
            "testing" | "test" => Environment::Testing,
            "staging" | "stage" => Environment::Staging,
            "production" | "prod" => Environment::Production,
            custom => Environment::Custom(custom.to_string()),
        }
    }

    /// Get environment name as string
    pub fn as_str(&self) -> &str {
        match self {
            Environment::Development => "development",
            Environment::Testing => "testing",
            Environment::Staging => "staging",
            Environment::Production => "production",
            Environment::Custom(name) => name,
        }
    }

    /// Check if this is a safe environment for seeding
    pub fn is_safe_for_seeding(&self) -> bool {
        match self {
            Environment::Development | Environment::Testing => true,
            Environment::Staging => true,     // Usually safe
            Environment::Production => false, // Requires explicit opt-in
            Environment::Custom(_) => false,  // Requires explicit opt-in
        }
    }
}

/// Seeder trait for implementing database seeders
#[async_trait::async_trait]
pub trait Seeder: Send + Sync {
    /// Get the seeder name for logging and tracking
    fn name(&self) -> &str;

    /// Get environments where this seeder should run
    fn environments(&self) -> Vec<Environment> {
        vec![Environment::Development, Environment::Testing]
    }

    /// Check if this seeder should run in the given environment
    fn should_run(&self, env: &Environment) -> bool {
        self.environments().contains(env)
    }

    /// Run the seeder
    async fn run(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()>;

    /// Optional: Clean up data created by this seeder
    async fn rollback(&self, _pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Get seeder priority (lower numbers run first)
    fn priority(&self) -> i32 {
        100
    }

    /// Check dependencies (other seeders that must run first)
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }
}

/// Factory-based seeder for easy model creation
pub struct FactorySeeder<T: Model, F: Factory<T>> {
    name: String,
    factory: F,
    count: usize,
    environments: Vec<Environment>,
    priority: i32,
    dependencies: Vec<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Model, F: Factory<T>> FactorySeeder<T, F> {
    pub fn new(name: impl Into<String>, factory: F, count: usize) -> Self {
        Self {
            name: name.into(),
            factory,
            count,
            environments: vec![Environment::Development, Environment::Testing],
            priority: 100,
            dependencies: vec![],
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn environments(mut self, envs: Vec<Environment>) -> Self {
        self.environments = envs;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn depends_on(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[async_trait::async_trait]
impl<T: Model + Send + Sync, F: Factory<T> + Send + Sync> Seeder for FactorySeeder<T, F> {
    fn name(&self) -> &str {
        &self.name
    }

    fn environments(&self) -> Vec<Environment> {
        self.environments.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    async fn run(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        tracing::info!(
            "Running seeder: {} (creating {} records)",
            self.name,
            self.count
        );

        let models = self.factory.create_many(pool, self.count).await?;

        tracing::info!(
            "Seeder {} completed: created {} records",
            self.name,
            models.len()
        );
        Ok(())
    }

    async fn rollback(&self, _pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        tracing::info!("Rolling back seeder: {}", self.name);

        // TODO: Implement rollback logic
        // This would require tracking created records or using truncation

        tracing::warn!("Rollback not yet implemented for factory seeders");
        Ok(())
    }
}

/// Seeder manager for running multiple seeders
#[derive(Default)]
pub struct SeederManager {
    seeders: Vec<Box<dyn Seeder>>,
}

impl SeederManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a seeder to the manager
    pub fn add<S: Seeder + 'static>(mut self, seeder: S) -> Self {
        self.seeders.push(Box::new(seeder));
        self
    }

    /// Add a factory seeder
    pub fn add_factory<T, F>(self, name: impl Into<String>, factory: F, count: usize) -> Self
    where
        T: Model + Send + Sync + 'static,
        F: Factory<T> + Send + Sync + 'static,
    {
        let seeder = FactorySeeder::new(name, factory, count);
        self.add(seeder)
    }

    /// Run all seeders for the given environment
    pub async fn run_for_environment(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        env: &Environment,
    ) -> OrmResult<()> {
        // Filter seeders for this environment
        let mut applicable_seeders: Vec<&Box<dyn Seeder>> = self
            .seeders
            .iter()
            .filter(|seeder| seeder.should_run(env))
            .collect();

        // Sort by priority (lower numbers first)
        applicable_seeders.sort_by_key(|seeder| seeder.priority());

        tracing::info!(
            "Running {} seeders for environment: {}",
            applicable_seeders.len(),
            env.as_str()
        );

        // Check environment safety
        if !env.is_safe_for_seeding() {
            return Err(OrmError::Validation(format!(
                "Environment '{}' is not safe for automatic seeding. Use explicit opt-in.",
                env.as_str()
            )));
        }

        // Resolve dependencies with topological sorting
        let ordered_seeders = self.resolve_dependencies(applicable_seeders)?;

        for seeder in ordered_seeders {
            tracing::info!("Running seeder: {}", seeder.name());
            seeder.run(pool).await?;
        }

        tracing::info!("All seeders completed successfully");
        Ok(())
    }

    /// Run seeders in development environment
    pub async fn run_development(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        self.run_for_environment(pool, &Environment::Development)
            .await
    }

    /// Run seeders in testing environment
    pub async fn run_testing(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        self.run_for_environment(pool, &Environment::Testing).await
    }

    /// Force run seeders in production (use with caution)
    pub async fn run_production_force(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        let applicable_seeders: Vec<&Box<dyn Seeder>> = self
            .seeders
            .iter()
            .filter(|seeder| seeder.environments().contains(&Environment::Production))
            .collect();

        // Resolve dependencies even in production for safety
        let ordered_seeders = self.resolve_dependencies(applicable_seeders)?;

        tracing::warn!(
            "Force running {} seeders in PRODUCTION environment with dependency resolution",
            ordered_seeders.len()
        );

        for seeder in ordered_seeders {
            tracing::warn!("Running production seeder: {}", seeder.name());
            seeder.run(pool).await?;
        }

        Ok(())
    }

    /// Resolve seeder dependencies using topological sorting
    fn resolve_dependencies<'a>(
        &self,
        seeders: Vec<&'a Box<dyn Seeder>>,
    ) -> OrmResult<Vec<&'a Box<dyn Seeder>>> {
        use std::collections::{HashMap, HashSet, VecDeque};

        // Build a map of seeder name to seeder reference
        let mut seeder_map: HashMap<String, &'a Box<dyn Seeder>> = HashMap::new();
        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize maps
        for seeder in &seeders {
            let name = seeder.name().to_string();
            seeder_map.insert(name.clone(), seeder);
            dependencies.insert(name.clone(), seeder.dependencies());
            in_degree.insert(name.clone(), 0);
        }

        // Calculate in-degrees (number of dependencies pointing to each seeder)
        for (seeder_name, deps) in &dependencies {
            for dep in deps {
                // Validate that dependency exists
                if !seeder_map.contains_key(dep) {
                    return Err(OrmError::Validation(format!(
                        "Seeder '{}' depends on '{}', but '{}' was not found",
                        seeder_name, dep, dep
                    )));
                }
                
                // Increment in-degree for the dependent seeder
                if let Some(degree) = in_degree.get_mut(seeder_name) {
                    *degree += 1;
                }
            }
        }

        // Kahn's algorithm for topological sorting
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut result = Vec::new();
        let mut processed = HashSet::new();

        // Start with seeders that have no dependencies (in-degree = 0)
        for (name, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(name.clone());
            }
        }

        // Process queue
        while let Some(current) = queue.pop_front() {
            if processed.contains(&current) {
                continue;
            }
            
            processed.insert(current.clone());
            
            // Add current seeder to result
            if let Some(seeder) = seeder_map.get(&current) {
                result.push(*seeder);
            }

            // Update in-degrees for seeders that depend on current seeder
            for (dependent, deps) in &dependencies {
                if deps.contains(&current) {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        if *degree > 0 {
                            *degree -= 1;
                            if *degree == 0 && !processed.contains(dependent) {
                                queue.push_back(dependent.clone());
                            }
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        if result.len() != seeders.len() {
            let unprocessed: Vec<String> = seeders
                .iter()
                .map(|s| s.name().to_string())
                .filter(|name| !processed.contains(name))
                .collect();
            
            return Err(OrmError::Validation(format!(
                "Circular dependency detected in seeders: {}",
                unprocessed.join(", ")
            )));
        }

        // Secondary sort by priority for seeders at the same dependency level
        result.sort_by_key(|seeder| seeder.priority());

        Ok(result)
    }

    /// Get current environment from environment variable
    pub fn current_environment() -> Environment {
        std::env::var("ELIF_ENV")
            .or_else(|_| std::env::var("ENV"))
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .map(|env| Environment::from_str(&env))
            .unwrap_or(Environment::Development)
    }

    /// Run seeders for current environment
    pub async fn run(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        let env = Self::current_environment();
        self.run_for_environment(pool, &env).await
    }
}

/// Custom seeder implementation for complex seeding logic
pub struct CustomSeeder {
    name: String,
    environments: Vec<Environment>,
    priority: i32,
    dependencies: Vec<String>,
    run_fn: Box<
        dyn Fn(
                &sqlx::Pool<sqlx::Postgres>,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = OrmResult<()>> + Send>>
            + Send
            + Sync,
    >,
}

impl CustomSeeder {
    pub fn new<F, Fut>(name: impl Into<String>, run_fn: F) -> Self
    where
        F: Fn(&sqlx::Pool<sqlx::Postgres>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = OrmResult<()>> + Send + 'static,
    {
        Self {
            name: name.into(),
            environments: vec![Environment::Development, Environment::Testing],
            priority: 100,
            dependencies: vec![],
            run_fn: Box::new(move |pool| Box::pin(run_fn(pool))),
        }
    }

    pub fn environments(mut self, envs: Vec<Environment>) -> Self {
        self.environments = envs;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn depends_on(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[async_trait::async_trait]
impl Seeder for CustomSeeder {
    fn name(&self) -> &str {
        &self.name
    }

    fn environments(&self) -> Vec<Environment> {
        self.environments.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    async fn run(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<()> {
        (self.run_fn)(pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_parsing() {
        assert_eq!(
            Environment::from_str("development"),
            Environment::Development
        );
        assert_eq!(Environment::from_str("dev"), Environment::Development);
        assert_eq!(Environment::from_str("testing"), Environment::Testing);
        assert_eq!(Environment::from_str("test"), Environment::Testing);
        assert_eq!(Environment::from_str("production"), Environment::Production);
        assert_eq!(Environment::from_str("prod"), Environment::Production);
        assert_eq!(
            Environment::from_str("custom"),
            Environment::Custom("custom".to_string())
        );
    }

    #[test]
    fn test_environment_safety() {
        assert!(Environment::Development.is_safe_for_seeding());
        assert!(Environment::Testing.is_safe_for_seeding());
        assert!(Environment::Staging.is_safe_for_seeding());
        assert!(!Environment::Production.is_safe_for_seeding());
        assert!(!Environment::Custom("custom".to_string()).is_safe_for_seeding());
    }

    #[test]
    fn test_seeder_manager_creation() {
        let manager = SeederManager::new();
        assert_eq!(manager.seeders.len(), 0);
    }

    #[test]
    fn test_current_environment() {
        // This will use the default since we're not setting env vars in tests
        let env = SeederManager::current_environment();
        assert_eq!(env, Environment::Development);
    }

    #[tokio::test]
    async fn test_custom_seeder() {
        let seeder = CustomSeeder::new("test_seeder", |_pool| async { Ok(()) });

        assert_eq!(seeder.name(), "test_seeder");
        assert_eq!(Seeder::priority(&seeder), 100);
        assert!(seeder.should_run(&Environment::Development));
    }
}
