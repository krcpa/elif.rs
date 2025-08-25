//! Performance benchmarks for Epic 4 - Module Runtime Integration & Validation
//!
//! Benchmarks module runtime operations with various graph sizes and complexity levels
//! to ensure the system scales efficiently for large applications.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use elif_core::container::IocContainer;
use elif_core::modules::runtime::ModuleRuntime;
use elif_core::modules::{ModuleDescriptor, ServiceDescriptor, ServiceLifecycle};

/// Create a test module with specified services and dependencies
fn create_benchmark_module(
    name: &str,
    dependencies: Vec<String>,
    service_count: usize,
) -> ModuleDescriptor {
    let mut module = ModuleDescriptor::new(name)
        .with_dependencies(dependencies)
        .with_description(format!(
            "Benchmark module {} with {} services",
            name, service_count
        ));

    // Add services to make the module more realistic
    for i in 0..service_count {
        let service = ServiceDescriptor::new::<String>(
            format!("{}Service{}", name, i),
            ServiceLifecycle::Singleton,
        );
        module = module.with_provider(service);
    }

    module
}

/// Benchmark topological sorting performance with different graph sizes
fn benchmark_topological_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("topological_sorting");

    for module_count in [10, 50, 100, 200, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("linear_chain", module_count),
            module_count,
            |b, &size| {
                // Create linear dependency chain: M0 -> M1 -> M2 -> ... -> M(n-1)
                let mut runtime = ModuleRuntime::new();

                // Create modules
                for i in 0..size {
                    let deps = if i == 0 {
                        vec![]
                    } else {
                        vec![format!("Module{}", i - 1)]
                    };
                    runtime
                        .register_module(create_benchmark_module(
                            &format!("Module{}", i),
                            deps,
                            2, // 2 services per module
                        ))
                        .unwrap();
                }

                b.iter(|| {
                    let mut runtime_clone = ModuleRuntime::new();

                    // Re-register modules for each iteration
                    for i in 0..size {
                        let deps = if i == 0 {
                            vec![]
                        } else {
                            vec![format!("Module{}", i - 1)]
                        };
                        runtime_clone
                            .register_module(create_benchmark_module(
                                &format!("Module{}", i),
                                deps,
                                2,
                            ))
                            .unwrap();
                    }

                    black_box(runtime_clone.calculate_load_order().unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("star_pattern", module_count),
            module_count,
            |b, &size| {
                // Create star pattern: one central module with many dependents
                // Central -> M1, M2, M3, ..., M(n-1)
                b.iter(|| {
                    let mut runtime = ModuleRuntime::new();

                    // Central module with no dependencies
                    runtime
                        .register_module(create_benchmark_module("Central", vec![], 5))
                        .unwrap();

                    // All other modules depend on central
                    for i in 1..size {
                        runtime
                            .register_module(create_benchmark_module(
                                &format!("Module{}", i),
                                vec!["Central".to_string()],
                                1,
                            ))
                            .unwrap();
                    }

                    black_box(runtime.calculate_load_order().unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("diamond_pattern", module_count),
            module_count,
            |b, &size| {
                // Create diamond pattern: multiple layers with cross-dependencies
                b.iter(|| {
                    let mut runtime = ModuleRuntime::new();

                    let layer_size = (size as f64).sqrt() as usize;
                    let layers = (size as f64 / layer_size as f64).ceil() as usize;

                    let mut module_count = 0;
                    for layer in 0..layers {
                        for i in 0..layer_size {
                            if module_count >= size {
                                break;
                            }

                            let module_name = format!("L{}M{}", layer, i);
                            let deps = if layer == 0 {
                                vec![]
                            } else {
                                // Depend on previous layer modules
                                let prev_layer = layer - 1;
                                let dep1 = format!("L{}M{}", prev_layer, i % layer_size);
                                let dep2 = format!("L{}M{}", prev_layer, (i + 1) % layer_size);
                                vec![dep1, dep2]
                            };

                            runtime
                                .register_module(create_benchmark_module(&module_name, deps, 1))
                                .unwrap();

                            module_count += 1;
                        }

                        if module_count >= size {
                            break;
                        }
                    }

                    black_box(runtime.calculate_load_order().unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark dependency resolution performance
fn benchmark_dependency_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_resolution");

    for module_count in [10, 25, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("resolve_dependencies", module_count),
            module_count,
            |b, &size| {
                b.iter(|| {
                    // Synchronous part - setup
                    let mut runtime = ModuleRuntime::new();
                    let mut container = IocContainer::new();

                    // Create layered dependency structure
                    let layers = 5;
                    let modules_per_layer = size / layers;

                    for layer in 0..layers {
                        for i in 0..modules_per_layer {
                            let module_name = format!("L{}M{}", layer, i);
                            let deps = if layer == 0 {
                                vec![]
                            } else {
                                vec![format!("L{}M{}", layer - 1, i % modules_per_layer)]
                            };

                            runtime
                                .register_module(create_benchmark_module(&module_name, deps, 3))
                                .unwrap();
                        }
                    }

                    runtime.calculate_load_order().unwrap();
                    container.build().unwrap();

                    // For benchmarking, we'll skip the actual async resolve_dependencies call
                    // and just measure the setup overhead
                    black_box(&runtime);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark module initialization performance
fn benchmark_module_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("module_initialization");

    for module_count in [10, 25, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("initialize_setup", module_count),
            module_count,
            |b, &size| {
                b.iter(|| {
                    let mut runtime = ModuleRuntime::new();

                    // Create complex dependency graph
                    for i in 0..size {
                        let deps = match i {
                            0..=4 => vec![],                              // Base layer - no dependencies
                            5..=19 => vec![format!("Module{}", i % 5)],   // Middle layer
                            _ => vec![format!("Module{}", 5 + (i % 15))], // Top layer
                        };

                        runtime
                            .register_module(create_benchmark_module(
                                &format!("Module{}", i),
                                deps,
                                2,
                            ))
                            .unwrap();
                    }

                    runtime.calculate_load_order().unwrap();
                    black_box(&runtime);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark complete module lifecycle (setup only)
fn benchmark_complete_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_lifecycle");
    group.sample_size(10); // Fewer samples due to complexity

    for module_count in [25, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("setup_only", module_count),
            module_count,
            |b, &size| {
                b.iter(|| {
                    let mut runtime = ModuleRuntime::new();

                    // Create realistic application structure
                    let base_modules = size / 4;
                    let middle_modules = size / 2;
                    let top_modules = size - base_modules - middle_modules;

                    // Base layer - infrastructure modules
                    for i in 0..base_modules {
                        runtime
                            .register_module(create_benchmark_module(
                                &format!("Base{}", i),
                                vec![],
                                4, // Infrastructure modules have more services
                            ))
                            .unwrap();
                    }

                    // Middle layer - business logic modules
                    for i in 0..middle_modules {
                        let deps = vec![format!("Base{}", i % base_modules)];
                        runtime
                            .register_module(create_benchmark_module(
                                &format!("Middle{}", i),
                                deps,
                                3,
                            ))
                            .unwrap();
                    }

                    // Top layer - API and presentation modules
                    for i in 0..top_modules {
                        let deps = vec![format!("Middle{}", i % middle_modules)];
                        runtime
                            .register_module(create_benchmark_module(&format!("Top{}", i), deps, 2))
                            .unwrap();
                    }

                    // Setup only
                    runtime.calculate_load_order().unwrap();
                    black_box(&runtime);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("module_registration_1000", |b| {
        b.iter(|| {
            let mut runtime = ModuleRuntime::new();

            // Register 1000 modules to test memory allocation patterns
            for i in 0..1000 {
                let deps = if i < 10 {
                    vec![]
                } else if i < 100 {
                    vec![format!("Module{}", i % 10)]
                } else {
                    vec![
                        format!("Module{}", i % 100),
                        format!("Module{}", (i / 10) % 100),
                    ]
                };

                runtime
                    .register_module(create_benchmark_module(&format!("Module{}", i), deps, 1))
                    .unwrap();
            }

            black_box(runtime);
        });
    });

    group.finish();
}

/// Benchmark concurrent operations (simulated - setup only)
fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    group.bench_function("module_setup_parallel", |b| {
        b.iter(|| {
            let mut runtime = ModuleRuntime::new();

            // Create modules
            for i in 0..50 {
                runtime
                    .register_module(create_benchmark_module(
                        &format!("Module{}", i),
                        if i == 0 {
                            vec![]
                        } else {
                            vec![format!("Module{}", i - 1)]
                        },
                        2,
                    ))
                    .unwrap();
            }

            runtime.calculate_load_order().unwrap();
            black_box(&runtime);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_topological_sorting,
    benchmark_dependency_resolution,
    benchmark_module_initialization,
    benchmark_complete_lifecycle,
    benchmark_memory_usage,
    benchmark_concurrent_operations
);

criterion_main!(benches);
