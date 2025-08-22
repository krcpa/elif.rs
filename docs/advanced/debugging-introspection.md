# Debugging and Introspection

The elif.rs IoC container provides powerful debugging and introspection tools to help you understand, monitor, and troubleshoot your dependency injection setup. These tools are essential for maintaining complex applications and diagnosing issues in production.

## Container Inspector

The `ContainerInspector` provides comprehensive analysis of your container configuration:

```rust
use elif_core::container::{ContainerInspector, InspectionLevel};

let container = builder.build()?;
let inspector = ContainerInspector::new(&container);

// Basic container information
let summary = inspector.get_summary();
println!("Container Summary:");
println!("  Services registered: {}", summary.total_services);
println!("  Singleton services: {}", summary.singleton_count);
println!("  Scoped services: {}", summary.scoped_count);
println!("  Transient services: {}", summary.transient_count);
println!("  Total dependencies: {}", summary.total_dependencies);

// Detailed inspection
let report = inspector.inspect(InspectionLevel::Detailed)?;
println!("Detailed Inspection Report:");
println!("{}", report.to_string());
```

### Service Analysis

Analyze individual services and their dependencies:

```rust
// Analyze a specific service
let service_analysis = inspector.analyze_service::<UserService>()?;

println!("UserService Analysis:");
println!("  Lifetime: {:?}", service_analysis.lifetime);
println!("  Dependencies: {:?}", service_analysis.dependencies);
println!("  Dependents: {:?}", service_analysis.dependents);
println!("  Dependency depth: {}", service_analysis.dependency_depth);
println!("  Memory usage estimate: {} bytes", service_analysis.estimated_memory_usage);

// Check if service is properly configured
if service_analysis.has_issues() {
    for issue in &service_analysis.issues {
        println!("  Issue: {}", issue.description);
        if let Some(suggestion) = &issue.suggestion {
            println!("    Suggestion: {}", suggestion);
        }
    }
}
```

## Resolution Tracing

Track service resolution performance and behavior:

```rust
use elif_core::container::{ResolutionTracer, TracingLevel};

// Enable tracing (only in development/debug builds)
#[cfg(debug_assertions)]
{
    let tracer = ResolutionTracer::new()
        .with_level(TracingLevel::Detailed)
        .with_timing(true)
        .with_memory_tracking(true);
    
    container.set_tracer(tracer);
}

// Resolve services with tracing
let user_service = container.resolve::<UserService>()?;

// Get tracing report
let trace_report = container.get_trace_report();
for entry in trace_report.entries {
    println!("Resolved: {} in {:?}", entry.service_type, entry.resolution_time);
    if let Some(memory) = entry.memory_allocated {
        println!("  Memory allocated: {} bytes", memory);
    }
    
    for step in entry.resolution_steps {
        println!("  Step: {} ({:?})", step.description, step.duration);
    }
}
```

### Conditional Tracing

Enable tracing only for specific services:

```rust
let tracer = ResolutionTracer::new()
    .trace_service::<UserService>()
    .trace_service::<EmailService>()
    .trace_pattern("*Repository") // Trace all repositories
    .with_threshold(Duration::from_millis(10)); // Only slow resolutions

container.set_tracer(tracer);
```

## Performance Profiling

Monitor container performance in production:

```rust
use elif_core::container::{PerformanceProfiler, ProfilerConfig};

let config = ProfilerConfig {
    sample_rate: 0.1, // Sample 10% of resolutions
    enable_memory_tracking: true,
    enable_timing: true,
    slow_resolution_threshold: Duration::from_millis(50),
};

let profiler = PerformanceProfiler::new(config);
container.set_profiler(profiler);

// Later, get performance metrics
let metrics = container.get_performance_metrics();
println!("Performance Metrics:");
println!("  Total resolutions: {}", metrics.total_resolutions);
println!("  Average resolution time: {:?}", metrics.average_resolution_time);
println!("  Slow resolutions: {}", metrics.slow_resolutions);
println!("  Memory allocated: {} bytes", metrics.total_memory_allocated);

// Top 10 slowest services
for (service, time) in metrics.slowest_services.iter().take(10) {
    println!("  {}: {:?}", service, time);
}
```

## Health Checking

Built-in health checks for container services:

```rust
use elif_core::container::{ContainerHealthChecker, HealthCheckConfig};

let health_checker = ContainerHealthChecker::new()
    .with_timeout(Duration::from_secs(5))
    .check_circular_dependencies()
    .check_missing_dependencies()
    .check_factory_health()
    .check_service_availability();

// Custom health check
health_checker.add_check("database", |container| {
    Box::pin(async move {
        let db = container.resolve::<dyn Database>()?;
        db.ping().await.map_err(|e| e.into())
    })
});

// Run health checks
let health_report = health_checker.check(&container).await?;

if health_report.is_healthy() {
    println!("Container is healthy");
} else {
    println!("Container health issues:");
    for issue in health_report.issues {
        println!("  {}: {}", issue.check_name, issue.message);
    }
}
```

## Dependency Visualization

Generate visual representations of your dependency graph:

```rust
use elif_core::container::{DependencyVisualizer, VisualizationFormat, VisualizationStyle};

let visualizer = DependencyVisualizer::new(&container);

// Generate DOT format for Graphviz
let dot_graph = visualizer.visualize(VisualizationFormat::Dot, VisualizationStyle::default())?;
std::fs::write("dependencies.dot", dot_graph)?;

// Generate Mermaid diagram
let mermaid = visualizer.visualize(VisualizationFormat::Mermaid, VisualizationStyle::default())?;
std::fs::write("dependencies.mmd", mermaid)?;

// ASCII tree for console output
let ascii = visualizer.visualize(VisualizationFormat::Ascii, VisualizationStyle::default())?;
println!("Dependency Tree:\n{}", ascii);

// JSON for programmatic analysis
let json = visualizer.visualize(VisualizationFormat::Json, VisualizationStyle::default())?;
std::fs::write("dependencies.json", json)?;
```

### Visualization Styling

Customize visualization appearance:

```rust
let style = VisualizationStyle {
    show_lifetimes: true,
    color_by_lifetime: true,
    group_by_module: true,
    filter_types: Some(vec!["UserService".to_string(), "EmailService".to_string()]),
    max_depth: Some(5),
    include_stats: true,
    ..Default::default()
};

let filtered_graph = visualizer.visualize(VisualizationFormat::Dot, style)?;
```

## Service Exploration

Explore service relationships and dependencies:

```rust
use elif_core::container::ServiceExplorer;

let explorer = ServiceExplorer::new(&container);

// Find all paths between services
let paths = explorer.find_paths::<UserController, DatabaseConnection>();
println!("Paths from UserController to DatabaseConnection:");
for (i, path) in paths.iter().enumerate() {
    println!("  Path {}: {}", i + 1, path.join(" -> "));
}

// Get services that depend on a specific service
let dependents = explorer.get_dependents::<DatabaseConnection>();
println!("Services depending on DatabaseConnection:");
for dependent in dependents {
    println!("  {}", dependent);
}

// Calculate dependency depth
let depth = explorer.get_dependency_depth::<UserController>();
println!("UserController dependency depth: {}", depth);

// Generate analysis report
let report = explorer.generate_report();
println!("{}", report);
```

## Development Tools

### Container Validation

Validate container configuration during development:

```rust
#[cfg(debug_assertions)]
fn validate_container_in_development(builder: &IocContainerBuilder) {
    use elif_core::container::{DependencyValidator, ValidationReport};
    
    let descriptors = builder.get_descriptors();
    let validator = DependencyValidator::new(&descriptors);
    let report = validator.validate();
    
    match report.status {
        ValidationStatus::Valid => {
            println!("✅ Container validation passed");
        }
        ValidationStatus::HasWarnings => {
            println!("⚠️  Container has warnings:");
            for warning in &report.warnings {
                println!("    {}", warning.message);
            }
        }
        ValidationStatus::Invalid => {
            println!("❌ Container validation failed:");
            for error in &report.errors {
                println!("    {}", error.message);
            }
            panic!("Container validation failed in development");
        }
    }
}
```

### Debug Logging

Enable detailed logging for container operations:

```rust
use log::{debug, info, warn};

// Enable debug logging in development
#[cfg(debug_assertions)]
fn setup_container_logging() {
    env_logger::Builder::new()
        .filter_module("elif_core::container", log::LevelFilter::Debug)
        .init();
}

// Container will log:
// - Service registrations
// - Resolution attempts
// - Circular dependency detection
// - Performance warnings
// - Memory usage information
```

## Production Monitoring

### Metrics Collection

Collect container metrics for monitoring:

```rust
use elif_core::container::{ContainerMetrics, MetricsCollector};

let metrics_collector = MetricsCollector::new()
    .with_resolution_tracking()
    .with_memory_tracking()
    .with_error_tracking();

container.set_metrics_collector(metrics_collector);

// Periodically export metrics
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        
        let metrics = container.get_metrics();
        
        // Export to monitoring system (Prometheus, DataDog, etc.)
        export_to_prometheus(&metrics);
    }
});
```

### Error Monitoring

Track and report container-related errors:

```rust
use elif_core::container::{ErrorTracker, ErrorLevel};

let error_tracker = ErrorTracker::new()
    .track_resolution_failures()
    .track_circular_dependencies()
    .track_performance_issues()
    .with_alert_threshold(ErrorLevel::Warning, 10); // Alert after 10 warnings

container.set_error_tracker(error_tracker);

// Get error summary
let error_summary = container.get_error_summary();
if error_summary.should_alert() {
    // Send alert to monitoring system
    send_alert("IoC Container Issues", &error_summary.to_string());
}
```

## Best Practices

### 1. Development vs Production

Use different debugging configurations:

```rust
#[cfg(debug_assertions)]
fn setup_development_debugging(container: &mut IocContainer) {
    let tracer = ResolutionTracer::new()
        .with_level(TracingLevel::Detailed)
        .with_timing(true);
    container.set_tracer(tracer);
    
    let health_checker = ContainerHealthChecker::new()
        .check_all();
    container.set_health_checker(health_checker);
}

#[cfg(not(debug_assertions))]
fn setup_production_monitoring(container: &mut IocContainer) {
    let profiler = PerformanceProfiler::new(ProfilerConfig {
        sample_rate: 0.01, // 1% sampling in production
        slow_resolution_threshold: Duration::from_millis(100),
        ..Default::default()
    });
    container.set_profiler(profiler);
}
```

### 2. Conditional Debugging

Enable debugging based on environment:

```rust
if env::var("ELIF_DEBUG_CONTAINER").is_ok() {
    let tracer = ResolutionTracer::new()
        .with_level(TracingLevel::Detailed);
    container.set_tracer(tracer);
}
```

### 3. Performance Impact

Be aware of debugging overhead:

```rust
// Heavy debugging - development only
#[cfg(debug_assertions)]
{
    container.enable_full_tracing();
    container.enable_memory_tracking();
}

// Lightweight monitoring - production
#[cfg(not(debug_assertions))]
{
    container.enable_performance_sampling(0.01); // 1% sampling
}
```

### 4. Automated Analysis

Set up automated container analysis in CI:

```rust
#[test]
fn analyze_container_health() {
    let container = create_production_container();
    
    let inspector = ContainerInspector::new(&container);
    let analysis = inspector.analyze_all()?;
    
    // Fail CI if there are critical issues
    assert!(analysis.critical_issues.is_empty(), 
            "Critical container issues found: {:?}", analysis.critical_issues);
    
    // Warn about performance issues
    if !analysis.performance_warnings.is_empty() {
        println!("Performance warnings: {:?}", analysis.performance_warnings);
    }
}
```

The debugging and introspection tools in elif.rs provide comprehensive visibility into your IoC container behavior, helping you build robust, maintainable applications with confidence.