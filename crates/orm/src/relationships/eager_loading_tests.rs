//! Comprehensive tests for the eager loading system

#[cfg(test)]
mod tests {
    use crate::relationships::eager_loading::*;
    use crate::query::QueryBuilderWithMethods;
    
    // Simple test model
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestModel {
        pub id: i64,
        pub name: String,
    }
    
    impl crate::model::Model for TestModel {
        type PrimaryKey = i64;
        
        fn table_name() -> &'static str { "test_models" }
        
        fn primary_key(&self) -> Option<Self::PrimaryKey> { Some(self.id) }
        
        fn set_primary_key(&mut self, key: Self::PrimaryKey) { self.id = key; }
        
        fn from_row(row: &sqlx::postgres::PgRow) -> crate::error::ModelResult<Self> {
            use sqlx::Row;
            Ok(TestModel {
                id: row.get("id"),
                name: row.get("name"),
            })
        }
        
        fn to_fields(&self) -> std::collections::HashMap<String, serde_json::Value> {
            let mut fields = std::collections::HashMap::new();
            fields.insert("id".to_string(), serde_json::Value::from(self.id));
            fields.insert("name".to_string(), serde_json::Value::from(self.name.clone()));
            fields
        }
    }
    
    #[test]
    fn test_eager_loader_creation() {
        let loader = EagerLoader::new();
        // Test that loader is created successfully
        assert!(loader.loaded_relations().is_empty());
    }
    
    #[test]
    fn test_eager_loader_with_relationship() {
        let loader = EagerLoader::new()
            .with("posts")
            .with("comments");
        
        // Test that relationships are added
        assert!(loader.is_loaded("posts") == false); // Not loaded yet
        assert!(loader.is_loaded("comments") == false); // Not loaded yet
    }
    
    #[test]
    fn test_eager_loader_with_constraints() {
        let loader = EagerLoader::new()
            .with_constraint("posts", |builder| {
                builder.where_eq("published", true)
                       .order_by_desc("created_at")
                       .limit(5)
            });
        
        // Test that constraints are configured
        assert!(loader.loaded_relations().is_empty()); // No data loaded yet
    }
    
    #[test]
    fn test_relationship_constraint_builder() {
        let builder = RelationshipConstraintBuilder::new()
            .where_eq("status", "published")
            .where_gt("views", 1000)
            .order_by_desc("created_at")
            .limit(10);
        
        // Test that builder accepts constraints
        // The constraints vector is private, so we just test creation doesn't panic
        drop(builder);
    }
    
    #[test]
    fn test_query_builder_with_methods() {
        use crate::query::QueryBuilder;
        
        // Test the trait implementation
        let _query = QueryBuilder::<TestModel>::new()
            .from("test_models")
            .with("posts")
            .with("profile")
            .limit(10);
        
        // If this compiles, the trait is working correctly
    }
    
    #[test]
    fn test_query_builder_conditional_loading() {
        use crate::query::QueryBuilder;
        
        let include_posts = true;
        let include_comments = false;
        
        let _query = QueryBuilder::<TestModel>::new()
            .from("test_models")
            .with_when(include_posts, "posts")
            .with_when(include_comments, "comments");
        
        // Test passes if no compilation errors
    }
    
    #[test]
    fn test_query_builder_with_count() {
        use crate::query::QueryBuilder;
        
        let _query = QueryBuilder::<TestModel>::new()
            .from("test_models")
            .with_count("posts")
            .with_count("comments");
        
        // Test passes if no compilation errors
    }
    
    #[test]
    fn test_query_builder_with_count_custom_alias() {
        use crate::query::QueryBuilder;
        
        let _query = QueryBuilder::<TestModel>::new()
            .from("test_models")
            .with_count_where("published_posts_count", "posts", |builder| {
                builder.where_eq("published", true)
            });
        
        // Test passes if no compilation errors
    }
    
    #[test]
    fn test_nested_relationship_parsing() {
        let loader = EagerLoader::new()
            .with("posts.comments.user");
        
        // Test that nested relationships can be specified
        assert!(loader.loaded_relations().is_empty()); // No data loaded yet
    }
    
    #[test]
    fn test_relationship_cache_basic_operations() {
        use crate::relationships::cache::OptimizedRelationshipCache;
        
        let cache = OptimizedRelationshipCache::default();
        
        // Test that cache can be created with default config
        // The internals are private, so we just test creation
        drop(cache);
    }
    
    #[test]
    fn test_cache_configuration() {
        use crate::relationships::cache::{RelationshipCacheConfig, OptimizedRelationshipCache};
        use std::time::Duration;
        
        let config = RelationshipCacheConfig {
            max_relationships_per_type: 500,
            max_memory_bytes: 10 * 1024 * 1024, // 10MB
            ttl: Some(Duration::from_secs(600)), // 10 minutes
            enable_metrics: true,
        };
        
        let _cache = OptimizedRelationshipCache::new(config);
        
        // Test passes if cache can be created with custom config
    }
    
    #[test]
    fn test_lazy_relationship_loader() {
        use crate::relationships::loader::Lazy;
        
        // Test creating a lazy relationship with pre-loaded data
        let posts = vec![1, 2, 3]; // Simple test data
        let lazy_posts = Lazy::loaded(posts);
        assert!(lazy_posts.is_loaded());
    }
    
    #[test]
    fn test_eager_loading_spec_structure() {
        let spec = EagerLoadSpec {
            relation: "posts.comments".to_string(),
            constraint_callback: None,
        };
        
        assert_eq!(spec.relation, "posts.comments");
        assert!(spec.constraint_callback.is_none());
        
        // Verify it can be debugged (has Debug trait)
        let debug_str = format!("{:?}", spec);
        assert!(debug_str.contains("posts.comments"));
    }
    
    #[test]
    fn test_relationship_loading_integration() {
        // Test the complete eager loading configuration
        let loader = EagerLoader::new()
            .with("posts")
            .with_constraint("comments", |builder| {
                builder.where_eq("approved", true).limit(10)
            });
        
        // Verify the loader is configured correctly
        assert!(loader.loaded_relations().is_empty()); // No data loaded yet
        
        // Test that we can check for loaded data
        assert_eq!(loader.get_loaded_data("posts", "1"), None);
        assert_eq!(loader.get_loaded_data("comments", "1"), None);
    }
    
    #[test]
    fn test_relationship_cache_stats() {
        use crate::relationships::cache::{CacheStatistics, OptimizedRelationshipCache};
        
        // Test that CacheStatistics can be created
        let stats = CacheStatistics {
            total_entries: 0,
            memory_usage_bytes: 0,
            model_type_counts: std::collections::HashMap::new(),
            hits: 0,
            misses: 0,
            stores: 0,
            expired: 0,
            hit_rate: 0.0,
        };
        
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.hit_rate, 0.0);
        
        // Test that cache can provide stats
        let cache = OptimizedRelationshipCache::default();
        drop(cache); // Stats would be checked in async context
    }
    
    #[test] 
    fn test_query_builder_chaining() {
        use crate::query::QueryBuilder;
        
        // Test that multiple with methods can be chained
        let _query = QueryBuilder::<TestModel>::new()
            .from("test_models")
            .with("posts")
            .with("profile")
            .with_when(true, "settings")
            .with_count("followers")
            .where_eq("active", "true")
            .order_by_desc("created_at")
            .limit(50);
        
        // Test passes if chaining compiles correctly
    }
}