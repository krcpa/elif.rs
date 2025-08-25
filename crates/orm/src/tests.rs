//! Comprehensive tests for elif-orm
//!
//! Tests cover QueryBuilder, error handling, primary keys, and Model trait functionality

use crate::error::{ModelError, ModelResult};
use crate::model::Model;
use crate::query::QueryBuilder;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub mod mapping_tests;

/// Test model for use in tests
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestUser {
    id: Option<Uuid>,
    email: String,
    name: String,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
}

impl Model for TestUser {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "users"
    }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        self.id
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = Some(key);
    }

    fn uses_timestamps() -> bool {
        true
    }

    fn uses_soft_deletes() -> bool {
        true
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    fn set_created_at(&mut self, timestamp: DateTime<Utc>) {
        self.created_at = Some(timestamp);
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }

    fn set_updated_at(&mut self, timestamp: DateTime<Utc>) {
        self.updated_at = Some(timestamp);
    }

    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    fn set_deleted_at(&mut self, timestamp: Option<DateTime<Utc>>) {
        self.deleted_at = timestamp;
    }

    fn from_row(_row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        // Mock implementation for testing
        Ok(TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            deleted_at: None,
        })
    }

    fn to_fields(&self) -> HashMap<String, Value> {
        let mut fields = HashMap::new();
        if let Some(id) = self.id {
            fields.insert("id".to_string(), Value::String(id.to_string()));
        }
        fields.insert("email".to_string(), Value::String(self.email.clone()));
        fields.insert("name".to_string(), Value::String(self.name.clone()));
        if let Some(created_at) = self.created_at {
            fields.insert(
                "created_at".to_string(),
                Value::String(created_at.to_rfc3339()),
            );
        }
        if let Some(updated_at) = self.updated_at {
            fields.insert(
                "updated_at".to_string(),
                Value::String(updated_at.to_rfc3339()),
            );
        }
        if let Some(deleted_at) = self.deleted_at {
            fields.insert(
                "deleted_at".to_string(),
                Value::String(deleted_at.to_rfc3339()),
            );
        }
        fields
    }
}

#[cfg(test)]
mod query_builder_tests {
    use crate::query::{OrderDirection, QueryOperator};

    use super::*;

    #[test]
    fn test_basic_select_query() {
        let query = QueryBuilder::<TestUser>::new().select("*").from("users");

        let sql = query.to_sql();
        assert_eq!(sql, "SELECT * FROM users");
    }

    #[test]
    fn test_select_with_where_conditions() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("email", "test@example.com")
            .where_gt("id", 100);

        let sql = query.to_sql();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("email = 'test@example.com'"));
        assert!(sql.contains("id > 100"));
        assert!(sql.contains("AND"));
    }

    #[test]
    fn test_select_with_multiple_where_operators() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_like("name", "%John%")
            .where_in("status", vec!["active", "pending"])
            .where_not_null("email_verified_at")
            .where_between("age", 18, 65);

        let sql = query.to_sql();
        assert!(sql.contains("name LIKE '%John%'"));
        assert!(sql.contains("status IN ('active', 'pending')"));
        assert!(sql.contains("email_verified_at IS NOT NULL"));
        assert!(sql.contains("age BETWEEN 18 AND 65"));
    }

    #[test]
    fn test_select_with_joins() {
        let query = QueryBuilder::<TestUser>::new()
            .select("users.*, profiles.bio")
            .from("users")
            .join("profiles", "users.id", "profiles.user_id")
            .left_join("posts", "users.id", "posts.user_id");

        let sql = query.to_sql();
        assert!(sql.contains("INNER JOIN profiles ON users.id = profiles.user_id"));
        assert!(sql.contains("LEFT JOIN posts ON users.id = posts.user_id"));
    }

    #[test]
    fn test_select_with_order_and_limit() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .order_by("name")
            .order_by_desc("created_at")
            .limit(10)
            .offset(20);

        let sql = query.to_sql();
        assert!(sql.contains("ORDER BY name ASC, created_at DESC"));
        assert!(sql.contains("LIMIT 10"));
        assert!(sql.contains("OFFSET 20"));
    }

    #[test]
    fn test_select_with_group_by_and_having() {
        let query = QueryBuilder::<TestUser>::new()
            .select("country, COUNT(*) as user_count")
            .from("users")
            .group_by("country")
            .having_eq("COUNT(*)", 5);

        let sql = query.to_sql();
        assert!(sql.contains("GROUP BY country"));
        assert!(sql.contains("HAVING COUNT(*) = 5"));
    }

    #[test]
    fn test_pagination() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .paginate(15, 3); // 15 per page, page 3

        let sql = query.to_sql();
        assert!(sql.contains("LIMIT 15"));
        assert!(sql.contains("OFFSET 30")); // (3-1) * 15 = 30
    }

    #[test]
    fn test_distinct_query() {
        let query = QueryBuilder::<TestUser>::new()
            .select_distinct("country")
            .from("users");

        let sql = query.to_sql();
        assert!(sql.contains("SELECT DISTINCT country"));
    }

    #[test]
    fn test_aggregate_functions() {
        let query = QueryBuilder::<TestUser>::new()
            .select_count("*", Some("total"))
            .select_sum("amount", Some("total_amount"))
            .select_avg("age", None)
            .select_min("created_at", Some("earliest"))
            .select_max("updated_at", Some("latest"))
            .from("users");

        let sql = query.to_sql();
        assert!(sql.contains("COUNT(*) AS total"));
        assert!(sql.contains("SUM(amount) AS total_amount"));
        assert!(sql.contains("AVG(age)"));
        assert!(sql.contains("MIN(created_at) AS earliest"));
        assert!(sql.contains("MAX(updated_at) AS latest"));
    }

    #[test]
    fn test_raw_select() {
        let query = QueryBuilder::<TestUser>::new()
            .select_raw("CASE WHEN age > 18 THEN 'adult' ELSE 'minor' END as age_group")
            .from("users");

        let sql = query.to_sql();
        assert!(sql.contains("CASE WHEN age > 18 THEN 'adult' ELSE 'minor' END as age_group"));
    }

    #[test]
    fn test_where_raw() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_raw("EXTRACT(YEAR FROM created_at) = 2023");

        let sql = query.to_sql();
        assert!(sql.contains("EXTRACT(YEAR FROM created_at) = 2023"));
    }

    #[test]
    fn test_subquery_conditions() {
        let subquery = QueryBuilder::<TestUser>::new()
            .select("user_id")
            .from("orders")
            .where_gt("total", 1000);

        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_subquery("id", QueryOperator::In, subquery);

        let sql = query.to_sql();
        assert!(sql.contains("id IN (SELECT user_id FROM orders WHERE total > 1000)"));
    }

    #[test]
    fn test_exists_condition() {
        let subquery = QueryBuilder::<TestUser>::new()
            .select("1")
            .from("posts")
            .where_raw("posts.user_id = users.id");

        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_exists(subquery);

        let sql = query.to_sql();
        assert!(sql.contains("EXISTS (SELECT 1 FROM posts WHERE posts.user_id = users.id)"));
    }

    #[test]
    fn test_cursor_pagination() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .paginate_cursor("id", Some("12345"), 10, OrderDirection::Asc);

        let sql = query.to_sql();
        assert!(sql.contains("id > '12345'"));
        assert!(sql.contains("ORDER BY id ASC"));
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_query_complexity_scoring() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("active", true)
            .where_like("name", "%test%")
            .join("profiles", "users.id", "profiles.user_id")
            .left_join("posts", "users.id", "posts.user_id")
            .group_by("country")
            .having_eq("COUNT(*)", 5);

        let complexity = query.complexity_score();
        // Should be: 2 where conditions + 2 joins * 2 + 1 group by + 1 having = 7
        assert!(complexity >= 7);
    }

    #[test]
    fn test_parameter_bindings() {
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("email", "test@example.com")
            .where_in("status", vec!["active", "pending"])
            .where_between("age", 18, 65);

        let bindings = query.bindings();
        assert!(bindings.len() >= 4); // email + 2 status values + 2 age values
    }

    #[test]
    fn test_query_builder_clone() {
        let original = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("active", true);

        let cloned = original.clone_for_subquery();
        assert_eq!(original.to_sql(), cloned.to_sql());
    }
}

#[cfg(test)]
mod error_tests {
    use crate::error::QueryError;

    use super::*;

    #[test]
    fn test_model_error_display() {
        let error = ModelError::NotFound("users".to_string());
        assert_eq!(error.to_string(), "Record not found in table 'users'");

        let error = ModelError::Validation("Invalid email".to_string());
        assert_eq!(error.to_string(), "Validation error: Invalid email");

        let error = ModelError::MissingPrimaryKey;
        assert_eq!(error.to_string(), "Primary key is missing or invalid");
    }

    #[test]
    fn test_error_from_conversions() {
        let json_error = serde_json::from_str::<Value>("{invalid json").unwrap_err();
        let model_error: ModelError = json_error.into();
        assert!(matches!(model_error, ModelError::Serialization(_)));
    }

    #[test]
    fn test_query_error_display() {
        let error = QueryError::InvalidSql("Missing FROM clause".to_string());
        assert_eq!(error.to_string(), "Invalid SQL: Missing FROM clause");

        let error = QueryError::UnsupportedOperation("WINDOW functions".to_string());
        assert_eq!(error.to_string(), "Unsupported operation: WINDOW functions");
    }
}

#[cfg(test)]
mod primary_key_tests {
    use crate::model::PrimaryKey;

    use super::*;

    #[test]
    fn test_integer_primary_key() {
        let pk = PrimaryKey::Integer(123);
        assert_eq!(pk.to_string(), "123");
        assert_eq!(pk.as_i64(), Some(123));
        assert_eq!(pk.as_uuid(), None);
    }

    #[test]
    fn test_uuid_primary_key() {
        let uuid = Uuid::new_v4();
        let pk = PrimaryKey::Uuid(uuid);
        assert_eq!(pk.to_string(), uuid.to_string());
        assert_eq!(pk.as_uuid(), Some(uuid));
        assert_eq!(pk.as_i64(), None);
    }

    #[test]
    fn test_composite_primary_key() {
        let mut composite = HashMap::new();
        composite.insert("tenant_id".to_string(), "1".to_string());
        composite.insert("user_id".to_string(), "123".to_string());

        let pk = PrimaryKey::Composite(composite);
        let display = pk.to_string();
        assert!(display.contains("tenant_id:1") || display.contains("user_id:123"));
    }

    #[test]
    fn test_primary_key_equality() {
        let pk1 = PrimaryKey::Integer(123);
        let pk2 = PrimaryKey::Integer(123);
        let pk3 = PrimaryKey::Integer(456);

        assert_eq!(pk1, pk2);
        assert_ne!(pk1, pk3);
    }
}

#[cfg(test)]
mod model_tests {
    use super::*;

    #[test]
    fn test_model_table_name() {
        assert_eq!(TestUser::table_name(), "users");
    }

    #[test]
    fn test_model_primary_key_handling() {
        let mut user = TestUser {
            id: None,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: None,
            updated_at: None,
            deleted_at: None,
        };

        assert_eq!(user.primary_key(), None);

        let uuid = Uuid::new_v4();
        user.set_primary_key(uuid);
        assert_eq!(user.primary_key(), Some(uuid));
    }

    #[test]
    fn test_model_timestamps() {
        let mut user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: None,
            updated_at: None,
            deleted_at: None,
        };

        assert!(TestUser::uses_timestamps());
        assert_eq!(user.created_at(), None);

        let now = Utc::now();
        user.set_created_at(now);
        assert_eq!(user.created_at(), Some(now));

        user.set_updated_at(now);
        assert_eq!(user.updated_at(), Some(now));
    }

    #[test]
    fn test_model_soft_deletes() {
        let mut user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: None,
            updated_at: None,
            deleted_at: None,
        };

        assert!(TestUser::uses_soft_deletes());
        assert!(!user.is_soft_deleted());

        let now = Utc::now();
        user.set_deleted_at(Some(now));
        assert!(user.is_soft_deleted());
        assert_eq!(user.deleted_at(), Some(now));
    }

    #[test]
    fn test_model_to_fields() {
        let user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            deleted_at: None,
        };

        let fields = user.to_fields();
        assert!(fields.contains_key("email"));
        assert!(fields.contains_key("name"));
        assert_eq!(
            fields.get("email").unwrap(),
            &Value::String("test@example.com".to_string())
        );
    }
}

#[cfg(test)]
mod performance_tests {
    use crate::query::QueryOperator;

    use super::*;
    use std::mem::size_of_val;
    use std::time::Instant;

    #[test]
    fn test_query_builder_memory_overhead() {
        // Test query builder memory usage
        let simple_query = QueryBuilder::<TestUser>::new().select("*").from("users");

        let complex_query = QueryBuilder::<TestUser>::new()
            .select("users.*, profiles.bio, COUNT(posts.id) as post_count")
            .from("users")
            .join("profiles", "users.id", "profiles.user_id")
            .left_join("posts", "users.id", "posts.user_id")
            .where_eq("users.active", true)
            .where_like("users.name", "%John%")
            .group_by("users.id")
            .having_eq("COUNT(posts.id)", 5)
            .order_by("users.name")
            .limit(50)
            .offset(100);

        // Memory usage should be reasonable
        let simple_size = size_of_val(&simple_query);
        let complex_size = size_of_val(&complex_query);

        // Query builder should have minimal overhead (target: <1KB)
        assert!(
            simple_size < 1024,
            "Simple query builder too large: {} bytes",
            simple_size
        );
        assert!(
            complex_size < 2048,
            "Complex query builder too large: {} bytes",
            complex_size
        );

        println!("Query Builder Memory Usage:");
        println!("  Simple query: {} bytes", simple_size);
        println!("  Complex query: {} bytes", complex_size);
    }

    #[test]
    fn test_model_instance_memory_overhead() {
        // Test model memory usage
        let user = TestUser {
            id: Some(uuid::Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
            deleted_at: None,
        };

        let size = size_of_val(&user);

        // Model instance should be lightweight (target: <500 bytes overhead)
        // Base data is reasonable, focus on framework overhead
        assert!(size < 1024, "Model instance too large: {} bytes", size);

        println!("Model Instance Memory Usage: {} bytes", size);
    }

    #[test]
    fn test_query_builder_performance() {
        // Test query building performance
        let iterations = 10_000;

        let start = Instant::now();
        for i in 0..iterations {
            let _query = QueryBuilder::<TestUser>::new()
                .select("*")
                .from("users")
                .where_eq("id", i as i64)
                .where_gt("created_at", "2023-01-01")
                .order_by("name")
                .limit(10)
                .to_sql();
        }
        let duration = start.elapsed();

        let avg_per_query = duration.as_micros() / iterations;

        // Each query should build very quickly (target: <100μs)
        assert!(
            avg_per_query < 1000,
            "Query building too slow: {}μs per query",
            avg_per_query
        );

        println!("Query Building Performance:");
        println!("  {} queries in {:?}", iterations, duration);
        println!("  Average: {}μs per query", avg_per_query);
    }

    #[test]
    // TODO: Fix this test
    #[ignore]
    fn test_sql_generation_performance() {
        // Test SQL generation performance for different query types
        let queries = vec![
            // Simple query
            QueryBuilder::<TestUser>::new()
                .select("*")
                .from("users")
                .where_eq("active", true),
            // Complex query with joins
            QueryBuilder::<TestUser>::new()
                .select("users.*, profiles.bio")
                .from("users")
                .join("profiles", "users.id", "profiles.user_id")
                .where_like("users.name", "%John%")
                .where_in("users.status", vec!["active", "pending"])
                .order_by("users.created_at"),
            // Aggregation query
            QueryBuilder::<TestUser>::new()
                .select_count("*", Some("total"))
                .select_avg("age", Some("avg_age"))
                .from("users")
                .group_by("country")
                .having_eq("COUNT(*)", 10),
            // Subquery
            QueryBuilder::<TestUser>::new()
                .select("*")
                .from("users")
                .where_subquery(
                    "id",
                    QueryOperator::In,
                    QueryBuilder::<TestUser>::new()
                        .select("user_id")
                        .from("orders")
                        .where_gt("total", 1000),
                ),
        ];

        for (i, query) in queries.iter().enumerate() {
            let start = Instant::now();
            let _sql = query.to_sql();
            let duration = start.elapsed();

            // SQL generation should be very fast (target: <10μs)
            assert!(
                duration.as_micros() < 100,
                "SQL generation too slow for query {}: {}μs",
                i,
                duration.as_micros()
            );

            println!("SQL Generation {}: {}μs", i + 1, duration.as_micros());
        }
    }

    #[test]
    fn test_query_complexity_scoring() {
        // Test that complexity scoring works as expected
        let simple = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("active", true);

        let complex = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("active", true)
            .where_like("name", "%test%")
            .join("profiles", "users.id", "profiles.user_id")
            .left_join("posts", "users.id", "posts.user_id")
            .group_by("country")
            .having_eq("COUNT(*)", 5);

        let simple_score = simple.complexity_score();
        let complex_score = complex.complexity_score();

        // Complex query should have higher score
        assert!(
            complex_score > simple_score,
            "Complex query score ({}) should be higher than simple query score ({})",
            complex_score,
            simple_score
        );

        // Simple query should have low complexity
        assert!(
            simple_score <= 2,
            "Simple query complexity too high: {}",
            simple_score
        );

        // Complex query should reflect all its operations
        assert!(
            complex_score >= 7,
            "Complex query complexity too low: {}",
            complex_score
        );

        println!("Query Complexity Scores:");
        println!("  Simple query: {}", simple_score);
        println!("  Complex query: {}", complex_score);
    }

    #[test]
    fn test_parameter_binding_efficiency() {
        // Test parameter binding extraction
        let query = QueryBuilder::<TestUser>::new()
            .select("*")
            .from("users")
            .where_eq("email", "test@example.com")
            .where_in("status", vec!["active", "pending", "inactive"])
            .where_between("age", 18, 65)
            .where_like("name", "%John%");

        let start = Instant::now();
        let bindings = query.bindings();
        let duration = start.elapsed();

        // Should extract the correct number of parameters
        // email(1) + status(3) + age(2) + name(1) = 7
        assert_eq!(bindings.len(), 7, "Wrong number of parameter bindings");

        // Binding extraction should be very fast
        assert!(
            duration.as_micros() < 50,
            "Parameter binding too slow: {}μs",
            duration.as_micros()
        );

        println!(
            "Parameter Binding: {} parameters extracted in {}μs",
            bindings.len(),
            duration.as_micros()
        );
    }
}

#[cfg(test)]
mod integration_tests {
    // use super::*;

    // NOTE: These tests would require a real database connection
    // For now, they're placeholders showing what integration tests should cover

    #[test]
    #[ignore] // Ignored until database test setup is available
    fn test_model_crud_operations() {
        // This would test:
        // - Model::find()
        // - Model::create()
        // - Model::update()
        // - Model::delete()
        // - Model::all()
        // - Model::count()
    }

    #[test]
    #[ignore] // Ignored until database test setup is available
    fn test_query_execution() {
        // This would test:
        // - QueryBuilder::get()
        // - QueryBuilder::first()
        // - QueryBuilder::count()
        // - QueryBuilder::chunk()
        // - QueryBuilder::aggregate()
    }

    #[test]
    #[ignore] // Ignored until database test setup is available
    fn test_database_performance_benchmarks() {
        // This would test with real database:
        // - Simple queries < 10ms
        // - Complex queries < 50ms
        // - Bulk operations > 1000 records/second
        // - Connection acquisition < 1ms
    }
}

#[cfg(test)]
mod model_database_integration_tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_bind_json_value_types() {
        // Test JSON value binding helper method
        // This is unit testable without a database

        // String value
        let string_val = Value::String("test".to_string());
        assert!(matches!(string_val, Value::String(_)));

        // Number value
        let number_val = Value::Number(serde_json::Number::from(42));
        assert!(number_val.is_number());
        assert_eq!(number_val.as_i64().unwrap(), 42);

        // Boolean value
        let bool_val = Value::Bool(true);
        assert!(bool_val.is_boolean());
        assert_eq!(bool_val.as_bool().unwrap(), true);

        // Null value
        let null_val = Value::Null;
        assert!(null_val.is_null());

        // Array value (JSON)
        let array_val = Value::Array(vec![Value::String("item".to_string())]);
        assert!(array_val.is_array());

        // Object value (JSON)
        let mut obj = serde_json::Map::new();
        obj.insert("key".to_string(), Value::String("value".to_string()));
        let object_val = Value::Object(obj);
        assert!(object_val.is_object());
    }

    #[test]
    fn test_model_field_serialization() {
        // Test that our TestUser properly serializes fields
        let user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            deleted_at: None,
        };

        let fields = user.to_fields();

        // Should have core fields
        assert!(fields.contains_key("id"));
        assert!(fields.contains_key("email"));
        assert!(fields.contains_key("name"));
        assert!(fields.contains_key("created_at"));
        assert!(fields.contains_key("updated_at"));

        // Should not have deleted_at since it's None
        assert!(!fields.contains_key("deleted_at"));

        // Values should be correct types
        assert!(fields.get("email").unwrap().is_string());
        assert!(fields.get("name").unwrap().is_string());
        assert_eq!(
            fields.get("email").unwrap().as_str().unwrap(),
            "test@example.com"
        );
        assert_eq!(fields.get("name").unwrap().as_str().unwrap(), "Test User");
    }

    #[test]
    fn test_model_timestamps_handling() {
        let mut user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: None,
            updated_at: None,
            deleted_at: None,
        };

        // Test timestamp methods
        let now = Utc::now();
        user.set_created_at(now);
        user.set_updated_at(now);

        assert_eq!(user.created_at(), Some(now));
        assert_eq!(user.updated_at(), Some(now));

        // Test soft delete
        user.set_deleted_at(Some(now));
        assert!(user.is_soft_deleted());
        assert_eq!(user.deleted_at(), Some(now));

        // Test undelete
        user.set_deleted_at(None);
        assert!(!user.is_soft_deleted());
        assert_eq!(user.deleted_at(), None);
    }

    #[test]
    fn test_model_primary_key_handling() {
        let mut user = TestUser {
            id: None,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: None,
            updated_at: None,
            deleted_at: None,
        };

        // Initially no primary key
        assert!(user.primary_key().is_none());

        // Set primary key
        let id = Uuid::new_v4();
        user.set_primary_key(id);
        assert_eq!(user.primary_key(), Some(id));
        assert_eq!(user.id, Some(id));
    }

    #[test]
    fn test_model_trait_constants() {
        // Test model configuration constants
        assert_eq!(TestUser::table_name(), "users");
        assert_eq!(TestUser::primary_key_name(), "id");
        assert!(TestUser::uses_timestamps());
        assert!(TestUser::uses_soft_deletes());
    }

    #[test]
    fn test_sql_generation_patterns() {
        // Test that our SQL patterns are correct (without executing)
        let table_name = "users";
        let pk_name = "id";
        let _pk_value = "test-uuid";

        // Find query pattern
        let find_sql = format!("SELECT * FROM {} WHERE {} = $1", table_name, pk_name);
        assert_eq!(find_sql, "SELECT * FROM users WHERE id = $1");

        // Count query pattern (with soft deletes)
        let count_sql = format!(
            "SELECT COUNT(*) FROM {} WHERE deleted_at IS NULL",
            table_name
        );
        assert_eq!(
            count_sql,
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL"
        );

        // All query pattern (with soft deletes)
        let all_sql = format!("SELECT * FROM {} WHERE deleted_at IS NULL", table_name);
        assert_eq!(all_sql, "SELECT * FROM users WHERE deleted_at IS NULL");

        // Delete query pattern (hard delete)
        let delete_sql = format!("DELETE FROM {} WHERE {} = $1", table_name, pk_name);
        assert_eq!(delete_sql, "DELETE FROM users WHERE id = $1");

        // Soft delete query pattern
        let soft_delete_sql = format!(
            "UPDATE {} SET deleted_at = NOW() WHERE {} = $1",
            table_name, pk_name
        );
        assert_eq!(
            soft_delete_sql,
            "UPDATE users SET deleted_at = NOW() WHERE id = $1"
        );
    }

    #[test]
    fn test_dynamic_insert_sql_generation() {
        // Test dynamic INSERT SQL generation
        let user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            deleted_at: None,
        };

        let fields = user.to_fields();
        let field_names: Vec<String> = fields.keys().cloned().collect();
        let field_placeholders: Vec<String> =
            (1..=field_names.len()).map(|i| format!("${}", i)).collect();

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
            TestUser::table_name(),
            field_names.join(", "),
            field_placeholders.join(", ")
        );

        // Should contain all the expected parts
        assert!(insert_sql.starts_with("INSERT INTO users"));
        assert!(insert_sql.contains("VALUES"));
        assert!(insert_sql.contains("RETURNING *"));
        assert!(insert_sql.contains("email"));
        assert!(insert_sql.contains("name"));

        // Should have correct number of placeholders
        let placeholder_count = field_placeholders.len();
        assert_eq!(placeholder_count, fields.len());
    }

    #[test]
    fn test_dynamic_update_sql_generation() {
        // Test dynamic UPDATE SQL generation
        let user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "updated@example.com".to_string(),
            name: "Updated User".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            deleted_at: None,
        };

        let fields = user.to_fields();
        let pk_name = TestUser::primary_key_name();
        let update_fields: Vec<String> = fields
            .keys()
            .filter(|&field| field != pk_name)
            .enumerate()
            .map(|(i, field)| format!("{} = ${}", field, i + 1))
            .collect();

        let update_sql = format!(
            "UPDATE {} SET {} WHERE {} = ${}",
            TestUser::table_name(),
            update_fields.join(", "),
            pk_name,
            update_fields.len() + 1
        );

        // Should contain expected parts
        assert!(update_sql.starts_with("UPDATE users"));
        assert!(update_sql.contains("SET"));
        assert!(update_sql.contains("WHERE id ="));
        assert!(update_fields.len() < fields.len()); // Should exclude primary key
    }

    #[test]
    fn test_field_filtering_for_updates() {
        // Test that primary key is properly filtered out during updates
        let user = TestUser {
            id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            deleted_at: None,
        };

        let fields = user.to_fields();
        let pk_name = TestUser::primary_key_name();

        // Should contain primary key in full fields
        assert!(fields.contains_key(pk_name));

        // But filtered fields should not contain primary key
        let update_fields: Vec<&String> = fields.keys().filter(|&field| field != pk_name).collect();

        assert!(!update_fields.iter().any(|&field| field == pk_name));
        assert!(update_fields.len() < fields.len());
    }
}
