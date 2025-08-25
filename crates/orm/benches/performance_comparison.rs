//! Performance Comparison Benchmark
//!
//! Compares optimized vs unoptimized query building and SQL generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use elif_orm::query::{
    acquire_query_builder, release_query_builder, QueryBuilder, QueryBuilderPool,
};

fn bench_sql_generation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_generation_comparison");

    // Create test queries of varying complexity
    let simple_query: QueryBuilder<()> = QueryBuilder::new()
        .from("users")
        .select("id, name, email")
        .where_eq("active", "true");

    let complex_query: QueryBuilder<()> = QueryBuilder::new()
        .from("users u")
        .select("u.id, u.name, p.title, a.street")
        .join("profiles p", "p.user_id", "u.id")
        .left_join("addresses a", "a.user_id", "u.id")
        .where_eq("u.active", "true")
        .where_not_null("p.title")
        .where_gt("u.created_at", "2023-01-01")
        .order_by_desc("u.created_at")
        .limit(50);

    // Benchmark simple query generation
    group.bench_function("simple_query_original", |b| {
        b.iter(|| black_box(simple_query.to_sql()))
    });

    group.bench_function("simple_query_optimized", |b| {
        b.iter(|| black_box(simple_query.to_sql_optimized()))
    });

    // Benchmark complex query generation
    group.bench_function("complex_query_original", |b| {
        b.iter(|| black_box(complex_query.to_sql()))
    });

    group.bench_function("complex_query_optimized", |b| {
        b.iter(|| black_box(complex_query.to_sql_optimized()))
    });

    group.finish();
}

fn bench_placeholder_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("placeholder_generation");

    for &count in &[1, 5, 10, 25, 50, 100] {
        group.bench_with_input(BenchmarkId::new("uncached", count), &count, |b, &count| {
            b.iter(|| {
                let placeholders: String = (1..=count)
                    .map(|i| format!("${}", i))
                    .collect::<Vec<_>>()
                    .join(", ");
                black_box(placeholders)
            })
        });

        group.bench_with_input(BenchmarkId::new("cached", count), &count, |b, &count| {
            b.iter(|| {
                let placeholders =
                    QueryBuilder::<()>::generate_placeholders_cached(black_box(count));
                black_box(placeholders)
            })
        });
    }

    group.finish();
}

fn bench_query_builder_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_builder_reuse");

    // Test traditional approach - create new builders each time
    group.bench_function("create_new_builders", |b| {
        b.iter(|| {
            let mut queries = Vec::new();

            for i in 0..10 {
                let query: QueryBuilder<()> = QueryBuilder::new()
                    .from("users")
                    .select("id, name, email")
                    .where_eq("status", format!("status_{}", i))
                    .limit(50);

                queries.push(black_box(query.to_sql()));
            }

            black_box(queries)
        })
    });

    // Test pool approach - reuse builders
    group.bench_function("reuse_pooled_builders", |b| {
        let pool = QueryBuilderPool::new(5);

        b.iter(|| {
            let mut queries = Vec::new();

            for i in 0..10 {
                let query = pool
                    .acquire()
                    .from("users")
                    .select("id, name, email")
                    .where_eq("status", format!("status_{}", i))
                    .limit(50);

                queries.push(black_box(query.to_sql()));
                pool.release(query);
            }

            black_box(queries)
        })
    });

    // Test global pool approach
    group.bench_function("global_pool_builders", |b| {
        b.iter(|| {
            let mut queries = Vec::new();

            for i in 0..10 {
                let query = acquire_query_builder()
                    .from("users")
                    .select("id, name, email")
                    .where_eq("status", format!("status_{}", i))
                    .limit(50);

                queries.push(black_box(query.to_sql()));
                release_query_builder(query);
            }

            black_box(queries)
        })
    });

    group.finish();
}

fn bench_memory_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation_patterns");

    // Test string concatenation approaches
    group.bench_function("string_push_str", |b| {
        b.iter(|| {
            let mut sql = String::new();
            sql.push_str("SELECT ");
            sql.push_str("id, name, email");
            sql.push_str(" FROM ");
            sql.push_str("users");
            sql.push_str(" WHERE ");
            sql.push_str("active = $1");
            sql.push_str(" ORDER BY ");
            sql.push_str("created_at DESC");
            sql.push_str(" LIMIT 50");

            black_box(sql)
        })
    });

    group.bench_function("string_with_capacity", |b| {
        b.iter(|| {
            let mut sql = String::with_capacity(100);
            sql.push_str("SELECT ");
            sql.push_str("id, name, email");
            sql.push_str(" FROM ");
            sql.push_str("users");
            sql.push_str(" WHERE ");
            sql.push_str("active = $1");
            sql.push_str(" ORDER BY ");
            sql.push_str("created_at DESC");
            sql.push_str(" LIMIT 50");

            black_box(sql)
        })
    });

    group.bench_function("format_macro", |b| {
        b.iter(|| {
            let sql = format!(
                "SELECT {} FROM {} WHERE {} ORDER BY {} LIMIT {}",
                "id, name, email", "users", "active = $1", "created_at DESC", "50"
            );

            black_box(sql)
        })
    });

    group.finish();
}

fn bench_where_clause_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("where_clause_building");

    for &num_conditions in &[1, 5, 10, 25, 50] {
        group.bench_with_input(
            BenchmarkId::new("traditional_approach", num_conditions),
            &num_conditions,
            |b, &num_conditions| {
                b.iter(|| {
                    let mut query: QueryBuilder<()> =
                        QueryBuilder::new().from("users").select("id, name, email");

                    for i in 0..num_conditions {
                        query = query.where_eq(&format!("field_{}", i), format!("value_{}", i));
                    }

                    black_box(query.to_sql())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized_approach", num_conditions),
            &num_conditions,
            |b, &num_conditions| {
                b.iter(|| {
                    let mut query = acquire_query_builder()
                        .from("users")
                        .select("id, name, email");

                    for i in 0..num_conditions {
                        query = query.where_eq(&format!("field_{}", i), format!("value_{}", i));
                    }

                    let sql = query.to_sql_optimized();
                    release_query_builder(query);

                    black_box(sql)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sql_generation_comparison,
    bench_placeholder_generation,
    bench_query_builder_reuse,
    bench_memory_allocation_patterns,
    bench_where_clause_building
);
criterion_main!(benches);
