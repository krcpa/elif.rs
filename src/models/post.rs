use elif_orm::prelude::*;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[table_name = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub user_id: i32,
    
    #[timestamp]
    pub created_at: DateTime<Utc>,
    
    #[timestamp]
    pub updated_at: DateTime<Utc>,
}

impl Post {
    // <<<ELIF:BEGIN agent-editable:post-model-methods>>>
    
    // Add your custom model methods here
    
    // <<<ELIF:END agent-editable:post-model-methods>>>
}


#[cfg(test)]
mod tests {
    use super::*;
    use elif_testing::prelude::*;

    // <<<ELIF:BEGIN agent-editable:post-model-tests>>>
    
    #[test_database]
    async fn test_post_creation() {
        let post = Post {
            title: "test".to_string(),
            content: "test".to_string(),
            user_id: Default::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let saved_post = post.save().await.unwrap();
        assert_eq!(saved_post.id, post.id);
    }
    
    // <<<ELIF:END agent-editable:post-model-tests>>>
}
