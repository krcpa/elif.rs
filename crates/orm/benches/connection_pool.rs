//! Connection Pool Performance Benchmarks
//!
//! Tests connection acquisition, release, and pool management performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use elif_orm::database::{DatabasePool, PoolConfig};
use std::time::Duration;
use tokio::runtime::Runtime;

async fn bench_pool_acquisition(pool: &DatabasePool, operations: usize) {
    for _ in 0..operations {
        if let Ok(conn) = pool.acquire().await {
            // Simulate minimal work with connection
            let _ = black_box(conn);
            // Connection is automatically returned to pool when dropped
        }
    }
}

fn bench_pool_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("pool_creation");

    // Different pool sizes
    for &max_connections in &[5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("create_pool", max_connections),
            &max_connections,
            |b, &max_connections| {
                b.iter(|| {
                    rt.block_on(async {
                        let config = PoolConfig::builder()
                            .max_connections(black_box(max_connections))
                            .min_connections(black_box(1))
                            .connection_timeout(Duration::from_secs(30))
                            .build_with_defaults();

                        // Note: In a real benchmark, we'd create actual pools
                        // For now, just benchmark the config creation
                        black_box(config)
                    })
                })
            },
        );
    }

    group.finish();
}

fn bench_pool_config_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_config_builder");

    group.bench_function("simple_config", |b| {
        b.iter(|| {
            let config = PoolConfig::builder()
                .max_connections(black_box(20))
                .build_with_defaults();

            black_box(config)
        })
    });

    group.bench_function("complex_config", |b| {
        b.iter(|| {
            let config = PoolConfig::builder()
                .max_connections(black_box(50))
                .min_connections(black_box(5))
                .connection_timeout(Duration::from_secs(black_box(30)))
                .idle_timeout(Duration::from_secs(black_box(600)))
                .max_lifetime(Duration::from_secs(black_box(3600)))
                .test_before_acquire(black_box(true))
                .build_with_defaults();

            black_box(config)
        })
    });

    group.finish();
}

// Mock pool for benchmarking pool-like behavior without real DB connections
#[derive(Clone)]
struct MockPool {
    max_connections: usize,
    current_connections: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl MockPool {
    fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            current_connections: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    async fn acquire(&self) -> Result<MockConnection, String> {
        use std::sync::atomic::Ordering;

        let current = self.current_connections.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_connections {
            self.current_connections.fetch_sub(1, Ordering::SeqCst);
            return Err("Pool exhausted".to_string());
        }

        // Simulate connection acquisition delay
        tokio::time::sleep(Duration::from_micros(100)).await;

        Ok(MockConnection { pool: self.clone() })
    }
}

struct MockConnection {
    pool: MockPool,
}

impl Drop for MockConnection {
    fn drop(&mut self) {
        self.pool
            .current_connections
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

fn bench_mock_pool_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("mock_pool_operations");

    // Sequential acquisition
    for &pool_size in &[5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("sequential_acquisition", pool_size),
            &pool_size,
            |b, &pool_size| {
                let pool = MockPool::new(pool_size);
                b.iter(|| {
                    rt.block_on(async {
                        let mut connections = Vec::new();

                        // Acquire connections up to pool limit
                        for _ in 0..pool_size {
                            if let Ok(conn) = pool.acquire().await {
                                connections.push(conn);
                            }
                        }

                        black_box(connections)
                        // Connections are dropped here, returning to pool
                    })
                })
            },
        );
    }

    // Concurrent acquisition
    for &concurrency in &[2, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_acquisition", concurrency),
            &concurrency,
            |b, &concurrency| {
                let pool = MockPool::new(20);
                b.iter(|| {
                    rt.block_on(async {
                        let handles: Vec<_> = (0..concurrency)
                            .map(|_| {
                                let pool = pool.clone();
                                tokio::spawn(async move {
                                    let mut connections = Vec::new();
                                    for _ in 0..5 {
                                        if let Ok(conn) = pool.acquire().await {
                                            connections.push(conn);
                                        }
                                        tokio::time::sleep(Duration::from_micros(10)).await;
                                    }
                                    black_box(connections)
                                })
                            })
                            .collect();

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    })
                })
            },
        );
    }

    group.finish();
}

fn bench_pool_stress_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("pool_stress_patterns");

    group.bench_function("rapid_acquire_release", |b| {
        let pool = MockPool::new(10);
        b.iter(|| {
            rt.block_on(async {
                // Rapidly acquire and release connections
                for _ in 0..50 {
                    if let Ok(conn) = pool.acquire().await {
                        let _ = black_box(conn);
                        // Connection released when it goes out of scope
                    }
                }
            })
        })
    });

    group.bench_function("mixed_hold_times", |b| {
        let pool = MockPool::new(15);
        b.iter(|| {
            rt.block_on(async {
                let mut long_lived = Vec::new();

                // Some connections held for longer
                for i in 0..5 {
                    if let Ok(conn) = pool.acquire().await {
                        if i % 2 == 0 {
                            long_lived.push(conn);
                        }
                        // else: short-lived connection dropped immediately
                    }
                }

                // Additional short-lived connections
                for _ in 0..20 {
                    if let Ok(conn) = pool.acquire().await {
                        let _ = black_box(conn);
                    }
                }

                black_box(long_lived)
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pool_creation,
    bench_pool_config_builder,
    bench_mock_pool_operations,
    bench_pool_stress_patterns
);
criterion_main!(benches);
