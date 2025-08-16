use elif_testing::prelude::*;
use crate::models::post::Post;
use crate::controllers::post_controller::PostController;

mod post_tests {
    use super::*;

    // <<<ELIF:BEGIN agent-editable:post-model-tests>>>
    
    #[test_database]
    async fn test_create_post() -> TestResult<()> {
        let post = PostFactory::new().create().await?;
        
        assert!(!post.id.is_nil());
        // Assert title field
        // Assert content field
        // Assert user_id field
        
        Ok(())
    }

    #[test_database]
    async fn test_post_validation() -> TestResult<()> {
        // Test required field validation
        let result = Post::create(Post {
            title: String::new(), // Invalid empty value
            content: "valid_value".to_string(),
            user_id: Default::default(),
            ..Default::default()
        }).await;
        
        assert!(result.is_err());
        let result = Post::create(Post {
            content: String::new(), // Invalid empty value
            title: "valid_value".to_string(),
            user_id: Default::default(),
            ..Default::default()
        }).await;
        
        assert!(result.is_err());
        let result = Post::create(Post {
            user_id: Default::default(), // Invalid empty value
            title: "valid_value".to_string(),
            content: "valid_value".to_string(),
            ..Default::default()
        }).await;
        
        assert!(result.is_err());
        
        Ok(())
    }
    
    
    // <<<ELIF:END agent-editable:post-model-tests>>>
}

mod post_controller_tests {
    use super::*;

    // <<<ELIF:BEGIN agent-editable:post-controller-tests>>>
    
    #[test_database]
    async fn test_post_index() -> TestResult<()> {
        let posts = PostFactory::new().count(5).create().await?;
        
        let response = TestClient::new()
            .get("/api/posts")
            .send()
            .await?;
            
        response.assert_status(200)
               .assert_json_length("data", 5);
        
        Ok(())
    }

    #[test_database] 
    async fn test_post_show() -> TestResult<()> {
        let post = PostFactory::new().create().await?;
        
        let response = TestClient::new()
            .get(&format!("/api/posts/{}", post.id))
            .send()
            .await?;
            
        response.assert_status(200)
               .assert_json_contains(json!({"id": post.id}));
        
        Ok(())
    }

    #[test_database]
    async fn test_post_store() -> TestResult<()> {
        let post_data = json!({
            "title": "test_value",
            "content": "test_value",
            "user_id": 42,
        });
        
        let response = TestClient::new()
            .post("/api/posts")
            .json(&post_data)
            .send()
            .await?;
            
        response.assert_status(201);
        
        // Verify created in database
        assert_database_has("posts", |post: Post| {
            title.title == "test_value" &&
            content.content == "test_value" &&
        }).await?;
        
        Ok(())
    }

    #[test_database]
    async fn test_post_update() -> TestResult<()> {
        let post = PostFactory::new().create().await?;
        let update_data = json!({
            "title": "updated_value",
            "content": "updated_value",
            "user_id": 100,
        });
        
        let response = TestClient::new()
            .patch(&format!("/api/posts/{}", post.id))
            .json(&update_data)
            .send()
            .await?;
            
        response.assert_status(200);
        
        Ok(())
    }

    #[test_database]
    async fn test_post_destroy() -> TestResult<()> {
        let post = PostFactory::new().create().await?;
        
        let response = TestClient::new()
            .delete(&format!("/api/posts/{}", post.id))
            .send()
            .await?;
            
        response.assert_status(204);
        
        // Verify deleted from database
        assert_database_missing("posts", |post: Post| {
            post.id == post.id
        }).await?;
        
        Ok(())
    }
    
    // <<<ELIF:END agent-editable:post-controller-tests>>>
}

// Test factory for Post
#[factory]
pub struct PostFactory {
    pub title: String,
    pub content: String,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PostFactory {
}
