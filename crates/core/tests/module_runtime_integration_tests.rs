//! Integration tests for Epic 4 - Module Runtime Integration & Validation
//!
//! Tests the complete module runtime system with complex dependency chains,
//! lifecycle management, error scenarios, and performance validation.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use elif_core::container::IocContainer;
use elif_core::modules::runtime::{
    DefaultLifecycleHook, HealthCheckConfig, HealthStatus, ModuleLifecycleHook, ModuleRuntime,
    ModuleRuntimeError, ModuleState,
};
use elif_core::modules::{
    ControllerDescriptor, ModuleDescriptor, ServiceDescriptor, ServiceLifecycle,
};

/// Custom lifecycle hook for testing
struct TestLifecycleHook {
    events: Arc<Mutex<Vec<String>>>,
}

impl TestLifecycleHook {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ModuleLifecycleHook for TestLifecycleHook {
    fn before_init(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("before_init:{}", module_name));
        Ok(())
    }

    fn after_init(&self, module_name: &str, duration: Duration) -> Result<(), ModuleRuntimeError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("after_init:{}:{:?}", module_name, duration));
        Ok(())
    }

    fn on_init_failure(&self, module_name: &str, error: &ModuleRuntimeError) {
        self.events
            .lock()
            .unwrap()
            .push(format!("init_failure:{}:{}", module_name, error));
    }

    fn before_shutdown(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("before_shutdown:{}", module_name));
        Ok(())
    }

    fn after_shutdown(&self, module_name: &str) -> Result<(), ModuleRuntimeError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("after_shutdown:{}", module_name));
        Ok(())
    }

    fn health_check(&self, module_name: &str) -> Result<HealthStatus, ModuleRuntimeError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("health_check:{}", module_name));
        Ok(HealthStatus::Healthy)
    }
}

/// Helper to create test modules with services and controllers
fn create_test_module_with_services(
    name: &str,
    dependencies: Vec<String>,
    service_count: usize,
    controller_count: usize,
) -> ModuleDescriptor {
    let mut module = ModuleDescriptor::new(name)
        .with_dependencies(dependencies)
        .with_description(format!(
            "Test module {} with {} services and {} controllers",
            name, service_count, controller_count
        ));

    // Add test services
    for i in 0..service_count {
        let service = ServiceDescriptor::new::<String>(
            format!("{}Service{}", name, i),
            ServiceLifecycle::Singleton,
        );
        module = module.with_provider(service);
    }

    // Add test controllers
    for i in 0..controller_count {
        let controller = ControllerDescriptor::new::<String>(format!("{}Controller{}", name, i))
            .with_base_path(&format!("/api/{}/v{}", name.to_lowercase(), i));
        module = module.with_controller(controller);
    }

    module
}

/// Test 1: Simple Linear Dependency Chain
#[tokio::test]
async fn test_simple_linear_dependency_chain() {
    let mut runtime = ModuleRuntime::new();

    // Create a simple A -> B -> C -> D chain
    runtime
        .register_module(create_test_module_with_services("A", vec![], 2, 1))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "B",
            vec!["A".to_string()],
            3,
            2,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "C",
            vec!["B".to_string()],
            1,
            1,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "D",
            vec!["C".to_string()],
            2,
            0,
        ))
        .unwrap();

    // Calculate load order
    let load_order = runtime.calculate_load_order().unwrap();
    assert_eq!(load_order, vec!["A", "B", "C", "D"]);

    // Initialize with container
    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    // Verify all modules are ready
    for module_name in &load_order {
        let info = runtime.get_module_info(module_name).unwrap();
        assert_eq!(info.state, ModuleState::Ready);
        assert!(info.init_duration.is_some());
        assert!(info.config_duration.is_some());
    }

    // Test runtime validation
    runtime.validate_runtime_state().unwrap();

    // Test performance metrics
    let metrics = runtime.get_performance_metrics();
    assert_eq!(metrics.total_modules, 4);
    assert!(metrics.initialization_duration > Duration::ZERO);

    println!("✅ Simple linear dependency chain test passed");
}

/// Test 2: Complex Diamond Dependency Pattern
#[tokio::test]
async fn test_complex_diamond_dependency_pattern() {
    let mut runtime = ModuleRuntime::new();

    // Create diamond pattern:
    //     A
    //   /   \
    //  B     C
    //   \   /
    //     D
    //     |
    //     E
    runtime
        .register_module(create_test_module_with_services("A", vec![], 3, 2))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "B",
            vec!["A".to_string()],
            2,
            1,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "C",
            vec!["A".to_string()],
            1,
            3,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "D",
            vec!["B".to_string(), "C".to_string()],
            4,
            1,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "E",
            vec!["D".to_string()],
            1,
            1,
        ))
        .unwrap();

    let load_order = runtime.calculate_load_order().unwrap();

    // Verify correct ordering constraints
    let a_pos = load_order.iter().position(|x| x == "A").unwrap();
    let b_pos = load_order.iter().position(|x| x == "B").unwrap();
    let c_pos = load_order.iter().position(|x| x == "C").unwrap();
    let d_pos = load_order.iter().position(|x| x == "D").unwrap();
    let e_pos = load_order.iter().position(|x| x == "E").unwrap();

    // A must come first
    assert!(a_pos < b_pos && a_pos < c_pos);
    // B and C must come before D
    assert!(b_pos < d_pos && c_pos < d_pos);
    // D must come before E
    assert!(d_pos < e_pos);

    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    // Verify statistics
    let stats = runtime.get_runtime_statistics();
    assert_eq!(stats.total_modules, 5);
    assert_eq!(stats.ready_modules, 5);
    assert_eq!(stats.failed_modules, 0);

    println!("✅ Complex diamond dependency pattern test passed");
}

/// Test 3: Circular Dependency Detection
#[tokio::test]
async fn test_circular_dependency_detection() {
    let mut runtime = ModuleRuntime::new();

    // Create circular dependency: A -> B -> C -> A
    runtime
        .register_module(create_test_module_with_services(
            "A",
            vec!["C".to_string()],
            1,
            0,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "B",
            vec!["A".to_string()],
            1,
            0,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "C",
            vec!["B".to_string()],
            1,
            0,
        ))
        .unwrap();

    let result = runtime.calculate_load_order();

    assert!(result.is_err());
    match result.unwrap_err() {
        ModuleRuntimeError::CircularDependency { cycle, message } => {
            assert!(cycle.len() >= 3);
            assert!(message.contains("Circular dependency detected"));
            println!(
                "✅ Correctly detected circular dependency: {} - {}",
                cycle.join(" -> "),
                message
            );
        }
        _ => panic!("Expected CircularDependency error"),
    }

    println!("✅ Circular dependency detection test passed");
}

/// Test 4: Missing Dependency Detection
#[tokio::test]
async fn test_missing_dependency_detection() {
    let mut runtime = ModuleRuntime::new();

    // Create module with missing dependency
    runtime
        .register_module(create_test_module_with_services(
            "A",
            vec!["NonExistent".to_string()],
            1,
            1,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "B",
            vec!["A".to_string(), "AlsoMissing".to_string()],
            2,
            0,
        ))
        .unwrap();

    let result = runtime.calculate_load_order();

    assert!(result.is_err());
    let error = result.unwrap_err();
    match &error {
        ModuleRuntimeError::MissingDependency {
            module,
            missing_dependency,
            message,
        } => {
            assert!(module == "A" || module == "B");
            assert!(missing_dependency == "NonExistent" || missing_dependency == "AlsoMissing");
            assert!(message.contains("not registered"));
            println!(
                "✅ Correctly detected missing dependency: {} -> {} ({})",
                module, missing_dependency, message
            );
        }
        _ => panic!("Expected MissingDependency error, got: {:?}", error),
    }

    println!("✅ Missing dependency detection test passed");
}

/// Test 5: Module Lifecycle Hooks
#[tokio::test]
async fn test_module_lifecycle_hooks() {
    let mut runtime = ModuleRuntime::new();

    // Create test modules
    runtime
        .register_module(create_test_module_with_services("CoreModule", vec![], 2, 1))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "AuthModule",
            vec!["CoreModule".to_string()],
            1,
            2,
        ))
        .unwrap();

    // Add lifecycle hooks
    let core_hook = TestLifecycleHook::new();
    let auth_hook = TestLifecycleHook::new();
    let core_events = core_hook.events.clone();
    let auth_events = auth_hook.events.clone();

    runtime.add_lifecycle_hook("CoreModule".to_string(), core_hook);
    runtime.add_lifecycle_hook("AuthModule".to_string(), auth_hook);

    runtime.calculate_load_order().unwrap();

    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    // Verify lifecycle events were called
    {
        let core_events_locked = core_events.lock().unwrap();
        let auth_events_locked = auth_events.lock().unwrap();

        assert!(core_events_locked
            .iter()
            .any(|e| e.contains("before_init:CoreModule")));
        assert!(core_events_locked
            .iter()
            .any(|e| e.contains("after_init:CoreModule")));
        assert!(auth_events_locked
            .iter()
            .any(|e| e.contains("before_init:AuthModule")));
        assert!(auth_events_locked
            .iter()
            .any(|e| e.contains("after_init:AuthModule")));
    }

    // Test health checks
    runtime.health_check_all_modules().await.unwrap();

    {
        let core_events_locked = core_events.lock().unwrap();
        let auth_events_locked = auth_events.lock().unwrap();
        assert!(core_events_locked
            .iter()
            .any(|e| e.contains("health_check:CoreModule")));
        assert!(auth_events_locked
            .iter()
            .any(|e| e.contains("health_check:AuthModule")));
    }

    // Test shutdown
    runtime.shutdown_all_modules().await.unwrap();

    {
        let core_events_locked = core_events.lock().unwrap();
        let auth_events_locked = auth_events.lock().unwrap();
        assert!(core_events_locked
            .iter()
            .any(|e| e.contains("before_shutdown:CoreModule")));
        assert!(core_events_locked
            .iter()
            .any(|e| e.contains("after_shutdown:CoreModule")));
        assert!(auth_events_locked
            .iter()
            .any(|e| e.contains("before_shutdown:AuthModule")));
        assert!(auth_events_locked
            .iter()
            .any(|e| e.contains("after_shutdown:AuthModule")));
    }

    println!("✅ Module lifecycle hooks test passed");
}

/// Test 6: Health Check System
#[tokio::test]
async fn test_health_check_system() {
    let health_config = HealthCheckConfig {
        interval: Duration::from_millis(100),
        timeout: Duration::from_secs(1),
        enabled: true,
    };

    let mut runtime = ModuleRuntime::with_health_config(health_config);

    // Create modules with different health states
    runtime
        .register_module(create_test_module_with_services(
            "HealthyModule",
            vec![],
            1,
            0,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services("TestModule", vec![], 1, 0))
        .unwrap();

    runtime.add_lifecycle_hook("HealthyModule".to_string(), DefaultLifecycleHook);

    // Custom hook that reports degraded health
    struct DegradedHealthHook;
    impl ModuleLifecycleHook for DegradedHealthHook {
        fn health_check(&self, _module_name: &str) -> Result<HealthStatus, ModuleRuntimeError> {
            Ok(HealthStatus::Degraded)
        }
    }
    runtime.add_lifecycle_hook("TestModule".to_string(), DegradedHealthHook);

    runtime.calculate_load_order().unwrap();

    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    // Run health checks
    let health_results = runtime.health_check_all_modules().await.unwrap();

    assert_eq!(health_results.len(), 2);
    assert_eq!(health_results["HealthyModule"], HealthStatus::Healthy);
    assert_eq!(health_results["TestModule"], HealthStatus::Degraded);

    // Verify health status is updated in module info
    let healthy_info = runtime.get_module_info("HealthyModule").unwrap();
    let test_info = runtime.get_module_info("TestModule").unwrap();

    assert_eq!(healthy_info.health_status, HealthStatus::Healthy);
    assert_eq!(test_info.health_status, HealthStatus::Degraded);
    assert!(healthy_info.last_health_check.is_some());
    assert!(test_info.last_health_check.is_some());

    println!("✅ Health check system test passed");
}

/// Test 7: Large Scale Performance Test (50+ modules)
#[tokio::test]
async fn test_large_scale_performance() {
    let mut runtime = ModuleRuntime::new();

    // Create a large dependency graph with 60 modules
    const MODULE_COUNT: usize = 60;

    // Create base modules (no dependencies)
    for i in 0..10 {
        runtime
            .register_module(create_test_module_with_services(
                &format!("Base{}", i),
                vec![],
                2,
                1,
            ))
            .unwrap();
    }

    // Create intermediate modules (depend on base modules)
    for i in 0..25 {
        let deps = vec![format!("Base{}", i % 10), format!("Base{}", (i + 1) % 10)];
        runtime
            .register_module(create_test_module_with_services(
                &format!("Middle{}", i),
                deps,
                1,
                2,
            ))
            .unwrap();
    }

    // Create top-level modules (depend on intermediate modules)
    for i in 0..25 {
        let deps = vec![format!("Middle{}", i), format!("Middle{}", (i + 5) % 25)];
        runtime
            .register_module(create_test_module_with_services(
                &format!("Top{}", i),
                deps,
                3,
                0,
            ))
            .unwrap();
    }

    let start_time = std::time::Instant::now();

    // Test topological sorting performance
    let load_order = runtime.calculate_load_order().unwrap();
    let sort_duration = start_time.elapsed();

    assert_eq!(load_order.len(), MODULE_COUNT);
    println!(
        "⚡ Topological sort of {} modules completed in {:?}",
        MODULE_COUNT, sort_duration
    );

    // Test dependency resolution and initialization performance
    let mut container = IocContainer::new();
    container.build().unwrap();

    let dep_start = std::time::Instant::now();
    runtime.resolve_dependencies(&mut container).await.unwrap();
    let dep_duration = dep_start.elapsed();

    let init_start = std::time::Instant::now();
    runtime.initialize_all_modules(&container).await.unwrap();
    let init_duration = init_start.elapsed();

    println!("⚡ Dependency resolution completed in {:?}", dep_duration);
    println!("⚡ Module initialization completed in {:?}", init_duration);

    // Verify performance metrics
    let metrics = runtime.get_performance_metrics();
    assert_eq!(metrics.total_modules, MODULE_COUNT);
    assert!(metrics.avg_init_time_per_module > Duration::ZERO);
    assert!(metrics.slowest_module.is_some());

    // Performance assertions (should complete within reasonable time)
    assert!(
        sort_duration < Duration::from_millis(100),
        "Topological sort took too long: {:?}",
        sort_duration
    );
    assert!(
        dep_duration < Duration::from_millis(500),
        "Dependency resolution took too long: {:?}",
        dep_duration
    );
    assert!(
        init_duration < Duration::from_secs(2),
        "Initialization took too long: {:?}",
        init_duration
    );

    // Verify all modules are ready
    let stats = runtime.get_runtime_statistics();
    assert_eq!(stats.ready_modules, MODULE_COUNT);
    assert_eq!(stats.failed_modules, 0);

    println!(
        "✅ Large scale performance test passed ({} modules)",
        MODULE_COUNT
    );
}

/// Test 8: Error Recovery and Validation
#[tokio::test]
async fn test_error_recovery_and_validation() {
    let mut runtime = ModuleRuntime::new();

    // Add some valid modules
    runtime
        .register_module(create_test_module_with_services(
            "ValidModule1",
            vec![],
            1,
            1,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "ValidModule2",
            vec!["ValidModule1".to_string()],
            2,
            0,
        ))
        .unwrap();

    // Try to register duplicate module (should fail)
    let duplicate_result = runtime.register_module(create_test_module_with_services(
        "ValidModule1",
        vec![],
        1,
        0,
    ));
    assert!(duplicate_result.is_err());
    match duplicate_result.unwrap_err() {
        ModuleRuntimeError::ConfigurationConflict {
            module1,
            module2,
            conflict,
        } => {
            assert_eq!(module1, "ValidModule1");
            assert_eq!(module2, "ValidModule1");
            assert!(conflict.contains("already registered"));
        }
        _ => panic!("Expected ConfigurationConflict error"),
    }

    runtime.calculate_load_order().unwrap();

    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    // Test runtime state validation
    let validation_result = runtime.validate_runtime_state();
    assert!(validation_result.is_ok());

    // For now, just verify that validation works with good state
    // In a real implementation, we'd simulate module failures through proper interfaces

    println!("✅ Error recovery and validation test passed");
}

/// Test 9: Shutdown Order Verification
#[tokio::test]
async fn test_shutdown_order_verification() {
    let mut runtime = ModuleRuntime::new();

    // Create dependency chain
    runtime
        .register_module(create_test_module_with_services("A", vec![], 1, 0))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "B",
            vec!["A".to_string()],
            1,
            0,
        ))
        .unwrap();
    runtime
        .register_module(create_test_module_with_services(
            "C",
            vec!["B".to_string()],
            1,
            0,
        ))
        .unwrap();

    // Add hooks to track shutdown order
    let shutdown_order = Arc::new(Mutex::new(Vec::new()));

    for module_name in ["A", "B", "C"] {
        let order_clone = shutdown_order.clone();
        let name = module_name.to_string();

        struct ShutdownOrderHook {
            order: Arc<Mutex<Vec<String>>>,
            name: String,
        }

        impl ModuleLifecycleHook for ShutdownOrderHook {
            fn before_shutdown(&self, _: &str) -> Result<(), ModuleRuntimeError> {
                self.order.lock().unwrap().push(self.name.clone());
                Ok(())
            }
        }

        runtime.add_lifecycle_hook(
            module_name.to_string(),
            ShutdownOrderHook {
                order: order_clone,
                name,
            },
        );
    }

    runtime.calculate_load_order().unwrap();

    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    // Shutdown should happen in reverse dependency order (C -> B -> A)
    runtime.shutdown_all_modules().await.unwrap();

    let final_shutdown_order = shutdown_order.lock().unwrap().clone();
    assert_eq!(final_shutdown_order, vec!["C", "B", "A"]);

    // Verify all modules are shut down
    for module_name in ["A", "B", "C"] {
        let info = runtime.get_module_info(module_name).unwrap();
        assert_eq!(info.state, ModuleState::Shutdown);
    }

    println!("✅ Shutdown order verification test passed");
}

/// Test 10: Comprehensive Integration Test
#[tokio::test]
async fn test_comprehensive_integration() {
    let mut runtime = ModuleRuntime::new();

    // Create a realistic application structure:
    // CoreModule (logging, config)
    // DatabaseModule -> CoreModule
    // AuthModule -> CoreModule, DatabaseModule
    // ApiModule -> AuthModule, DatabaseModule
    // WebModule -> ApiModule, AuthModule
    // AdminModule -> WebModule, DatabaseModule

    let modules = vec![
        ("CoreModule", vec![], 3, 0),
        ("DatabaseModule", vec!["CoreModule"], 2, 0),
        ("AuthModule", vec!["CoreModule", "DatabaseModule"], 4, 1),
        ("ApiModule", vec!["AuthModule", "DatabaseModule"], 6, 3),
        ("WebModule", vec!["ApiModule", "AuthModule"], 2, 5),
        ("AdminModule", vec!["WebModule", "DatabaseModule"], 1, 2),
    ];

    for (name, deps, service_count, controller_count) in modules {
        runtime
            .register_module(create_test_module_with_services(
                name,
                deps.into_iter().map(String::from).collect(),
                service_count,
                controller_count,
            ))
            .unwrap();
    }

    // Add comprehensive lifecycle hooks
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    for module_name in [
        "CoreModule",
        "DatabaseModule",
        "AuthModule",
        "ApiModule",
        "WebModule",
        "AdminModule",
    ] {
        let events_clone = lifecycle_events.clone();
        let name = module_name.to_string();

        struct ComprehensiveHook {
            events: Arc<Mutex<Vec<String>>>,
            name: String,
        }

        impl ModuleLifecycleHook for ComprehensiveHook {
            fn before_init(&self, _: &str) -> Result<(), ModuleRuntimeError> {
                self.events
                    .lock()
                    .unwrap()
                    .push(format!("init_start:{}", self.name));
                Ok(())
            }

            fn after_init(&self, _: &str, duration: Duration) -> Result<(), ModuleRuntimeError> {
                self.events
                    .lock()
                    .unwrap()
                    .push(format!("init_end:{}:{:?}", self.name, duration));
                Ok(())
            }

            fn health_check(&self, _: &str) -> Result<HealthStatus, ModuleRuntimeError> {
                self.events
                    .lock()
                    .unwrap()
                    .push(format!("health:{}", self.name));
                Ok(HealthStatus::Healthy)
            }
        }

        runtime.add_lifecycle_hook(
            name.clone(),
            ComprehensiveHook {
                events: events_clone,
                name,
            },
        );
    }

    // Test complete lifecycle
    let start_time = std::time::Instant::now();

    let load_order = runtime.calculate_load_order().unwrap();
    println!("📋 Load order: {}", load_order.join(" -> "));

    // Verify load order constraints
    let core_pos = load_order.iter().position(|x| x == "CoreModule").unwrap();
    let db_pos = load_order
        .iter()
        .position(|x| x == "DatabaseModule")
        .unwrap();
    let auth_pos = load_order.iter().position(|x| x == "AuthModule").unwrap();
    let api_pos = load_order.iter().position(|x| x == "ApiModule").unwrap();
    let web_pos = load_order.iter().position(|x| x == "WebModule").unwrap();
    let admin_pos = load_order.iter().position(|x| x == "AdminModule").unwrap();

    assert!(core_pos < db_pos);
    assert!(core_pos < auth_pos && db_pos < auth_pos);
    assert!(auth_pos < api_pos && db_pos < api_pos);
    assert!(api_pos < web_pos && auth_pos < web_pos);
    assert!(web_pos < admin_pos && db_pos < admin_pos);

    let mut container = IocContainer::new();
    container.build().unwrap();

    runtime.resolve_dependencies(&mut container).await.unwrap();
    runtime.initialize_all_modules(&container).await.unwrap();

    let init_time = start_time.elapsed();
    println!("⏱️ Total initialization time: {:?}", init_time);

    // Run health checks
    let health_results = runtime.health_check_all_modules().await.unwrap();
    assert_eq!(health_results.len(), 6);
    assert!(health_results
        .values()
        .all(|status| *status == HealthStatus::Healthy));

    // Verify runtime statistics
    let stats = runtime.get_runtime_statistics();
    assert_eq!(stats.total_modules, 6);
    assert_eq!(stats.ready_modules, 6);
    assert_eq!(stats.healthy_modules, 6);

    // Test validation
    runtime.validate_runtime_state().unwrap();

    // Test shutdown
    runtime.shutdown_all_modules().await.unwrap();

    // Verify all events were recorded
    let events = lifecycle_events.lock().unwrap();
    assert!(events.iter().any(|e| e.contains("init_start:CoreModule")));
    assert!(events.iter().any(|e| e.contains("health:AdminModule")));

    // Get performance metrics after shutdown
    let metrics = runtime.get_performance_metrics();
    assert!(metrics.avg_init_time_per_module > Duration::ZERO);
    assert!(metrics.slowest_module.is_some());

    println!("✅ Comprehensive integration test passed");
    println!(
        "📊 Final stats: {} modules, {:?} avg init time",
        stats.total_modules, metrics.avg_init_time_per_module
    );
}
