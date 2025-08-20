use elif_orm::{ModelObserver, EventError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

// Example Post model (simplified)
#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
}

// Search indexing service example (simplified)
pub struct SearchIndexService;

impl SearchIndexService {
    pub async fn index_post(post: &Post) -> Result<(), Box<dyn std::error::Error>> {
        println!("Indexing post '{}' for search", post.title);
        // In real implementation, this would add to search index (Elasticsearch, etc.)
        Ok(())
    }
    
    pub async fn remove_from_index(post_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        println!("Removing post {} from search index", post_id);
        Ok(())
    }
}

// Cache service example (simplified)
pub struct CacheService;

impl CacheService {
    pub async fn invalidate_post_cache(post_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        println!("Invalidating cache for post {}", post_id);
        Ok(())
    }
    
    pub async fn warm_post_cache(post: &Post) -> Result<(), Box<dyn std::error::Error>> {
        println!("Warming cache for post '{}'", post.title);
        Ok(())
    }
}

/// Observer for Post model that handles search indexing, caching, and slug generation
pub struct PostObserver;

impl PostObserver {
    fn generate_slug(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[async_trait]
impl ModelObserver<Post> for PostObserver {
    async fn creating(&self, post: &mut Post) -> Result<(), EventError> {
        // Auto-generate slug if empty
        if post.slug.is_empty() {
            post.slug = Self::generate_slug(&post.title);
            println!("Post creating: generated slug '{}' from title '{}'", post.slug, post.title);
        }
        
        // Validate title length
        if post.title.len() < 3 {
            return Err(EventError::validation("Post title must be at least 3 characters long"));
        }
        
        // Set published_at if published
        if post.published && post.published_at.is_none() {
            post.published_at = Some(Utc::now());
            println!("Post creating: set published_at timestamp");
        }
        
        Ok(())
    }
    
    async fn created(&self, post: &Post) -> Result<(), EventError> {
        // Index for search if published
        if post.published {
            if let Err(e) = SearchIndexService::index_post(post).await {
                return Err(EventError::observer(&format!("Failed to index post: {}", e)));
            }
        }
        
        // Warm cache
        if let Err(e) = CacheService::warm_post_cache(post).await {
            return Err(EventError::observer(&format!("Failed to warm cache: {}", e)));
        }
        
        println!("Post created: '{}' by author {}", post.title, post.author_id);
        Ok(())
    }
    
    async fn updating(&self, post: &mut Post, original: &Post) -> Result<(), EventError> {
        // Update slug if title changed
        if post.title != original.title {
            post.slug = Self::generate_slug(&post.title);
            println!("Post updating: updated slug to '{}' due to title change", post.slug);
        }
        
        // Set/unset published_at based on published status
        if post.published && !original.published {
            // Publishing the post
            post.published_at = Some(Utc::now());
            println!("Post updating: publishing post, set published_at timestamp");
        } else if !post.published && original.published {
            // Unpublishing the post
            post.published_at = None;
            println!("Post updating: unpublishing post, cleared published_at timestamp");
        }
        
        // Validate title length
        if post.title.len() < 3 {
            return Err(EventError::validation("Post title must be at least 3 characters long"));
        }
        
        Ok(())
    }
    
    async fn updated(&self, post: &Post, original: &Post) -> Result<(), EventError> {
        // Handle search index updates
        if post.published && !original.published {
            // Newly published - add to index
            if let Err(e) = SearchIndexService::index_post(post).await {
                return Err(EventError::observer(&format!("Failed to index post: {}", e)));
            }
        } else if !post.published && original.published {
            // Unpublished - remove from index
            if let Err(e) = SearchIndexService::remove_from_index(post.id).await {
                return Err(EventError::observer(&format!("Failed to remove from index: {}", e)));
            }
        } else if post.published && (post.title != original.title || post.content != original.content) {
            // Published post content updated - reindex
            if let Err(e) = SearchIndexService::index_post(post).await {
                return Err(EventError::observer(&format!("Failed to reindex post: {}", e)));
            }
        }
        
        // Invalidate cache
        if let Err(e) = CacheService::invalidate_post_cache(post.id).await {
            return Err(EventError::observer(&format!("Failed to invalidate cache: {}", e)));
        }
        
        // Warm cache for updated post
        if let Err(e) = CacheService::warm_post_cache(post).await {
            return Err(EventError::observer(&format!("Failed to warm cache: {}", e)));
        }
        
        println!("Post updated: '{}'", post.title);
        Ok(())
    }
    
    async fn deleting(&self, post: &Post) -> Result<(), EventError> {
        println!("Post deleting: preparing to delete '{}'", post.title);
        
        // Could check for dependencies (comments, likes, etc.)
        // Could implement soft delete logic
        
        Ok(())
    }
    
    async fn deleted(&self, post: &Post) -> Result<(), EventError> {
        // Remove from search index
        if post.published {
            if let Err(e) = SearchIndexService::remove_from_index(post.id).await {
                return Err(EventError::observer(&format!("Failed to remove from search index: {}", e)));
            }
        }
        
        // Invalidate cache
        if let Err(e) = CacheService::invalidate_post_cache(post.id).await {
            return Err(EventError::observer(&format!("Failed to invalidate cache: {}", e)));
        }
        
        println!("Post deleted: '{}' and cleaned up related resources", post.title);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_post_observer_slug_generation() {
        let observer = PostObserver;
        let mut post = Post {
            id: 1,
            title: "Hello World! This is a Test Post".to_string(),
            content: "Content here".to_string(),
            author_id: 1,
            published: false,
            published_at: None,
            slug: String::new(),
        };
        
        let result = observer.creating(&mut post).await;
        assert!(result.is_ok());
        assert_eq!(post.slug, "hello-world-this-is-a-test-post");
    }
    
    #[tokio::test]
    async fn test_post_observer_title_validation() {
        let observer = PostObserver;
        let mut post = Post {
            id: 1,
            title: "Hi".to_string(), // Too short
            content: "Content here".to_string(),
            author_id: 1,
            published: false,
            published_at: None,
            slug: String::new(),
        };
        
        let result = observer.creating(&mut post).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            EventError::Validation { message, .. } => {
                assert_eq!(message, "Post title must be at least 3 characters long");
            }
            _ => panic!("Expected validation error"),
        }
    }
    
    #[tokio::test]
    async fn test_post_observer_published_timestamp() {
        let observer = PostObserver;
        let mut post = Post {
            id: 1,
            title: "Test Post".to_string(),
            content: "Content here".to_string(),
            author_id: 1,
            published: true,
            published_at: None,
            slug: String::new(),
        };
        
        let result = observer.creating(&mut post).await;
        assert!(result.is_ok());
        assert!(post.published_at.is_some());
    }
}