//! Comprehensive Lazy Loading Tests
//!
//! Tests for the lazy loading system including transparent loading, caching,
//! memory efficiency, and integration with the relationship system.

#[cfg(test)]
pub mod tests {
    use crate::error::ModelResult;
    use crate::relationships::loader::{Lazy, RelationshipLoader, CachedRelationshipLoader};
    use sqlx::{Pool, Postgres};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Mock data structures for testing
    #[derive(Debug, Clone, PartialEq)]
    struct User {
        id: i64,
        name: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Post {
        id: i64,
        user_id: i64,
        title: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Profile {
        id: i64,
        user_id: i64,
        bio: String,
    }

    // Mock loader for testing
    pub struct MockLoader<T> {
        value: T,
        call_count: Arc<RwLock<usize>>,
    }

    impl<T> MockLoader<T> 
    where
        T: Clone,
    {
        pub fn new(value: T) -> Self {
            Self {
                value,
                call_count: Arc::new(RwLock::new(0)),
            }
        }

        #[allow(dead_code)]
        async fn get_call_count(&self) -> usize {
            *self.call_count.read().await
        }
    }

    #[async_trait::async_trait]
    impl<T> RelationshipLoader<T> for MockLoader<T>
    where
        T: Clone + Send + Sync,
    {
        async fn load(&self, _pool: &Pool<Postgres>) -> ModelResult<T> {
            // Increment call count
            {
                let mut count = self.call_count.write().await;
                *count += 1;
            }
            Ok(self.value.clone())
        }

        async fn reload(&self, _pool: &Pool<Postgres>) -> ModelResult<T> {
            // Increment call count
            {
                let mut count = self.call_count.write().await;
                *count += 1;
            }
            Ok(self.value.clone())
        }
    }

    #[test]
    fn test_lazy_creation_and_state() {
        // Test creating lazy relationships
        let posts = vec![
            Post { id: 1, user_id: 1, title: "First Post".to_string() },
            Post { id: 2, user_id: 1, title: "Second Post".to_string() },
        ];
        
        let loader = MockLoader::new(posts.clone());
        let lazy_posts = Lazy::new(loader);
        
        // Should not be loaded initially
        assert!(!lazy_posts.is_loaded());
        
        // Create with pre-loaded value
        let lazy_preloaded = Lazy::loaded(posts);
        assert!(lazy_preloaded.is_loaded());
    }

    // Note: This test is commented out because it requires a real database pool
    // In a real implementation, we'd use an integration test with a test database
    // 
    // #[tokio::test]
    // async fn test_transparent_loading() {
    //     // Implementation would go here with a real test database
    // }

    // Async tests that need database pools would go here
    // For now, we focus on synchronous lazy loading behavior

    #[test]
    fn test_lazy_manipulation() {
        let posts = vec![
            Post { id: 1, user_id: 1, title: "Post".to_string() },
        ];
        
        let mut lazy_posts = Lazy::loaded(posts.clone());
        
        // Test taking the value
        let taken = lazy_posts.take();
        assert_eq!(taken, Some(posts.clone()));
        assert!(!lazy_posts.is_loaded());
        
        // Test setting a value
        lazy_posts.set(posts.clone());
        assert!(lazy_posts.is_loaded());
        
        // Test clearing
        lazy_posts.clear();
        assert!(!lazy_posts.is_loaded());
    }

    #[test]
    fn test_cached_relationship_loader_creation() {
        // Test that we can create cached loaders (async testing requires database)
        let posts = vec![
            Post { id: 1, user_id: 1, title: "Cached Post".to_string() },
        ];
        
        let cached_loader = CachedRelationshipLoader::new(move |_pool| {
            let posts = posts.clone();
            async move { Ok(posts) }
        });
        
        // Just test that we can create the loader
        // Actual loading tests would require a real database
        std::mem::drop(cached_loader);
    }

    #[test]
    fn test_memory_efficiency() {
        // Test that unloaded relationships don't consume significant memory
        let loader = MockLoader::new(Vec::<Post>::new());
        let lazy_posts = Lazy::new(loader);
        
        // Lazy wrapper should be small (< 100 bytes for reasonable overhead)
        let size = std::mem::size_of_val(&lazy_posts);
        assert!(size < 100, "Lazy wrapper size {} is too large", size);
        
        // Should not be loaded
        assert!(!lazy_posts.is_loaded());
    }

    #[test]
    fn test_multiple_relationship_types() {
        // Test HasOne relationship (Profile)
        let profile = Profile {
            id: 1,
            user_id: 1,
            bio: "User bio".to_string(),
        };
        let lazy_profile = Lazy::loaded(Some(profile.clone()));
        assert!(lazy_profile.is_loaded());
        
        // Test HasMany relationship (Posts)
        let posts = vec![
            Post { id: 1, user_id: 1, title: "Post 1".to_string() },
            Post { id: 2, user_id: 1, title: "Post 2".to_string() },
        ];
        let lazy_posts = Lazy::loaded(posts);
        assert!(lazy_posts.is_loaded());
        
        // Test BelongsTo relationship (User)
        let user = User { id: 1, name: "John Doe".to_string() };
        let lazy_user = Lazy::loaded(user);
        assert!(lazy_user.is_loaded());
    }

    #[test] 
    fn test_error_loader_creation() {
        // Test that we can create error-returning loaders
        // Actual error testing would require database integration tests
        
        struct ErrorLoader;
        
        #[async_trait::async_trait]
        impl RelationshipLoader<Vec<Post>> for ErrorLoader {
            async fn load(&self, _pool: &Pool<Postgres>) -> ModelResult<Vec<Post>> {
                Err(crate::error::ModelError::Database("Connection failed".to_string()))
            }
            
            async fn reload(&self, _pool: &Pool<Postgres>) -> ModelResult<Vec<Post>> {
                Err(crate::error::ModelError::Database("Connection failed".to_string()))
            }
        }
        
        let lazy_posts = Lazy::new(ErrorLoader);
        assert!(!lazy_posts.is_loaded());
    }

    #[test]
    fn test_relationship_state_transitions() {
        let posts = vec![Post { id: 1, user_id: 1, title: "Post".to_string() }];
        let mut lazy_posts = Lazy::loaded(posts.clone());
        
        // Start loaded
        assert!(lazy_posts.is_loaded());
        
        // Clear should make it unloaded
        lazy_posts.clear();
        assert!(!lazy_posts.is_loaded());
        
        // Set should make it loaded again
        lazy_posts.set(posts.clone());
        assert!(lazy_posts.is_loaded());
        
        // Take should make it unloaded
        let taken = lazy_posts.take();
        assert_eq!(taken, Some(posts));
        assert!(!lazy_posts.is_loaded());
    }

    #[test]
    fn test_lazy_relationship_builder() {
        // Test that we can build lazy relationships with different loaders
        let posts = vec![Post { id: 1, user_id: 1, title: "Post".to_string() }];
        
        // With mock loader
        let mock_loader = MockLoader::new(posts.clone());
        let lazy_with_mock = Lazy::new(mock_loader);
        assert!(!lazy_with_mock.is_loaded());
        
        // With preloaded data
        let lazy_preloaded = Lazy::loaded(posts);
        assert!(lazy_preloaded.is_loaded());
    }

    #[tokio::test]
    async fn test_access_pattern_tracking() {
        let posts = vec![Post { id: 1, user_id: 1, title: "Post".to_string() }];
        let loader = MockLoader::new(posts);
        let lazy_posts = Lazy::new(loader);
        
        // Initially should not be marked for auto-load
        assert!(!lazy_posts.should_auto_load().await);
        
        // Check initial access pattern
        let pattern = lazy_posts.get_access_pattern().await;
        assert_eq!(pattern.access_count, 0);
        assert!(!pattern.should_auto_load);
    }

    #[tokio::test]
    async fn test_auto_load_control() {
        let posts = vec![Post { id: 1, user_id: 1, title: "Post".to_string() }];
        let loader = MockLoader::new(posts);
        let lazy_posts = Lazy::new(loader);
        
        // Initially should not auto-load
        assert!(!lazy_posts.should_auto_load().await);
        
        // Enable auto-loading
        lazy_posts.enable_auto_load().await;
        assert!(lazy_posts.should_auto_load().await);
        
        // Disable auto-loading
        lazy_posts.disable_auto_load().await;
        assert!(!lazy_posts.should_auto_load().await);
    }

    #[test]
    fn test_lazy_relationship_types() {
        use crate::relationships::loader::{LazyHasOne, LazyHasMany, LazyBelongsTo};
        
        // Test lazy relationship type aliases work
        let posts = vec![Post { id: 1, user_id: 1, title: "Post".to_string() }];
        let profile = Profile { id: 1, user_id: 1, bio: "Bio".to_string() };
        let user = User { id: 1, name: "John".to_string() };
        
        // LazyHasOne for optional single relationship
        let lazy_profile: LazyHasOne<Profile> = Lazy::loaded(Some(profile));
        assert!(lazy_profile.is_loaded());
        
        // LazyHasMany for collection relationships
        let lazy_posts: LazyHasMany<Post> = Lazy::loaded(posts);
        assert!(lazy_posts.is_loaded());
        
        // LazyBelongsTo for required single relationship
        let lazy_user: LazyBelongsTo<User> = Lazy::loaded(user);
        assert!(lazy_user.is_loaded());
    }

}