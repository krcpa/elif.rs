//! Query Performance Benchmarks
//!
//! Benchmarks for testing query building and execution performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use elif_orm::{error::ModelResult, query::QueryBuilder, Model, PrimaryKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

// Test model for benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Model for BenchUser {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "bench_users"
    }
    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }

    fn to_fields(&self) -> ModelResult<HashMap<String, Value>> {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), serde_json::to_value(&self.id)?);
        fields.insert("name".to_string(), serde_json::to_value(&self.name)?);
        fields.insert("email".to_string(), serde_json::to_value(&self.email)?);
        fields.insert(
            "created_at".to_string(),
            serde_json::to_value(&self.created_at)?,
        );
        fields.insert(
            "updated_at".to_string(),
            serde_json::to_value(&self.updated_at)?,
        );
        Ok(fields)
    }

    fn from_row(row: &dyn elif_orm::model::DatabaseRow) -> ModelResult<Self> {
        unimplemented!("Not needed for benchmarks")
    }
}

impl PrimaryKey<Uuid> for BenchUser {
    fn primary_key_type() -> PrimaryKeyType {
        PrimaryKeyType::Uuid
    }
}

fn bench_query_builder_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_builder_creation");

    // Benchmark creating query builders of different complexities
    for &complexity in &[1, 5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("simple_select", complexity),
            &complexity,
            |b, &complexity| {
                b.iter(|| {
                    let mut builder: QueryBuilder<()> = QueryBuilder::new()
                        .from(black_box("bench_users"))
                        .select(black_box("id, name, email"));

                    // Add complexity through multiple where clauses
                    for i in 0..complexity {
                        builder = builder.where_eq(
                            black_box(&format!("field_{}", i)),
                            black_box(&format!("value_{}", i)),
                        );
                    }

                    black_box(builder)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("complex_query", complexity),
            &complexity,
            |b, &complexity| {
                b.iter(|| {
                    let mut builder = QueryBuilder::new()
                        .from(black_box("bench_users u"))
                        .select(black_box("u.id, u.name, u.email, p.title"))
                        .join(
                            black_box("profiles p"),
                            black_box("p.user_id"),
                            black_box("u.id"),
                        );

                    // Add complexity through multiple conditions
                    for i in 0..complexity {
                        builder = builder
                            .where_eq(
                                black_box(&format!("u.field_{}", i)),
                                black_box(&format!("value_{}", i)),
                            )
                            .where_like(
                                black_box(&format!("p.field_{}", i)),
                                black_box(&format!("%{}%", i)),
                            );
                    }

                    builder = builder
                        .group_by(black_box("u.id, p.id"))
                        .having(black_box("COUNT(*) > 1"))
                        .order_by_desc(black_box("u.created_at"))
                        .limit(black_box(100))
                        .offset(black_box(i64::from(complexity * 10)));

                    black_box(builder)
                })
            },
        );
    }

    group.finish();
}

fn bench_sql_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_generation");

    // Pre-build queries of different complexities
    let queries: Vec<(String, QueryBuilder)> = [1, 5, 10, 20, 50]
        .iter()
        .map(|&complexity| {
            let mut builder = QueryBuilder::new()
                .from("bench_users u")
                .select("u.id, u.name, u.email")
                .join("profiles p", "p.user_id", "u.id")
                .join("addresses a", "a.user_id", "u.id");

            for i in 0..complexity {
                builder = builder
                    .where_eq(&format!("u.field_{}", i), &format!("value_{}", i))
                    .where_in(
                        &format!("p.status_{}", i),
                        &[format!("active_{}", i), format!("inactive_{}", i)],
                    );
            }

            builder = builder
                .group_by("u.id, u.name")
                .having("COUNT(p.id) > 1")
                .order_by_desc("u.created_at")
                .limit(100)
                .offset(complexity as i64 * 10);

            (format!("complexity_{}", complexity), builder)
        })
        .collect();

    for (name, query) in queries {
        group.bench_function(&name, |b| b.iter(|| black_box(query.to_sql())));
    }

    group.finish();
}

fn bench_parameter_binding(c: &mut Criterion) {
    let mut group = c.benchmark_group("parameter_binding");

    for &param_count in &[1, 10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("bind_parameters", param_count),
            &param_count,
            |b, &param_count| {
                // Create test data
                let mut builder = QueryBuilder::new().from("bench_users").select("*");

                let values: Vec<String> =
                    (0..param_count).map(|i| format!("value_{}", i)).collect();

                b.iter(|| {
                    let mut query_builder = builder.clone();

                    // Add parameters through where_in clause
                    query_builder = query_builder.where_in(black_box("status"), black_box(&values));

                    black_box(query_builder.to_sql())
                })
            },
        );
    }

    group.finish();
}

fn bench_query_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_cloning");

    // Create queries of different sizes
    let queries: Vec<(String, QueryBuilder)> = [1, 5, 10, 25, 50]
        .iter()
        .map(|&size| {
            let mut builder = QueryBuilder::new().from("bench_users u").select("u.*");

            // Add multiple joins and where clauses
            for i in 0..size {
                builder = builder
                    .join(
                        &format!("table_{} t{}", i, i),
                        &format!("t{}.user_id", i),
                        "u.id",
                    )
                    .where_eq(&format!("u.field_{}", i), &format!("value_{}", i))
                    .where_not_null(&format!("t{}.status", i));
            }

            (format!("size_{}", size), builder)
        })
        .collect();

    for (name, query) in queries {
        group.bench_function(&name, |b| b.iter(|| black_box(query.clone())));
    }

    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Test memory allocation patterns
    group.bench_function("query_builder_allocation", |b| {
        b.iter(|| {
            let mut queries = Vec::new();

            for i in 0..100 {
                let query = QueryBuilder::new()
                    .from(&format!("table_{}", i))
                    .select("id, name, status")
                    .where_eq("active", "true")
                    .where_gt("created_at", "2023-01-01")
                    .order_by_desc("created_at")
                    .limit(50);

                queries.push(black_box(query));
            }

            black_box(queries)
        })
    });

    // Test reuse patterns
    group.bench_function("query_builder_reuse", |b| {
        let base_query = QueryBuilder::new()
            .from("base_table")
            .select("id, name, status");

        b.iter(|| {
            let mut results = Vec::new();

            for i in 0..100 {
                let specialized_query = base_query
                    .clone()
                    .where_eq("category_id", &format!("{}", i))
                    .limit(25);

                results.push(black_box(specialized_query));
            }

            black_box(results)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_query_builder_creation,
    bench_sql_generation,
    bench_parameter_binding,
    bench_query_cloning,
    bench_memory_usage
);
criterion_main!(benches);
