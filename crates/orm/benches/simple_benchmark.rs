//! Simple Performance Baseline Benchmark
//!
//! Basic benchmark to establish performance baselines and verify ORM Phase 3 improvements

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use elif_orm::query::QueryBuilder;

fn bench_basic_query_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_query_building");

    // Simple query creation
    group.bench_function("create_simple_query", |b| {
        b.iter(|| {
            let query: QueryBuilder<()> = QueryBuilder::new()
                .from(black_box("users"))
                .select(black_box("id, name, email"));

            black_box(query)
        })
    });

    // Query with conditions
    group.bench_function("query_with_where", |b| {
        b.iter(|| {
            let query: QueryBuilder<()> = QueryBuilder::new()
                .from(black_box("users"))
                .select(black_box("id, name, email"))
                .where_eq(black_box("active"), black_box("true"))
                .where_gt(black_box("created_at"), black_box("2023-01-01"));

            black_box(query)
        })
    });

    // Multiple condition complexity
    for &num_conditions in &[1, 5, 10, 25] {
        group.bench_with_input(
            BenchmarkId::new("multiple_conditions", num_conditions),
            &num_conditions,
            |b, &num_conditions| {
                b.iter(|| {
                    let mut query: QueryBuilder<()> = QueryBuilder::new()
                        .from(black_box("users"))
                        .select(black_box("id, name, email"));

                    for i in 0..num_conditions {
                        query = query.where_eq(
                            black_box(&format!("field_{}", i)),
                            black_box(format!("value_{}", i)),
                        );
                    }

                    black_box(query)
                })
            },
        );
    }

    group.finish();
}

fn bench_sql_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_generation");

    // Simple SQL generation
    let simple_query: QueryBuilder<()> =
        QueryBuilder::new().from("users").select("id, name, email");

    group.bench_function("simple_sql", |b| {
        b.iter(|| black_box(simple_query.to_sql()))
    });

    // Complex SQL generation
    let complex_query: QueryBuilder<()> = QueryBuilder::new()
        .from("users u")
        .select("u.id, u.name, p.title")
        .join("profiles p", "p.user_id", "u.id")
        .where_eq("u.active", "true")
        .where_not_null("p.title")
        .order_by_desc("u.created_at")
        .limit(50);

    group.bench_function("complex_sql", |b| {
        b.iter(|| black_box(complex_query.to_sql()))
    });

    group.finish();
}

fn bench_query_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_cloning");

    // Base query for cloning tests
    let base_query: QueryBuilder<()> = QueryBuilder::new()
        .from("users")
        .select("id, name, email")
        .where_eq("active", "true")
        .limit(100);

    group.bench_function("clone_simple_query", |b| {
        b.iter(|| black_box(base_query.clone()))
    });

    // Test query reuse pattern (common optimization)
    group.bench_function("reuse_base_query", |b| {
        b.iter(|| {
            let mut results = Vec::new();

            for i in 0..10 {
                let specialized = base_query.clone().where_eq("category_id", format!("{}", i));

                results.push(black_box(specialized));
            }

            black_box(results)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_query_building,
    bench_sql_generation,
    bench_query_cloning
);
criterion_main!(benches);
