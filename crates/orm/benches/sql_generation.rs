//! SQL Generation Performance Benchmarks
//!
//! Tests the performance of SQL generation, template caching, and string manipulation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use elif_orm::query::QueryBuilder;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

fn bench_basic_sql_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_sql_generation");

    // Simple SELECT queries
    group.bench_function("simple_select", |b| {
        let query: QueryBuilder<()> = QueryBuilder::new().from("users").select("id, name, email");

        b.iter(|| black_box(query.to_sql()))
    });

    // SELECT with WHERE
    group.bench_function("select_with_where", |b| {
        let query: QueryBuilder<()> = QueryBuilder::new()
            .from("users")
            .select("id, name, email")
            .where_eq("active", "true")
            .where_gt("created_at", "2023-01-01");

        b.iter(|| black_box(query.to_sql()))
    });

    // Complex SELECT with JOINs
    group.bench_function("select_with_joins", |b| {
        let query: QueryBuilder<()> = QueryBuilder::new()
            .from("users u")
            .select("u.id, u.name, p.title, a.street")
            .join("profiles p", "p.user_id", "u.id")
            .left_join("addresses a", "a.user_id", "u.id")
            .where_eq("u.active", "true")
            .where_not_null("p.title")
            .order_by_desc("u.created_at")
            .limit(50);

        b.iter(|| black_box(query.to_sql()))
    });

    group.finish();
}

fn bench_sql_template_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_template_patterns");

    // Test different approaches to SQL building
    group.bench_function("string_concatenation", |b| {
        b.iter(|| {
            let mut sql = String::new();
            sql.push_str("SELECT ");
            sql.push_str(black_box("id, name, email"));
            sql.push_str(" FROM ");
            sql.push_str(black_box("users"));
            sql.push_str(" WHERE ");
            sql.push_str(black_box("active = $1"));
            sql.push_str(" AND ");
            sql.push_str(black_box("created_at > $2"));
            sql.push_str(" ORDER BY ");
            sql.push_str(black_box("created_at DESC"));
            sql.push_str(" LIMIT ");
            sql.push_str(black_box("50"));

            black_box(sql)
        })
    });

    group.bench_function("format_macro", |b| {
        b.iter(|| {
            let sql = format!(
                "SELECT {} FROM {} WHERE {} AND {} ORDER BY {} LIMIT {}",
                black_box("id, name, email"),
                black_box("users"),
                black_box("active = $1"),
                black_box("created_at > $2"),
                black_box("created_at DESC"),
                black_box("50")
            );

            black_box(sql)
        })
    });

    group.bench_function("vec_join", |b| {
        b.iter(|| {
            let parts = vec![
                black_box("SELECT id, name, email"),
                black_box("FROM users"),
                black_box("WHERE active = $1"),
                black_box("AND created_at > $2"),
                black_box("ORDER BY created_at DESC"),
                black_box("LIMIT 50"),
            ];

            let sql = parts.join(" ");
            black_box(sql)
        })
    });

    group.finish();
}

fn bench_parameter_placeholder_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("parameter_placeholders");

    for &param_count in &[1, 5, 10, 25, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("generate_placeholders", param_count),
            &param_count,
            |b, &param_count| {
                b.iter(|| {
                    let mut placeholders = Vec::with_capacity(param_count);
                    for i in 1..=param_count {
                        placeholders.push(format!("${}", i));
                    }
                    let result = placeholders.join(", ");
                    black_box(result)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached_placeholders", param_count),
            &param_count,
            |b, &param_count| {
                // Use thread-safe caching with once_cell::sync::Lazy
                static CACHE: Lazy<RwLock<HashMap<usize, String>>> =
                    Lazy::new(|| RwLock::new(HashMap::new()));

                b.iter(|| {
                    // Try to read from cache first
                    if let Ok(cache_read) = CACHE.read() {
                        if let Some(cached) = cache_read.get(&param_count) {
                            return black_box(cached.clone());
                        }
                    }

                    // Generate new placeholders and cache them
                    let mut placeholders = Vec::with_capacity(param_count);
                    for i in 1..=param_count {
                        placeholders.push(format!("${}", i));
                    }
                    let result = placeholders.join(", ");

                    if let Ok(mut cache_write) = CACHE.write() {
                        cache_write.insert(param_count, result.clone());
                    }

                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn bench_identifier_escaping(c: &mut Criterion) {
    let mut group = c.benchmark_group("identifier_escaping");

    let test_identifiers = vec![
        "simple_table",
        "table_with_underscores",
        "TableWithCaps",
        "table123",
        "user",   // keyword
        "order",  // keyword
        "select", // keyword
        "very_long_table_name_that_might_need_escaping",
    ];

    group.bench_function("escape_identifiers", |b| {
        b.iter(|| {
            let mut escaped = Vec::new();
            for identifier in &test_identifiers {
                // Simulate identifier escaping logic
                let escaped_id = if identifier.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !identifier.chars().next().unwrap().is_ascii_digit()
                    && !matches!(*identifier, "user" | "order" | "select" | "from" | "where")
                {
                    identifier.to_string()
                } else {
                    format!("\"{}\"", identifier)
                };
                escaped.push(escaped_id);
            }
            black_box(escaped)
        })
    });

    group.finish();
}

fn bench_where_clause_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("where_clause_building");

    for &condition_count in &[1, 5, 10, 25, 50] {
        group.bench_with_input(
            BenchmarkId::new("build_where_clauses", condition_count),
            &condition_count,
            |b, &condition_count| {
                b.iter(|| {
                    let mut conditions = Vec::new();
                    let mut param_index = 1;

                    for i in 0..condition_count {
                        let condition = match i % 4 {
                            0 => format!("field_{} = ${}", i, param_index),
                            1 => format!("field_{} > ${}", i, param_index),
                            2 => format!("field_{} LIKE ${}", i, param_index),
                            _ => format!("field_{} IS NOT NULL", i),
                        };

                        conditions.push(condition);
                        if i % 4 != 3 {
                            param_index += 1;
                        }
                    }

                    let where_clause = if !conditions.is_empty() {
                        format!("WHERE {}", conditions.join(" AND "))
                    } else {
                        String::new()
                    };

                    black_box(where_clause)
                })
            },
        );
    }

    group.finish();
}

fn bench_join_clause_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("join_clause_building");

    for &join_count in &[1, 3, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("build_joins", join_count),
            &join_count,
            |b, &join_count| {
                b.iter(|| {
                    let mut joins = Vec::new();

                    for i in 0..join_count {
                        let join_type = match i % 3 {
                            0 => "INNER JOIN",
                            1 => "LEFT JOIN",
                            _ => "RIGHT JOIN",
                        };

                        let join_clause = format!(
                            "{} table_{} t{} ON t{}.parent_id = t0.id",
                            join_type, i, i, i
                        );

                        joins.push(join_clause);
                    }

                    let join_sql = joins.join(" ");
                    black_box(join_sql)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_sql_generation,
    bench_sql_template_patterns,
    bench_parameter_placeholder_generation,
    bench_identifier_escaping,
    bench_where_clause_building,
    bench_join_clause_building
);
criterion_main!(benches);
