//! Hydration Performance Comparison Benchmark
//!
//! Tests our JSON-based hydration vs direct sqlx access to understand
//! the performance difference for model deserialization

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use elif_orm::tests::mapping_tests::{MockDatabaseRow, TestUser};
use elif_orm::{Model, ModelResult};
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

fn bench_hydration_approaches(c: &mut Criterion) {
    let mut group = c.benchmark_group("hydration_comparison");
    
    // Test data setup
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    
    // Our JSON-based approach
    let mock_row = MockDatabaseRow::new()
        .with_column("id", user_id.to_string())
        .with_column("name", "John Doe")
        .with_column("email", "john@example.com")
        .with_column("age", 30)
        .with_column("active", true)
        .with_column("created_at", now.to_rfc3339())
        .with_column("updated_at", Value::Null);
    
    // Benchmark our current JSON-based hydration
    group.bench_function("json_based_hydration", |b| {
        b.iter(|| {
            let user: TestUser = TestUser::from_row(black_box(&mock_row)).unwrap();
            black_box(user)
        })
    });
    
    // Simulate what direct access would look like
    group.bench_function("direct_field_access", |b| {
        b.iter(|| {
            // Simulate direct field access (what Diesel-like approach would do)
            let user = TestUser {
                id: black_box(user_id),
                name: black_box("John Doe".to_string()),
                email: black_box("john@example.com".to_string()),
                age: black_box(Some(30)),
                active: black_box(true),
                created_at: black_box(now),
                updated_at: black_box(None),
            };
            black_box(user)
        })
    });
    
    // Test bulk hydration scenarios
    for &count in &[10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("bulk_json_hydration", count),
            &count,
            |b, &count| {
                let rows: Vec<_> = (0..count)
                    .map(|i| {
                        MockDatabaseRow::new()
                            .with_column("id", Uuid::new_v4().to_string())
                            .with_column("name", format!("User {}", i))
                            .with_column("email", format!("user{}@example.com", i))
                            .with_column("age", 20 + (i % 50))
                            .with_column("active", i % 2 == 0)
                            .with_column("created_at", now.to_rfc3339())
                            .with_column("updated_at", Value::Null)
                    })
                    .collect();
                
                b.iter(|| {
                    let users: Result<Vec<TestUser>, _> = rows
                        .iter()
                        .map(|row| TestUser::from_row(black_box(row)))
                        .collect();
                    black_box(users.unwrap())
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("bulk_direct_access", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let users: Vec<TestUser> = (0..count)
                        .map(|i| {
                            TestUser {
                                id: Uuid::new_v4(),
                                name: format!("User {}", i),
                                email: format!("user{}@example.com", i),
                                age: Some(20 + (i % 50)),
                                active: i % 2 == 0,
                                created_at: now,
                                updated_at: None,
                            }
                        })
                        .collect();
                    black_box(users)
                })
            },
        );
    }
    
    group.finish();
}

fn bench_individual_field_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_access_patterns");
    
    let mock_row = MockDatabaseRow::new()
        .with_column("id", Uuid::new_v4().to_string())
        .with_column("name", "John Doe")
        .with_column("email", "john@example.com")
        .with_column("age", 30)
        .with_column("active", true);
    
    // Test individual field access patterns
    group.bench_function("string_field_access", |b| {
        b.iter(|| {
            let name: String = mock_row.get(black_box("name")).unwrap();
            black_box(name)
        })
    });
    
    group.bench_function("integer_field_access", |b| {
        b.iter(|| {
            let age: i32 = mock_row.get(black_box("age")).unwrap();
            black_box(age)
        })
    });
    
    group.bench_function("boolean_field_access", |b| {
        b.iter(|| {
            let active: bool = mock_row.get(black_box("active")).unwrap();
            black_box(active)
        })
    });
    
    group.bench_function("optional_field_access", |b| {
        b.iter(|| {
            let age: Option<i32> = mock_row.try_get(black_box("age")).unwrap();
            black_box(age)
        })
    });
    
    group.finish();
}

fn bench_memory_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    // Test memory allocation during hydration
    group.bench_function("json_conversion_overhead", |b| {
        let raw_value = json!("test_string");
        
        b.iter(|| {
            // Simulate our JSON conversion path
            let db_value = elif_orm::backends::DatabaseValue::from_json(black_box(raw_value.clone()));
            let json_back = db_value.to_json();
            let final_value: String = serde_json::from_value(json_back).unwrap();
            black_box(final_value)
        })
    });
    
    group.bench_function("direct_value_access", |b| {
        b.iter(|| {
            // Direct access without JSON conversion
            let value = black_box("test_string".to_string());
            black_box(value)
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_hydration_approaches,
    bench_individual_field_access, 
    bench_memory_allocation_patterns
);
criterion_main!(benches);