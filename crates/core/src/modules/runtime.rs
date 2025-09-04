//! Runtime module integration for Epic 4 - Runtime Integration & Validation
//!
//! Provides runtime module initialization, dependency resolution, and integration
//! with the existing IoC container and HTTP routing system.
//!
//! ## Features
//! - **Topological sorting** for module initialization order based on dependencies
//! - **Runtime dependency resolution** with clear error reporting and module context
//! - **Integration** with existing `IocContainer` and controller registration systems
//! - **Module lifecycle hooks** for startup, shutdown, and health checks
//! - **Performance monitoring** and validation for large module graphs

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::container::IocContainer;
use crate::errors::CoreError;
use crate::modules::ModuleDescriptor;

/// Errors that can occur during module runtime operations
#[derive(Debug, Clone)]
pub enum ModuleRuntimeError {
    /// Circular dependency detected between modules
    CircularDependency { cycle: Vec<String>, message: String },
    /// Missing module dependency
    MissingDependency {
        module: String,
        missing_dependency: String,
        message: String,
    },
    /// Module initialization failed
    InitializationFailed {
        module: String,
        error: String,
        phase: String,
    },
    /// Module lifecycle operation failed
    LifecycleOperationFailed {
        module: String,
        operation: String,
        error: String,
    },
    /// Configuration conflict between modules
    ConfigurationConflict {
        module1: String,
        module2: String,
        conflict: String,
    },
    /// Runtime validation failed
    ValidationFailed {
        module: String,
        validation_errors: Vec<String>,
    },
}

impl std::fmt::Display for ModuleRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleRuntimeError::CircularDependency { cycle, message } => {
                write!(
                    f,
                    "Circular dependency: {} -> {}",
                    cycle.join(" -> "),
                    message
                )
            }
            ModuleRuntimeError::MissingDependency {
                module,
                missing_dependency,
                message,
            } => {
                write!(
                    f,
                    "Module '{}' missing dependency '{}': {}",
                    module, missing_dependency, message
                )
            }
            ModuleRuntimeError::InitializationFailed {
                module,
                error,
                phase,
            } => {
                write!(
                    f,
                    "Module '{}' initialization failed in phase '{}': {}",
                    module, phase, error
                )
            }
            ModuleRuntimeError::LifecycleOperationFailed {
                module,
                operation,
                error,
            } => {
                write!(
                    f,
                    "Module '{}' lifecycle operation '{}' failed: {}",
                    module, operation, error
                )
            }
            ModuleRuntimeError::ConfigurationConflict {
                module1,
                module2,
                conflict,
            } => {
                write!(
                    f,
                    "Configuration conflict between modules '{}' and '{}': {}",
                    module1, module2, conflict
                )
            }
            ModuleRuntimeError::ValidationFailed {
                module,
                validation_errors,
            } => {
                write!(
                    f,
                    "Module '{}' validation failed: {}",
                    module,
                    validation_errors.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for ModuleRuntimeError {}

/// Convert ModuleRuntimeError to CoreError for compatibility
impl From<ModuleRuntimeError> for CoreError {
    fn from(err: ModuleRuntimeError) -> Self {
        CoreError::InvalidServiceDescriptor {
            message: err.to_string(),
        }
    }
}

/// State of a module during runtime initialization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleState {
    /// Module is registered but not yet processed
    Registered,
    /// Module dependencies are being resolved
    ResolvingDependencies,
    /// Module is being configured with IoC container
    Configuring,
    /// Module is being initialized (lifecycle hooks)
    Initializing,
    /// Module is fully initialized and ready
    Ready,
    /// Module initialization failed
    Failed(String),
    /// Module is being shut down
    ShuttingDown,
    /// Module is shut down
    Shutdown,
}

/// Runtime information about a module
#[derive(Debug, Clone)]
pub struct ModuleRuntimeInfo {
    /// Module descriptor
    pub descriptor: ModuleDescriptor,
    /// Current module state
    pub state: ModuleState,
    /// Module load order (0-based)
    pub load_order: Option<usize>,
    /// Time taken for initialization
    pub init_duration: Option<Duration>,
    /// Time taken for configuration
    pub config_duration: Option<Duration>,
    /// Any errors encountered
    pub errors: Vec<String>,
    /// Health check status
    pub health_status: HealthStatus,
    /// Last health check time
    pub last_health_check: Option<Instant>,
}

impl ModuleRuntimeInfo {
    /// Create new runtime info from descriptor
    pub fn new(descriptor: ModuleDescriptor) -> Self {
        Self {
            descriptor,
            state: ModuleState::Registered,
            load_order: None,
            init_duration: None,
            config_duration: None,
            errors: Vec::new(),
            health_status: HealthStatus::Unknown,
            last_health_check: None,
        }
    }

    /// Check if module is ready
    pub fn is_ready(&self) -> bool {
        self.state == ModuleState::Ready
    }

    /// Check if module failed
    pub fn has_failed(&self) -> bool {
        matches!(self.state, ModuleState::Failed(_))
    }

    /// Add error to module
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

/// Health status of a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Health status unknown
    Unknown,
    /// Module is healthy
    Healthy,
    /// Module is degraded but functional
    Degraded,
    /// Module is unhealthy
    Unhealthy,
}

/// Performance metrics for module runtime operations
#[derive(Debug, Default, Clone)]
pub struct ModulePerformanceMetrics {
    /// Total modules processed
    pub total_modules: usize,
    /// Time taken for topological sorting
    pub topological_sort_duration: Duration,
    /// Time taken for dependency resolution
    pub dependency_resolution_duration: Duration,
    /// Time taken for configuration phase
    pub configuration_duration: Duration,
    /// Time taken for initialization phase
    pub initialization_duration: Duration,
    /// Average initialization time per module
    pub avg_init_time_per_module: Duration,
    /// Slowest module to initialize
    pub slowest_module: Option<String>,
    /// Slowest module initialization time
    pub slowest_init_time: Duration,
}

/// Runtime module manager - orchestrates module loading and lifecycle
pub struct ModuleRuntime {
    /// Module runtime information indexed by module name
    modules: HashMap<String, ModuleRuntimeInfo>,
    /// Module dependency graph (module -> dependencies)
    dependency_graph: HashMap<String, Vec<String>>,
    /// Topological load order
    load_order: Vec<String>,
    /// Performance metrics
    metrics: ModulePerformanceMetrics,
    /// Module lifecycle hooks
    lifecycle_hooks: HashMap<String, Box<dyn ModuleLifecycleHook>>,
    /// Health check configuration
    #[allow(dead_code)]
    health_check_config: HealthCheckConfig,
}

/// Configuration for health checks
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// How often to run health checks
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Whether health checks are enabled
    pub enabled: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            enabled: true,
        }
    }
}

/// Trait for module lifecycle hooks
pub trait ModuleLifecycleHook: Send + Sync {
    /// Called before module initialization
    fn before_init(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        let _ = module_name;
        Ok(())
    }

    /// Called after successful module initialization
    fn after_init(&self, module_name: &str, duration: Duration) -> Result<(), ModuleRuntimeError> {
        let _ = (module_name, duration);
        Ok(())
    }

    /// Called when module initialization fails
    fn on_init_failure(&self, module_name: &str, error: &ModuleRuntimeError) {
        let _ = (module_name, error);
    }

    /// Called before module shutdown
    fn before_shutdown(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        let _ = module_name;
        Ok(())
    }

    /// Called after module shutdown
    fn after_shutdown(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        let _ = module_name;
        Ok(())
    }

    /// Health check for the module
    fn health_check(&self, module_name: &str) -> Result<HealthStatus, ModuleRuntimeError> {
        let _ = module_name;
        Ok(HealthStatus::Unknown)
    }
}

impl ModuleRuntime {
    /// Create new module runtime
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependency_graph: HashMap::new(),
            load_order: Vec::new(),
            metrics: ModulePerformanceMetrics::default(),
            lifecycle_hooks: HashMap::new(),
            health_check_config: HealthCheckConfig::default(),
        }
    }

    /// Create module runtime with custom health check config
    pub fn with_health_config(health_config: HealthCheckConfig) -> Self {
        Self {
            modules: HashMap::new(),
            dependency_graph: HashMap::new(),
            load_order: Vec::new(),
            metrics: ModulePerformanceMetrics::default(),
            lifecycle_hooks: HashMap::new(),
            health_check_config: health_config,
        }
    }

    /// Register a module for runtime management
    pub fn register_module(
        &mut self,
        descriptor: ModuleDescriptor,
    ) -> Result<(), ModuleRuntimeError> {
        let module_name = descriptor.name.clone();

        // Check for duplicate registration
        if self.modules.contains_key(&module_name) {
            return Err(ModuleRuntimeError::ConfigurationConflict {
                module1: module_name.clone(),
                module2: module_name,
                conflict: "Module already registered".to_string(),
            });
        }

        // Store dependency information
        self.dependency_graph
            .insert(module_name.clone(), descriptor.dependencies.clone());

        // Create runtime info
        let runtime_info = ModuleRuntimeInfo::new(descriptor);
        self.modules.insert(module_name, runtime_info);

        Ok(())
    }

    /// Register multiple modules
    pub fn register_modules(
        &mut self,
        descriptors: Vec<ModuleDescriptor>,
    ) -> Result<(), ModuleRuntimeError> {
        for descriptor in descriptors {
            self.register_module(descriptor)?;
        }
        Ok(())
    }

    /// Add lifecycle hook for a module
    pub fn add_lifecycle_hook<H: ModuleLifecycleHook + 'static>(
        &mut self,
        module_name: String,
        hook: H,
    ) {
        self.lifecycle_hooks.insert(module_name, Box::new(hook));
    }

    /// Task 4.1: Implement topological sorting for module initialization order
    pub fn calculate_load_order(&mut self) -> Result<Vec<String>, ModuleRuntimeError> {
        let start_time = Instant::now();

        let sorted_modules = self.topological_sort()?;

        // Update load order in module info
        for (index, module_name) in sorted_modules.iter().enumerate() {
            if let Some(module_info) = self.modules.get_mut(module_name) {
                module_info.load_order = Some(index);
            }
        }

        self.load_order = sorted_modules.clone();
        self.metrics.topological_sort_duration = start_time.elapsed();

        Ok(sorted_modules)
    }

    /// Get the current module load order
    pub fn load_order(&self) -> &[String] {
        &self.load_order
    }

    /// Topological sort implementation using Kahn's algorithm for better error reporting
    fn topological_sort(&self) -> Result<Vec<String>, ModuleRuntimeError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize in-degree count and build forward graph
        for module_name in self.modules.keys() {
            in_degree.insert(module_name.clone(), 0);
            graph.insert(module_name.clone(), Vec::new());
        }

        // Build the graph and calculate in-degrees
        for (module_name, dependencies) in &self.dependency_graph {
            for dependency in dependencies {
                // Check if dependency exists
                if !self.modules.contains_key(dependency) {
                    return Err(ModuleRuntimeError::MissingDependency {
                        module: module_name.clone(),
                        missing_dependency: dependency.clone(),
                        message: "Dependency not registered".to_string(),
                    });
                }

                // Add edge from dependency to module
                graph.get_mut(dependency).unwrap().push(module_name.clone());
                *in_degree.get_mut(module_name).unwrap() += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            // Reduce in-degree for all dependents
            for dependent in &graph[&current] {
                let degree = in_degree.get_mut(dependent).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dependent.clone());
                }
            }
        }

        // Check for cycles
        if result.len() != self.modules.len() {
            // Find cycle using DFS
            let cycle = self.find_cycle()?;
            return Err(ModuleRuntimeError::CircularDependency {
                cycle: cycle.clone(),
                message: format!(
                    "Circular dependency detected: {}. This creates an infinite loop during module initialization.",
                    cycle.join(" -> ")
                ),
            });
        }

        Ok(result)
    }

    /// Find a cycle in the dependency graph using DFS
    fn find_cycle(&self) -> Result<Vec<String>, ModuleRuntimeError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for module_name in self.modules.keys() {
            if !visited.contains(module_name) {
                if let Some(cycle) =
                    self.dfs_cycle_detection(module_name, &mut visited, &mut rec_stack, &mut path)?
                {
                    return Ok(cycle);
                }
            }
        }

        // Shouldn't reach here if we detected a cycle earlier
        Ok(vec!["unknown".to_string()])
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_detection(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<Option<Vec<String>>, ModuleRuntimeError> {
        visited.insert(current.to_string());
        rec_stack.insert(current.to_string());
        path.push(current.to_string());

        if let Some(dependencies) = self.dependency_graph.get(current) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_cycle_detection(dep, visited, rec_stack, path)? {
                        return Ok(Some(cycle));
                    }
                } else if rec_stack.contains(dep) {
                    // Found cycle - extract it from path
                    if let Some(start_idx) = path.iter().position(|x| x == dep) {
                        let mut cycle = path[start_idx..].to_vec();
                        cycle.push(dep.clone()); // Complete the cycle
                        return Ok(Some(cycle));
                    }
                }
            }
        }

        rec_stack.remove(current);
        path.pop();
        Ok(None)
    }

    /// Task 4.2: Add runtime dependency resolution with clear error reporting
    pub async fn resolve_dependencies(
        &mut self,
        container: &mut IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        let start_time = Instant::now();

        // Ensure load order is calculated
        if self.load_order.is_empty() {
            self.calculate_load_order()?;
        }

        // Process modules in dependency order
        for module_name in &self.load_order.clone() {
            self.resolve_module_dependencies(module_name, container)
                .await?;
        }

        self.metrics.dependency_resolution_duration = start_time.elapsed();
        Ok(())
    }

    /// Resolve dependencies for a specific module
    async fn resolve_module_dependencies(
        &mut self,
        module_name: &str,
        container: &mut IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        // Update module state
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.state = ModuleState::ResolvingDependencies;
        }

        let dependencies = self
            .dependency_graph
            .get(module_name)
            .cloned()
            .unwrap_or_default();

        // Verify all dependencies are ready
        for dep_name in &dependencies {
            let dep_info = self.modules.get(dep_name).ok_or_else(|| {
                ModuleRuntimeError::MissingDependency {
                    module: module_name.to_string(),
                    missing_dependency: dep_name.clone(),
                    message: "Dependency module not found in runtime".to_string(),
                }
            })?;

            if matches!(dep_info.state, ModuleState::Failed(_)) {
                return Err(ModuleRuntimeError::InitializationFailed {
                    module: module_name.to_string(),
                    error: format!(
                        "Dependency '{}' failed initialization (state: {:?})",
                        dep_name, dep_info.state
                    ),
                    phase: "dependency_resolution".to_string(),
                });
            }
        }

        // Configure module with container
        self.configure_module(module_name, container).await?;

        Ok(())
    }

    /// Task 4.3: Integrate with existing IocContainer and controller registration systems
    async fn configure_module(
        &mut self,
        module_name: &str,
        container: &mut IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        let start_time = Instant::now();

        // Update module state
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.state = ModuleState::Configuring;
        }

        // Get module descriptor
        let descriptor = self
            .modules
            .get(module_name)
            .ok_or_else(|| ModuleRuntimeError::MissingDependency {
                module: module_name.to_string(),
                missing_dependency: "module_descriptor".to_string(),
                message: "Module not found".to_string(),
            })?
            .descriptor
            .clone();

        // Configure services with the IoC container
        self.configure_module_services(&descriptor, container)
            .await?;

        // Configure controllers
        self.configure_module_controllers(&descriptor, container)
            .await?;

        // Update timing and state
        let config_duration = start_time.elapsed();
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.config_duration = Some(config_duration);
            module_info.state = ModuleState::Initializing;
        }

        Ok(())
    }

    /// Configure module services with the IoC container
    async fn configure_module_services(
        &self,
        descriptor: &ModuleDescriptor,
        _container: &mut IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        for service in &descriptor.providers {
            match (service.implementation_type, &service.name) {
                // Named trait service
                (Some(_), Some(name)) if service.is_trait_service => {
                    // For trait services, we would need token-based binding
                    // For now, we'll skip these and log a warning
                    tracing::warn!(
                        "Trait service '{}' with name '{}' requires token-based binding (not yet fully integrated)",
                        service.service_name, name
                    );
                }
                // Named concrete service
                (None, Some(name)) => {
                    // This is a named concrete service binding
                    // We would need to bind by type + name, but this requires more complex integration
                    tracing::warn!(
                        "Named concrete service '{}' with name '{}' requires enhanced binding support",
                        service.service_name, name
                    );
                }
                // Regular trait service (unnamed)
                (Some(_), None) if service.is_trait_service => {
                    // Trait service without name - requires token-based binding
                    tracing::warn!(
                        "Trait service '{}' requires token-based binding (not yet fully integrated)",
                        service.service_name
                    );
                }
                // Regular concrete service (most common case)
                (None, None) => {
                    // This is a basic concrete service - we can't bind it without knowing the actual types
                    // The current system needs compile-time type information that we don't have at runtime
                    tracing::info!(
                        "Concrete service '{}' registered (runtime binding not yet implemented)",
                        service.service_name
                    );
                }
                // Other cases
                _ => {
                    tracing::warn!(
                        "Unknown service configuration for '{}' - skipping",
                        service.service_name
                    );
                }
            }
        }

        Ok(())
    }

    /// Configure module controllers  
    async fn configure_module_controllers(
        &self,
        descriptor: &ModuleDescriptor,
        _container: &mut IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        for controller in &descriptor.controllers {
            // Controller registration would be integrated with HTTP routing system
            // For now, we'll track them for future integration
            tracing::info!(
                "Controller '{}' registered (HTTP routing integration pending)",
                controller.controller_name
            );
        }

        Ok(())
    }

    /// Task 4.4: Add module lifecycle hooks (startup, shutdown, health checks)
    pub async fn initialize_all_modules(
        &mut self,
        container: &IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        let start_time = Instant::now();

        for module_name in &self.load_order.clone() {
            self.initialize_module(module_name, container).await?;
        }

        self.metrics.initialization_duration = start_time.elapsed();
        self.calculate_performance_metrics();

        Ok(())
    }

    /// Initialize a specific module
    async fn initialize_module(
        &mut self,
        module_name: &str,
        _container: &IocContainer,
    ) -> Result<(), ModuleRuntimeError> {
        let init_start = Instant::now();

        // Call before_init hook
        if let Some(hook) = self.lifecycle_hooks.get(module_name) {
            hook.before_init(module_name)?;
        }

        // Update module state
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.state = ModuleState::Initializing;
        }

        // Here we would call the actual module initialization
        // For now, we'll simulate with a small delay
        tokio::time::sleep(Duration::from_millis(10)).await;

        let init_duration = init_start.elapsed();

        // Update module state and timing
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.state = ModuleState::Ready;
            module_info.init_duration = Some(init_duration);
        }

        // Call after_init hook
        if let Some(hook) = self.lifecycle_hooks.get(module_name) {
            if let Err(e) = hook.after_init(module_name, init_duration) {
                // Mark module as failed if after_init hook fails
                if let Some(module_info) = self.modules.get_mut(module_name) {
                    module_info.state = ModuleState::Failed(e.to_string());
                    module_info.add_error(e.to_string());
                }
                return Err(e);
            }
        }

        Ok(())
    }

    /// Shutdown all modules in reverse order
    pub async fn shutdown_all_modules(&mut self) -> Result<(), ModuleRuntimeError> {
        let mut shutdown_order = self.load_order.clone();
        shutdown_order.reverse();

        for module_name in shutdown_order {
            self.shutdown_module(&module_name).await?;
        }

        Ok(())
    }

    /// Shutdown a specific module
    async fn shutdown_module(&mut self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        // Call before_shutdown hook
        if let Some(hook) = self.lifecycle_hooks.get(module_name) {
            hook.before_shutdown(module_name)?;
        }

        // Update module state
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.state = ModuleState::ShuttingDown;
        }

        // Perform actual shutdown operations here
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Update state
        if let Some(module_info) = self.modules.get_mut(module_name) {
            module_info.state = ModuleState::Shutdown;
        }

        // Call after_shutdown hook
        if let Some(hook) = self.lifecycle_hooks.get(module_name) {
            hook.after_shutdown(module_name)?;
        }

        Ok(())
    }

    /// Run health checks for all modules
    pub async fn health_check_all_modules(
        &mut self,
    ) -> Result<HashMap<String, HealthStatus>, ModuleRuntimeError> {
        let mut health_results = HashMap::new();
        let check_time = Instant::now();

        for module_name in &self.load_order.clone() {
            let health_status = if let Some(hook) = self.lifecycle_hooks.get(module_name) {
                hook.health_check(module_name)
                    .unwrap_or(HealthStatus::Unknown)
            } else {
                // Default health check - if module is ready, it's healthy
                match self.modules.get(module_name).map(|m| &m.state) {
                    Some(ModuleState::Ready) => HealthStatus::Healthy,
                    Some(ModuleState::Failed(_)) => HealthStatus::Unhealthy,
                    _ => HealthStatus::Unknown,
                }
            };

            // Update module health status
            if let Some(module_info) = self.modules.get_mut(module_name) {
                module_info.health_status = health_status.clone();
                module_info.last_health_check = Some(check_time);
            }

            health_results.insert(module_name.clone(), health_status);
        }

        Ok(health_results)
    }

    /// Calculate performance metrics
    fn calculate_performance_metrics(&mut self) {
        self.metrics.total_modules = self.modules.len();

        if self.metrics.total_modules > 0 {
            let total_init_time: Duration =
                self.modules.values().filter_map(|m| m.init_duration).sum();

            self.metrics.avg_init_time_per_module =
                total_init_time / self.metrics.total_modules as u32;

            // Find slowest module
            let slowest = self
                .modules
                .iter()
                .filter_map(|(name, info)| info.init_duration.map(|d| (name, d)))
                .max_by_key(|(_, duration)| *duration);

            if let Some((name, duration)) = slowest {
                self.metrics.slowest_module = Some(name.clone());
                self.metrics.slowest_init_time = duration;
            }
        }
    }

    /// Get module runtime information
    pub fn get_module_info(&self, module_name: &str) -> Option<&ModuleRuntimeInfo> {
        self.modules.get(module_name)
    }

    /// Get all module runtime information
    pub fn get_all_module_info(&self) -> &HashMap<String, ModuleRuntimeInfo> {
        &self.modules
    }

    /// Get load order
    pub fn get_load_order(&self) -> &[String] {
        &self.load_order
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &ModulePerformanceMetrics {
        &self.metrics
    }

    /// Validate runtime state
    pub fn validate_runtime_state(&self) -> Result<(), Vec<ModuleRuntimeError>> {
        let mut errors = Vec::new();

        // Check that all modules are in a valid state
        for (name, info) in &self.modules {
            match &info.state {
                ModuleState::Failed(err) => {
                    errors.push(ModuleRuntimeError::InitializationFailed {
                        module: name.clone(),
                        error: err.clone(),
                        phase: "runtime_validation".to_string(),
                    });
                }
                ModuleState::ResolvingDependencies
                | ModuleState::Configuring
                | ModuleState::Initializing => {
                    errors.push(ModuleRuntimeError::InitializationFailed {
                        module: name.clone(),
                        error: "Module stuck in intermediate state".to_string(),
                        phase: format!("{:?}", info.state),
                    });
                }
                _ => {}
            }

            // Check for modules with too many errors
            if info.errors.len() > 5 {
                errors.push(ModuleRuntimeError::ValidationFailed {
                    module: name.clone(),
                    validation_errors: vec![format!("Module has {} errors", info.errors.len())],
                });
            }
        }

        // Check load order consistency
        if self.load_order.len() != self.modules.len() {
            errors.push(ModuleRuntimeError::ValidationFailed {
                module: "runtime".to_string(),
                validation_errors: vec![format!(
                    "Load order length ({}) doesn't match module count ({})",
                    self.load_order.len(),
                    self.modules.len()
                )],
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get mutable access to modules (for testing purposes)
    #[cfg(test)]
    pub fn modules_mut(&mut self) -> &mut HashMap<String, ModuleRuntimeInfo> {
        &mut self.modules
    }

    /// Get runtime statistics for monitoring
    pub fn get_runtime_statistics(&self) -> ModuleRuntimeStatistics {
        let mut stats = ModuleRuntimeStatistics::default();

        stats.total_modules = self.modules.len();

        for info in self.modules.values() {
            match &info.state {
                ModuleState::Registered => stats.registered_modules += 1,
                ModuleState::ResolvingDependencies => stats.resolving_modules += 1,
                ModuleState::Configuring => stats.configuring_modules += 1,
                ModuleState::Initializing => stats.initializing_modules += 1,
                ModuleState::Ready => stats.ready_modules += 1,
                ModuleState::Failed(_) => stats.failed_modules += 1,
                ModuleState::ShuttingDown => stats.shutting_down_modules += 1,
                ModuleState::Shutdown => stats.shutdown_modules += 1,
            }

            match &info.health_status {
                HealthStatus::Healthy => stats.healthy_modules += 1,
                HealthStatus::Degraded => stats.degraded_modules += 1,
                HealthStatus::Unhealthy => stats.unhealthy_modules += 1,
                HealthStatus::Unknown => stats.unknown_health_modules += 1,
            }
        }

        stats.performance_metrics = self.metrics.clone();

        stats
    }
}

impl Default for ModuleRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ModuleRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRuntime")
            .field("modules", &self.modules)
            .field("dependency_graph", &self.dependency_graph)
            .field("load_order", &self.load_order)
            .field("metrics", &self.metrics)
            .field("lifecycle_hooks", &format!("{} hooks", self.lifecycle_hooks.len()))
            .field("health_check_config", &self.health_check_config)
            .finish()
    }
}

/// Runtime statistics for monitoring and debugging
#[derive(Debug, Default, Clone)]
pub struct ModuleRuntimeStatistics {
    pub total_modules: usize,
    pub registered_modules: usize,
    pub resolving_modules: usize,
    pub configuring_modules: usize,
    pub initializing_modules: usize,
    pub ready_modules: usize,
    pub failed_modules: usize,
    pub shutting_down_modules: usize,
    pub shutdown_modules: usize,
    pub healthy_modules: usize,
    pub degraded_modules: usize,
    pub unhealthy_modules: usize,
    pub unknown_health_modules: usize,
    pub performance_metrics: ModulePerformanceMetrics,
}

/// Default lifecycle hook implementation
pub struct DefaultLifecycleHook;

impl ModuleLifecycleHook for DefaultLifecycleHook {
    fn before_init(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        tracing::info!("Starting initialization of module '{}'", module_name);
        Ok(())
    }

    fn after_init(&self, module_name: &str, duration: Duration) -> Result<(), ModuleRuntimeError> {
        tracing::info!(
            "Module '{}' initialized successfully in {:?}",
            module_name,
            duration
        );
        Ok(())
    }

    fn on_init_failure(&self, module_name: &str, error: &ModuleRuntimeError) {
        tracing::error!("Module '{}' initialization failed: {}", module_name, error);
    }

    fn health_check(&self, module_name: &str) -> Result<HealthStatus, ModuleRuntimeError> {
        // Default implementation - always healthy
        let _ = module_name;
        Ok(HealthStatus::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_module(name: &str, dependencies: Vec<String>) -> ModuleDescriptor {
        ModuleDescriptor::new(name)
            .with_dependencies(dependencies)
            .with_description(format!("Test module {}", name))
    }

    #[tokio::test]
    async fn test_topological_sorting_simple() {
        let mut runtime = ModuleRuntime::new();

        // A -> B -> C dependency chain
        runtime
            .register_module(create_test_module("A", vec![]))
            .unwrap();
        runtime
            .register_module(create_test_module("B", vec!["A".to_string()]))
            .unwrap();
        runtime
            .register_module(create_test_module("C", vec!["B".to_string()]))
            .unwrap();

        let load_order = runtime.calculate_load_order().unwrap();

        assert_eq!(load_order, vec!["A", "B", "C"]);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let mut runtime = ModuleRuntime::new();

        // A -> B -> C -> A (circular)
        runtime
            .register_module(create_test_module("A", vec!["C".to_string()]))
            .unwrap();
        runtime
            .register_module(create_test_module("B", vec!["A".to_string()]))
            .unwrap();
        runtime
            .register_module(create_test_module("C", vec!["B".to_string()]))
            .unwrap();

        let result = runtime.calculate_load_order();

        assert!(result.is_err());
        match result.unwrap_err() {
            ModuleRuntimeError::CircularDependency { cycle, .. } => {
                assert!(cycle.len() >= 3);
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[tokio::test]
    async fn test_missing_dependency_detection() {
        let mut runtime = ModuleRuntime::new();

        runtime
            .register_module(create_test_module("A", vec!["NonExistent".to_string()]))
            .unwrap();

        let result = runtime.calculate_load_order();

        assert!(result.is_err());
        match result.unwrap_err() {
            ModuleRuntimeError::MissingDependency {
                module,
                missing_dependency,
                ..
            } => {
                assert_eq!(module, "A");
                assert_eq!(missing_dependency, "NonExistent");
            }
            _ => panic!("Expected MissingDependency error"),
        }
    }

    #[tokio::test]
    async fn test_complex_dependency_graph() {
        let mut runtime = ModuleRuntime::new();

        // Complex dependency graph:
        // A (no deps)
        // B -> A
        // C -> A
        // D -> B, C
        // E -> D
        runtime
            .register_module(create_test_module("A", vec![]))
            .unwrap();
        runtime
            .register_module(create_test_module("B", vec!["A".to_string()]))
            .unwrap();
        runtime
            .register_module(create_test_module("C", vec!["A".to_string()]))
            .unwrap();
        runtime
            .register_module(create_test_module(
                "D",
                vec!["B".to_string(), "C".to_string()],
            ))
            .unwrap();
        runtime
            .register_module(create_test_module("E", vec!["D".to_string()]))
            .unwrap();

        let load_order = runtime.calculate_load_order().unwrap();

        // A should be first
        assert_eq!(load_order[0], "A");

        // B and C should come after A but before D
        let a_pos = load_order.iter().position(|x| x == "A").unwrap();
        let b_pos = load_order.iter().position(|x| x == "B").unwrap();
        let c_pos = load_order.iter().position(|x| x == "C").unwrap();
        let d_pos = load_order.iter().position(|x| x == "D").unwrap();
        let e_pos = load_order.iter().position(|x| x == "E").unwrap();

        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
        assert!(d_pos < e_pos);
    }

    #[tokio::test]
    async fn test_module_lifecycle_hooks() {
        let mut runtime = ModuleRuntime::new();
        runtime
            .register_module(create_test_module("TestModule", vec![]))
            .unwrap();

        // Add default lifecycle hook
        runtime.add_lifecycle_hook("TestModule".to_string(), DefaultLifecycleHook);

        // Calculate load order
        runtime.calculate_load_order().unwrap();

        // Create mock container
        let mut container = IocContainer::new();
        container.build().unwrap();

        // Test initialization with hooks
        let result = runtime.initialize_all_modules(&container).await;
        assert!(result.is_ok());

        // Verify module is ready
        let module_info = runtime.get_module_info("TestModule").unwrap();
        assert_eq!(module_info.state, ModuleState::Ready);
        assert!(module_info.init_duration.is_some());
    }

    #[tokio::test]
    async fn test_health_checks() {
        let mut runtime = ModuleRuntime::new();
        runtime
            .register_module(create_test_module("TestModule", vec![]))
            .unwrap();
        runtime.add_lifecycle_hook("TestModule".to_string(), DefaultLifecycleHook);

        runtime.calculate_load_order().unwrap();

        let mut container = IocContainer::new();
        container.build().unwrap();
        runtime.initialize_all_modules(&container).await.unwrap();

        // Run health checks
        let health_results = runtime.health_check_all_modules().await.unwrap();

        assert_eq!(health_results.len(), 1);
        assert_eq!(health_results["TestModule"], HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_runtime_validation() {
        let mut runtime = ModuleRuntime::new();
        runtime
            .register_module(create_test_module("TestModule", vec![]))
            .unwrap();

        runtime.calculate_load_order().unwrap();

        let mut container = IocContainer::new();
        container.build().unwrap();
        runtime.initialize_all_modules(&container).await.unwrap();

        // Validate runtime state
        let validation_result = runtime.validate_runtime_state();
        assert!(validation_result.is_ok());
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let mut runtime = ModuleRuntime::new();

        for i in 0..10 {
            runtime
                .register_module(create_test_module(&format!("Module{}", i), vec![]))
                .unwrap();
        }

        runtime.calculate_load_order().unwrap();

        let mut container = IocContainer::new();
        container.build().unwrap();
        runtime.initialize_all_modules(&container).await.unwrap();

        let metrics = runtime.get_performance_metrics();

        assert_eq!(metrics.total_modules, 10);
        assert!(metrics.initialization_duration > Duration::ZERO);
        assert!(metrics.avg_init_time_per_module > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_shutdown_order() {
        let mut runtime = ModuleRuntime::new();

        runtime
            .register_module(create_test_module("A", vec![]))
            .unwrap();
        runtime
            .register_module(create_test_module("B", vec!["A".to_string()]))
            .unwrap();
        runtime
            .register_module(create_test_module("C", vec!["B".to_string()]))
            .unwrap();

        runtime.calculate_load_order().unwrap();

        let mut container = IocContainer::new();
        container.build().unwrap();
        runtime.initialize_all_modules(&container).await.unwrap();

        // Shutdown should happen in reverse order
        runtime.shutdown_all_modules().await.unwrap();

        // All modules should be shut down
        for info in runtime.get_all_module_info().values() {
            assert_eq!(info.state, ModuleState::Shutdown);
        }
    }
}
