use criterion::{black_box, criterion_group, criterion_main, Criterion};
use elif_cache::{CacheBackend, CacheConfig, MemoryBackend};
use std::time::Duration;
use tokio::runtime::Runtime;

async fn benchmark_lru_operations(backend: &MemoryBackend, num_operations: usize) {
    // Fill cache with initial data
    for i in 0..num_operations {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i).into_bytes();
        backend
            .put(&key, value, Some(Duration::from_secs(3600)))
            .await
            .unwrap();
    }

    // Perform access operations to test LRU performance
    for i in 0..num_operations {
        let key = format!("key_{}", i % (num_operations / 2)); // Access first half repeatedly
        backend.get(&key).await.unwrap();
    }
}

fn bench_lru_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("lru_tracker");

    // Benchmark different scales
    for &size in &[100, 500, 1000, 2000] {
        // Create a fresh backend for each size to ensure benchmark isolation
        let config = CacheConfig::builder()
            .max_entries_limit(1000) // Limited capacity
            .build_config();
        let backend = MemoryBackend::new(config);

        group.bench_with_input(format!("operations_{}", size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(benchmark_lru_operations(
                    black_box(&backend),
                    black_box(size),
                ))
            })
        });
    }

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_lru_access", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = CacheConfig::builder()
                    .max_entries_limit(1000)
                    .build_config();
                let backend = std::sync::Arc::new(MemoryBackend::new(config));

                let handles = (0..10).map(|i| {
                    let backend = backend.clone();
                    tokio::spawn(async move {
                        for j in 0..100 {
                            let key = format!("thread_{}_key_{}", i, j);
                            let value = format!("value_{}", j).into_bytes();
                            backend
                                .put(&key, value, Some(Duration::from_secs(60)))
                                .await
                                .unwrap();
                            backend.get(&key).await.unwrap();
                        }
                    })
                });

                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });
}

criterion_group!(benches, bench_lru_performance, bench_concurrent_access);
criterion_main!(benches);
